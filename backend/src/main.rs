mod init;

fn main() -> std::process::ExitCode {
    let cli_args: init::Cli = init::Cli::get();

    if let Err(code) = init::init_logger() {
        return code;
    }

    init::prepare_filesystem();

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
            use std::os::unix::fs::OpenOptionsExt;

            /*
             * Keep a read-end open for both FIFOs. This ensures that even if
             * the Service restarts, the Game process always sees at least one
             * reader and thus doesn't terminate when readers drop to 0.
             */
            let _keep_stdout_open: std::fs::File = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(nix::libc::O_NONBLOCK)
                .open(init::GAME_SERVER_FIFO_OUT)
                .expect("Failed to open dummy stdout reader");
            let _keep_stderr_open: std::fs::File = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(nix::libc::O_NONBLOCK)
                .open(init::GAME_SERVER_FIFO_ERR)
                .expect("Failed to open dummy stderr reader");

            let out_file: std::fs::File = std::fs::OpenOptions::new()
                .write(true)
                .open(init::GAME_SERVER_FIFO_OUT)
                .expect("Failed to open stdout FIFO");
            let err_file: std::fs::File = std::fs::OpenOptions::new()
                .write(true)
                .open(init::GAME_SERVER_FIFO_ERR)
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
            loop {
                let out_fifo: tokio::fs::File = tokio::fs::File::open(init::GAME_SERVER_FIFO_OUT)
                    .await
                    .expect("Failed to open stdout FIFO");
                let err_fifo: tokio::fs::File = tokio::fs::File::open(init::GAME_SERVER_FIFO_ERR)
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

                log::debug!("FIFO EOF reached, waiting for Game restart...");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }

    RtDone
}

struct RtDone;
