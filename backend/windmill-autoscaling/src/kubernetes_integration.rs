use anyhow::Result;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::{Api, Patch, PatchParams},
    config::{Config, KubeConfigOptions, Kubeconfig},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};
use windmill_common::DB;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesConfig {
    pub worker_group: String,
    pub namespace: Option<String>,
    // pub service_account: Option<String>,
    pub kubeconfig_path: Option<String>,
}

impl Default for KubernetesConfig {
    fn default() -> Self {
        Self {
            worker_group: "default".to_string(),
            namespace: Some("windmill".to_string()),
            kubeconfig_path: None,
        }
    }
}

pub struct KubernetesIntegration {
    client: Client,
    config: KubernetesConfig,
}

impl KubernetesIntegration {
    pub async fn new(config: KubernetesConfig) -> Result<Self> {
        let client = Self::create_client(&config).await?;

        Ok(Self { client, config })
    }

    async fn create_client(config: &KubernetesConfig) -> Result<Client> {
        let kube_config = if let Some(kubeconfig_path) = &config.kubeconfig_path {
            // Load from custom kubeconfig path
            let options = KubeConfigOptions { context: None, cluster: None, user: None };
            Config::from_custom_kubeconfig(Kubeconfig::read_from(kubeconfig_path)?, &options)
                .await?
        } else {
            // Try in-cluster config first, then default kubeconfig
            match Config::incluster() {
                Ok(config) => config,
                Err(_) => {
                    info!("Failed to load in-cluster config, trying default kubeconfig");
                    Config::infer().await?
                }
            }
        };

        Ok(Client::try_from(kube_config)?)
    }

    pub async fn scale_deployment(&self, desired_replicas: u16) -> Result<()> {
        let namespace = self.config.namespace.as_deref().unwrap_or("windmill");
        let scale_api: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);

        // info!(
        //     "Scaling deployment '{}' in namespace '{}' to {} replicas",
        //     self.config.deployment_name, namespace, desired_replicas
        // );

        // Create patch for scaling
        let scale_patch = json!({
            "spec": {
                "replicas": desired_replicas
            }
        });

        match scale_api
            .patch(
                &format!("windmill-workers-{}", self.config.worker_group),
                &PatchParams::apply("windmill-native-auto-scaler"),
                &Patch::Strategic(scale_patch),
            )
            .await
        {
            Ok(deployment) => {
                info!(
                    "Successfully scaled deployment '{:?}' to {} replicas",
                    self.config.namespace,
                    deployment
                        .spec
                        .as_ref()
                        .and_then(|s| s.replicas)
                        .unwrap_or(0)
                );
                Ok(())
            }
            Err(e) => {
                warn!(
                    "Failed to scale deployment '{:?}': {}",
                    self.config.namespace, e
                );
                Err(e.into())
            }
        }
    }

    // pub async fn get_current_replicas(&self) -> Result<u32> {
    //     let namespace = self.config.namespace.as_deref().unwrap_or("default");
    //     let deployment_api: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);

    //     let deployment = deployment_api.get(&self.config.deployment_name).await?;

    //     let replicas = deployment
    //         .spec
    //         .as_ref()
    //         .and_then(|s| s.replicas)
    //         .unwrap_or(0) as u32;

    //     Ok(replicas)
    // }

    // pub async fn validate_connection(&self) -> Result<()> {
    //     let namespace = self.config.namespace.as_deref().unwrap_or("default");
    //     let deployment_api: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);

    //     // Try to get the deployment to validate connection and permissions
    //     match deployment_api.get(&self.config.deployment_name).await {
    //         Ok(_) => {
    //             info!(
    //                 "Successfully validated connection to deployment '{}' in namespace '{}'",
    //                 self.config.deployment_name, namespace
    //             );
    //             Ok(())
    //         }
    //         Err(e) => {
    //             warn!(
    //                 "Failed to validate connection to deployment '{}' in namespace '{}': {}",
    //                 self.config.deployment_name, namespace, e
    //             );
    //             Err(e.into())
    //         }
    //     }
    // }
}

/// Apply kubernetes autoscaling for a specific worker group
/// This function is intended to be called from the main autoscaling logic
pub async fn apply_kubernetes_autoscaling(
    worker_group: &str,
    // current_workers: u32,
    desired_workers: u16,
    kubernetes_config: &KubernetesConfig,
    _db: &DB, // For logging autoscaling events
) -> Result<()> {
    // if current_workers == desired_workers {
    //     info!(
    //         "Worker group '{}': no scaling needed (current: {}, desired: {})",
    //         worker_group_name, current_workers, desired_workers
    //     );
    //     return Ok(());
    // }

    // info!(
    //     "Worker group '{}': scaling from {} to {} workers via Kubernetes",
    //     worker_group_name, current_workers, desired_workers
    // );

    let k8s_integration = KubernetesIntegration::new(kubernetes_config.clone()).await?;

    // Validate connection before attempting to scale
    // k8s_integration.validate_connection().await?;

    // Apply the scaling
    k8s_integration.scale_deployment(desired_workers).await?;

    info!(
        "Successfully scaled worker group '{}' to {} workers",
        worker_group, desired_workers
    );

    Ok(())
}
