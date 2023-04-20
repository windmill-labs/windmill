/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use anyhow::Result;
use const_format::concatcp;
use itertools::Itertools;
use once_cell::sync::OnceCell;
use sqlx::{Pool, Postgres};
use windmill_api_client::Client;
use std::{
    borrow::Borrow, collections::HashMap, io, os::unix::process::ExitStatusExt, panic,
    process::Stdio, time::{Duration},
    sync::{Arc},
};
use tracing::{trace_span, Instrument};
use uuid::Uuid;

use windmill_common::{
    error::{self, to_anyhow, Error},
    flows::{FlowModuleValue, FlowValue},
    scripts::{ScriptHash, ScriptLang},
    utils::{rd_string},
    variables, BASE_URL, users::SUPERADMIN_SECRET_EMAIL, METRICS_ENABLED, jobs::{JobKind, QueuedJob},
};
use windmill_queue::{canceled_job_to_result, get_queued_job, pull, CLOUD_HOSTED};

use serde_json::{json, Value};

use tokio::{
    fs::{metadata, symlink, DirBuilder, File},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{
        mpsc::{self, Sender},  watch, broadcast, RwLock, Barrier
    },
    time::{interval, sleep, Instant, MissedTickBehavior}
};

use futures::{
    future::{self, ready, FutureExt},
    stream, StreamExt,
};

use async_recursion::async_recursion;

#[cfg(feature = "enterprise")]
use rand::Rng;

#[cfg(feature = "enterprise")]
use crate::global_cache::{copy_cache_to_tmp_cache, cache_global, copy_tmp_cache_to_cache};

use crate::{
    jobs::{add_completed_job, add_completed_job_error},
    worker_flow::{
        handle_flow, update_flow_status_after_job_completion, update_flow_status_in_progress,
    }, python_executor::{create_dependencies_dir, pip_compile, handle_python_job, handle_python_reqs}, common::{read_result, set_logs}, go_executor::{handle_go_job, install_go_dependencies},
};




pub async fn create_token_for_owner_in_bg(db: &Pool<Postgres>, job: &QueuedJob) -> Arc<RwLock<String>> {
    let rw_lock = Arc::new(RwLock::new(String::new()));
    // skipping test runs
    if job.workspace_id != "" {
        let mut locked = rw_lock.clone().write_owned().await;
        let db = db.clone();
        let job = job.clone();
        tokio::spawn(async move {
            let job = job.clone();
            let token = create_token_for_owner(
                &db.clone(),
                &job.workspace_id,
                &job.permissioned_as,
                "ephemeral-script",
                *SESSION_TOKEN_EXPIRY,
                &job.email,
            )
            .await.expect("could not create job token");
            *locked = token;
        });
    };
    return rw_lock;
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn create_token_for_owner(
    db: &Pool<Postgres>,
    w_id: &str,
    owner: &str,
    label: &str,
    expires_in: i32,
    email: &str,
) -> error::Result<String> {
    // TODO: Bad implementation. We should not have access to this DB here.
    let token: String = rd_string(30);
    let is_super_admin = sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
            .fetch_optional(db)
            .await?
            .unwrap_or(false) || email == SUPERADMIN_SECRET_EMAIL;

    sqlx::query_scalar!(
        "INSERT INTO token
            (workspace_id, token, owner, label, expiration, super_admin, email)
            VALUES ($1, $2, $3, $4, now() + ($5 || ' seconds')::interval, $6, $7)",
        &w_id,
        token,
        owner,
        label,
        expires_in.to_string(),
        is_super_admin,
        email
    )
    .execute(db)
    .await?;
    Ok(token)
}

pub const TMP_DIR: &str = "/tmp/windmill";
pub const ROOT_CACHE_DIR: &str = "/tmp/windmill/cache/";
pub const ROOT_TMP_CACHE_DIR: &str = "/tmp/windmill/tmpcache/";
pub const PIP_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "pip");
pub const DENO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "deno");
pub const GO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "go");
pub const PIP_TMP_CACHE_DIR: &str = concatcp!(ROOT_TMP_CACHE_DIR, "pip");
pub const DENO_TMP_CACHE_DIR: &str = concatcp!(ROOT_TMP_CACHE_DIR, "deno");
pub const GO_TMP_CACHE_DIR: &str = concatcp!(ROOT_TMP_CACHE_DIR, "go");

const NUM_SECS_PING: u64 = 5;


const INCLUDE_DEPS_PY_SH_CONTENT: &str = include_str!("../nsjail/download_deps.py.sh");
const NSJAIL_CONFIG_RUN_BASH_CONTENT: &str = include_str!("../nsjail/run.bash.config.proto");


#[derive(Clone)]
pub struct Metrics {
    pub worker_execution_failed: prometheus::IntCounter,
}


pub const DEFAULT_TIMEOUT: u16 = 300;
pub const DEFAULT_SLEEP_QUEUE: u64 = 50;

lazy_static::lazy_static! {

    static ref SLEEP_QUEUE: u64 = std::env::var("SLEEP_QUEUE")
    .ok()
    .and_then(|x| x.parse::<u64>().ok())
    .unwrap_or(DEFAULT_SLEEP_QUEUE);

    pub static ref DISABLE_NUSER: bool = std::env::var("DISABLE_NUSER")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(false);

    pub static ref DISABLE_NSJAIL: bool = std::env::var("DISABLE_NSJAIL")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(true);

    pub static ref KEEP_JOB_DIR: bool = std::env::var("KEEP_JOB_DIR")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(false);


    pub static ref DENO_PATH: String = std::env::var("DENO_PATH").unwrap_or_else(|_| "/usr/bin/deno".to_string());
    pub static ref NSJAIL_PATH: String = std::env::var("NSJAIL_PATH").unwrap_or_else(|_| "nsjail".to_string());
    pub static ref PATH_ENV: String = std::env::var("PATH").unwrap_or_else(|_| String::new());
    pub static ref HOME_ENV: String = std::env::var("HOME").unwrap_or_else(|_| String::new());
    pub static ref GOPRIVATE: Option<String> = std::env::var("GOPRIVATE").ok();
    pub static ref NETRC: Option<String> = std::env::var("NETRC").ok();

    static ref DENO_AUTH_TOKENS: String = std::env::var("DENO_AUTH_TOKENS")
        .ok()
        .map(|x| format!(";{x}"))
        .unwrap_or_else(|| String::new());

    static ref NPM_CONFIG_REGISTRY: Option<String> = std::env::var("NPM_CONFIG_REGISTRY").ok();

    static ref DENO_FLAGS: Option<Vec<String>> = std::env::var("DENO_FLAGS")
        .ok()
        .map(|x| x.split(' ').map(|x| x.to_string()).collect());
        


    static ref WHITELIST_WORKSPACES: Option<Vec<String>> = std::env::var("WHITELIST_WORKSPACES")
        .ok()
        .map(|x| x.split(',').map(|x| x.to_string()).collect());
    static ref BLACKLIST_WORKSPACES: Option<Vec<String>>  = std::env::var("BLACKLIST_WORKSPACES")
        .ok()
        .map(|x| x.split(',').map(|x| x.to_string()).collect());

    pub static ref TAR_CACHE_RATE: i32 = std::env::var("TAR_CACHE_RATE")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(100);

    static ref WORKER_STARTED: prometheus::IntGauge = prometheus::register_int_gauge!(
        "worker_started",
        "Total number of workers started."
    )
    .unwrap();

    static ref WORKER_UPTIME_OPTS: prometheus::Opts = prometheus::opts!(
        "worker_uptime",
        "Total number of seconds since the worker has started"
    );

    static ref TIMEOUT: u16 = std::env::var("TIMEOUT")
        .ok()
        .and_then(|x| x.parse::<u16>().ok())
        .unwrap_or(DEFAULT_TIMEOUT as u16);

    static ref TIMEOUT_DURATION: Duration = Duration::from_secs(*TIMEOUT as u64);

    pub static ref SESSION_TOKEN_EXPIRY: i32 = (*TIMEOUT as i32) * 2;

    pub static ref GLOBAL_CACHE_INTERVAL: u64 = std::env::var("GLOBAL_CACHE_INTERVAL")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or(60 * 10);

    pub static ref S3_CACHE_BUCKET: Option<String> = std::env::var("S3_CACHE_BUCKET")
        .ok()
        .map(|e| Some(e))
        .unwrap_or(None);

    pub static ref CAN_PULL: Arc<RwLock<()>> = Arc::new(RwLock::new(()));

}

//only matter if CLOUD_HOSTED
pub const MAX_RESULT_SIZE: usize = 1024 * 1024 * 2; // 2MB

pub struct AuthedClientBackgroundTask {
    pub base_internal_url: String,
    pub workspace: String,
    pub token: Arc<RwLock<String>>,
    pub client: OnceCell<Client>
}

impl AuthedClientBackgroundTask {
    pub async fn get_authed(&self) -> AuthedClient {
        return AuthedClient {
            base_internal_url: self.base_internal_url.clone(),
            workspace: self.workspace.clone(),
            token: self.get_token().await,
            client: self.client.clone()
        }
    }
    pub async fn get_token(&self) -> String {
        return self.token.read().await.clone();
    }
}
#[derive(Clone)]
pub struct AuthedClient {
    pub base_internal_url: String,
    pub workspace: String,
    pub token: String,
    pub client: OnceCell<Client>
}

impl AuthedClient {
    pub fn get_client(&self) -> &Client {
        return self.client.get_or_init(|| {
            windmill_api_client::create_client(&self.base_internal_url, self.token.clone())
        });
    }
}


pub async fn run_worker<R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static>(
    db: &Pool<Postgres>,
    worker_instance: &str,
    worker_name: String,
    i_worker: u64,
    num_workers: u32,
    ip: &str,
    mut rx: tokio::sync::broadcast::Receiver<()>,
    base_internal_url: &str,
    rsmq: Option<R>,
    sync_barrier: Arc<RwLock<Option<Barrier>>>,
) {
    #[cfg(not(feature = "enterprise"))]
    if !*DISABLE_NSJAIL {
        tracing::warn!(
            "NSJAIL to sandbox process in untrusted environments is an enterprise feature but allowed to be used for testing purposes"
        );
    }

    let start_time = Instant::now();

    let worker_dir = format!("{TMP_DIR}/{worker_name}");
    tracing::debug!(worker_dir = %worker_dir, worker_name = %worker_name, "Creating worker dir");

    DirBuilder::new()
        .recursive(true)
        .create(&worker_dir)
        .await
        .expect("could not create initial worker dir");

    let _ = write_file(
        &worker_dir,
        "download_deps.py.sh",
        INCLUDE_DEPS_PY_SH_CONTENT,
    )
    .await;

    let mut last_ping = Instant::now() - Duration::from_secs(NUM_SECS_PING + 1);

    insert_initial_ping(worker_instance, &worker_name, ip, db).await;

    let uptime_metric = prometheus::register_counter!(WORKER_UPTIME_OPTS
        .clone()
        .const_label("name", &worker_name))
    .unwrap();


    let all_langs = [
        None, 
        Some(ScriptLang::Python3),
        Some(ScriptLang::Deno),
        Some(ScriptLang::Go),
        Some(ScriptLang::Bash)];

    let worker_execution_duration: HashMap<_, _>  = all_langs.clone().into_iter().map(|x| (x.clone(), prometheus::register_histogram!(
        prometheus::HistogramOpts::new(
            "worker_execution_duration",
            "Duration between receiving a job and completing it",
        )
        .const_label("name", &worker_name)
        .const_label("language", x.map(|x| x.as_str()).unwrap_or("none")))
        .expect("register prometheus metric"))
    ).collect();


    let worker_execution_duration_counter: HashMap<_, _>  = all_langs.clone().into_iter().map(|x| (x.clone(), prometheus::register_counter!(
        prometheus::Opts::new(
            "worker_execution_duration_counter",
            "Total number of seconds spent executing jobs"
        )
        .const_label("name", &worker_name)
        .const_label("language", x.map(|x| x.as_str()).unwrap_or("none")))
        .expect("register prometheus metric"))
    ).collect();


    let worker_sleep_duration_counter = prometheus::register_counter!(prometheus::opts!(
        "worker_sleep_duration_counter",
        "Total number of seconds spent sleeping between pulling jobs from the queue"
    )
        .const_label("name", &worker_name))
        .expect("register prometheus metric");


    let worker_pull_duration = prometheus::register_histogram!(prometheus::HistogramOpts::new(
        "worker_pull_duration",
        "Duration pulling next job",
    )
    .const_label("name", &worker_name),)
    .expect("register prometheus metric");

    let worker_pull_duration_counter = prometheus::register_counter!(prometheus::opts!(
        "worker_pull_duration_counter",
        "Total number of seconds spent pulling jobs (if growing large the db is undersized)"
    )
        .const_label("name", &worker_name))
        .expect("register prometheus metric");

    let worker_execution_failed: HashMap<_, _>  = all_langs.clone().into_iter().map(|x| (x.clone(), prometheus::register_int_counter!(
        prometheus::Opts::new(
            "worker_execution_failed", "Number of failed jobs",
        )
        .const_label("name", &worker_name)
        .const_label("language", x.map(|x| x.as_str()).unwrap_or("none")))
        .expect("register prometheus metric"))
    ).collect();


    let worker_execution_count: HashMap<_, _> = all_langs.into_iter().map(|x| (x.clone(), prometheus::register_int_counter!(
        prometheus::Opts::new(
            "worker_execution_count", "Number of executed jobs"
        )
        .const_label("name", &worker_name)
        .const_label("language", x.map(|x| x.as_str()).unwrap_or("none")))
        .expect("register prometheus metric"))
    ).collect();

    let worker_busy: prometheus::IntGauge = prometheus::register_int_gauge!(prometheus::Opts::new(
        "worker_busy",
        "Is the worker busy executing a job?",
    )
    .const_label("name", &worker_name))
    .unwrap();

    let mut jobs_executed = 0;

    if *METRICS_ENABLED {
        WORKER_STARTED.inc();
    }


    let (copy_to_bucket_tx, mut copy_to_bucket_rx) = mpsc::channel::<()>(2);

    let mut copy_cache_from_bucket_handle: Option<tokio::task::JoinHandle<()>> = None;

    tracing::info!(worker = %worker_name, "starting worker");

    #[cfg(feature = "enterprise")]
    let mut last_sync =
        Instant::now() + Duration::from_secs(rand::thread_rng().gen_range(0..*GLOBAL_CACHE_INTERVAL));

    let (same_worker_tx, mut same_worker_rx) = mpsc::channel::<Uuid>(5);

    let (job_completed_tx, mut job_completed_rx) = mpsc::channel::<JobCompleted>(10);

    let db2 = db.clone();
    let rsmq2 = rsmq.clone();
    let worker_name2 = worker_name.clone();
    let send_result = tokio::spawn(async move {
        while let Some(JobCompleted { job, logs, result}) = job_completed_rx.recv().await {
            if let Err(e) = add_completed_job(&db2, &job, true, false, result, logs, rsmq2.clone()).await {
                tracing::error!(worker = %worker_name2, "failed to add completed job: {}", e);
            }
        }
    });
    
    tracing::info!(worker = %worker_name, "listening for jobs");
    
    let mut first_run = true;

    // let mut barrier = Arc::new();
    loop {
        if *METRICS_ENABLED {
            worker_busy.set(0);
            uptime_metric.inc_by(
                ((start_time.elapsed().as_millis() as f64)/1000.0 - uptime_metric.get())
                    .try_into()
                    .unwrap(),
            );
        }

        #[cfg(feature = "enterprise")]
        let copy_tx = copy_to_bucket_tx.clone();

        let do_break = async {
            if last_ping.elapsed().as_secs() > NUM_SECS_PING {
                sqlx::query!(
                    "UPDATE worker_ping SET ping_at = now(), jobs_executed = $1 WHERE worker = $2",
                    jobs_executed,
                    &worker_name
                )
                .execute(db)
                .await
                .expect("update worker ping");

                last_ping = Instant::now();
            }

            #[cfg(feature = "enterprise")]
            if i_worker == 1 && S3_CACHE_BUCKET.is_some() {
                if last_sync.elapsed().as_secs() > *GLOBAL_CACHE_INTERVAL &&
                    (copy_cache_from_bucket_handle.is_none() || copy_cache_from_bucket_handle.as_ref().unwrap().is_finished()) {

                    tracing::debug!("CAN PULL LOCK START");
                    let _lock = CAN_PULL.write().await;

                    tracing::info!("Started syncing cache");
                    last_sync = Instant::now();
                    if num_workers > 1 {
                        create_barrier_for_all_workers(num_workers, sync_barrier.clone()).await;
                    }
                    if let Err(e) = copy_cache_to_tmp_cache().await {
                        tracing::error!("failed to copy cache to tmp cache: {}", e);
                    } else {
                        copy_cache_from_bucket_handle = Some(tokio::task::spawn(async move {
                            if let Some(ref s) = S3_CACHE_BUCKET.clone() {
                                if let Err(e) = cache_global(s, copy_tx).await { 
                                    tracing::error!("failed to sync cache: {}", e);
                                }
                            }
                        }));
                    }
                }
            }

            // The barrier is to avoid the sync to bucket syncing partial folders
            #[cfg(feature = "enterprise")]
            if num_workers > 1 && S3_CACHE_BUCKET.is_some()  {
                let read_barrier = sync_barrier.read().await;
                if let Some(b) = read_barrier.as_ref() {
                    tracing::debug!("worker #{i_worker} waiting for barrier");
                    b.wait().await;
                    tracing::debug!("worker #{i_worker} done waiting for barrier");
                    drop(read_barrier);
                    // wait for barrier to be reset
                    let _ = CAN_PULL.read().await;
                    tracing::debug!("worker #{i_worker} done waiting for lock");
                } else {
                    tracing::debug!("worker #{i_worker} no barrier");
                };
            }
            
            let (do_break, next_job) = if first_run { 
                (false, Ok(Some(QueuedJob::default())))
            } else {
                async {
                    tokio::select! {
                        biased;
                        _ = rx.recv() => {
                            if let Some(copy_cache_from_bucket_handle) = copy_cache_from_bucket_handle.as_ref() {
                                if !copy_cache_from_bucket_handle.is_finished() {
                                    copy_cache_from_bucket_handle.abort();
                                }
                            }
                            println!("received killpill for worker {}", i_worker);
                            (true, Ok(None))
                        },
                        _ = copy_to_bucket_rx.recv() => {
                            tracing::debug!("CAN PULL LOCK START");
                            let _lock = CAN_PULL.write().await;
                            if num_workers > 1 {
                                create_barrier_for_all_workers(num_workers, sync_barrier.clone()).await;
                            }
                            //Arc::new(tokio::sync::Barrier::new(num_workers as usize + 1));
                            #[cfg(feature = "enterprise")]
                            if let Err(e) = copy_tmp_cache_to_cache().await {
                                tracing::error!(worker = %worker_name, "failed to sync tmp cache to cache: {}", e);
                            }
                            tracing::debug!("CAN PULL LOCK END");
                            (false, Ok(None))
                        },
                        Some(job_id) = same_worker_rx.recv() => {
                            (false, sqlx::query_as::<_, QueuedJob>("SELECT * FROM queue WHERE id = $1")
                            .bind(job_id)
                            .fetch_optional(db)
                            .await
                            .map_err(|_| Error::InternalErr("Impossible to fetch same_worker job".to_string())))
                        },
                        (job, timer) = {
                            let timer = if *METRICS_ENABLED { Some(worker_pull_duration.start_timer()) } else { None }; 
                            pull(&db, WHITELIST_WORKSPACES.clone(), BLACKLIST_WORKSPACES.clone(), rsmq.clone()).map(|x| (x, timer)) 
                        } => {
                            timer.map(|timer| {
                                let duration_pull_s = timer.stop_and_record();
                                worker_pull_duration_counter.inc_by(duration_pull_s);
                            });
                            (false, job)
                        },
                    }
                }.await
            };

            first_run = false;
            if do_break {
                return true;
            }
            if *METRICS_ENABLED {
                worker_busy.set(1);
            }
            match next_job {
                Ok(Some(job)) => {
                    // println!("{:?}",  SystemTime::now());

                    let token = create_token_for_owner_in_bg(&db, &job).await;
                    let language = job.language.clone();
                    let _timer = worker_execution_duration
                        .get(&language)
                        .expect("no timer found")
                        .start_timer();

                    jobs_executed += 1;
                    if *METRICS_ENABLED {
                        worker_execution_count
                            .get(&language)
                            .expect("no timer found")
                            .inc();
                    }

                    let metrics = if *METRICS_ENABLED {
                        Some(Metrics {
                            worker_execution_failed: worker_execution_failed
                                .get(&language)
                                .expect("no timer found").clone(),
                        }) 
                    } else { None };

                    let job_root = job.root_job.map(|x| x.to_string()).unwrap_or_else(|| "none".to_string());

                    if job.id == Uuid::nil() {
                        tracing::info!(worker = %worker_name, "running warmup job");
                    } else {
                        tracing::info!(worker = %worker_name, id = %job.id, root_id = %job_root, "fetched job {}, root job: {}", job.id, job_root);
                    }

                    let job_dir = format!("{worker_dir}/{}", job.id);

                    DirBuilder::new()
                        .create(&job_dir)
                        .await
                        .expect("could not create job dir");

                    let same_worker = job.same_worker;

                    let target = &format!("{job_dir}/shared");

                    if same_worker && job.parent_job.is_some() {
                        let parent_flow = job.parent_job.unwrap();
                        let parent_shared_dir = format!("{worker_dir}/{parent_flow}/shared");
                        if metadata(&parent_shared_dir).await.is_err() {
                            DirBuilder::new()
                                .recursive(true)
                                .create(&parent_shared_dir)
                                .await
                                .expect("could not create parent shared dir");
                        }
                        symlink(&parent_shared_dir, target)
                            .await
                            .expect("could not symlink target");
                    } else {
                        DirBuilder::new()
                            .create(target)
                            .await
                            .expect("could not create shared dir");
                    }

                    let authed_client = AuthedClientBackgroundTask { base_internal_url: base_internal_url.to_string(), token, workspace: job.workspace_id.to_string(), client: OnceCell::new() };
                    let is_flow = job.job_kind == JobKind::Flow || job.job_kind == JobKind::FlowPreview || job.job_kind == JobKind::FlowDependencies;

                    if let Some(err) = handle_queued_job(
                        job.clone(),
                        db,
                        &authed_client,
                        &worker_name,
                        &worker_dir,
                        &job_dir,
                        metrics.clone(),
                        same_worker_tx.clone(),
                        base_internal_url,
                        rsmq.clone(),
                        job_completed_tx.clone(),
                    )
                    .await
                    .err()
                    {
                        handle_job_error(
                            db,
                            &authed_client.get_authed().await,
                            job,
                            err,
                            metrics,
                            false,
                            same_worker_tx.clone(),
                            &worker_dir,
                            base_internal_url,
                            rsmq.clone()
                        )
                        .await;
                    };

                    let duration = _timer.stop_and_record();
                    worker_execution_duration_counter
                        .get(&language)
                        .expect("no timer found").inc_by(duration);

                    if !*KEEP_JOB_DIR && !(is_flow && same_worker) {
                        let _ = tokio::fs::remove_dir_all(job_dir).await;
                    }
                }
                Ok(None) => {
                    let _timer =  if *METRICS_ENABLED { Some(Instant::now()) } else { None };
                    tokio::time::sleep(Duration::from_millis(*SLEEP_QUEUE)).await;
                    _timer.map(|timer| {
                        let duration = timer.elapsed().as_secs_f64();
                        worker_sleep_duration_counter.inc_by(duration);
                    });
                }
                Err(err) => {
                    tracing::error!(worker = %worker_name, "run_worker: pulling jobs: {}", err);
                }
            };

            false
        }
        .instrument(trace_span!("worker_loop_iteration"))
        .await;
        if do_break {
            break;
        }
    }

    drop(job_completed_tx);

    send_result.await.expect("send result failed");

}

pub async fn create_barrier_for_all_workers(num_workers: u32, sync_barrier: Arc<RwLock<Option<tokio::sync::Barrier>>>) {
    tracing::debug!("acquiring write lock");
    let mut barrier = sync_barrier.write().await;
    *barrier = Some(tokio::sync::Barrier::new(num_workers as usize));
    drop(barrier);
    tracing::debug!("dropped write lock");
    if let Some(b) = sync_barrier.read().await.as_ref() {
        tracing::debug!("leader worker waiting for barrier");
        b.wait().await;
        tracing::debug!("leader worker done waiting for barrier");
    };
    let mut barrier = sync_barrier.write().await;
    *barrier = None;
    tracing::debug!("leader worker done waiting for");
}
pub async fn handle_job_error<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    db: &Pool<Postgres>,
    client: &AuthedClient,
    job: QueuedJob,
    err: Error,
    metrics: Option<Metrics>,
    unrecoverable: bool,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    base_internal_url: &str,
    rsmq: Option<R>,
) {
    let err = match err {
        Error::JsonErr(err) => err,
        _ => json!({"message": err.to_string(), "name": "InternalErr"}),
    };

    let rsmq_2 = rsmq.clone();
    let update_job_future = || {
        add_completed_job_error(
            db,
            &job,
            format!("Unexpected error during job execution:\n{err}"),
            err.clone(),
            metrics.clone(),
            rsmq_2,
        )
    };

    let update_job_future = if job.is_flow_step || job.job_kind == JobKind::FlowPreview || job.job_kind == JobKind::Flow {
        let (flow, job_status_to_update, update_job_future) = if let Some(parent_job_id) = job.parent_job {
            let _ = update_job_future().await;
            (parent_job_id, job.id, None)
        } else {
            (job.id, Uuid::nil(), Some(update_job_future))
        };

        let updated_flow = update_flow_status_after_job_completion(
            db,
            client,
            flow,
            &job_status_to_update,
            &job.workspace_id,
            false,
            json!({ "error": err }),
            metrics.clone(),
            unrecoverable,
            same_worker_tx,
            worker_dir,
            None,
            base_internal_url,
            rsmq.clone(),
        )
        .await;

        if let Err(err) = updated_flow {
            if let Some(parent_job_id) = job.parent_job {
                if let Ok(mut tx) = db.begin().await {
                    if let Ok(Some(parent_job)) =
                        get_queued_job(parent_job_id, &job.workspace_id, &mut tx).await
                    {
                        let _ = add_completed_job_error(
                            db,
                            &parent_job,
                            format!("Unexpected error during flow job error handling:\n{err}"),
                            json!({"message": err.to_string(), "name": "InternalErr"}),
                            metrics.clone(),
                            rsmq,
                        )
                        .await;
                    }
                }
            }
        }

        update_job_future
    } else {
        Some(update_job_future)
    };
    if let Some(f) = update_job_future {
        let _ = f().await;
    }
    tracing::error!(job_id = %job.id, "error handling job: {err:#?} {} {} {}", job.id, job.workspace_id, job.created_by);
}

async fn insert_initial_ping(
    worker_instance: &str,
    worker_name: &str,
    ip: &str,
    db: &Pool<Postgres>,
) {
    sqlx::query!(
        "INSERT INTO worker_ping (worker_instance, worker, ip) VALUES ($1, $2, $3)",
        worker_instance,
        worker_name,
        ip
    )
    .execute(db)
    .await
    .expect("insert worker_ping initial value");
}


fn extract_error_value(log_lines: &str, i: i32) -> serde_json::Value {
    return json!({"message": format!("ExitCode: {i}, last log lines:\n{}", log_lines.to_string().trim().to_string()), "name": "ExecutionErr"});
}

#[derive(Debug, Clone)]
struct JobCompleted {
    job: QueuedJob,
    result: serde_json::Value,
    logs: String,
}
#[tracing::instrument(level = "trace", skip_all)]
async fn handle_queued_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    job: QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    worker_name: &str,
    worker_dir: &str,
    job_dir: &str,
    metrics: Option<Metrics>,
    same_worker_tx: Sender<Uuid>,
    base_internal_url: &str,
    rsmq: Option<R>,
    job_completed_tx: Sender<JobCompleted>,
) -> windmill_common::error::Result<()> {
    if job.canceled {
        return Err(Error::JsonErr(canceled_job_to_result(&job)))?;
    }
    if let Some(e) = job.pre_run_error {
        return Err(Error::ExecutionErr(e));
    }

    match job.job_kind {
        JobKind::FlowPreview | JobKind::Flow => {
            let args = job.args.clone().unwrap_or(Value::Null);
            handle_flow(
                &job,
                db,
                &client.get_authed().await,
                args,
                same_worker_tx,
                worker_dir,
                base_internal_url,
                rsmq,
            )
            .await?;
        }
        _ => {
            let mut logs = "".to_string();
            // println!("handle queue {:?}",  SystemTime::now());
            if let Some(log_str) = &job.logs {
                logs.push_str(&log_str);
            }

            if job.is_flow_step {
                update_flow_status_in_progress(
                    db,
                    &job.workspace_id,
                    job.parent_job
                        .ok_or_else(|| Error::InternalErr(format!("expected parent job")))?,
                    job.id,
                )
                .await?;
            }

            tracing::debug!(
                worker = %worker_name,
                job_id = %job.id,
                workspace_id = %job.workspace_id,
                "handling job {}",
                job.id
            );

            logs.push_str(&format!("job {} on worker {}\n", &job.id, &worker_name));
            let result = match job.job_kind {
                JobKind::Dependencies => {
                    handle_dependency_job(&job, &mut logs, job_dir, db, worker_name, worker_dir).await
                }
                JobKind::FlowDependencies => {
                    handle_flow_dependency_job(&job, &mut logs, job_dir, db, worker_name, worker_dir)
                        .await
                        .map(|()| Value::Null)
                }
                JobKind::Identity => match job.args.clone() {
                    Some(Value::Object(args))
                        if args.len() == 1 && args.contains_key("previous_result") =>
                    {
                        Ok(args.get("previous_result").unwrap().clone())
                    }
                    args @ _ => Ok(args.unwrap_or_else(|| Value::Null)),
                },
                _ => {
                    handle_code_execution_job(
                        &job,
                        db,
                        client,
                        job_dir,
                        worker_dir,
                        &mut logs,
                        base_internal_url,
                        worker_name
                    )
                    .await
                }
            };

            //it's a test job, no need to update the db
            if job.workspace_id == "" {
                return Ok(());
            }
            let client = &client.get_authed().await;
            match result {
                Ok(r) => {
                    // println!("bef completed job{:?}",  SystemTime::now());
                    if job.is_flow_step {
                        add_completed_job(db, &job, true, false, r.clone(), logs, rsmq.clone()).await?;
                        if let Some(parent_job) = job.parent_job {
                            update_flow_status_after_job_completion(
                                db,
                                client,
                                parent_job,
                                &job.id,
                                &job.workspace_id,
                                true,
                                r,
                                metrics.clone(),
                                false,
                                same_worker_tx.clone(),
                                worker_dir,
                                None,
                                base_internal_url,
                                rsmq.clone()
                            )
                            .await?;
                        }
                    } else {
                        // in the happy path and if job not a flow step, we can delegate updating the completed job in the background
                        job_completed_tx.send(JobCompleted{job,result:r,logs:logs,}).await.expect("send job completed");
                        
                    }
                }
                Err(e) => {
                    let error_value = match e {
                        Error::ExitStatus(i) => {
                            let res = read_result(job_dir).await.ok();

                            if res.is_some() && res.clone().unwrap().is_object() {
                                res.unwrap()
                            } else {
                                let last_10_log_lines = logs
                                    .lines()
                                    .skip(logs.lines().count().max(13) - 13)
                                    .join("\n")
                                    .to_string()
                                    .replace("\n\n", "\n");

                                let log_lines = last_10_log_lines
                                    .split("CODE EXECUTION ---")
                                    .last()
                                    .unwrap_or(&logs);

                                extract_error_value(log_lines, i)
                            }
                        }
                        err @ _ => {
                            json!({"message": format!("error during execution of the script:\n{}", err), "name": "ExecutionErr"})
                        }
                    };

                    let result =
                        add_completed_job_error(db, &job, logs, error_value, metrics.clone(), rsmq.clone())
                            .await?;
                    if job.is_flow_step {
                        if let Some(parent_job) = job.parent_job {
                            update_flow_status_after_job_completion(
                                db,
                                client,
                                parent_job,
                                &job.id,
                                &job.workspace_id,
                                false,
                                result,
                                metrics,
                                false,
                                same_worker_tx,
                                worker_dir,
                                None,
                                base_internal_url,
                                rsmq
                            )
                            .await?;
                        }
                    }
                }
            };
        }
    }
    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn write_file(dir: &str, path: &str, content: &str) -> error::Result<File> {
    let path = format!("{}/{}", dir, path);
    let mut file = File::create(&path).await?;
    file.write_all(content.as_bytes()).await?;
    file.flush().await?;
    Ok(file)
}

#[async_recursion]
async fn transform_json_value(
    name: &str,
    client: &AuthedClient,
    workspace: &str,
    v: Value,
) -> error::Result<Value> {
    match v {
        Value::String(y) if y.starts_with("$var:") => {
            let path = y.strip_prefix("$var:").unwrap();
            let v = client.get_client()
                .get_variable(workspace, path, Some(true))
                .await
                .map_err(|_| Error::NotFound(format!("Variable {path} not found for `{name}`")))
                .map(|v| v.into_inner())?
                .value
                .unwrap_or_else(|| String::new());
            Ok(Value::String(v))
        }
        Value::String(y) if y.starts_with("$res:") => {
            let path = y.strip_prefix("$res:").unwrap();
            if path.split("/").count() < 2 {
                return Err(Error::InternalErr(format!(
                    "Argument `{name}` is an invalid resource path: {path}",
                )));
            }
            let v = client.get_client()
                .get_resource_value(workspace, path)
                .await
                .map_err(|_| Error::NotFound(format!("Resource {path} not found for `{name}`")))?
                .into_inner();
            transform_json_value(name, client, workspace, v).await
        }
        Value::Object(mut m) => {
            for (a, b) in m.clone().into_iter() {
                m.insert(
                    a.clone(),
                    transform_json_value(&a, client, workspace, b).await?,
                );
            }
            Ok(Value::Object(m))
        }
        a @ _ => Ok(a),
    }
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_code_execution_job(
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    job_dir: &str,
    worker_dir: &str,
    logs: &mut String,
    base_internal_url: &str,
    worker_name: &str

) -> error::Result<serde_json::Value> {
    let (inner_content, requirements_o, language) = match job.job_kind {
        JobKind::Preview | JobKind::Script_Hub => (
            job.raw_code
                .clone()
                .unwrap_or_else(|| "no raw code".to_owned()),
            job.raw_lock.clone(),
            job.language.to_owned(),
        ),
        JobKind::Script => sqlx::query_as::<_, (String, Option<String>, Option<ScriptLang>)>(
            "SELECT content, lock, language FROM script WHERE hash = $1 AND workspace_id = $2",
        )
        .bind(&job.script_hash.unwrap_or(ScriptHash(0)).0)
        .bind(&job.workspace_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| Error::InternalErr(format!("expected content and lock")))?,
        _ => unreachable!(
            "handle_code_execution_job should never be reachable with a non-code execution job"
        ),
    };
    let lang_str = job
        .language
        .as_ref()
        .map(|x| format!("{x:?}"))
        .unwrap_or_else(|| "NO_LANG".to_string());

    tracing::debug!(
        worker_name = %worker_name,
        job_id = %job.id,
        workspace_id = %job.workspace_id,
        "started {} job {}",
        &lang_str,
        job.id
    );

    let shared_mount = if job.same_worker && job.language != Some(ScriptLang::Deno) {
        format!(
            r#"
mount {{
    src: "{job_dir}/shared"
    dst: "/tmp/shared"
    is_bind: true
    rw: true
}}
        "#
        )
    } else {
        "".to_string()
    };

    // println!("handle lang job {:?}",  SystemTime::now());

    let result: error::Result<serde_json::Value> = match language {
        None => {
            return Err(Error::ExecutionErr(
                "Require language to be not null".to_string(),
            ))?;
        }
        Some(ScriptLang::Python3) => {
            handle_python_job(
                requirements_o,
                job_dir,
                worker_dir,
                worker_name,
                job,
                logs,
                db,
                client,
                &inner_content,
                &shared_mount,
                base_internal_url,
            )
            .await
        }
        Some(ScriptLang::Deno) => {
            handle_deno_job(
                logs,
                job,
                db,
                client,
                job_dir,
                &inner_content,
                base_internal_url,
                worker_name
            )
            .await
        }
        Some(ScriptLang::Go) => {
            handle_go_job(
                logs,
                job,
                db,
                client,
                &inner_content,
                job_dir,
                requirements_o,
                &shared_mount,
                base_internal_url,
                worker_name
            )
            .await
        }
        Some(ScriptLang::Bash) => {
            handle_bash_job(
                logs,
                job,
                db,
                client,
                &inner_content,
                job_dir,
                &shared_mount,
                base_internal_url,
                worker_name
            )
            .await
        }
    };
    tracing::info!(
        worker_name = %worker_name,
        job_id = %job.id,
        workspace_id = %job.workspace_id,
        is_ok = result.is_ok(),
        "finished {} job {}",
        &lang_str,
        job.id
    );
    // println!("handled job: {:?}",  SystemTime::now());

    result
}


#[tracing::instrument(level = "trace", skip_all)]
async fn handle_bash_job(
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    content: &str,
    job_dir: &str,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
) -> Result<serde_json::Value, Error> {
    logs.push_str("\n\n--- BASH CODE EXECUTION ---\n");
    set_logs(logs, &job.id, db).await;
    write_file(job_dir, "main.sh", &format!("{content}\nsleep 0.02")).await?;
    let token = client.get_token().await;
    let mut reserved_variables = get_reserved_variables(job, &token, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let hm = match job.args {
        Some(Value::Object(ref hm)) => hm.clone(),
        _ => serde_json::Map::new(),
    };
    let args_owned = windmill_parser_bash::parse_bash_sig(&content)?
        .args
        .iter()
        .map(|arg| {
            hm.get(&arg.name)
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => serde_json::to_string(v).ok(),
                })
                .unwrap_or_else(String::new)
        })
        .collect::<Vec<String>>();
    let args = args_owned.iter().map(|s| &s[..]).collect::<Vec<&str>>();

    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_BASH_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )
        .await?;
        let mut cmd_args = vec!["--config", "run.config.proto", "--", "/bin/bash", "main.sh"];
        cmd_args.extend(args);
        Command::new(NSJAIL_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        let mut cmd_args = vec!["main.sh"];
        cmd_args.extend(&args);
        Command::new("/bin/bash")
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    handle_child(&job.id, db, logs,  child, !*DISABLE_NSJAIL, worker_name, &job.workspace_id, "bash run").await?;
    //for now bash jobs have an empty result object
    Ok(serde_json::json!(logs
        .lines()
        .last()
        .map(|x| x.to_string())
        .unwrap_or_else(String::new)))
}

fn get_common_deno_proc_envs(token: &str, base_internal_url: &str) -> HashMap<String, String> {
    let hostname_base = BASE_URL.split("://").last().unwrap_or("localhost");
    let hostname_internal = base_internal_url.split("://").last().unwrap_or("localhost");
    let deno_auth_tokens_base = DENO_AUTH_TOKENS.as_str();
    let deno_auth_tokens =
        format!("{token}@{hostname_base};{token}@{hostname_internal}{deno_auth_tokens_base}",);

    let mut deno_envs: HashMap<String, String> = HashMap::from([
        (String::from("PATH"), PATH_ENV.clone()),
        (String::from("DENO_AUTH_TOKENS"), deno_auth_tokens),
        (String::from("BASE_INTERNAL_URL"), base_internal_url.to_string()),
        (String::from("NO_COLOR"), String::from("true")),
    ]);

    if let Some(ref s) = *NPM_CONFIG_REGISTRY {
        deno_envs.insert(String::from("NPM_CONFIG_REGISTRY"), s.clone());
    }
    return deno_envs;
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_deno_job(
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    job_dir: &str,
    inner_content: &String,
    base_internal_url: &str,
    worker_name: &str
) -> error::Result<serde_json::Value> {

    // let mut start = Instant::now();
    logs.push_str("\n\n--- DENO CODE EXECUTION ---\n");

    let logs_to_set = logs.clone();
    let id = job.id.clone();
    let db2 = db.clone();

    let set_logs_f = async {
        set_logs(&logs_to_set, &id, &db2).await;
        Ok(()) as error::Result<()>
    };
    
    let write_main_f = write_file(job_dir, "main.ts", inner_content);

    let write_wrapper_f = async {
        // let mut start = Instant::now();
        let spread =  windmill_parser_ts::parse_deno_signature(inner_content, true)?.args.into_iter().map(|x| x.name).join(",");
        // logs.push_str(format!("infer args: {:?}\n", start.elapsed().as_micros()).as_str());
        let wrapper_content: String = format!(
            r#"
    import {{ main }} from "./main.ts";

    const args = await Deno.readTextFile("args.json")
        .then(JSON.parse)
        .then(({{ {spread} }}) => [ {spread} ])

    async function run() {{
        let res: any = await main(...args);
        const res_json = JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value);
        await Deno.writeTextFile("result.json", res_json);
        Deno.exit(0);
    }}
    run().catch(async (e) => {{
        await Deno.writeTextFile("result.json", JSON.stringify({{ message: e.message, name: e.name, stack: e.stack }}));
        Deno.exit(1);
    }});
    "#,
        );
        write_file(job_dir, "wrapper.ts", &wrapper_content).await?;
        Ok(()) as error::Result<()>
    };

    let write_import_map_f = async {
        let w_id = job.workspace_id.clone();
        let script_path_split = job.script_path().split("/");
        let script_path_parts_len = script_path_split.clone().count();
        let mut relative_mounts = "".to_string();
        for c in 0..script_path_parts_len {
            relative_mounts += ",\n          ";
            relative_mounts += &format!("\"./{}\": \"{base_internal_url}/api/w/{w_id}/scripts/raw/p/{}{}\"",
                (0..c).map(|_| "../").join(""),
                &script_path_split.clone().take(script_path_parts_len - c - 1).join("/"),
                if c == script_path_parts_len - 1 { "" } else { "/" },
            );
        }
        let import_map = format!(
            r#"{{
            "imports": {{
              "{base_internal_url}/api/w/{w_id}/scripts/raw/p/": "{base_internal_url}/api/w/{w_id}/scripts/raw/p/",
              "{base_internal_url}": "{base_internal_url}/api/w/{w_id}/scripts/raw/p/",
              "/": "{base_internal_url}/api/w/{w_id}/scripts/raw/p/",
              "./wrapper.ts": "./wrapper.ts",
              "./main.ts": "./main.ts"{relative_mounts}
            }}
          }}"#,
        );
        write_file(job_dir, "import_map.json", &import_map).await?;
        Ok(()) as error::Result<()>
    };

    let reserved_variables_args_out_f = async {
        let client = client.get_authed().await;
        let args_and_out_f = async {
            create_args_and_out_file(&client, job, job_dir).await?;
            Ok(()) as Result<()>
        };
        let reserved_variables_f = async {
            let mut vars = get_reserved_variables(job, &client.token, db).await?;
            vars.insert("RUST_LOG".to_string(), "info".to_string());
            Ok(vars) as Result<HashMap<String, String>>
        };
        let (_, reserved_variables) = tokio::try_join!(args_and_out_f, reserved_variables_f)?;
        Ok((reserved_variables, client.token)) as error::Result<(HashMap<String, String>, String)>
    };

    let (_, (reserved_variables, token), _, _, _) = tokio::try_join!(
        set_logs_f,
        reserved_variables_args_out_f,
        write_main_f,
        write_wrapper_f,
        write_import_map_f)?;

    let common_deno_proc_envs = get_common_deno_proc_envs(&token, base_internal_url);

    //do not cache local dependencies
    let reload = format!("--reload={base_internal_url}");
    let child = async {
            let script_path = format!("{job_dir}/wrapper.ts");
            let import_map_path = format!("{job_dir}/import_map.json");
            let mut args = Vec::with_capacity(12);
            args.push("run");
            args.push("--no-check");
            args.push("--import-map");
            args.push(&import_map_path);
            args.push(&reload);
            args.push("--unstable");
            if let Some(deno_flags) = DENO_FLAGS.as_ref() {
                for flag in deno_flags {
                    args.push(flag);
                }
            } else if !*DISABLE_NSJAIL {
                args.push("--allow-net");
                args.push("--allow-read=./");
                args.push("--allow-write=./");
                args.push("--allow-env");
            } else {
                args.push("-A");
            }
            args.push(&script_path);
            Command::new(DENO_PATH.as_str())
                .current_dir(job_dir)
                .env_clear()
                .envs(reserved_variables)
                .envs(common_deno_proc_envs)
                .env("DENO_DIR", DENO_CACHE_DIR)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
    }
    .await?;
    // logs.push_str(format!("prepare: {:?}\n", start.elapsed().as_micros()).as_str());
    // start = Instant::now();
    handle_child(&job.id, db, logs, child, false, worker_name, &job.workspace_id, "deno run").await?;
    // logs.push_str(format!("execute: {:?}\n", start.elapsed().as_millis()).as_str());
    if let Err(e) = tokio::fs::remove_dir_all(format!("{DENO_CACHE_DIR}/gen/file/{job_dir}")).await {
        tracing::error!("failed to remove deno gen tmp cache dir: {}", e);
    }
    read_result(job_dir).await
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn create_args_and_out_file(
    client: &AuthedClient,
    job: &QueuedJob,
    job_dir: &str,
) -> Result<(), Error> {
    let args = if let Some(args) = &job.args {
        Some(transform_json_value("args", client, &job.workspace_id, args.clone()).await?)
    } else {
        None
    };
    let ser_args = serde_json::to_string(&args).map_err(|e| Error::ExecutionErr(e.to_string()))?;
    write_file(job_dir, "args.json", &ser_args).await?;
    write_file(job_dir, "result.json", "").await?;
    Ok(())
}




#[tracing::instrument(level = "trace", skip_all)]
async fn handle_dependency_job(
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
) -> error::Result<serde_json::Value> {
    let content = capture_dependency_job(
        &job.id,
        job.language.as_ref().map(|v| Ok(v)).unwrap_or_else(|| {
            Err(Error::InternalErr(
                "Job Language required for dependency jobs".to_owned(),
            ))
        })?,
        job.raw_code
            .as_ref()
            .map(|a| a.as_str())
            .unwrap_or_else(|| "no raw code"),
        logs,
        job_dir,
        db,
        worker_name,
        &job.workspace_id,
        worker_dir,
    )
    .await;
    match content {
        Ok(content) => {
            sqlx::query!(
                "UPDATE script SET lock = $1 WHERE hash = $2 AND workspace_id = $3",
                &content,
                &job.script_hash.unwrap_or(ScriptHash(0)).0,
                &job.workspace_id
            )
            .execute(db)
            .await?;
            Ok(json!({ "success": "Successful lock file generation", "lock": content }))
        }
        Err(error) => {
            sqlx::query!(
                "UPDATE script SET lock_error_logs = $1 WHERE hash = $2 AND workspace_id = $3",
                &format!("{logs}\n{error}"),
                &job.script_hash.unwrap_or(ScriptHash(0)).0,
                &job.workspace_id
            )
            .execute(db)
            .await?;
            Err(Error::ExecutionErr(format!("Error locking file: {error}")))?
        }
    }
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_flow_dependency_job(
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
) -> error::Result<()> {
    let path = job.script_path.clone().ok_or_else(|| {
        error::Error::InternalErr(
            "Cannot resolve flow dependencies for flow without path".to_string(),
        )
    })?;
    let raw_flow = job.raw_flow.clone().map(|v| Ok(v)).unwrap_or_else(|| {
        Err(Error::InternalErr(
            "Flow Dependency requires raw flow".to_owned(),
        ))
    })?;
    let mut flow = serde_json::from_value::<FlowValue>(raw_flow).map_err(to_anyhow)?;
    let mut new_flow_modules = Vec::new();
    for mut e in flow.modules.into_iter() {
        let FlowModuleValue::RawScript { lock: _, path, content, language, input_transforms} = e.value else {
            new_flow_modules.push(e);
            continue;
        };
        // sync with windmill-api/scripts
        let dependencies = match language {
            ScriptLang::Python3 => windmill_parser_py::parse_python_imports(&content)?.join("\n"),
            _ => content.clone(),
        };
        let new_lock = capture_dependency_job(
            &job.id,
            &language,
            &dependencies,
            logs,
            job_dir,
            db,
            worker_name,
            &job.workspace_id,
            worker_dir,
        )
        .await;
        match new_lock {
            Ok(new_lock) => {
                e.value = FlowModuleValue::RawScript {
                    lock: Some(new_lock),
                    path: path,
                    input_transforms,
                    content,
                    language,
                };
                new_flow_modules.push(e);
                continue;
            }
            Err(error) => {
                // TODO: Record flow raw script error lock logs
                tracing::warn!(
                    path = path,
                    language = ?language,
                    error = ?error,
                    logs = ?logs,
                    "Failed to generate flow lock for raw script"
                );
                e.value = FlowModuleValue::RawScript {
                    lock: None,
                    path: path,
                    input_transforms,
                    content,
                    language,
                };
                new_flow_modules.push(e);
                continue;
            }
        }
    }
    flow.modules = new_flow_modules;
    let new_flow_value = serde_json::to_value(flow).map_err(to_anyhow)?;

    // Re-check cancelation to ensure we don't accidentially override a flow.
    if sqlx::query_scalar!("SELECT canceled FROM queue WHERE id = $1", job.id)
        .fetch_optional(db)
        .await
        .map(|v| Some(true) == v)
        .unwrap_or_else(|err| {
            tracing::error!(%job.id, %err, "error checking cancelation for job {0}: {err}", job.id);
            false
        })
    {
        return Ok(());
    }

    sqlx::query!(
        "UPDATE flow SET value = $1 WHERE path = $2 AND workspace_id = $3",
        new_flow_value,
        path,
        job.workspace_id
    )
    .execute(db)
    .await?;
    Ok(())
}

async fn capture_dependency_job(
    job_id: &Uuid,
    job_language: &ScriptLang,
    job_raw_code: &str,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    w_id: &str,
    worker_dir: &str,
) -> error::Result<String> {
    match job_language {
        ScriptLang::Python3 => {
            create_dependencies_dir(job_dir).await;
            let req = pip_compile(job_id, job_raw_code, logs, job_dir, db, worker_name, w_id).await;
            // install the dependencies to pre-fill the cache
            if let Ok(req) = req.as_ref() {
                handle_python_reqs(
                    req
                        .split("\n")
                        .filter(|x| !x.starts_with("--"))
                        .collect(),
                    job_id,
                    w_id,
                    logs,
                    db,
                    worker_name,
                    job_dir,
                    worker_dir,
                )
                .await?;
            }
            req
        }
        ScriptLang::Go => {
            install_go_dependencies(
                job_id,
                job_raw_code,
                logs,
                job_dir,
                db,
                false,
                false,
                false,
                worker_name,
                w_id
            )
            .await
        }
        ScriptLang::Deno => {
            Ok(String::new())
            // generate_deno_lock(job_id, job_raw_code, logs, job_dir, db, timeout).await
        }
        ScriptLang::Bash => Ok("".to_owned()),
    }
}


#[tracing::instrument(level = "trace", skip_all)]
pub async fn get_reserved_variables(
    job: &QueuedJob,
    token: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> Result<HashMap<String, String>, Error> {
    let flow_path = if let Some(uuid) = job.parent_job {
        sqlx::query_scalar!("SELECT script_path FROM queue WHERE id = $1", uuid)
            .fetch_optional(db)
            .await?
            .flatten()
    } else {
        None
    };

    let variables = variables::get_reserved_variables(
        &job.workspace_id,
        token,
        &job.email,
        &job.created_by,
        &job.id.to_string(),
        &job.permissioned_as,

        job.script_path.clone(),
        job.parent_job.map(|x| x.to_string()),
        flow_path,
        job.schedule_path.clone(),
    );
    Ok(variables
        .into_iter()
        .map(|rv| (rv.name, rv.value))
        .collect())
}

async fn get_mem_peak(pid: Option<u32>, nsjail: bool) -> i32 {
    if pid.is_none() {
        return -1
    }
    let pid = if nsjail {
        // This is a bit hacky, but the process id of the nsjail process is the pid of nsjail + 1. 
        // Ideally, we would get the number from fork() itself. This works in MOST cases.
        pid.unwrap() + 1
    } else {
        pid.unwrap()
    };
    
    if let Ok(file) = File::open(format!("/proc/{}/status", pid)).await {
        let mut lines = BufReader::new(file).lines();
        while let Some(line) = lines.next_line().await.unwrap_or(None) {
            if line.starts_with("VmHWM:") {
                return line.split_whitespace().nth(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(-1);
            };
        }
        -2
    } else {
        -3
    }
}
/// - wait until child exits and return with exit status
/// - read lines from stdout and stderr and append them to the "queue"."logs"
///   quitting early if output exceedes MAX_LOG_SIZE characters (not bytes)
/// - update the `last_line` and `logs` strings with the program output
/// - update "queue"."last_ping" every five seconds
/// - kill process if we exceed timeout or "queue"."canceled" is set
#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_child(
    job_id: &Uuid,
    db: &Pool<Postgres>,
    logs: &mut String,
    mut child: Child,
    nsjail: bool,
    worker_name: &str,
    _w_id: &str,
    child_name: &str,
) -> error::Result<()> {
    let start = Instant::now();
    let update_job_interval = Duration::from_millis(500);
    let write_logs_delay = Duration::from_millis(500);

    let pid = child.id();
    #[cfg(target_os = "linux")]
    if let Some(pid) = pid {
        //set the highest oom priority
        let mut file = File::create(format!("/proc/{pid}/oom_score_adj")).await?;
        let _ = file.write_all(b"1000").await;
    } else {
        tracing::info!("could not get child pid");
    }
    let (set_too_many_logs, mut too_many_logs) = watch::channel::<bool>(false);
    let (tx, mut rx) = broadcast::channel::<()>(3);
    let mut rx2 = tx.subscribe();


    let output = child_joined_output_stream(&mut child);

    let job_id = job_id.clone();


    /* the cancellation future is polled on by `wait_on_child` while
     * waiting for the child to exit normally */
    let update_job = async {
        let db = db.clone();

        let mut interval = interval(update_job_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let mut i = 1;
        loop {
            tokio::select!(
                _ = rx.recv() => break,
                _ = interval.tick() => {
                    // update the last_ping column every 5 seconds
                    i+=1;
                    if i % 10 == 0 {
                        sqlx::query!(
                            "UPDATE worker_ping SET ping_at = now() WHERE worker = $1",
                            &worker_name
                        )
                        .execute(&db)
                        .await
                        .expect("update worker ping");
                    }
                    let mem_peak = get_mem_peak(pid, nsjail).await;
                    tracing::info!("{job_id} still running. mem peak: {}kB", mem_peak);
                    let mem_peak = if mem_peak > 0 { Some(mem_peak) } else { None };
                    if sqlx::query_scalar!("UPDATE queue SET mem_peak = GREATEST($1, mem_peak), last_ping = now() WHERE id = $2 RETURNING canceled", mem_peak, job_id)
                        .fetch_optional(&db)
                        .await
                        .map(|v| Some(true) == v)
                        .unwrap_or_else(|err| {
                            tracing::error!(%job_id, %err, "error checking cancelation for job {job_id}: {err}");
                            false
                        })
                    {
                        break;
                    }
                },
            );
        }
    };

    #[derive(PartialEq, Debug)]
    enum KillReason {
        TooManyLogs,
        Timeout,
        Cancelled,
    }
    /* a future that completes when the child process exits */
    let wait_on_child = async {
        let db = db.clone();

        #[cfg(not(feature = "enterprise"))]
        let timeout_duration = *TIMEOUT_DURATION;

        #[cfg(feature = "enterprise")]
        let premium_workspace = *CLOUD_HOSTED && sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", _w_id)
            .fetch_one(&db)
            .await
            .map_err(|e| {
                tracing::error!(%e, "error getting premium workspace for job {job_id}: {e}");
            }).unwrap_or(false);
        
        #[cfg(feature = "enterprise")]
        let timeout_duration = if premium_workspace {
            *TIMEOUT_DURATION*6 //30mins
        } else {
            *TIMEOUT_DURATION
        };

        let kill_reason = tokio::select! {
            biased;
            result = child.wait() => return result.map(Ok),
            Ok(()) = too_many_logs.changed() => KillReason::TooManyLogs,
            _ = sleep(timeout_duration) => KillReason::Timeout,
            _ = update_job => KillReason::Cancelled,
        };
        tx.send(()).expect("rx should never be dropped");
        drop(tx);

        let set_reason = async {
            if kill_reason == KillReason::Timeout {
                if let Err(err) = sqlx::query(
                    r#"
                       UPDATE queue
                          SET canceled = true
                            , canceled_by = 'timeout'
                            , canceled_reason = $1
                        WHERE id = $2
                    "#,
                )
                .bind(format!("duration > {}", TIMEOUT_DURATION.as_secs()))
                .bind(job_id)
                .execute(&db)
                .await
                {
                    tracing::error!(%job_id, %err, "error setting cancelation reason for job {job_id}: {err}");
                }
            }
        };
        
        /* send SIGKILL and reap child process */
        let (_, kill) = future::join(set_reason, child.kill()).await;
        kill.map(|()| Err(kill_reason))
    };

    /* a future that reads output from the child and appends to the database */
    let lines = async move {
        let max_log_size = if *CLOUD_HOSTED {
            MAX_RESULT_SIZE
        } else {
            usize::MAX
        };
        /* log_remaining is zero when output limit was reached */
        let mut log_remaining = max_log_size.saturating_sub(logs.chars().count());
        let mut result = io::Result::Ok(());
        let mut output = output.take_until(rx2.recv()).boxed();
        /* `do_write` resolves the task, but does not contain the Result.
         * It's useful to know if the task completed. */
        let (mut do_write, mut write_result) = tokio::spawn(ready(())).remote_handle();

        while let Some(line) =  output.by_ref().next().await {

            let do_write_ = do_write.shared();

            let mut read_lines = stream::once(async { line })
                .chain(output.by_ref())
                /* after receiving a line, continue until some delay has passed
                 * _and_ the previous database write is complete */
                .take_until(future::join(sleep(write_logs_delay), do_write_.clone()))
                .boxed();

            /* Read up until an error is encountered,
             * handle log lines first and then the error... */
            let mut joined = String::new();

            while let Some(line) = read_lines.next().await {

                match line {
                    Ok(_) if log_remaining == 0 => (),
                    Ok(line) => {
                        if line.is_empty() {
                            continue;
                        }
                        append_with_limit(&mut joined, &line, &mut log_remaining);
                        if log_remaining == 0 {
                            tracing::info!(%job_id, "Too many logs lines for job {job_id}");
                            let _ = set_too_many_logs.send(true);
                            joined.push_str(&format!(
                                "Job logs or result reached character limit of {MAX_RESULT_SIZE}; killing job."
                            ));
                            /* stop reading and drop our streams fairly quickly */
                            break;
                        }
                    }
                    Err(err) => {
                        result = Err(err);
                        break;
                    }
                }
            }

            logs.push_str(&joined);


            /* Ensure the last flush completed before starting a new one.
             *
             * This shouldn't pause since `take_until()` reads lines until `do_write`
             * resolves. We only stop reading lines before `take_until()` resolves if we reach
             * EOF or a read error.  In those cases, waiting on a database query to complete is
             * fine because we're done. */

            if let Some(Ok(p)) = do_write_
                .then(|()| write_result)
                .await
                .err()
                .map(|err| err.try_into_panic())
            {
                panic::resume_unwind(p);
            }

            (do_write, write_result) = tokio::spawn(append_logs(job_id, joined, db.clone())).remote_handle();

            if let Err(err) = result {
                tracing::error!(%job_id, %err, "error reading output for job {job_id}: {err}");
                break;
            }

            if *set_too_many_logs.borrow() {
                break;
            }

        }

        /* drop our end of the pipe */
        drop(output);

        if let Some(Ok(p)) = do_write
            .then(|()| write_result)
            .await
            .err()
            .map(|err| err.try_into_panic())
        {
            panic::resume_unwind(p);
        }
    }.instrument(trace_span!("child_lines"));


    let (wait_result, _) = tokio::join!(wait_on_child, lines);

    tracing::info!(%job_id, "child process '{child_name}' for {job_id} took {}ms", start.elapsed().as_millis());
    match wait_result {
        _ if *too_many_logs.borrow() => Err(Error::ExecutionErr(format!(
            "logs or result reached limit. (current max size: {MAX_RESULT_SIZE} characters)"
        ))),
        Ok(Ok(status)) => {
            if status.success() {
                Ok(())
            } else if let Some(code) = status.code() {
                Err(error::Error::ExitStatus(code))
            } else {
                Err(error::Error::ExecutionErr(format!(
                    "process terminated by signal: {:#?}, stopped_signal: {:#?}, core_dumped: {}",
                    status.signal(),
                    status.stopped_signal(),
                    status.core_dumped()
                )))
            }
        }
        Ok(Err(kill_reason)) => Err(Error::ExecutionErr(format!(
            "job process killed because {kill_reason:#?}"
        ))),
        Err(err) => Err(Error::ExecutionErr(format!("job process io error: {err}"))),
    }
}

/// takes stdout and stderr from Child, panics if either are not present
///
/// builds a stream joining both stdout and stderr each read line by line
fn child_joined_output_stream(
    child: &mut Child,
) -> impl stream::FusedStream<Item = io::Result<String>> {
    let stderr = child
        .stderr
        .take()
        .expect("child did not have a handle to stdout");

    let stdout = child
        .stdout
        .take()
        .expect("child did not have a handle to stdout");

    let stdout = BufReader::new(stdout).lines();
    let stderr = BufReader::new(stderr).lines();
    stream::select(lines_to_stream(stderr), lines_to_stream(stdout))
}

fn lines_to_stream<R: tokio::io::AsyncBufRead + Unpin>(
    mut lines: tokio::io::Lines<R>,
) -> impl futures::Stream<Item = io::Result<String>> {
    stream::poll_fn(move |cx| {
        std::pin::Pin::new(&mut lines)
            .poll_next_line(cx)
            .map(|result| result.transpose())
    })
}

// as a detail, `BufReader::lines()` removes \n and \r\n from the strings it yields,
// so this pushes \n to thd destination string in each call
fn append_with_limit(dst: &mut String, src: &str, limit: &mut usize) {
    if *limit > 0 {
        dst.push('\n');
    }
    *limit -= 1;

    let src_len = src.chars().count();
    if src_len <= *limit {
        dst.push_str(&src);
        *limit -= src_len;
    } else {
        let byte_pos = src
            .char_indices()
            .skip(*limit)
            .next()
            .map(|(byte_pos, _)| byte_pos)
            .unwrap_or(0);
        dst.push_str(&src[0..byte_pos]);
        *limit = 0;
    }
}



/* TODO retry this? */
#[tracing::instrument(level = "trace", skip_all)]
async fn append_logs(job_id: uuid::Uuid, logs: impl AsRef<str>, db: impl Borrow<Pool<Postgres>>) {
    if logs.as_ref().is_empty() {
        return;
    }

    if let Err(err) = sqlx::query!(
        "UPDATE queue SET logs = concat(logs, $1::text) WHERE id = $2",
        logs.as_ref(),
        job_id,
    )
    .execute(db.borrow())
    .await
    {
        tracing::error!(%job_id, %err, "error updating logs for job {job_id}: {err}");
    }
}
