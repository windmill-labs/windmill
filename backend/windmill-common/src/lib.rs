pub mod error;
pub mod external_ip;
pub mod flows;
pub mod more_serde;
pub mod oauth2;
pub mod scripts;
pub mod users;
pub mod utils;
pub mod variables;
pub mod worker_flow;

pub const DEFAULT_NUM_WORKERS: usize = 3;
pub const DEFAULT_TIMEOUT: i32 = 300;
pub const DEFAULT_SLEEP_QUEUE: u64 = 50;
pub const DEFAULT_MAX_CONNECTIONS: u32 = 100;
