mod init;

fn main() -> std::process::ExitCode {
    let cli_args: init::Cli = init::Cli::get();

    if let Err(code) = init::init_logger() {
        return code;
    }

    let mut rt_builder: tokio::runtime::Builder = tokio::runtime::Builder::new_current_thread();
    rt_builder.enable_time();
    rt_builder.enable_io();

    let rt: tokio::runtime::Runtime = match rt_builder.build() {
        Ok(n) => n,
        Err(err) => {
            log::error!("Failed to build runtime: {err}");
            return std::process::ExitCode::from(42);
        }
    };
    log::debug!("Runtime built");

    let _rt_done: RtDone = rt.block_on(async_tasks(&cli_args));
    log::debug!("Runtime done with async tasks");

    log::debug!("Terminating");
    std::process::ExitCode::SUCCESS
}

async fn async_tasks(cli_args: &init::Cli) -> RtDone {
    const GAME_SERVER_FIFO_OUT: &str = "/tmp/rustctl/game-server.out";
    const GAME_SERVER_FIFO_ERR: &str = "/tmp/rustctl/game-server.err";

    let fifo_dir: &std::path::Path = std::path::Path::new("/tmp/rustctl");
    if !fifo_dir.exists() {
        std::fs::create_dir_all(fifo_dir).expect("Failed to create /tmp/rustctl");
    }

    let fifos: [&str; 2] = [GAME_SERVER_FIFO_OUT, GAME_SERVER_FIFO_ERR];
    for fifo in fifos {
        let path: &std::path::Path = std::path::Path::new(fifo);
        if !path.exists() {
            let mode: nix::sys::stat::Mode = nix::sys::stat::Mode::S_IRUSR
                | nix::sys::stat::Mode::S_IWUSR
                | nix::sys::stat::Mode::S_IRGRP
                | nix::sys::stat::Mode::S_IWGRP;
            nix::unistd::mkfifo(path, mode).expect("Failed to create FIFO");
        }
    }

    match cli_args.command {
        /*
         * Spawn the game server, i.e. an executable named `RustDedicated`.
         *
         * Write the game server's STDOUT and STDERR to FIFO pipes.
         */
        init::Command::Game => {
            let out_file: std::fs::File = std::fs::OpenOptions::new()
                .write(true)
                .open(GAME_SERVER_FIFO_OUT)
                .expect("Failed to open stdout FIFO");
            let err_file: std::fs::File = std::fs::OpenOptions::new()
                .write(true)
                .open(GAME_SERVER_FIFO_ERR)
                .expect("Failed to open stderr FIFO");

            let mut child: tokio::process::Child = tokio::process::Command::new("RustDedicated")
                .stdout(std::process::Stdio::from(out_file))
                .stderr(std::process::Stdio::from(err_file))
                .spawn()
                .expect("Failed to spawn game server");

            let _status: std::process::ExitStatus =
                child.wait().await.expect("Child process failed");
        }

        /*
         * Log the outputs of a game server running as a separate OS process:
         * Read from FIFO pipes.
         */
        init::Command::Service => {
            let out_fifo: tokio::fs::File = tokio::fs::File::open(GAME_SERVER_FIFO_OUT)
                .await
                .expect("Failed to open stdout FIFO");
            let err_fifo: tokio::fs::File = tokio::fs::File::open(GAME_SERVER_FIFO_ERR)
                .await
                .expect("Failed to open stderr FIFO");

            let out_reader: tokio::io::BufReader<tokio::fs::File> =
                tokio::io::BufReader::new(out_fifo);
            let err_reader: tokio::io::BufReader<tokio::fs::File> =
                tokio::io::BufReader::new(err_fifo);

            let mut out_lines: tokio::io::Lines<tokio::io::BufReader<tokio::fs::File>> =
                tokio::io::AsyncBufReadExt::lines(out_reader);
            let mut err_lines: tokio::io::Lines<tokio::io::BufReader<tokio::fs::File>> =
                tokio::io::AsyncBufReadExt::lines(err_reader);

            loop {
                tokio::select! {
                    res = out_lines.next_line() => {
                        match res {
                            Ok(Some(line)) => log::debug!("[STDOUT] {}", line),
                            _ => break,
                        }
                    }
                    res = err_lines.next_line() => {
                        match res {
                            Ok(Some(line)) => log::debug!("[STDERR] {}", line),
                            _ => break,
                        }
                    }
                }
            }
        }
    }

    RtDone
}

struct RtDone;
