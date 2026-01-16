pub const GAME_SERVER_FIFO_OUT: &str = "/tmp/rustctl/game-server.out";
pub const GAME_SERVER_FIFO_ERR: &str = "/tmp/rustctl/game-server.err";
const FIFO_DIR: &str = "/tmp/rustctl";

struct FifoPair {
    stdout: tokio::fs::File,
    stderr: tokio::fs::File,
}

impl FifoPair {
    async fn prepare_filesystem() {
        tokio::fs::create_dir_all(FIFO_DIR).await.unwrap();

        for path in [GAME_SERVER_FIFO_OUT, GAME_SERVER_FIFO_ERR] {
            if !std::path::Path::new(path).exists() {
                match nix::unistd::mkfifo(
                    path,
                    nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
                ) {
                    Ok(_) => log::debug!("Created FIFO: {}", path),
                    Err(nix::errno::Errno::EEXIST) => {}
                    Err(e) => panic!("Failed to create FIFO {}: {}", path, e),
                }
            }
        }
    }

    async fn open_with_options(options: &tokio::fs::OpenOptions) -> Self {
        Self::prepare_filesystem().await;
        Self {
            stdout: options.open(GAME_SERVER_FIFO_OUT).await.unwrap(),
            stderr: options.open(GAME_SERVER_FIFO_ERR).await.unwrap(),
        }
    }

    async fn open_for_reading() -> Self {
        Self::open_with_options(tokio::fs::OpenOptions::new().read(true)).await
    }

    async fn open_for_writing() -> Self {
        Self::open_with_options(tokio::fs::OpenOptions::new().write(true)).await
    }

    async fn open_dummy_handles() -> Self {
        let mut options = tokio::fs::OpenOptions::new();
        options.read(true).custom_flags(nix::libc::O_NONBLOCK);
        Self::open_with_options(&options).await
    }
}

pub async fn log_game_server_output() {
    loop {
        let fifos = FifoPair::open_for_reading().await;

        let out_reader = tokio::io::BufReader::new(fifos.stdout);
        let err_reader = tokio::io::BufReader::new(fifos.stderr);

        let mut out_lines = tokio::io::AsyncBufReadExt::lines(out_reader);
        let mut err_lines = tokio::io::AsyncBufReadExt::lines(err_reader);

        loop {
            tokio::select! {
                res = out_lines.next_line() => {
                    match res {
                        Ok(Some(line)) => log::debug!("[STDOUT] {}", line),
                        /* EOF reached: break inner loop to re-open FIFOs */
                        _ => break,
                    }
                }
                res = err_lines.next_line() => {
                    match res {
                        Ok(Some(line)) => log::debug!("[STDERR] {}", line),
                        /* EOF reached: break inner loop to re-open FIFOs */
                        _ => break,
                    }
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

pub async fn spawn(executable: &str) -> () {
    /*
     * Keep a read-end open for both FIFOs. This ensures that even if
     * the Service restarts, the Game process always sees at least one
     * reader and thus doesn't terminate when readers drop to 0.
     */
    let _dummies = FifoPair::open_dummy_handles().await;

    let fifos = FifoPair::open_for_writing().await;

    let stdout_std = fifos.stdout.into_std().await;
    let stderr_std = fifos.stderr.into_std().await;

    let mut child: tokio::process::Child = tokio::process::Command::new(executable)
        .stdout(stdout_std)
        .stderr(stderr_std)
        .spawn()
        .expect("Failed to spawn game server");
    let _status: std::process::ExitStatus = child.wait().await.unwrap();
}
