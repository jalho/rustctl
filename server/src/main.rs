mod actors;
mod init;
mod storage;
mod util;

fn main() -> std::process::ExitCode {
    let cli_args: init::CliArgs = <init::CliArgs as clap::Parser>::parse();

    let _logg: log4rs::Handle = match init::initialize_logger(cli_args.log_level) {
        Ok(n) => n,
        Err(code) => return code,
    };
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
    );

    let config_shared: storage::GameServerConfigurationShared = storage::GameServerConfigurationShared::init();

    let runtime: tokio::runtime::Runtime = match init::build_runtime() {
        Ok(n) => n,
        Err(code) => return code,
    };

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
    let (tx_broadcast, _) = tokio::sync::broadcast::channel::<rustctl_common::snapshot::Snapshot>(1);

    /*
     * The actors.
     */
    let monitor = actors::monitor::Monitor::new(ctoken.child_token(), tx_activate.clone(), tx_resuse);
    let aggregator = actors::aggregator::Aggregator::new(
        ctoken.child_token(),
        tx_activate.clone(),
        rx_resuse,
        rx_gss,
        rx_command_collect,
        tx_cmd_relay,
        tx_broadcast.clone(),
    );
    let controller = actors::gsc::GameServerController::new(
        ctoken.child_token(),
        tx_activate.clone(),
        config_shared.clone(),
        rx_command_relay,
        tx_gss,
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
            web_server.work()
        )
    };
    let _runtime_done: (
        actors::terminator::Summary,
        actors::aggregator::Summary,
        actors::monitor::Summary,
        actors::gsc::Summary,
        actors::web_server::Summary,
    ) = runtime.block_on(runtime_job);

    std::process::ExitCode::SUCCESS
}
