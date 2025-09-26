#[cfg(debug_assertions)]
pub const BACKEND_URL: &str = "http://rustctl.internal:8080";
#[cfg(debug_assertions)]
pub const WS_URL: &str = "ws://rustctl.internal:8080";

#[cfg(not(debug_assertions))]
pub const BACKEND_URL: &str = "https://rustctl.internal";
#[cfg(not(debug_assertions))]
pub const WS_URL: &str = "wss://rustctl.internal";
