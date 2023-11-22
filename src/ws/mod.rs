pub mod bridge;
pub mod user;

use tokio::time::Duration;

pub const WS_PING_INTERVAL: Duration = Duration::from_secs(50);
