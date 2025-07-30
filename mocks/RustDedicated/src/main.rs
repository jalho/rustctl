fn main() -> std::process::ExitCode {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
    {
        Ok(n) => n,
        Err(err) => {
            eprintln!("failed to build async runtime: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let runtime_done: std::process::ExitCode = runtime.block_on(async {
        let summary = tokio::select!(
            code = launch_game(std::time::Duration::from_secs(20)) => code,
            code = wait_signal() => code,
        );
        summary
    });

    runtime_done
}

async fn launch_game(duration: std::time::Duration) -> std::process::ExitCode {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    println!("SteamServer Initialized");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    println!("Server startup complete");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    println!("SteamServer Connected");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    println!("Sleeping {duration:?}");
    tokio::time::sleep(duration).await;
    println!("Done sleeping");

    std::process::ExitCode::FAILURE
}

async fn wait_signal() -> std::process::ExitCode {
    let mut sigint =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv() => {
            println!("SIGINT");

            println!("Sleeping a bit...");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            println!("Done!");
        },
        _ = sigterm.recv() => {
            println!("SIGTERM");

            println!("Sleeping a bit...");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            println!("Done!");
        },
    };
    std::process::ExitCode::SUCCESS
}
