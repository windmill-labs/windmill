#[cfg(feature = "private")]
pub mod autoscaling_ee;
mod autoscaling_oss;
pub mod kubernetes_integration;
pub use autoscaling_oss::*;
pub use kubernetes_integration::{
    apply_kubernetes_autoscaling, KubernetesConfig, KubernetesIntegration,
};
