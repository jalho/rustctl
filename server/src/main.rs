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

    // actors's connectors

    // the actors
    let terminator: actors::terminator::Terminator = actors::terminator::Terminator::new();
    let aggregator: actors::aggregator::Aggregator = actors::aggregator::Aggregator::new();
    let res_usage_monitor: actors::monitor::ResUseMonitor = actors::monitor::ResUseMonitor::new();

    // let's go!
    let runtime_job = async { tokio::join!(terminator.work(), aggregator.work(), res_usage_monitor.work()) };
    let _runtime_done: (
        actors::terminator::Summary,
        actors::aggregator::Summary,
        actors::monitor::Summary,
    ) = runtime.block_on(runtime_job);

    return std::process::ExitCode::SUCCESS;
}
