#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn get() -> Self {
        use clap::Parser;
        Self::parse()
    }
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Start the game server, reading startup parameters from storage and
    /// emitting STDOUT and STDERR to FIFO pipes. Not to be confused with the
    /// controlling and observability service that is run as a separate OS
    /// process. Terminating the game does not affect the running controlling &
    /// observability service.
    Game,

    /// Start the controlling & observability service that reads the outputs of
    /// a game server that is running as a separate OS process. Terminating the
    /// service does not affect the running game server.
    Service,
}

pub fn init_logger() -> Result<log4rs::Handle, std::process::ExitCode> {
    const APPENDER_NAME: &str = "stdout";

    let stdout = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{h({d(%H:%M:%S)(utc)} UTC [{l}] {m})} [{f}:{L}]\n",
        )))
        .build();

    let config: log4rs::Config = match log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build(APPENDER_NAME, Box::new(stdout)))
        .build(
            log4rs::config::Root::builder()
                .appender(APPENDER_NAME)
                .build(log::LevelFilter::max()),
        ) {
        Ok(n) => n,
        Err(err) => {
            eprintln!("{err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    let handle: log4rs::Handle = match log4rs::init_config(config) {
        Ok(n) => n,
        Err(err) => {
            eprintln!("{err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    Ok(handle)
}

pub const GAME_SERVER_FIFO_DIR: &str = "/tmp/rustctl";
pub const GAME_SERVER_FIFO_OUT: &str = "/tmp/rustctl/game-server.out";
pub const GAME_SERVER_FIFO_ERR: &str = "/tmp/rustctl/game-server.err";
pub fn prepare_filesystem() {
    let fifo_dir: &std::path::Path = std::path::Path::new(GAME_SERVER_FIFO_DIR);
    if !fifo_dir.exists() {
        std::fs::create_dir_all(fifo_dir).expect("Failed to create FIFO dir");
    }

    let fifos: [&str; 2] = [GAME_SERVER_FIFO_OUT, GAME_SERVER_FIFO_ERR];
    for fifo in fifos {
        let path: &std::path::Path = std::path::Path::new(fifo);
        if !path.exists() {
            let mode: nix::sys::stat::Mode = nix::sys::stat::Mode::S_IRUSR
                | nix::sys::stat::Mode::S_IWUSR
                | nix::sys::stat::Mode::S_IRGRP
                | nix::sys::stat::Mode::S_IWGRP;
            nix::unistd::mkfifo(path, mode).expect("Failed to create FIFO pipe");
        }
    }
}
