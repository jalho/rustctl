mod ctl;
mod database;
mod game;
mod init;
mod rcon;
mod web;

fn main() -> std::process::ExitCode {
    /*
     * Terminate on panic. This is to prevent the async runtime from continuing
     * with other tasks when a panic occurs in one of them.
     */
    std::panic::set_hook(Box::new(|panic_info| {
        log::error!("{panic_info}");
        std::process::exit(1);
    }));

    let cli_args: init::Cli = init::Cli::get();

    if let Err(code) = init::init_logger(log::LevelFilter::max()) {
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

    let _rt_done: RtDone = rt.block_on(async_tasks(&cli_args));

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
            let (mut db_engine, db_client): (database::Engine, database::Client) =
                database::Engine::new();

            /*
             * Channel for passing commands from web clients to a Controller
             * (see module `ctl`).
             */
            let (tx_cmd, rx_cmd) = tokio::sync::mpsc::channel::<ctl::Command>(1);
            let (tx_cmd_rws, rx_cmd_rws) = tokio::sync::mpsc::channel::<ctl::CommandRWS>(1);

            let expose: web::Expose = if cfg!(debug_assertions) {
                web::Expose::LocalLoopback
            } else {
                web::Expose::Any
            };

            /*
             * Each task here is supposed to run indefinitely. Therefore, any
             * premature termination should be logged as an ERROR, and in any
             * case the whole program should terminate.
             */
            tokio::select! {
                _ = db_engine.keep_connected() => {
                    log::error!("Task terminated: db_engine.keep_connected");
                }
                _ = game::log_game_server_output() => {
                    log::error!("Task terminated: game::log_game_server_output");
                }
                _ = web::serve(&expose, tx_cmd, db_client.clone(), std::sync::Arc::new(tokio::sync::Mutex::new(rx_cmd_rws))) => {
                    log::error!("Task terminated: web::serve");
                }
                _ = ctl::handle_commands_from_web_clients(rx_cmd, tx_cmd_rws) => {
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

/*
 * TODO: Use `fn get_full_error_message` everywhere!
 */
fn get_full_error_message(err: &dyn std::error::Error) -> String {
    let mut message = err.to_string();
    let mut current = err.source();

    while let Some(cause) = current {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        current = cause.source();
    }

    message
}
