use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use kube::api::{Api, Patch, PatchParams};
use kube::runtime::controller::Action;
use kube::runtime::Controller;
use kube::{Client, ResourceExt};
use sqlx::{Pool, Postgres};

use crate::crd::{WindmillInstance, WindmillInstanceStatus};
use crate::db_sync;
use crate::resolve;

/// Shared state available to the reconciler.
struct Context {
    db: Pool<Postgres>,
    client: Client,
}

/// Run the operator controller loop. Blocks until shutdown.
pub async fn run(db: Pool<Postgres>) -> anyhow::Result<()> {
    let client = Client::try_default().await?;

    // Verify the CRD is installed by attempting to list
    let api: Api<WindmillInstance> = Api::all(client.clone());
    api.list(&Default::default()).await.map_err(|e| {
        anyhow::anyhow!("Failed to list WindmillInstance CRDs. Is the CRD installed? Error: {e}")
    })?;
    tracing::info!("WindmillInstance CRD verified, starting controller");

    let ctx = Arc::new(Context { db, client: client.clone() });

    Controller::new(api, Default::default())
        .shutdown_on_signal()
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok(o) => tracing::debug!("Reconciled: {:?}", o),
                Err(e) => tracing::error!("Reconcile error: {:?}", e),
            }
        })
        .await;

    tracing::info!("Operator controller shut down");
    Ok(())
}

/// Main reconciliation logic for a single WindmillInstance resource.
async fn reconcile(
    instance: Arc<WindmillInstance>,
    ctx: Arc<Context>,
) -> Result<Action, kube::Error> {
    let name = instance.name_any();
    let ns = instance.namespace().unwrap_or_default();
    tracing::info!("Reconciling WindmillInstance {name} in namespace {ns}");

    let generation = instance.metadata.generation.unwrap_or(0);

    // Resolve any secretKeyRef fields by reading K8s Secrets
    let resolved = match resolve::resolve_secret_refs(
        &ctx.client,
        &ns,
        &instance.spec.global_settings,
    )
    .await
    {
        Ok(gs) => gs,
        Err(e) => {
            tracing::error!("Failed to resolve secret refs for {name}: {e:#}");
            update_status(
                &ctx.client,
                &instance,
                false,
                format!("Error resolving secret references: {e}"),
                generation,
            )
            .await?;
            return Ok(Action::requeue(Duration::from_secs(30)));
        }
    };

    // Convert typed structs to BTreeMaps for db_sync
    let settings_map = resolved.to_settings_map();
    let configs_map: BTreeMap<String, serde_json::Value> = instance
        .spec
        .worker_configs
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                serde_json::to_value(v).expect("WorkerGroupConfig serialization cannot fail"),
            )
        })
        .collect();

    // Sync global settings
    if let Err(e) = db_sync::sync_global_settings(&ctx.db, &settings_map).await {
        tracing::error!("Failed to sync global settings for {name}: {e:#}");
        update_status(
            &ctx.client,
            &instance,
            false,
            format!("Error syncing global settings: {e}"),
            generation,
        )
        .await?;
        // Requeue after 30s on error
        return Ok(Action::requeue(Duration::from_secs(30)));
    }

    // Sync worker configs
    if let Err(e) = db_sync::sync_worker_configs(&ctx.db, &configs_map).await {
        tracing::error!("Failed to sync worker configs for {name}: {e:#}");
        update_status(
            &ctx.client,
            &instance,
            false,
            format!("Error syncing worker configs: {e}"),
            generation,
        )
        .await?;
        return Ok(Action::requeue(Duration::from_secs(30)));
    }

    tracing::info!("Successfully synced WindmillInstance {name}");
    update_status(
        &ctx.client,
        &instance,
        true,
        "Synced successfully".to_string(),
        generation,
    )
    .await?;

    // Periodic re-sync every 5 minutes for drift detection
    Ok(Action::requeue(Duration::from_secs(300)))
}

/// Error policy: requeue after 60 seconds on unhandled errors.
fn error_policy(
    _instance: Arc<WindmillInstance>,
    _error: &kube::Error,
    _ctx: Arc<Context>,
) -> Action {
    Action::requeue(Duration::from_secs(60))
}

/// Patch the status subresource of the WindmillInstance.
async fn update_status(
    client: &Client,
    instance: &WindmillInstance,
    synced: bool,
    message: String,
    observed_generation: i64,
) -> Result<(), kube::Error> {
    let name = instance.name_any();
    let ns = instance.namespace().unwrap_or_default();
    let api: Api<WindmillInstance> = Api::namespaced(client.clone(), &ns);

    let now = chrono::Utc::now().to_rfc3339();
    let status = WindmillInstanceStatus {
        synced,
        message,
        observed_generation,
        last_synced_at: if synced { Some(now) } else { None },
    };

    let patch = serde_json::json!({ "status": status });
    api.patch_status(
        &name,
        &PatchParams::apply("windmill-operator"),
        &Patch::Merge(&patch),
    )
    .await?;

    Ok(())
}
