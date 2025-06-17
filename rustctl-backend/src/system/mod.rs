use crate::core::{CrossTasksSharedState, coroutines::Coroutine, error::NonRecoverableError};
use std::{sync::Arc, time::Duration};
use tokio::{
    signal::unix::{SignalKind, signal},
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;

pub async fn monitor_usage(
    coroutine_identity: Coroutine,
    cancel: CancellationToken,
    _shutdown_tx: tokio::sync::mpsc::Sender<Coroutine>,
    interval: Duration,
    _shared: Arc<Mutex<CrossTasksSharedState>>,
) -> Result<(), NonRecoverableError> {
    let mut interval = tokio::time::interval(interval);
    loop {
        let is_cancelled: bool = cancel.is_cancelled();
        if is_cancelled {
            break;
        } else {
            interval.tick().await;
            /*
             * TODO: Read CPU & memory usage on a regular interval and write to
             *       the cross-tasks shared state
             */
        }
    }
    log::info!("Coroutine done: {coroutine_identity}");
    Ok(())
}

pub async fn wait_signal(
    _coroutine_identity: Coroutine,
    cancel: CancellationToken,
    mut shutdown_rx: tokio::sync::mpsc::Receiver<Coroutine>,
) -> Result<(), NonRecoverableError> {
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv() => log::info!("SIGINT"),
        _ = sigterm.recv() => log::info!("SIGTERM"),
        Some(coroutine) = shutdown_rx.recv() => log::info!("Shutdown requested by coroutine {coroutine}"),
    }
    cancel.cancel();
    Ok(())
}

pub mod fs {
    use std::path::{Path, PathBuf};
    use tokio::fs;

    pub async fn find_absolute_path(executable_name: &str) -> Option<PathBuf> {
        let path = Path::new(executable_name);
        if path.components().count() > 1 {
            if fs::metadata(path).await.ok()?.is_file() {
                return Some(path.canonicalize().ok()?);
            } else {
                return None;
            }
        }

        let raw_path = std::env::var_os("PATH")?;
        let path_str = raw_path.to_string_lossy();

        let dirs = if path_str.contains(':') {
            std::env::split_paths(&raw_path).collect::<Vec<_>>()
        } else {
            path_str
                .split_whitespace()
                .map(PathBuf::from)
                .collect::<Vec<_>>()
        };

        'paths: for dir in dirs {
            let full_path = dir.join(executable_name);
            let found_path = match fs::metadata(&full_path).await {
                Ok(n) => n,
                Err(_) => continue 'paths,
            };
            if found_path.is_file() {
                return Some(full_path);
            } else {
                continue 'paths;
            }
        }
        None
    }
}
