use crate::{constants::INTERVAL_MONITOR_SYSTEM, core::SharedState};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(serde::Serialize, Clone)]
struct ResourceUsage {
    process_id: (),
    cpu: (),
    memory: (),
}

#[derive(serde::Serialize, Clone)]
pub struct SystemState {
    game_server: Option<ResourceUsage>,
    game_server_installer: Option<ResourceUsage>,
}

impl SystemState {
    pub fn read() -> Self {
        Self {
            game_server: None,
            game_server_installer: None,
        }
    }
}

pub async fn monitor_usage(shared: Arc<Mutex<SharedState>>) {
    let mut interval = tokio::time::interval(INTERVAL_MONITOR_SYSTEM);
    loop {
        interval.tick().await;

        // TODO: Read system CPU, network, memory usage etc.

        let usage = SystemState::read();

        {
            let mut shared = shared.lock().await;
            shared.system = usage;
        }
    }
}
