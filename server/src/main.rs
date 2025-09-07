mod actors;
mod init;
mod steam;
mod storage;
mod util;

fn main() -> std::process::ExitCode {
    let cli_args: init::CliArgs = <init::CliArgs as clap::Parser>::parse();

    let config_client: storage::ConfigurationClient = storage::ConfigurationClient::init();

    let runtime: tokio::runtime::Runtime = match init::build_runtime() {
        Ok(n) => n,
        Err(code) => return code,
    };

    let config: storage::Configuration = runtime.block_on(config_client.get_config());
    let (_logg, log_file) = match init::initialize_logger(cli_args.log_level, &config) {
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

    /*
     * The actors.
     */
    let monitor = actors::monitor::Monitor::new(ctoken.child_token(), tx_activate.clone(), tx_resuse);
    let aggregator = actors::aggregator::Aggregator::new(
        ctoken.child_token(),
        rx_resuse,
        rx_gss,
        rx_igs,
        rx_command_collect,
        tx_cmd_relay,
        tx_broadcast.clone(),
        config.game_world_size.into(),
    );
    let controller = actors::gsc::GameServerController::new(
        ctoken.child_token(),
        tx_activate.clone(),
        cli_args.skip,
        config_client.clone(),
        rx_command_relay,
        tx_gss,
        tx_rconready,
    );
    /*
     * TODO: Add new responsibility for GameMonitor (previous called RconClient)
     *       actor:
     *
     *       Periodically (e.g., every 10 minutes) query the latest available game
     *       server build from SteamCMD. Compare this to the currently installed
     *       build ID (from the local manifest file). If a newer build is
     *       available and there are no players on the server, instruct the game
     *       server controller to stop and update and restart the game.
     *
     *       If this occurs on the first Thursday of the month, assume it
     *       is _the_ mandatory monthly ("forced") update. In that case, the
     *       command to the game server controller may include instructions to
     *       use a new map seed and either wipe or preserve players' blueprints.
     *
     *       The player check avoids unnecessary interruptions: during a forced
     *       update, clients cannot connect to outdated servers, so the server
     *       should naturally empty out. Likewise even if an update is not
     *       mandatory, we might as well install it if the server is empty.
     *
     *       Command for querying the latest available build ID (hallucinated by
     *       LLM, not tested!):
     *
     *       ```
     *       $ steamcmd +login anonymous +app_info_print 258550 +quit | grep -m 1 buildid
     *       ```
     */
    let game_monitor =
        actors::game_monitor::GameMonitor::new(ctoken.child_token(), config_client.clone(), tx_igs, rx_rconready);
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
        )
    };
    let _runtime_done: (
        actors::terminator::Summary,
        actors::aggregator::Summary,
        actors::monitor::Summary,
        actors::gsc::Summary,
        actors::game_monitor::Summary,
        actors::web_server::Summary,
    ) = runtime.block_on(runtime_job);

    std::process::ExitCode::SUCCESS
}
