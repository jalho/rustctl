mod game;
mod init;
mod web;

fn main() -> std::process::ExitCode {
    let cli_args: init::Cli = init::Cli::get();

    if let Err(code) = init::init_logger() {
        return code;
    }

    log::info!("rustctl v{}", env!("CARGO_PKG_VERSION"));

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
            let (tx, rx) = tokio::sync::mpsc::channel::<ctl::Command>(1);

            tokio::select! {
                _ = game::log_game_server_output() => {
                    log::debug!("Task terminated: game::log_game_server_output");
                }
                _ = web::serve(("0.0.0.0", 8080), tx) => {
                    log::debug!("Task terminated: web::serve");
                }
                _ = ctl::handle_commands_from_web_clients(rx) => {
                    log::debug!("Task terminated: ctl::handle_commands_from_web_clients");
                }
            }
        }
    }

    RtDone
}

struct RtDone;

mod ctl {
    pub enum Command {
        Reboot,
    }

    pub async fn handle_commands_from_web_clients(mut rx: tokio::sync::mpsc::Receiver<Command>) {
        loop {
            if let Some(n) = rx.recv().await {
                match n {
                    Command::Reboot => reboot().await,
                }
            }
        }
    }

    async fn reboot() {
        log::debug!("TODO: Reboot");
    }
}
