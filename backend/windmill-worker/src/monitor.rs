use std::{
    env,
    sync::{atomic::AtomicU16, Arc},
};

use sqlx::{PgPool, Pool, Postgres};
use tokio::sync::mpsc;
use uuid::Uuid;

#[cfg(feature = "prometheus")]
use windmill_common::METRICS_ENABLED;
use windmill_common::{
    error,
    flow_status::{FlowStatus, FlowStatusModule},
    jobs::QueuedJob,
    utils::{now_from_db, report_critical_error},
    BASE_URL,
};
use windmill_queue::cancel_job;

use crate::{
    create_token_for_owner, handle_job_error, AuthedClient, SameWorkerPayload, SameWorkerSender,
    SendResult, SCRIPT_TOKEN_EXPIRY,
};

lazy_static::lazy_static! {
    static ref ZOMBIE_JOB_TIMEOUT: String = env::var("ZOMBIE_JOB_TIMEOUT")
        .ok()
        .and_then(|x| x.parse::<String>().ok())
        .unwrap_or_else(|| "60".to_string());

    static ref FLOW_ZOMBIE_TRANSITION_TIMEOUT: String = env::var("FLOW_ZOMBIE_TRANSITION_TIMEOUT")
        .ok()
        .and_then(|x| x.parse::<String>().ok())
        .unwrap_or_else(|| "60".to_string());

    pub static ref RESTART_ZOMBIE_JOBS: bool = env::var("RESTART_ZOMBIE_JOBS")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(true);
}

#[cfg(feature = "prometheus")]
lazy_static::lazy_static! {
    static ref QUEUE_ZOMBIE_RESTART_COUNT: prometheus::IntCounter =
        prometheus::register_int_counter!(
            "queue_zombie_restart_count",
            "Total number of jobs restarted due to ping timeout."
        )
        .unwrap();

    static ref QUEUE_ZOMBIE_DELETE_COUNT: prometheus::IntCounter =
        prometheus::register_int_counter!(
            "queue_zombie_delete_count",
            "Total number of jobs deleted due to their ping timing out in an unrecoverable state."
        )
        .unwrap();

    static ref QUEUE_COUNT: prometheus::IntGaugeVec =
        prometheus::register_int_gauge_vec!(
            "queue_count",
            "Number of jobs in the queue",
            &["tag"]
        ).unwrap();
}

pub async fn monitor_once(
    db: &PgPool,
    base_internal_url: &str,
    worker_name: &str,
    timeout_seconds: Option<&str>,
    restart_zombies: Option<bool>,
    transition_timeout_seconds: Option<&str>,
) {
    let timeout_seconds = timeout_seconds.unwrap_or_else(|| &*ZOMBIE_JOB_TIMEOUT);
    let restart_zombies = restart_zombies.unwrap_or(*RESTART_ZOMBIE_JOBS);
    handle_zombie_jobs(
        &db,
        base_internal_url,
        worker_name,
        timeout_seconds,
        restart_zombies,
    )
    .await;
    let transition_timeout_seconds =
        transition_timeout_seconds.unwrap_or_else(|| &*FLOW_ZOMBIE_TRANSITION_TIMEOUT);
    let _ = handle_zombie_flows(&db, transition_timeout_seconds)
        .await
        .inspect_err(|err| tracing::error!("Error handling zombie flows: {:#}", err));
}

async fn handle_zombie_jobs(
    db: &Pool<Postgres>,
    base_internal_url: &str,
    worker_name: &str,
    timeout_seconds: &str,
    restart_zombies: bool,
) {
    if restart_zombies {
        let restarted = sqlx::query!(
            "WITH zombie_jobs AS (
                UPDATE v2_job_queue q SET running = false, started_at = null
                FROM v2_job j, v2_job_runtime r
                WHERE j.id = q.id AND j.id = r.id
                    AND ping < now() - ($1 || ' seconds')::interval
                    AND running = true
                    AND kind NOT IN ('flow', 'flowpreview', 'flownode', 'singlescriptflow')
                    AND same_worker = false
                RETURNING q.id, q.workspace_id, ping
            ),
            update_concurrency AS (
                UPDATE concurrency_counter cc
                SET job_uuids = job_uuids - zj.id::text
                FROM zombie_jobs zj
                INNER JOIN concurrency_key ck ON ck.job_id = zj.id
                WHERE cc.concurrency_id = ck.key
            )
            SELECT id, workspace_id, ping FROM zombie_jobs",
            timeout_seconds,
        )
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

        #[cfg(feature = "prometheus")]
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            QUEUE_ZOMBIE_RESTART_COUNT.inc_by(restarted.len() as _);
        }

        let base_url = BASE_URL.read().await.clone();
        for r in restarted {
            let last_ping = r
                .ping
                .map(|x| format!("last ping at {x}"))
                .unwrap_or_else(|| "no last ping".into());
            let url = format!("{}/run/{}?workspace={}", base_url, r.id, r.workspace_id,);
            let error_message = format!(
                "Zombie job {} on {} ({}) detected, restarting it, {}",
                r.id, r.workspace_id, url, last_ping
            );

            let _ = sqlx::query!("
                INSERT INTO job_logs (job_id, logs)
                VALUES ($1, 'Restarted job after not receiving job''s ping for too long the ' || now() || '\n\n')
                ON CONFLICT (job_id) DO UPDATE SET logs = job_logs.logs || '\n' || EXCLUDED.logs
                WHERE job_logs.job_id = $1",
                r.id
            )
            .execute(db)
            .await;
            tracing::error!(error_message);
            report_critical_error(error_message, db.clone(), Some(&r.workspace_id), None).await;
        }
    }

    let same_worker_timeout_jobs = {
        let long_same_worker_jobs = sqlx::query!(
            "SELECT worker, array_agg(v2_job_queue.id) as ids
            FROM v2_job_queue
                JOIN v2_job USING (id)
                LEFT JOIN v2_job_runtime USING (id)
            WHERE v2_job_queue.created_at < now() - ('60 seconds')::interval 
                AND running = true AND ping IS NULL
                AND same_worker = true
                AND worker IS NOT NULL
            GROUP BY worker",
        )
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

        let worker_ids = long_same_worker_jobs
            .iter()
            .map(|x| x.worker.clone().unwrap_or_default())
            .collect::<Vec<_>>();

        let long_dead_workers: std::collections::HashSet<String> = sqlx::query_scalar!(
            "WITH worker_ids AS (SELECT unnest($1::text[]) as worker) 
            SELECT worker_ids.worker FROM worker_ids 
            LEFT JOIN worker_ping ON worker_ids.worker = worker_ping.worker 
                WHERE worker_ping.worker IS NULL OR worker_ping.ping_at < now() - ('60 seconds')::interval",
            &worker_ids[..]
        )
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![])
        .into_iter()
        .filter_map(|x| x)
        .collect();

        let mut timeouts: Vec<Uuid> = vec![];
        for worker in long_same_worker_jobs {
            if worker.worker.is_some() && long_dead_workers.contains(&worker.worker.unwrap()) {
                if let Some(ids) = worker.ids {
                    timeouts.extend(ids);
                }
            }
        }
        if !timeouts.is_empty() {
            tracing::error!(
                "Failing same worker zombie jobs: {:?}",
                timeouts
                    .iter()
                    .map(|x| x.hyphenated().to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }

        let jobs = sqlx::query_as::<_, QueuedJob>("SELECT * FROM v2_as_queue WHERE id = ANY($1)")
            .bind(&timeouts[..])
            .fetch_all(db)
            .await
            .map_err(|e| tracing::error!("Error fetching same worker jobs: {:?}", e))
            .unwrap_or_default();

        jobs
    };

    let non_restartable_jobs = if restart_zombies {
        vec![]
    } else {
        sqlx::query_as::<_, QueuedJob>(
            "SELECT * FROM v2_as_queue
            WHERE last_ping < now() - ($1 || ' seconds')::interval 
                AND running = true
              AND job_kind NOT IN ('flow', 'flowpreview', 'flownode', 'singlescriptflow')
              AND same_worker = false",
        )
        .bind(timeout_seconds)
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![])
    };

    let timeouts = non_restartable_jobs
        .into_iter()
        .chain(same_worker_timeout_jobs)
        .collect::<Vec<_>>();

    #[cfg(feature = "prometheus")]
    if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        QUEUE_ZOMBIE_DELETE_COUNT.inc_by(timeouts.len() as _);
    }

    for job in timeouts {
        // since the job is unrecoverable, the same worker queue should never be sent anything
        let (same_worker_tx_never_used, _same_worker_rx_never_used) =
            mpsc::channel::<SameWorkerPayload>(1);
        let same_worker_tx_never_used =
            SameWorkerSender(same_worker_tx_never_used, Arc::new(AtomicU16::new(0)));
        let (send_result_never_used, _send_result_rx_never_used) = mpsc::channel::<SendResult>(1);

        let label = if job.permissioned_as != format!("u/{}", job.created_by)
            && job.permissioned_as != job.created_by
        {
            format!("ephemeral-script-end-user-{}", job.created_by)
        } else {
            "ephemeral-script".to_string()
        };
        let token = create_token_for_owner(
            &db,
            &job.workspace_id,
            &job.permissioned_as,
            &label,
            *SCRIPT_TOKEN_EXPIRY,
            &job.email,
            &job.id,
        )
        .await
        .expect("could not create job token");

        let client = AuthedClient {
            base_internal_url: base_internal_url.to_string(),
            token,
            workspace: job.workspace_id.to_string(),
            force_client: None,
        };

        let last_ping = job.last_ping.clone();
        let _ = handle_job_error(
            db,
            &client,
            &job,
            0,
            None,
            error::Error::ExecutionErr(format!(
                "Job timed out after no ping from job since {} (timeout: {}, same_worker: {})",
                last_ping
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| "no ping".to_string()),
                timeout_seconds,
                job.same_worker
            )),
            true,
            same_worker_tx_never_used,
            "",
            worker_name,
            send_result_never_used,
            #[cfg(feature = "benchmark")]
            &mut crate::bench::BenchmarkIter::new(),
        )
        .await;
    }
}

async fn handle_zombie_flows(db: &PgPool, transition_timeout_seconds: &str) -> error::Result<()> {
    let flows = sqlx::query!(
        r#"
        SELECT
            id AS "id!", workspace_id AS "workspace_id!", parent_job, is_flow_step,
            flow_status AS "flow_status: Box<str>", last_ping, same_worker
        FROM v2_as_queue
        WHERE running = true AND suspend = 0 AND suspend_until IS null AND scheduled_for <= now()
            AND (job_kind = 'flow' OR job_kind = 'flowpreview' OR job_kind = 'flownode')
            AND last_ping IS NOT NULL AND last_ping < NOW() - ($1 || ' seconds')::interval
            AND canceled = false
        "#,
        transition_timeout_seconds
    )
    .fetch_all(db)
    .await?;

    for flow in flows {
        let status = flow
            .flow_status
            .as_deref()
            .and_then(|x| serde_json::from_str::<FlowStatus>(x).ok());
        if !flow.same_worker.unwrap_or(false)
            && status.is_some_and(|s| {
                s.modules
                    .get(0)
                    .is_some_and(|x| matches!(x, FlowStatusModule::WaitingForPriorSteps { .. }))
            })
        {
            let error_message = format!(
                "Zombie flow detected: {} in workspace {}. It hasn't started yet, restarting it.",
                flow.id, flow.workspace_id
            );
            tracing::error!(error_message);
            report_critical_error(error_message, db.clone(), Some(&flow.workspace_id), None).await;
            // if the flow hasn't started and is a zombie, we can simply restart it
            let mut tx = db.begin().await?;

            let concurrency_key =
                sqlx::query_scalar!("SELECT key FROM concurrency_key WHERE job_id = $1", flow.id)
                    .fetch_optional(&mut *tx)
                    .await?;

            if let Some(key) = concurrency_key {
                sqlx::query!(
                    "UPDATE concurrency_counter SET job_uuids = job_uuids - $2 WHERE concurrency_id = $1",
                    key,
                    flow.id.hyphenated().to_string()
                )
                .execute(&mut *tx)
                .await?;
            }

            sqlx::query!(
                "UPDATE v2_job_queue SET running = false, started_at = null
                WHERE id = $1 AND canceled_by IS NULL",
                flow.id
            )
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;
        } else {
            let id = flow.id.clone();
            let last_ping = flow.last_ping.clone();
            let now = now_from_db(db).await?;
            let reason = format!(
                "{} was hanging in between 2 steps. Last ping: {last_ping:?} (now: {now})",
                if flow.is_flow_step.unwrap_or(false) && flow.parent_job.is_some() {
                    format!("Flow was cancelled because subflow {id}")
                } else {
                    format!("Flow {id} was cancelled because it")
                }
            );
            report_critical_error(reason.clone(), db.clone(), Some(&flow.workspace_id), None).await;
            cancel_zombie_flow_job(db, flow.id, &flow.workspace_id, reason).await?;
        }
    }

    let flows2 = sqlx::query!(
        r#"
        DELETE
        FROM parallel_monitor_lock
        WHERE last_ping IS NOT NULL AND last_ping < NOW() - ($1 || ' seconds')::interval 
        RETURNING parent_flow_id, job_id, last_ping, (SELECT workspace_id FROM v2_job_queue q
            WHERE q.id = parent_flow_id AND q.running = true AND q.canceled_by IS NULL
        ) AS workspace_id
        "#,
        transition_timeout_seconds
    )
    .fetch_all(db)
    .await?;

    for flow in flows2 {
        if let Some(parent_flow_workspace_id) = flow.workspace_id {
            tracing::error!(
                "parallel Zombie flow detected: {} in workspace {}. Last ping was: {:?}.",
                flow.parent_flow_id,
                parent_flow_workspace_id,
                flow.last_ping
            );
            cancel_zombie_flow_job(db, flow.parent_flow_id, &parent_flow_workspace_id, format!(
                "Flow {} cancelled as one of the parallel branch {} was unable to make the last transition ",
                flow.parent_flow_id,
                flow.job_id
            ))
            .await?;
        } else {
            tracing::info!("releasing lock for parallel flow: {}", flow.parent_flow_id);
        }
    }
    Ok(())
}

async fn cancel_zombie_flow_job(
    db: &Pool<Postgres>,
    id: Uuid,
    workspace_id: &str,
    message: String,
) -> Result<(), error::Error> {
    let mut tx = db.begin().await?;
    tracing::error!(
        "zombie flow detected: {} in workspace {}. Cancelling it.",
        id,
        workspace_id
    );
    (tx, _) = cancel_job(
        "monitor",
        Some(message),
        id,
        workspace_id,
        tx,
        db,
        true,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
