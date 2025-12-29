mod init;

fn main() -> std::process::ExitCode {
    let _cli_args: init::Cli = init::Cli::get();

    if let Err(code) = init::init_logger() {
        return code;
    }

    let mut rt_builder: tokio::runtime::Builder = tokio::runtime::Builder::new_current_thread();
    rt_builder.enable_time();

    let rt: tokio::runtime::Runtime = match rt_builder.build() {
        Ok(n) => n,
        Err(err) => {
            log::error!("Failed to build runtime: {err}");
            return std::process::ExitCode::from(42);
        }
    };
    log::debug!("Runtime built");

    let _rt_done: Thing = rt.block_on(do_a_thing());
    log::debug!("Runtime done with async tasks");

    log::debug!("Terminating");
    std::process::ExitCode::SUCCESS
}

async fn do_a_thing() -> Thing {
    let _slept: () = tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    Thing
}

struct Thing;
