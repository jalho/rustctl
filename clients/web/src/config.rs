#[cfg(debug_assertions)]
pub const BACKEND_URL: &str = "http://192.168.0.103:8080";
#[cfg(debug_assertions)]
pub const WS_URL: &str = "ws://192.168.0.103:8080";

#[cfg(not(debug_assertions))]
pub const BACKEND_URL: &str = "https://rustctl.internal";
#[cfg(not(debug_assertions))]
pub const WS_URL: &str = "wss://rustctl.internal";
