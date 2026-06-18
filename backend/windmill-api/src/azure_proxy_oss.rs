// OSS stub for Azure proxy functionality
// The actual implementation is in azure_proxy_ee.rs (Enterprise Edition)

#[cfg(all(feature = "private", feature = "parquet"))]
#[allow(unused)]
pub use crate::azure_proxy_ee::*;
