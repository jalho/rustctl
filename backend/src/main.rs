mod game;
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
         * Spawn the game server. Write the game server's STDOUT and STDERR to
         * FIFO pipes.
         */
        init::Command::Game => {
            game::spawn("RustDedicated").await;
        }

        /*
         * Log the outputs of a game server running as a separate OS process by
         * reading from FIFO pipes.
         */
        init::Command::Service => {
            game::log_game_server_output().await;
        }
    }

    RtDone
}

struct RtDone;
