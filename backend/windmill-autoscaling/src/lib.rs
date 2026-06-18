#[cfg(feature = "private")]
pub mod autoscaling_ee;
mod autoscaling_oss;
pub use autoscaling_oss::*;
#[cfg(feature = "private")]
pub mod kubernetes_integration_ee;
#[cfg(feature = "private")]
pub use kubernetes_integration_ee::{apply_kubernetes_autoscaling, KubernetesIntegration};
