use std::time::Duration;

pub const ADDR_WEB_SERVICE_LISTEN: &str = "0.0.0.0:8080";

pub const INTERVAL_MONITOR_SYSTEM: Duration = Duration::from_millis(500);

pub const INTERVAL_FETCH_GAME_STATE: Duration = Duration::from_millis(200);

pub const INTERVAL_SYNC_CLIENT: Duration = Duration::from_millis(200);

pub const URL_PATH_WEBSOCKET_CONNECT: &str = "/sock";

pub const MESSAGES_PER_CLIENT_INMEM_MAX: usize = 16;
