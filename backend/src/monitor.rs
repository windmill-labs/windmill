use once_cell::sync::OnceCell;
use sqlx::{Pool, Postgres};
use tokio::{join, sync::mpsc};
use uuid::Uuid;
use windmill_common::{
    error,
    jobs::{JobKind, QueuedJob},
    worker::{load_worker_config, reload_custom_tags_setting, WORKER_CONFIG},
    METRICS_ENABLED,
};
use windmill_worker::{
    create_token_for_owner, handle_job_error, AuthedClient, SCRIPT_TOKEN_EXPIRY,
};

lazy_static::lazy_static! {
    static ref ZOMBIE_JOB_TIMEOUT: String = std::env::var("ZOMBIE_JOB_TIMEOUT")
    .ok()
    .and_then(|x| x.parse::<String>().ok())
    .unwrap_or_else(|| "30".to_string());


    pub static ref RESTART_ZOMBIE_JOBS: bool = std::env::var("RESTART_ZOMBIE_JOBS")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(true);

    static ref QUEUE_ZOMBIE_RESTART_COUNT: prometheus::IntCounter = prometheus::register_int_counter!(
        "queue_zombie_restart_count",
        "Total number of jobs restarted due to ping timeout."
    )
    .unwrap();
    static ref QUEUE_ZOMBIE_DELETE_COUNT: prometheus::IntCounter = prometheus::register_int_counter!(
        "queue_zombie_delete_count",
        "Total number of jobs deleted due to their ping timing out in an unrecoverable state."
    )
    .unwrap();

    static ref QUEUE_COUNT: prometheus::IntGaugeVec = prometheus::register_int_gauge_vec!(
        "queue_count",
        "Number of jobs in the queue",
        &["tag"]
    ).unwrap();
}

pub async fn monitor_db<R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static>(
    db: &Pool<Postgres>,
    tx: tokio::sync::broadcast::Sender<()>,
    base_internal_url: &str,
    rsmq: Option<R>,
    worker_mode: bool,
    server_mode: bool,
) {
    let zombie_jobs_f = async {
        if server_mode {
            handle_zombie_jobs(db, base_internal_url, rsmq.clone()).await;
        }
    };
    let expired_items_f = async {
        if server_mode {
            windmill_api::delete_expired_items(&db).await;
        }
    };
    let reload_worker_config_f = async {
        if worker_mode {
            reload_worker_config(&db, tx).await;
        }
    };
    let reload_custom_tags_f = async {
        if server_mode {
            if let Err(e) = reload_custom_tags_setting(db).await {
                tracing::error!("Error reloading custom tags: {:?}", e)
            }
        }
    };
    let expose_queue_metrics_f = async {
        if *METRICS_ENABLED && server_mode {
            expose_queue_metrics(&db).await;
        }
    };
    join!(
        expired_items_f,
        zombie_jobs_f,
        reload_worker_config_f,
        reload_custom_tags_f,
        expose_queue_metrics_f
    );
}

pub async fn expose_queue_metrics(db: &Pool<Postgres>) {
    let queue_counts = sqlx::query!("SELECT tag, count(*) as count FROM queue GROUP BY tag")
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);
    for q in queue_counts {
        let count = q.count.unwrap_or(0);
        let tag = q.tag;
        let metric = (*QUEUE_COUNT).with_label_values(&[&tag]);
        metric.set(count as i64);
    }
}

pub async fn reload_worker_config(db: &Pool<Postgres>, tx: tokio::sync::broadcast::Sender<()>) {
    let config = load_worker_config(&db).await;
    if let Err(e) = config {
        tracing::error!("Error reloading worker config: {:?}", e)
    } else {
        let wc = WORKER_CONFIG.read().await;
        let config = config.unwrap();
        if *wc != config {
            if (*wc).dedicated_worker != config.dedicated_worker {
                tracing::info!("Dedicated worker config changed, sending killpill. Expecting to be restarted by supervisor.");
                let _ = tx.send(());
            }
            drop(wc);

            let mut wc = WORKER_CONFIG.write().await;
            tracing::info!("Reloading worker config...");
            *wc = config
        }
    }
}

async fn handle_zombie_jobs<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    db: &Pool<Postgres>,
    base_internal_url: &str,
    rsmq: Option<R>,
) {
    if *RESTART_ZOMBIE_JOBS {
        let restarted = sqlx::query!(
                "UPDATE queue SET running = false, started_at = null, logs = logs || '\nRestarted job after not receiving job''s ping for too long the ' || now() || '\n\n' WHERE last_ping < now() - ($1 || ' seconds')::interval AND running = true AND job_kind != $2 AND job_kind != $3 AND same_worker = false RETURNING id, workspace_id, last_ping",
                *ZOMBIE_JOB_TIMEOUT,
                JobKind::Flow as JobKind,
                JobKind::FlowPreview as JobKind,
            )
            .fetch_all(db)
            .await
            .ok()
            .unwrap_or_else(|| vec![]);

        if *METRICS_ENABLED {
            QUEUE_ZOMBIE_RESTART_COUNT.inc_by(restarted.len() as _);
        }
        for r in restarted {
            tracing::info!(
                "restarted zombie job {} {} {}",
                r.id,
                r.workspace_id,
                r.last_ping
            );
        }
    }

    let mut timeout_query = "SELECT * FROM queue WHERE last_ping < now() - ($1 || ' seconds')::interval AND running = true AND job_kind != $2 AND job_kind != $3".to_string();
    if *RESTART_ZOMBIE_JOBS {
        timeout_query.push_str(" AND same_worker = true");
    };
    let timeouts = sqlx::query_as::<_, QueuedJob>(&timeout_query)
        .bind(ZOMBIE_JOB_TIMEOUT.as_str())
        .bind(JobKind::Flow)
        .bind(JobKind::FlowPreview)
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

    if *METRICS_ENABLED {
        QUEUE_ZOMBIE_DELETE_COUNT.inc_by(timeouts.len() as _);
    }

    for job in timeouts {
        tracing::info!("timedout zombie job {} {}", job.id, job.workspace_id,);

        // since the job is unrecoverable, the same worker queue should never be sent anything
        let (same_worker_tx_never_used, _same_worker_rx_never_used) = mpsc::channel::<Uuid>(1);

        let token = create_token_for_owner(
            &db,
            &job.workspace_id,
            &job.permissioned_as,
            "ephemeral-zombie-jobs",
            *SCRIPT_TOKEN_EXPIRY,
            &job.email,
        )
        .await
        .expect("could not create job token");

        let client = AuthedClient {
            base_internal_url: base_internal_url.to_string(),
            token,
            workspace: job.workspace_id.to_string(),
            client: OnceCell::new(),
        };

        let last_ping = job.last_ping.clone();
        let _ = handle_job_error(
            db,
            &client,
            &job,
            error::Error::ExecutionErr(format!(
                "Job timed out after no ping from job since {} (ZOMBIE_JOB_TIMEOUT: {})",
                last_ping
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| "no ping".to_string()),
                *ZOMBIE_JOB_TIMEOUT
            )),
            None,
            true,
            same_worker_tx_never_used,
            "",
            rsmq.clone(),
        )
        .await;
    }
}
