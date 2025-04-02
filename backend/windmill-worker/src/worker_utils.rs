use backon::{BackoffBuilder, ConstantBuilder, Retryable};
use tracing::Instrument;
use windmill_common::{
    agent_workers::{AGENT_HTTP_CLIENT, BASE_INTERNAL_URL},
    worker::{
        get_memory, get_vcpus, get_windmill_memory_usage, get_worker_memory_usage, Connection,
        Ping, WORKER_CONFIG,
    },
    KillpillSender,
};

use crate::common::OccupancyMetrics;

pub(crate) async fn update_worker_ping_full(
    conn: &Connection,
    read_cgroups: bool,
    jobs_executed: i32,
    worker_name: &str,
    hostname: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    killpill_tx: &KillpillSender,
) {
    let tags = WORKER_CONFIG.read().await.worker_tags.clone();

    let memory_usage = get_worker_memory_usage();
    let wm_memory_usage = get_windmill_memory_usage();

    let (vcpus, memory) = if read_cgroups {
        (get_vcpus(), get_memory())
    } else {
        (None, None)
    };

    let (occupancy_rate, occupancy_rate_15s, occupancy_rate_5m, occupancy_rate_30m) =
        occupancy_metrics.update_occupancy_metrics();

    if let Err(e) = (|| {
        update_worker_ping_full_inner(
            conn,
            jobs_executed,
            &worker_name,
            &tags,
            memory_usage,
            wm_memory_usage,
            vcpus,
            memory,
            occupancy_rate,
            occupancy_rate_15s,
            occupancy_rate_5m,
            occupancy_rate_30m,
        )
    })
    .retry(
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_secs(2))
            .with_max_times(10)
            .build(),
    )
    .notify(|err, dur| {
        tracing::error!(
            worker = %worker_name, hostname = %hostname,
            "retrying updating worker ping in {dur:#?}, err: {err:#?}"
        );
    })
    .sleep(tokio::time::sleep)
    .await
    {
        tracing::error!(
                    worker = %worker_name, hostname = %hostname,
                    "failed to update worker ping, exiting: {}", e);
        killpill_tx.send();
    }
    tracing::info!(
        worker = %worker_name, hostname = %hostname,
        "ping update, memory: container={}MB, windmill={}MB",
        memory_usage.unwrap_or_default() / (1024 * 1024),
        wm_memory_usage.unwrap_or_default() / (1024 * 1024)
    );
}

async fn update_worker_ping_full_inner(
    conn: &Connection,
    jobs_executed: i32,
    worker_name: &str,
    tags: &[String],
    memory_usage: Option<i64>,
    wm_memory_usage: Option<i64>,
    vcpus: Option<i64>,
    memory: Option<i64>,
    occupancy_rate: f32,
    occupancy_rate_15s: Option<f32>,
    occupancy_rate_5m: Option<f32>,
    occupancy_rate_30m: Option<f32>,
) -> anyhow::Result<()> {
    match conn {
        Connection::Sql(db) => {
            sqlx::query!(
                "UPDATE worker_ping SET ping_at = now(), jobs_executed = $1, custom_tags = $2,
                 occupancy_rate = $3, memory_usage = $4, wm_memory_usage = $5, vcpus = COALESCE($7, vcpus),
                 memory = COALESCE($8, memory), occupancy_rate_15s = $9, occupancy_rate_5m = $10, occupancy_rate_30m = $11 WHERE worker = $6",
                jobs_executed,
                tags,
                occupancy_rate,
                memory_usage,
                wm_memory_usage,
                &worker_name,
                vcpus,
                memory,
                occupancy_rate_15s,
                occupancy_rate_5m,
                occupancy_rate_30m
            ).execute(db)
            .await?;
        }
        Connection::Http => {
            let response = AGENT_HTTP_CLIENT
                .post(format!(
                    "{}/api/agent_workers/update_ping",
                    *BASE_INTERNAL_URL
                ))
                .body(serde_json::to_string(&Ping {
                    worker_instance: None,
                    ip: None,
                    tags: tags.to_vec(),
                    dw: None,
                    jobs_executed: Some(jobs_executed),
                    occupancy_rate: Some(occupancy_rate),
                    occupancy_rate_15s: Some(occupancy_rate_15s.unwrap_or(0.0)),
                    occupancy_rate_5m: Some(occupancy_rate_5m.unwrap_or(0.0)),
                    occupancy_rate_30m: Some(occupancy_rate_30m.unwrap_or(0.0)),
                    version: None,
                    vcpus: vcpus,
                    memory: memory,
                    memory_usage: get_worker_memory_usage(),
                    wm_memory_usage: get_windmill_memory_usage(),
                    update: true,
                })?)
                .header("Content-Type", "application/json")
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(format!(
                    "Failed to send update ping request {:?}",
                    response.status()
                )));
            }
        }
    }
    Ok(())
}

pub(crate) async fn queue_vacuum(conn: &Connection, worker_name: &str, hostname: &str) {
    match conn {
        Connection::Sql(db) => {
            let db2 = db.clone();
            let current_span = tracing::Span::current();
            let worker_name = worker_name.to_string();
            let hostname = hostname.to_string();
            tokio::task::spawn(
                (async move {
                    tracing::info!(worker = %worker_name, hostname = %hostname, "vacuuming queue");
                    if let Err(e) = sqlx::query!("VACUUM v2_job_queue, v2_job_runtime, v2_job_status")
                        .execute(&db2)
                        .await
                    {
                        tracing::error!(worker = %worker_name, hostname = %hostname, "failed to vacuum queue: {}", e);
                    }
                    tracing::info!(worker = %worker_name, hostname = %hostname, "vacuumed queue");
                })
                .instrument(current_span),
            );
        }
        Connection::Http => {
            // do nothing in http mode
            ()
        }
    }
}
