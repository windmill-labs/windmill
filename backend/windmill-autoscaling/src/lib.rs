#[cfg(feature = "private")]
pub mod autoscaling_ee;
mod autoscaling_oss;
pub use autoscaling_oss::*;
