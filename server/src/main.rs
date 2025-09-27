mod actors;
mod data;
mod init;
mod steam;
mod util;

fn main() -> std::process::ExitCode {
    let cli_args: init::CliArgs = match init::CliArgs::parse() {
        Ok(n) => n,
        Err(code) => return code,
    };

    let runtime: tokio::runtime::Runtime = match init::build_runtime() {
        Ok(n) => n,
        Err(code) => return code,
    };

    let (_logg, log_file) = match init::initialize_logger(cli_args.log_level) {
        Ok(n) => n,
        Err(code) => return code,
    };
    log::info!(
        r#"{name} {version} -- Logs: "{log_file}""#,
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
    );

    /*
     * Actors's connectors.
     */
    let ctoken: tokio_util::sync::CancellationToken = tokio_util::sync::CancellationToken::new();
    let (tx_activate, rx_activate) = tokio::sync::mpsc::channel::<actors::terminator::Activator>(1);
    let (tx_resuse, rx_resuse) = tokio::sync::mpsc::channel::<actors::monitor::SystemResourceUsageReading>(1);
    let (tx_cmd_collect, rx_command_collect) =
        tokio::sync::mpsc::channel::<rustctl_common::command::DownstreamClientMessage>(1);
    let (tx_cmd_relay, rx_command_relay) =
        tokio::sync::mpsc::channel::<rustctl_common::command::DownstreamClientMessage>(1);
    let (tx_gss, rx_gss) = tokio::sync::mpsc::channel::<rustctl_common::snapshot::GameServerStateExposed>(1);
    let (tx_igs, rx_igs) = tokio::sync::mpsc::channel::<rustctl_common::snapshot::InGameStateExposed>(1);
    let (tx_broadcast, _) = tokio::sync::broadcast::channel::<rustctl_common::BroadcastMessage>(1);
    let (tx_rconready, rx_rconready) = tokio::sync::mpsc::channel::<actors::gsc::gssm::ReadyForRcon>(1);
    let (tx_buildid, rx_buildid) = tokio::sync::mpsc::channel::<actors::game_monitor::GameBuildIDUpdate>(1);
    let (tx_query, rx_query) = tokio::sync::mpsc::channel::<actors::database::client::Query>(1);

    /*
     * The actors.
     */
    let database: actors::database::Database = match actors::database::Database::init_connect(
        ctoken.child_token(),
        &cli_args.populate_privileged_users,
        rx_query,
    ) {
        Ok(n) => n,
        Err(code) => return code,
    };
    let monitor = actors::monitor::Monitor::new(ctoken.child_token(), tx_activate.clone(), tx_resuse);
    let aggregator = actors::aggregator::Aggregator::new(
        ctoken.child_token(),
        rx_resuse,
        rx_gss,
        rx_igs,
        rx_command_collect,
        tx_cmd_relay,
        tx_broadcast.clone(),
        actors::database::client::Client::new(tx_query.clone()),
    );
    let controller = actors::gsc::GameServerController::new(
        ctoken.child_token(),
        tx_activate.clone(),
        cli_args.skip,
        actors::database::client::Client::new(tx_query.clone()),
        rx_command_relay,
        tx_gss,
        tx_rconready,
        rx_buildid,
    );
    let game_monitor = actors::game_monitor::GameMonitor::new(
        ctoken.child_token(),
        actors::database::client::Client::new(tx_query.clone()),
        tx_igs,
        rx_rconready,
        tx_buildid,
        !cli_args.skip,
    );
    let web_server = actors::web_server::WebServer::new(
        ctoken.child_token(),
        tx_activate.clone(),
        (cli_args.web_server_listen_ip_addr, cli_args.web_server_listen_port),
        tx_cmd_collect,
        tx_broadcast,
    );
    let terminator = actors::terminator::Terminator::new(ctoken, rx_activate);

    /*
     * Let's go!
     */
    let runtime_job = async {
        tokio::join!(
            terminator.work(),
            aggregator.work(),
            monitor.work(),
            controller.work(),
            game_monitor.work(),
            web_server.work(),
            database.work(),
        )
    };
    let _runtime_done: (
        actors::terminator::Summary,
        actors::aggregator::Summary,
        actors::monitor::Summary,
        actors::gsc::Summary,
        actors::game_monitor::Summary,
        actors::web_server::Summary,
        actors::database::Summary,
    ) = runtime.block_on(runtime_job);

    std::process::ExitCode::SUCCESS
}
