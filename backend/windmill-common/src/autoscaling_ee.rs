#[derive(Debug, Serialize, Deserialize)]
struct AutoscalingConfig {
    enabled: bool,
    min_workers: usize,
    max_workers: usize,
    cooldown_seconds: usize,
    inc_upscale_jobs_waiting: usize,
    full_upscale_cooldown_seconds: usize,
    full_upscale_jobs_waiting: usize,
    inc_percent: usize,
    integration: Option<AutoscalingIntegration>,
}

#[derive(Debug, Serialize, Deserialize)]
enum AutoscalingIntegration {
    AdminWorkspaceRunnable(String),
}

// #Every 30s check if we need to autoscale:

// do the first of:

// # Rules to autoscale full
// - if jobs_waiting > full_upscale_jobs_waiting

// # Rules to autoscale inc
//   - if cooldown > full_upscale_cooldown_seconds
//     - set cooldown = full_upscale_cooldown_seconds
//     - set workers = max(workers + 1, max_workers)
//   - else
//     - do nothing

// # Rules to scale down
// - if jobs_waiting < full_upscale_jobs_waiting
//   - if cooldown > full_upscale_cooldown_seconds
//     - set cooldown = full_upscale_cooldown_seconds
//     - set workers = max(workers - 1, min_workers)
//   - else
//     - do nothing

pub fn apply_autoscaling {
    let configs = sqlx::query_as!(
        ,
        "SELECT config->>'autoscaling' FROM config WHERE enabled = TRUE",
    )
    .fetch_all(&db)
    .await?;

    for config in configs {
        
    }
}