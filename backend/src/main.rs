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
    match cli_args.command {
        /*
         * Spawn the game server, i.e. an executable named `RustDedicated`.
         *
         * Write the game server's STDOUT and STDERR to FIFO pipes.
         */
        init::Command::Game => {
            fifo::ensure_fifos_exist().await;

            /*
             * Keep a read-end open for both FIFOs. This ensures that even if
             * the Service restarts, the Game process always sees at least one
             * reader and thus doesn't terminate when readers drop to 0.
             */
            use std::os::unix::fs::OpenOptionsExt;
            let _keep_stdout_open: std::fs::File = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(nix::libc::O_NONBLOCK)
                .open(fifo::GAME_SERVER_FIFO_OUT)
                .unwrap();
            let _keep_stderr_open: std::fs::File = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(nix::libc::O_NONBLOCK)
                .open(fifo::GAME_SERVER_FIFO_ERR)
                .unwrap();

            let out_file: std::fs::File = std::fs::OpenOptions::new()
                .write(true)
                .open(fifo::GAME_SERVER_FIFO_OUT)
                .unwrap();
            let err_file: std::fs::File = std::fs::OpenOptions::new()
                .write(true)
                .open(fifo::GAME_SERVER_FIFO_ERR)
                .unwrap();

            let mut child: tokio::process::Child = tokio::process::Command::new("RustDedicated")
                .stdout(std::process::Stdio::from(out_file))
                .stderr(std::process::Stdio::from(err_file))
                .spawn()
                .expect("Failed to spawn game server");

            let _status: std::process::ExitStatus = child.wait().await.unwrap();
        }

        /*
         * Log the outputs of a game server running as a separate OS process:
         * Read from FIFO pipes.
         */
        init::Command::Service => {
            fifo::log_game_server_output().await;
        }
    }

    RtDone
}

struct RtDone;

mod fifo {
    const GAME_SERVER_FIFO_DIR: &str = "/tmp/rustctl";
    pub const GAME_SERVER_FIFO_OUT: &str = "/tmp/rustctl/game-server.out";
    pub const GAME_SERVER_FIFO_ERR: &str = "/tmp/rustctl/game-server.err";

    pub async fn ensure_fifos_exist() {
        let fifo_dir: &std::path::Path = std::path::Path::new(GAME_SERVER_FIFO_DIR);
        if !fifo_dir.exists() {
            tokio::fs::create_dir_all(fifo_dir).await.unwrap();
        }

        let fifos: [&str; 2] = [GAME_SERVER_FIFO_OUT, GAME_SERVER_FIFO_ERR];
        for fifo in fifos {
            let path: &std::path::Path = std::path::Path::new(fifo);
            if !path.exists() {
                let mode: nix::sys::stat::Mode = nix::sys::stat::Mode::S_IRUSR
                    | nix::sys::stat::Mode::S_IWUSR
                    | nix::sys::stat::Mode::S_IRGRP
                    | nix::sys::stat::Mode::S_IWGRP;
                nix::unistd::mkfifo(path, mode).unwrap();
            }
        }
    }

    pub async fn log_game_server_output() {
        ensure_fifos_exist().await;

        loop {
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
                            // EOF reached: break inner loop to re-open FIFOs
                            _ => break,
                        }
                    }
                    res = err_lines.next_line() => {
                        match res {
                            Ok(Some(line)) => log::debug!("[STDERR] {}", line),
                            // EOF reached: break inner loop to re-open FIFOs
                            _ => break,
                        }
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}
