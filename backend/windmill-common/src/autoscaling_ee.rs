use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::types::Json;
use sqlx::FromRow;

use crate::queue::get_queue_counts;
use crate::DB;

#[derive(Debug, Serialize, Deserialize)]
struct AutoscalingConfig {
    enabled: bool,
    min_workers: u16,
    max_workers: u16,
    cooldown_seconds: Option<u16>,
    inc_scale_num_jobs_waiting: Option<u16>,
    full_scale_cooldown_seconds: Option<u16>,
    full_scale_jobs_waiting: Option<usize>,
    dec_scale_occupancy_rate: Option<u8>, // occupancy rate of 30s, 5m, 30m to scale down
    inc_scale_occupancy_rate: Option<u8>, // occupancy rate of 30s, 5m to scale up
    inc_percent: Option<usize>,
    integration: Option<AutoscalingIntegration>,
    custom_tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum AutoscalingIntegration {
    Script { path: String },
}

#[derive(Debug, Deserialize, FromRow)]
struct ConfigWithOccupancy {
    autoscaling: Option<Json<Box<RawValue>>>,
    worker_group: String,
    custom_tags: Vec<String>,
    occupancy_rate_15s: Option<f64>,
    occupancy_rate_5m: Option<f64>,
    occupancy_rate_30m: Option<f64>,
}

pub async fn apply_all_autoscaling(db: &DB) -> anyhow::Result<()> {
    tracing::info!("applying all autoscaling");
    if crate::ee::acquire_lock(db, "autoscaling", 15).await? {
        let configs = sqlx::query_as::<_, ConfigWithOccupancy>(
        "
        WITH occupancy AS (
            SELECT worker_group, custom_tags, AVG(occupancy_rate_15s) AS occupancy_rate_15s, AVG(occupancy_rate_5m) AS occupancy_rate_5m, AVG(occupancy_rate_30m) AS occupancy_rate_30m
            FROM worker_ping
            WHERE ping_at > NOW() - INTERVAL '30s'
            GROUP BY worker_group, custom_tags
        )
        SELECT config.config->'autoscaling' as autoscaling, worker_group, custom_tags, occupancy_rate_15s, occupancy_rate_5m, occupancy_rate_30m FROM config, occupancy WHERE config.config->'autoscaling'->>'enabled' = 'true'",
            )
            .fetch_all(db)
            .await?;
        let queue_counts = get_queue_counts(db).await;

        // tracing::info!("configs: {:?} {queue_counts:?}", configs);

        let configs = configs
            .iter()
            .map(|c| {
                (
                    c.autoscaling
                        .as_ref()
                        .map(|v| serde_json::from_str(v.get()).ok())
                        .flatten(),
                    c,
                )
            })
            .collect::<Vec<_>>();

        for config_w_occupancy in configs {
            if let Some(config) = config_w_occupancy.0 {
                apply_autoscaling(db, &config, &config_w_occupancy.1, &queue_counts).await?;
            } else {
                tracing::error!("error parsing autoscaling config: {:?}", config_w_occupancy);
            }
        }
    } else {
        tracing::info!("did not acquire lock for autoscaling");
    }
    Ok(())
}

// #Every 30s check if we need to autoscale:

// do the first of:

// - if now - last_checked_at > full_upscale_cooldown_seconds
//   - if jobs_waiting > full_upscale_jobs_waiting:
//     - set last_checked_at_full = now
//     - set last_checked_at = now
//     - set workers = max_workers # full upscale
//     - break

// If now - last_checked_at < cooldown_seconds:
//   - do break

// INC_WORKERS = inc_percent/100 * (max_workers - min_workers)
//
// - if jobs_waiting > inc_upscale_jobs_waiting || (inc_scale_occupancy_rate > occupancy_rate_30s && inc_scale_occupancy_rate > occupancy_rate_5m)
//   - set last_checked_at = now
//   - set workers = min(workers + INC_WORKERS, max_workers) # inc upscale
//   - break

// if jobs_waiting == 0 && (dec_scale_occupancy_rate > occupancy_rate_30s && dec_scale_occupancy_rate > occupancy_rate_5m && dec_scale_occupancy_rate > occupancy_rate_30m)
//   - set last_checked_at = now
//   - set workers = max(workers - INC_WORKERS, min_workers) # dec upscale
//   - break

pub async fn apply_autoscaling(
    db: &DB,
    config: &AutoscalingConfig,
    occupancy: &ConfigWithOccupancy,
    queue_counts: &HashMap<String, i64>,
) -> anyhow::Result<()> {
    tracing::info!("{:?}", config);
    let worker_group = occupancy.worker_group.clone();
    let custom_tags = config
        .custom_tags
        .as_ref()
        .unwrap_or(&occupancy.custom_tags);
    match config.integration.as_ref() {
        Some(AutoscalingIntegration::Script { path }) => {
            tracing::info!("running script: {}", path);
        }
        _ => {}
    }
    Ok(())
}
