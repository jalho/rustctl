mod actors;
mod init;

fn main() -> std::process::ExitCode {
    let cli_args: init::CliArgs = <init::CliArgs as clap::Parser>::parse();

    let _logg: log4rs::Handle = init::initialize_logger(cli_args.log_level);
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
    );

    let runtime: tokio::runtime::Runtime = match init::build_runtime() {
        Ok(n) => n,
        Err(code) => return code,
    };

    /*
     * Actors's connectors.
     */
    let ctoken: tokio_util::sync::CancellationToken = tokio_util::sync::CancellationToken::new();
    let (tx_activate, rx_activate): (
        tokio::sync::mpsc::Sender<actors::terminator::Activator>,
        tokio::sync::mpsc::Receiver<actors::terminator::Activator>,
    ) = tokio::sync::mpsc::channel::<actors::terminator::Activator>(1);

    /*
     * The actors.
     */
    let aggregator = actors::aggregator::Aggregator::new(ctoken.child_token(), tx_activate.clone());
    let monitor = actors::monitor::Monitor::new(ctoken.child_token(), tx_activate.clone());
    let terminator = actors::terminator::Terminator::new(ctoken, rx_activate);

    /*
     * Let's go!
     */
    let runtime_job = async { tokio::join!(terminator.work(), aggregator.work(), monitor.work()) };
    let _runtime_done: (
        actors::terminator::Summary,
        actors::aggregator::Summary,
        actors::monitor::Summary,
    ) = runtime.block_on(runtime_job);

    return std::process::ExitCode::SUCCESS;
}
