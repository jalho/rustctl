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
        let summary = tokio::join!(sleep(std::time::Duration::from_secs(3)));
        return summary;
    });

    std::process::ExitCode::SUCCESS
}

async fn sleep(duration: std::time::Duration) {
    println!("Sleeping {duration:?}...");
    tokio::time::sleep(duration).await;
    println!("Done sleeping");
}
