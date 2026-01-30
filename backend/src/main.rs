mod ctl;
mod game;
mod init;
mod rcon;
mod web;

fn main() -> std::process::ExitCode {
    let cli_args: init::Cli = init::Cli::get();

    if let Err(code) = init::init_logger(log::LevelFilter::Debug) {
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
    let params: game::GameServerParameters = game::GameServerParameters::default();

    match cli_args.command {
        /*
         * Spawn the game server. Write the game server's STDOUT and STDERR to
         * FIFO pipes.
         */
        init::Command::Game => {
            game::install_and_spawn_game_server(&params).await;
        }

        /*
         * Log the outputs of a game server running as a separate OS process by
         * reading from FIFO pipes.
         */
        init::Command::Service => {
            /*
             * Channel for passing commands from web clients to a Controller
             * (see module `ctl`).
             */
            let (tx, rx) = tokio::sync::mpsc::channel::<ctl::Command>(1);
            let addr: std::net::SocketAddr = "0.0.0.0:8080".parse().unwrap();

            /*
             * Each task here is supposed to run indefinitely. Therefore, any
             * premature termination should be logged as an ERROR, and in any
             * case the whole program should terminate.
             */
            tokio::select! {
                _ = game::log_game_server_output() => {
                    log::error!("Task terminated: game::log_game_server_output");
                }
                _ = web::serve(addr, tx) => {
                    log::error!("Task terminated: web::serve");
                }
                _ = ctl::handle_commands_from_web_clients(rx) => {
                    log::error!("Task terminated: ctl::handle_commands_from_web_clients");
                }
                _ = rcon::relay(&params) => {
                    log::error!("Task terminated: rcon::relay");
                }
            }
        }
    }

    RtDone
}

struct RtDone;
