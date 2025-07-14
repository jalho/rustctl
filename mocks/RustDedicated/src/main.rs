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

    let _runtime_done = runtime.block_on(async {
        let summary = tokio::join!(launch_game(std::time::Duration::from_secs(5)));
        return summary;
    });

    std::process::ExitCode::FAILURE
}

async fn launch_game(duration: std::time::Duration) {
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
}
