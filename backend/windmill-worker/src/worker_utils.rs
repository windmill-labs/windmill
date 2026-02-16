use backon::{BackoffBuilder, ConstantBuilder, Retryable};
use tracing::Instrument;
use uuid::Uuid;
use windmill_common::{
    agent_workers::{PingJobStatus, PingJobStatusResponse},
    cache,
    worker::{
        get_memory, get_vcpus, get_windmill_memory_usage, get_worker_memory_usage,
        insert_ping_query, update_job_ping_query, update_worker_ping_from_job_query,
        update_worker_ping_main_loop_query, Connection, Ping, PingType, NATIVE_MODE_RESOLVED,
        WORKER_CONFIG, WORKER_GROUP,
    },
    KillpillSender, DB,
};

use crate::{
    agent_workers::UPDATE_PING_URL,
    common::{OccupancyMetrics, OccupancyResult},
};

pub(crate) async fn update_worker_ping_full(
    conn: &Connection,
    read_cgroups: bool,
    jobs_executed: i32,
    worker_name: &str,
    hostname: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    killpill_tx: &KillpillSender,
) {
    let wc = WORKER_CONFIG.read().await;
    let tags = wc.worker_tags.clone();
    let native_mode = wc.native_mode;
    drop(wc);

    let memory_usage = get_worker_memory_usage();
    let wm_memory_usage = get_windmill_memory_usage();

    let (vcpus, memory) = if read_cgroups {
        (get_vcpus(), get_memory())
    } else {
        (None, None)
    };

    let OccupancyResult {
        occupancy_rate,
        occupancy_rate_15s,
        occupancy_rate_5m,
        occupancy_rate_30m,
    } = occupancy_metrics.update_occupancy_metrics();

    let ping_start = std::time::Instant::now();
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
            native_mode,
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
    let db_latency_ms = ping_start.elapsed().as_millis();
    tracing::info!(
        worker = %worker_name, hostname = %hostname,
        "ping update, memory: container={}MB, windmill={}MB, db_latency={}ms",
        memory_usage.unwrap_or_default() / (1024 * 1024),
        wm_memory_usage.unwrap_or_default() / (1024 * 1024),
        db_latency_ms
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
    native_mode: bool,
) -> anyhow::Result<()> {
    match conn {
        Connection::Sql(db) => {
            update_worker_ping_main_loop_query(
                worker_name,
                tags,
                vcpus,
                memory,
                Some(jobs_executed),
                Some(occupancy_rate),
                memory_usage,
                wm_memory_usage,
                occupancy_rate_15s,
                occupancy_rate_5m,
                occupancy_rate_30m,
                native_mode,
                db,
            )
            .await?;
        }
        Connection::Http(client) => {
            client
                .post::<_, ()>(
                    UPDATE_PING_URL,
                    None,
                    &Ping {
                        last_job_executed: None,
                        last_job_workspace_id: None,
                        worker_instance: None,
                        ip: None,
                        tags: Some(tags.to_vec()),
                        dw: None,
                        dws: None,
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
                        job_isolation: None,
                        native_mode: Some(native_mode),
                        ping_type: PingType::MainLoop,
                    },
                )
                .await?;
        }
    }
    Ok(())
}

pub async fn insert_ping(
    worker_instance: &str,
    worker_name: &str,
    ip: &str,
    db: &Connection,
) -> anyhow::Result<()> {
    let (tags, dw, dws, native_mode) = {
        let wc = WORKER_CONFIG.read().await.clone();
        (
            wc.worker_tags,
            wc.dedicated_worker
                .as_ref()
                .map(|x| format!("{}:{}", x.workspace_id, x.path)),
            wc.dedicated_workers.as_ref().map(|workers| {
                workers
                    .iter()
                    .map(|x| format!("{}:{}", x.workspace_id, x.path))
                    .collect::<Vec<_>>()
            }),
            wc.native_mode,
        )
    };

    let vcpus = get_vcpus();
    let memory = get_memory();

    let job_isolation = if crate::is_sandboxing_enabled() {
        Some("nsjail".to_string())
    } else if crate::is_unshare_enabled() {
        Some("unshare".to_string())
    } else {
        Some("none".to_string())
    };

    match db {
        Connection::Sql(db) => {
            insert_ping_query(
                worker_instance,
                worker_name,
                WORKER_GROUP.as_str(),
                ip,
                tags.as_slice(),
                dw,
                dws.as_deref(),
                windmill_common::utils::GIT_VERSION,
                vcpus,
                memory,
                job_isolation,
                native_mode,
                db,
            )
            .await?;
        }
        Connection::Http(client) => {
            client
                .post::<_, ()>(
                    UPDATE_PING_URL,
                    None,
                    &Ping {
                        last_job_executed: None,
                        last_job_workspace_id: None,
                        worker_instance: Some(worker_instance.to_string()),
                        ip: Some(ip.to_string()),
                        tags: Some(tags.to_vec()),
                        dw: dw,
                        dws: dws,
                        jobs_executed: None,
                        occupancy_rate: None,
                        occupancy_rate_15s: None,
                        occupancy_rate_5m: None,
                        occupancy_rate_30m: None,
                        version: Some(windmill_common::utils::GIT_VERSION.to_string()),
                        vcpus: vcpus,
                        memory: memory,
                        memory_usage: get_worker_memory_usage(),
                        wm_memory_usage: get_windmill_memory_usage(),
                        job_isolation,
                        native_mode: Some(native_mode),
                        ping_type: PingType::Initial,
                    },
                )
                .await?;
        }
    }
    Ok(())
}

pub async fn update_worker_ping_from_job(
    conn: &Connection,
    job_id: &Uuid,
    w_id: &str,
    worker_name: &str,
    memory_usage: Option<i64>,
    wm_memory_usage: Option<i64>,
    occupancy: Option<OccupancyResult>,
) -> anyhow::Result<()> {
    let occupancy_rate = occupancy.as_ref().map(|x| x.occupancy_rate);
    let occupancy_rate_15s = occupancy.as_ref().and_then(|x| x.occupancy_rate_15s);
    let occupancy_rate_5m = occupancy.as_ref().and_then(|x| x.occupancy_rate_5m);
    let occupancy_rate_30m = occupancy.as_ref().and_then(|x| x.occupancy_rate_30m);

    let job_isolation = if crate::is_sandboxing_enabled() {
        Some("nsjail".to_string())
    } else if crate::is_unshare_enabled() {
        Some("unshare".to_string())
    } else {
        Some("none".to_string())
    };

    match conn.clone() {
        Connection::Sql(ref db) => {
            update_worker_ping_from_job_query(
                job_id,
                w_id,
                worker_name,
                memory_usage,
                wm_memory_usage,
                occupancy_rate,
                occupancy_rate_15s,
                occupancy_rate_5m,
                occupancy_rate_30m,
                job_isolation,
                db,
            )
            .await?;
        }
        Connection::Http(client) => {
            client
                .post::<Ping, ()>(
                    UPDATE_PING_URL,
                    None,
                    &Ping {
                        last_job_executed: Some(job_id.clone()),
                        last_job_workspace_id: Some(w_id.to_string()),
                        ping_type: PingType::Job,
                        worker_instance: None,
                        ip: None,
                        tags: None,
                        dw: None,
                        dws: None,
                        version: None,
                        vcpus: None,
                        memory: None,
                        memory_usage: memory_usage,
                        wm_memory_usage: wm_memory_usage,
                        jobs_executed: None,
                        occupancy_rate: occupancy_rate,
                        occupancy_rate_15s: occupancy_rate_15s,
                        occupancy_rate_5m: occupancy_rate_5m,
                        occupancy_rate_30m: occupancy_rate_30m,
                        job_isolation,
                        native_mode: Some(
                            NATIVE_MODE_RESOLVED.load(std::sync::atomic::Ordering::Relaxed),
                        ),
                    },
                )
                .await?;
        }
    }
    Ok(())
}

pub async fn ping_job_status(
    conn: &Connection,
    job_id: &Uuid,
    mem_peak: Option<i32>,
    current_mem: Option<i32>,
) -> anyhow::Result<PingJobStatusResponse> {
    match conn {
        Connection::Sql(ref db) => update_job_ping_query(job_id, db, mem_peak).await,
        Connection::Http(client) => {
            client
                .post(
                    &format!("/api/agent_workers/ping_job_status/{}", job_id),
                    None,
                    &PingJobStatus { mem_peak, current_mem },
                )
                .await
        }
    }
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
                    if let Err(e) = sqlx::query!("VACUUM (SKIP_LOCKED) v2_job_queue, v2_job_runtime, v2_job_status, job_perms")
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
        Connection::Http(_) => {
            // do nothing in http mode
            ()
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TagAndConcurrencyKey {
    pub tag: Option<String>,
    pub concurrency_key: Option<String>,
    pub concurrent_limit: Option<i32>,
    pub concurrency_time_window_s: Option<i32>,
    pub version: Option<i64>,
}

pub async fn get_tag_and_concurrency(job_id: &Uuid, db: &DB) -> Option<TagAndConcurrencyKey> {
    let r = sqlx::query_as!(
        TagAndConcurrencyKey,
        "
        WITH j AS (
            SELECT 
                raw_flow->>'concurrency_key' as concurrency_key, 
                raw_flow->>'concurrency_time_window_s' as concurrency_time_window_s,
                raw_flow->>'concurrency_limit' as concurrent_limit,
                runnable_path, 
                runnable_id as version FROM v2_job
            WHERE id = $1
        )
        SELECT tag, j.concurrency_key, j.concurrency_time_window_s::int, j.concurrent_limit::int, j.version
            FROM flow, j
            WHERE path = j.runnable_path
        ",
        job_id
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten();
    if let Some(tag_and_concurrency_key) = r {
        if tag_and_concurrency_key.concurrency_key.as_ref().is_some()
            || tag_and_concurrency_key.version.as_ref().is_none()
        {
            return Some(tag_and_concurrency_key);
        } else {
            let version = tag_and_concurrency_key.version.unwrap();

            let r = cache::flow::fetch_version_lite(db, version).await;
            let flow = match r {
                Ok(data) => Ok(data),
                Err(_) => cache::flow::fetch_version(db, version).await,
            };
            let flow_value = flow.map(|f| f.value().clone()).ok();

            let concurrency_key = flow_value
                .as_ref()
                .and_then(|fv| fv.concurrency_settings.concurrency_key.to_owned());

            let concurrent_limit = flow_value
                .as_ref()
                .and_then(|fv| fv.concurrency_settings.concurrent_limit);

            let concurrent_time_window_s = flow_value
                .as_ref()
                .and_then(|fv| fv.concurrency_settings.concurrency_time_window_s);

            Some(TagAndConcurrencyKey {
                tag: tag_and_concurrency_key.tag,
                concurrency_key,
                concurrent_limit,
                concurrency_time_window_s: concurrent_time_window_s,
                version: None,
            })
        }
    } else {
        None
    }
}
