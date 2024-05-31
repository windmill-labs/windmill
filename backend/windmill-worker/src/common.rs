use async_recursion::async_recursion;
use deno_ast::swc::parser::lexer::util::CharExt;
use futures::Future;
use itertools::Itertools;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use nix::sys::signal::{self, Signal};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use nix::unistd::Pid;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use object_store::path::Path;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{json, Value};
use sqlx::types::Json;
use sqlx::{Pool, Postgres};
use tokio::process::Command;
use tokio::{fs::File, io::AsyncReadExt};
use windmill_common::error::to_anyhow;
use windmill_common::jobs::ENTRYPOINT_OVERRIDE;
#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS;
#[cfg(feature = "parquet")]
use windmill_common::s3_helpers::{
    get_etag_or_empty, LargeFileStorage, ObjectStoreResource, S3Object,
};
use windmill_common::variables::{build_crypt_with_key_suffix, decrypt_value_with_mc};
use windmill_common::worker::{
    get_windmill_memory_usage, get_worker_memory_usage, CLOUD_HOSTED, TMP_DIR, WORKER_CONFIG,
};
use windmill_common::{
    error::{self, Error},
    jobs::QueuedJob,
    variables::ContextualVariable,
};

use anyhow::Result;
use windmill_queue::{append_logs, CanceledBy};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::process::ExitStatusExt;

use std::process::ExitStatus;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    io, panic,
    time::Duration,
};

use tracing::{trace_span, Instrument};
use uuid::Uuid;
use windmill_common::{variables, DB};

#[cfg(feature = "enterprise")]
use windmill_common::job_metrics;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Child,
    sync::{broadcast, watch},
    time::{interval, sleep, Instant, MissedTickBehavior},
};

use futures::{
    future::{self, ready, FutureExt},
    stream, StreamExt,
};

use crate::{
    AuthedClient, AuthedClientBackgroundTask, JOB_DEFAULT_TIMEOUT, MAX_RESULT_SIZE,
    MAX_TIMEOUT_DURATION, MAX_WAIT_FOR_SIGINT, MAX_WAIT_FOR_SIGTERM, ROOT_CACHE_DIR,
};

pub async fn build_args_map<'a>(
    job: &'a QueuedJob,
    client: &AuthedClientBackgroundTask,
    db: &Pool<Postgres>,
) -> error::Result<Option<HashMap<String, Box<RawValue>>>> {
    if let Some(args) = &job.args {
        return transform_json(client, &job.workspace_id, &args.0, &job, db).await;
    }
    return Ok(None);
}

pub async fn build_args_values(
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    db: &Pool<Postgres>,
) -> error::Result<HashMap<String, serde_json::Value>> {
    if let Some(args) = &job.args {
        transform_json_as_values(client, &job.workspace_id, &args.0, &job, db).await
    } else {
        Ok(HashMap::new())
    }
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn create_args_and_out_file(
    client: &AuthedClientBackgroundTask,
    job: &QueuedJob,
    job_dir: &str,
    db: &Pool<Postgres>,
) -> Result<(), Error> {
    if let Some(args) = &job.args {
        if let Some(x) = transform_json(client, &job.workspace_id, &args.0, job, db).await? {
            write_file(
                job_dir,
                "args.json",
                &serde_json::to_string(&x).unwrap_or_else(|_| "{}".to_string()),
            )
            .await?;
        } else {
            write_file(
                job_dir,
                "args.json",
                &serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string()),
            )
            .await?;
        }
    } else {
        write_file(job_dir, "args.json", "{}").await?;
    };

    write_file(job_dir, "result.json", "").await?;
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

#[tracing::instrument(level = "trace", skip_all)]
pub async fn write_file_binary(dir: &str, path: &str, content: &[u8]) -> error::Result<File> {
    let path = format!("{}/{}", dir, path);
    let mut file = File::create(&path).await?;
    file.write_all(content).await?;
    file.flush().await?;
    Ok(file)
}

lazy_static::lazy_static! {
    static ref RE_RES_VAR: Regex = Regex::new(r#"\$(?:var|res|encrypted)\:"#).unwrap();
}

pub async fn transform_json<'a>(
    client: &AuthedClientBackgroundTask,
    workspace: &str,
    vs: &'a HashMap<String, Box<RawValue>>,
    job: &QueuedJob,
    db: &Pool<Postgres>,
) -> error::Result<Option<HashMap<String, Box<RawValue>>>> {
    let mut has_match = false;
    for (_, v) in vs {
        let inner_vs = v.get();
        if (*RE_RES_VAR).is_match(inner_vs) {
            has_match = true;
            break;
        }
    }
    if !has_match {
        return Ok(None);
    }
    let mut r = HashMap::new();
    for (k, v) in vs {
        let inner_vs = v.get();
        if (*RE_RES_VAR).is_match(inner_vs) {
            let value = serde_json::from_str(inner_vs).map_err(|e| {
                error::Error::InternalErr(format!("Error while parsing inner arg: {e:#}"))
            })?;
            let transformed =
                transform_json_value(&k, &client.get_authed().await, workspace, value, job, db)
                    .await?;
            let as_raw = serde_json::from_value(transformed).map_err(|e| {
                error::Error::InternalErr(format!("Error while parsing inner arg: {e:#}"))
            })?;
            r.insert(k.to_string(), as_raw);
        } else {
            r.insert(k.to_string(), v.to_owned());
        }
    }
    Ok(Some(r))
}

pub async fn transform_json_as_values<'a>(
    client: &AuthedClientBackgroundTask,
    workspace: &str,
    vs: &'a HashMap<String, Box<RawValue>>,
    job: &QueuedJob,
    db: &Pool<Postgres>,
) -> error::Result<HashMap<String, serde_json::Value>> {
    let mut r: HashMap<String, serde_json::Value> = HashMap::new();
    for (k, v) in vs {
        let inner_vs = v.get();
        if (*RE_RES_VAR).is_match(inner_vs) {
            let value = serde_json::from_str(inner_vs).map_err(|e| {
                error::Error::InternalErr(format!("Error while parsing inner arg: {e:#}"))
            })?;
            let transformed =
                transform_json_value(&k, &client.get_authed().await, workspace, value, job, db)
                    .await?;
            let as_raw = serde_json::from_value(transformed).map_err(|e| {
                error::Error::InternalErr(format!("Error while parsing inner arg: {e:#}"))
            })?;
            r.insert(k.to_string(), as_raw);
        } else {
            r.insert(
                k.to_string(),
                serde_json::from_str(v.get()).unwrap_or_else(|_| serde_json::Value::Null),
            );
        }
    }
    Ok(r)
}

pub fn parse_npm_config(s: &str) -> (String, Option<String>) {
    let (url, token_opt) = if s.contains(":_authToken=") {
        let split_url = s.split(":_authToken=").collect::<Vec<&str>>();
        let url = split_url
            .get(0)
            .map(|u| u.to_string())
            .unwrap_or("".to_string());
        let token = split_url
            .get(1)
            .map(|t| t.to_string())
            .unwrap_or("".to_string());
        (url, Some(token))
    } else {
        (s.to_owned(), None)
    };
    return (url, token_opt);
}

#[async_recursion]
pub async fn transform_json_value(
    name: &str,
    client: &AuthedClient,
    workspace: &str,
    v: Value,
    job: &QueuedJob,
    db: &Pool<Postgres>,
) -> error::Result<Value> {
    match v {
        Value::String(y) if y.starts_with("$var:") => {
            let path = y.strip_prefix("$var:").unwrap();
            client
                .get_variable_value(path)
                .await
                .map(|x| json!(x))
                .map_err(|e| {
                    Error::NotFound(format!("Variable {path} not found for `{name}`: {e:#}"))
                })
        }
        Value::String(y) if y.starts_with("$res:") => {
            let path = y.strip_prefix("$res:").unwrap();
            if path.split("/").count() < 2 {
                return Err(Error::InternalErr(format!(
                    "Argument `{name}` is an invalid resource path: {path}",
                )));
            }
            client
                .get_resource_value_interpolated::<serde_json::Value>(
                    path,
                    Some(job.id.to_string()),
                )
                .await
                .map_err(|e| {
                    Error::NotFound(format!("Resource {path} not found for `{name}`: {e:#}"))
                })
        }
        Value::String(y) if y.starts_with("$encrypted:") => {
            let encrypted = y.strip_prefix("$encrypted:").unwrap();
            let mut tx = db.begin().await?;
            let mc = build_crypt_with_key_suffix(&mut tx, &job.workspace_id, &job.id.to_string())
                .await?;
            tx.commit().await?;
            decrypt_value_with_mc(encrypted.to_string(), mc)
                .await
                .and_then(|x| {
                    serde_json::from_str(&x).map_err(|e| Error::InternalErr(e.to_string()))
                })

            // let path = y.strip_prefix("$res:").unwrap();
        }
        Value::String(y) if y.starts_with("$") => {
            let flow_path = if let Some(uuid) = job.parent_job {
                sqlx::query_scalar!("SELECT script_path FROM queue WHERE id = $1", uuid)
                    .fetch_optional(db)
                    .await?
                    .flatten()
            } else {
                None
            };

            let variables = variables::get_reserved_variables(
                db,
                &job.workspace_id,
                &client.token,
                &job.email,
                &job.created_by,
                &job.id.to_string(),
                &job.permissioned_as,
                job.script_path.clone(),
                job.parent_job.map(|x| x.to_string()),
                flow_path,
                job.schedule_path.clone(),
                job.flow_step_id.clone(),
                job.root_job.clone().map(|x| x.to_string()),
                None,
            )
            .await;

            let name = y.strip_prefix("$").unwrap();

            let value = variables
                .iter()
                .find(|x| x.name == name)
                .map(|x| x.value.clone())
                .unwrap_or_else(|| y);
            Ok(json!(value))
        }
        Value::Object(mut m) => {
            for (a, b) in m.clone().into_iter() {
                m.insert(
                    a.clone(),
                    transform_json_value(&a, client, workspace, b, job, &db).await?,
                );
            }
            Ok(Value::Object(m))
        }
        a @ _ => Ok(a),
    }
}

pub async fn read_file_content(path: &str) -> error::Result<String> {
    let mut file = File::open(path).await?;
    let mut content = "".to_string();
    file.read_to_string(&mut content).await?;
    Ok(content)
}

pub async fn read_file_bytes(path: &str) -> error::Result<Vec<u8>> {
    let mut file = File::open(path).await?;
    let mut content = Vec::new();
    file.read_to_end(&mut content).await?;
    Ok(content)
}

//this skips more steps than from_str at the cost of being unsafe. The source must ALWAUS gemerate valid json or this can cause UB in the worst case
pub fn unsafe_raw(json: String) -> Box<RawValue> {
    unsafe { std::mem::transmute::<Box<str>, Box<RawValue>>(json.into()) }
}

pub async fn read_file(path: &str) -> error::Result<Box<RawValue>> {
    let content = read_file_content(path).await?;

    if *CLOUD_HOSTED && content.len() > MAX_RESULT_SIZE {
        return Err(error::Error::ExecutionErr("Result is too large for the cloud app (limit 2MB).
        If using this script as part of the flow, use the shared folder to pass heavy data between steps.".to_owned()));
    };

    let r = unsafe_raw(content);
    return Ok(r);
}
pub async fn read_result(job_dir: &str) -> error::Result<Box<RawValue>> {
    return read_file(&format!("{job_dir}/result.json")).await;
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
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
        db,
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
        job.flow_step_id.clone(),
        job.root_job.clone().map(|x| x.to_string()),
        None,
    )
    .await
    .to_vec();

    Ok(build_envs_map(variables).await)
}

pub async fn build_envs_map(context: Vec<ContextualVariable>) -> HashMap<String, String> {
    let mut r: HashMap<String, String> =
        context.into_iter().map(|rv| (rv.name, rv.value)).collect();

    let envs = WORKER_CONFIG.read().await.clone().env_vars;
    for env in envs {
        r.insert(env.0.clone(), env.1.clone());
    }

    r
}

pub fn get_main_override(args: Option<&Json<HashMap<String, Box<RawValue>>>>) -> Option<String> {
    return args
        .map(|x| {
            x.0.get(ENTRYPOINT_OVERRIDE)
                .map(|x| x.get().to_string().replace("\"", ""))
        })
        .flatten();
}

async fn get_mem_peak(pid: Option<u32>, nsjail: bool) -> i32 {
    if pid.is_none() {
        return -1;
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
                return line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(-1);
            };
        }
        -2
    } else {
        // rand::random::<i32>() % 100 // to remove - used to fake memory data on MacOS
        -3
    }
}

pub fn sizeof_val(v: &serde_json::Value) -> usize {
    std::mem::size_of::<serde_json::Value>()
        + match v {
            serde_json::Value::Null => 0,
            serde_json::Value::Bool(_) => 0,
            serde_json::Value::Number(_) => 4, // Incorrect if arbitrary_precision is enabled. oh well
            serde_json::Value::String(s) => s.capacity(),
            serde_json::Value::Array(a) => a.iter().map(sizeof_val).sum(),
            serde_json::Value::Object(o) => o
                .iter()
                .map(|(k, v)| {
                    std::mem::size_of::<String>()
                        + k.capacity()
                        + sizeof_val(v)
                        + std::mem::size_of::<usize>() * 3
                })
                .sum(),
        }
}

pub async fn run_future_with_polling_update_job_poller<Fut, T>(
    job_id: Uuid,
    timeout: Option<i32>,
    db: &DB,
    mem_peak: &mut i32,
    canceled_by_ref: &mut Option<CanceledBy>,
    result_f: Fut,
    worker_name: &str,
    w_id: &str,
) -> error::Result<T>
where
    Fut: Future<Output = anyhow::Result<T>>,
{
    let (tx, rx) = broadcast::channel::<()>(3);

    let update_job = update_job_poller(
        job_id,
        db,
        mem_peak,
        canceled_by_ref,
        || async { 0 },
        worker_name,
        w_id,
        rx,
    );

    let timeout_ms = u64::try_from(
        resolve_job_timeout(&db, &w_id, job_id, timeout)
            .await
            .0
            .as_millis(),
    )
    .unwrap_or(200000);

    let rows = tokio::select! {
        biased;
        result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), result_f) => result
        .map_err(|e| {
            tracing::error!("Query timeout: {}", e);
            Error::ExecutionErr(format!("Query timeout after (>{}s)", timeout_ms/1000))
        })?,
        ex = update_job, if job_id != Uuid::nil() => {
            match ex {
                UpdateJobPollingExit::Done => Err(Error::ExecutionErr("Job cancelled".to_string())).map_err(to_anyhow)?,
                UpdateJobPollingExit::AlreadyCompleted => Err(Error::AlreadyCompleted("Job already completed".to_string())).map_err(to_anyhow)?,
            }
        }
    }?;
    drop(tx);
    Ok(rows)
}

pub enum UpdateJobPollingExit {
    Done,
    AlreadyCompleted,
}

pub async fn update_job_poller<F, Fut>(
    job_id: Uuid,
    db: &DB,
    mem_peak: &mut i32,
    canceled_by_ref: &mut Option<CanceledBy>,
    get_mem: F,
    worker_name: &str,
    w_id: &str,
    mut rx: broadcast::Receiver<()>,
) -> UpdateJobPollingExit
where
    F: Fn() -> Fut,
    Fut: Future<Output = i32>,
{
    let update_job_interval = Duration::from_millis(500);
    if job_id == Uuid::nil() {
        return UpdateJobPollingExit::Done;
    }
    let db = db.clone();

    let mut interval = interval(update_job_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut i = 0;

    #[cfg(feature = "enterprise")]
    let mut memory_metric_id: Result<String, Error> =
        Err(Error::NotFound("not yet initialized".to_string()));

    loop {
        tokio::select!(
            _ = rx.recv() => break,
            _ = interval.tick() => {
                // update the last_ping column every 5 seconds
                i+=1;
                if i % 10 == 0 {
                    let memory_usage = get_worker_memory_usage();
                    let wm_memory_usage = get_windmill_memory_usage();
                    tracing::info!("{worker_name}/{job_id} in {w_id} worker memory snapshot {}kB/{}kB", memory_usage.unwrap_or_default()/1024, wm_memory_usage.unwrap_or_default()/1024);
                    sqlx::query!(
                        "UPDATE worker_ping SET ping_at = now(), current_job_id = $1, current_job_workspace_id = $2, memory_usage = $3, wm_memory_usage = $4 WHERE worker = $5",
                        &job_id,
                        &w_id,
                        memory_usage,
                        wm_memory_usage,
                        &worker_name
                    )
                    .execute(&db)
                    .await
                    .expect("update worker ping");
                }
                let current_mem = get_mem().await;
                if current_mem > *mem_peak {
                    *mem_peak = current_mem
                }
                tracing::info!("{worker_name}/{job_id} in {w_id} still running.  mem: {current_mem}kB, peak mem: {mem_peak}kB");


                let update_job_row = i == 2 || (!*SLOW_LOGS && (i < 20 || (i < 120 && i % 5 == 0) || i % 10 == 0)) || i % 20 == 0;
                if update_job_row {
                #[cfg(feature = "enterprise")]
                {
                    // tracking metric starting at i >= 2 b/c first point it useless and we don't want to track metric for super fast jobs
                    if i == 2 {
                        memory_metric_id = job_metrics::register_metric_for_job(
                            &db,
                            w_id.to_string(),
                            job_id,
                            "memory_kb".to_string(),
                            job_metrics::MetricKind::TimeseriesInt,
                            Some("Job Memory Footprint (kB)".to_string()),
                        )
                        .await;
                    }
                    if let Ok(ref metric_id) = memory_metric_id {
                        if let Err(err) = job_metrics::record_metric(&db, w_id.to_string(), job_id, metric_id.to_owned(), job_metrics::MetricNumericValue::Integer(current_mem)).await {
                            tracing::error!("Unable to save memory stat for job {} in workspace {}. Error was: {:?}", job_id, w_id, err);
                        }
                    }
                }

                let (canceled, canceled_by, canceled_reason, already_completed) = sqlx::query_as::<_, (bool, Option<String>, Option<String>, bool)>("UPDATE queue SET mem_peak = $1, last_ping = now() WHERE id = $2 RETURNING canceled, canceled_by, canceled_reason, false")
                    .bind(*mem_peak)
                    .bind(job_id)
                    .fetch_optional(&db)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!(%e, "error updating job {job_id}: {e:#}");
                        Some((false, None, None, false))
                    })
                    .unwrap_or_else(|| {
                        // if the job is not in queue, it can only be in the completed_job so it is already complete
                        (false, None, None, true)
                    });
                if already_completed {
                    return UpdateJobPollingExit::AlreadyCompleted
                }
                if canceled {
                    canceled_by_ref.replace(CanceledBy {
                        username: canceled_by.clone(),
                        reason: canceled_reason.clone(),
                    });
                    break
                }
            }
            },
        );
    }
    tracing::info!("job {job_id} finished");

    UpdateJobPollingExit::Done
}

pub enum CompactLogs {
    NotEE,
    NoS3,
    S3,
}

async fn compact_logs(
    job_id: Uuid,
    w_id: &str,
    db: &DB,
    nlogs: String,
    total_size: Arc<AtomicU32>,
    compact_kind: CompactLogs,
    _worker_name: &str,
) -> error::Result<(String, String)> {
    let mut prev_logs = sqlx::query_scalar!(
        "SELECT logs FROM job_logs WHERE job_id = $1 AND workspace_id = $2",
        job_id,
        w_id
    )
    .fetch_optional(db)
    .await?
    .flatten()
    .unwrap_or_default();
    let size = prev_logs.char_indices().count() as i32;
    let nlogs_len = nlogs.char_indices().count();
    let to_keep_in_db = usize::max(
        usize::min(nlogs_len, 3000),
        nlogs_len % LARGE_LOG_THRESHOLD_SIZE,
    );
    let extra_split = to_keep_in_db < nlogs_len;
    let stored_in_storage_len = if extra_split {
        nlogs_len - to_keep_in_db
    } else {
        0
    };
    let extra_to_newline = nlogs
        .chars()
        .skip(stored_in_storage_len)
        .find_position(|x| x.is_line_break())
        .map(|(i, _)| i)
        .unwrap_or(to_keep_in_db);
    let stored_in_storage_to_newline = stored_in_storage_len + extra_to_newline;

    let (append_to_storage, stored_in_db) = if extra_split {
        if stored_in_storage_to_newline == nlogs.len() {
            (nlogs.as_ref(), "".to_string())
        } else {
            let split_idx = nlogs
                .char_indices()
                .nth(stored_in_storage_to_newline)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let (append_to_storage, stored_in_db) = nlogs.split_at(split_idx);
            // tracing::error!("{append_to_storage} ||||| {stored_in_db}");
            // tracing::error!(
            //     "{:?} {:?} {} {}",
            //     excess_prev_logs.lines().last(),
            //     current_logs.lines().next(),
            //     split_idx,
            //     excess_size_modulo
            // );
            (append_to_storage, stored_in_db.to_string())
        }
    } else {
        // tracing::error!("{:?}", nlogs.lines().last());
        ("", nlogs.to_string())
    };

    let new_size_with_excess = size + stored_in_storage_to_newline as i32;

    let new_size = total_size.fetch_add(
        new_size_with_excess as u32,
        std::sync::atomic::Ordering::SeqCst,
    ) + new_size_with_excess as u32;

    let path = format!(
        "logs/{job_id}/{}_{new_size}.txt",
        chrono::Utc::now().timestamp_millis()
    );

    let mut new_current_logs = match compact_kind {
        CompactLogs::NoS3 => format!("\n[windmill] No object storage set in instance settings. Previous logs have been saved to disk at {path}"),
        CompactLogs::S3 => format!("\n[windmill] Previous logs have been saved to object storage at {path}"),
        CompactLogs::NotEE => format!("\n[windmill] Previous logs have been saved to disk at {path}"),
    };
    new_current_logs.push_str(&stored_in_db);

    sqlx::query!(
        "UPDATE job_logs SET logs = $1, log_offset = $2, 
        log_file_index = array_append(coalesce(log_file_index, array[]::text[]), $3) 
        WHERE workspace_id = $4 AND job_id = $5",
        new_current_logs,
        new_size as i32,
        path,
        w_id,
        job_id
    )
    .execute(db)
    .await?;
    prev_logs.push_str(&append_to_storage);

    return Ok((prev_logs, path));
}

async fn default_disk_log_storage(
    job_id: Uuid,
    w_id: &str,
    db: &DB,
    nlogs: String,
    total_size: Arc<AtomicU32>,
    compact_kind: CompactLogs,
    worker_name: &str,
) {
    match compact_logs(
        job_id,
        &w_id,
        &db,
        nlogs,
        total_size,
        compact_kind,
        worker_name,
    )
    .await
    {
        Err(e) => tracing::error!("Could not compact logs for job {job_id}: {e:?}",),
        Ok((prev_logs, path)) => {
            let path = format!("{}/{}", TMP_DIR, path);
            let splitted = &path.split("/").collect_vec();
            tokio::fs::create_dir_all(splitted.into_iter().take(splitted.len() - 1).join("/"))
                .await
                .map_err(|e| {
                    tracing::error!("Could not create logs directory: {e:?}",);
                    e
                })
                .ok();
            let created = tokio::fs::File::create(&path).await;
            if let Err(e) = created {
                tracing::error!("Could not create logs file {path}: {e:?}",);
                return;
            }
            if let Err(e) = tokio::fs::write(&path, prev_logs).await {
                tracing::error!("Could not write to logs file {path}: {e:?}");
            } else {
                tracing::info!("Logs length of {job_id} has exceeded a threshold. Previous logs have been saved to disk at {path}");
            }
        }
    }
}

async fn append_job_logs(
    job_id: Uuid,
    w_id: String,
    logs: String,
    db: DB,
    must_compact_logs: bool,
    total_size: Arc<AtomicU32>,
    worker_name: String,
) -> () {
    if must_compact_logs {
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
            match compact_logs(
                job_id,
                &w_id,
                &db,
                logs,
                total_size,
                CompactLogs::S3,
                &worker_name,
            )
            .await
            {
                Err(e) => tracing::error!("Could not compact logs for job {job_id}: {e:?}",),
                Ok((prev_logs, path)) => {
                    tracing::info!("Logs length of {job_id} has exceeded a threshold. Previous logs have been saved to object storage at {path}");
                    let path2 = path.clone();
                    if let Err(e) = os
                        .put(&Path::from(path), prev_logs.to_string().into_bytes().into())
                        .await
                    {
                        tracing::error!("Could not save logs to s3: {e:?}");
                    }
                    tracing::info!("Logs of {job_id} saved to object storage at {path2}");
                }
            }
        } else {
            default_disk_log_storage(
                job_id,
                &w_id,
                &db,
                logs,
                total_size,
                CompactLogs::NoS3,
                &worker_name,
            )
            .await;
        }

        #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
        {
            default_disk_log_storage(
                job_id,
                &w_id,
                &db,
                logs,
                total_size,
                CompactLogs::NotEE,
                &worker_name,
            )
            .await;
        }
    } else {
        append_logs(job_id, w_id, logs, db).await;
    }
}

pub const LARGE_LOG_THRESHOLD_SIZE: usize = 9000;
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
    mem_peak: &mut i32,
    canceled_by_ref: &mut Option<CanceledBy>,
    mut child: Child,
    nsjail: bool,
    worker_name: &str,
    w_id: &str,
    child_name: &str,
    custom_timeout: Option<i32>,
    sigterm: bool,
) -> error::Result<()> {
    let start = Instant::now();

    let pid = child.id();
    #[cfg(target_os = "linux")]
    if let Some(pid) = pid {
        //set the highest oom priority
        if let Some(mut file) = File::create(format!("/proc/{pid}/oom_score_adj"))
            .await
            .map_err(|e| {
                tracing::error!("Could not create oom_score_file to pid {pid}: {e:#}");
                e
            })
            .ok()
        {
            let _ = file.write_all(b"1000").await;
            let _ = file.sync_all().await;
        }
    } else {
        tracing::info!("could not get child pid");
    }
    let (set_too_many_logs, mut too_many_logs) = watch::channel::<bool>(false);
    let (tx, rx) = broadcast::channel::<()>(3);
    let mut rx2 = tx.subscribe();

    let output = child_joined_output_stream(&mut child);

    let job_id = job_id.clone();

    /* the cancellation future is polled on by `wait_on_child` while
     * waiting for the child to exit normally */
    let update_job = update_job_poller(
        job_id,
        db,
        mem_peak,
        canceled_by_ref,
        || get_mem_peak(pid, nsjail),
        worker_name,
        w_id,
        rx,
    );

    #[derive(PartialEq, Debug)]
    enum KillReason {
        TooManyLogs,
        Timeout,
        Cancelled,
        AlreadyCompleted,
    }

    let (timeout_duration, timeout_warn_msg) =
        resolve_job_timeout(&db, w_id, job_id, custom_timeout).await;
    if let Some(msg) = timeout_warn_msg {
        append_logs(job_id, w_id.to_string(), msg.as_str(), db).await;
    }

    /* a future that completes when the child process exits */
    let wait_on_child = async {
        let db = db.clone();

        let kill_reason = tokio::select! {
            biased;
            result = child.wait() => return result.map(Ok),
            Ok(()) = too_many_logs.changed() => KillReason::TooManyLogs,
            _ = sleep(timeout_duration) => KillReason::Timeout,
            ex = update_job, if job_id != Uuid::nil() => match ex {
                UpdateJobPollingExit::Done => KillReason::Cancelled,
                UpdateJobPollingExit::AlreadyCompleted => KillReason::AlreadyCompleted,
            },
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
                .bind(format!("duration > {}", timeout_duration.as_secs()))
                .bind(job_id)
                .execute(&db)
                .await
                {
                    tracing::error!(%job_id, %err, "error setting cancelation reason for job {job_id}: {err}");
                }
            }
        };

        if let Some(id) = child.id() {
            if *MAX_WAIT_FOR_SIGINT > 0 {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                signal::kill(Pid::from_raw(id as i32), Signal::SIGINT).unwrap();

                for _ in 0..*MAX_WAIT_FOR_SIGINT {
                    if child.try_wait().is_ok_and(|x| x.is_some()) {
                        break;
                    }
                    sleep(Duration::from_secs(1)).await;
                }
                if child.try_wait().is_ok_and(|x| x.is_some()) {
                    set_reason.await;
                    return Ok(Err(kill_reason));
                }
            }
            if sigterm {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                signal::kill(Pid::from_raw(id as i32), Signal::SIGTERM).unwrap();

                for _ in 0..*MAX_WAIT_FOR_SIGTERM {
                    if child.try_wait().is_ok_and(|x| x.is_some()) {
                        break;
                    }
                    sleep(Duration::from_secs(1)).await;
                }
                if child.try_wait().is_ok_and(|x| x.is_some()) {
                    set_reason.await;
                    return Ok(Err(kill_reason));
                }
            }
        }
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
        let mut log_remaining =  if *CLOUD_HOSTED {
            max_log_size
        } else {
            usize::MAX
        };
        let mut result = io::Result::Ok(());
        let mut output = output.take_until(async {
            let _ = rx2.recv().await;
            //wait at most 50ms after end of a script for output stream to end
            tokio::time::sleep(Duration::from_millis(50)).await;
         }).boxed();
        /* `do_write` resolves the task, but does not contain the Result.
         * It's useful to know if the task completed. */
        let (mut do_write, mut write_result) = tokio::spawn(ready(())).remote_handle();

        let mut log_total_size: u64 = 0;
        let pg_log_total_size = Arc::new(AtomicU32::new(0));

        while let Some(line) =  output.by_ref().next().await {

            let do_write_ = do_write.shared();

            let delay = if start.elapsed() < Duration::from_secs(10) {
                Duration::from_millis(500)
            } else if start.elapsed() < Duration::from_secs(60){
                Duration::from_millis(2500)
            } else {
                Duration::from_millis(5000)
            };

            let delay = if *SLOW_LOGS {
                delay * 10
            } else {
                delay
            };

            let mut read_lines = stream::once(async { line })
                .chain(output.by_ref())
                /* after receiving a line, continue until some delay has passed
                 * _and_ the previous database write is complete */
                .take_until(future::join(sleep(delay), do_write_.clone()))
                .boxed();

            /* Read up until an error is encountered,
             * handle log lines first and then the error... */
            let mut joined = String::new();

            while let Some(line) = read_lines.next().await {

                match line {
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


            let joined_len = joined.len() as u64;
            log_total_size += joined_len;
            let compact_logs = log_total_size > LARGE_LOG_THRESHOLD_SIZE as u64;
            if compact_logs {
                log_total_size = 0;
            }

            let worker_name = worker_name.to_string();
            let w_id2 = w_id.to_string();
            (do_write, write_result) = tokio::spawn(append_job_logs(job_id, w_id2, joined, db.clone(), compact_logs, pg_log_total_size.clone(), worker_name)).remote_handle();



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

    tracing::info!(%job_id, "child process '{child_name}' for {worker_name}/{job_id} took {}ms, mem_peak: {:?}", start.elapsed().as_millis(), mem_peak);
    match wait_result {
        _ if *too_many_logs.borrow() => Err(Error::ExecutionErr(format!(
            "logs or result reached limit. (current max size: {MAX_RESULT_SIZE} characters)"
        ))),
        Ok(Ok(status)) => process_status(status),
        Ok(Err(kill_reason)) => match kill_reason {
            KillReason::AlreadyCompleted => {
                Err(Error::AlreadyCompleted("Job already completed".to_string()))
            }
            _ => Err(Error::ExecutionErr(format!(
                "job process killed because {kill_reason:#?}"
            ))),
        },
        Err(err) => Err(Error::ExecutionErr(format!("job process io error: {err}"))),
    }
}

pub fn process_status(status: ExitStatus) -> error::Result<()> {
    if status.success() {
        Ok(())
    } else if let Some(code) = status.code() {
        Err(error::Error::ExitStatus(code))
    } else {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        return Err(error::Error::ExecutionErr(format!(
            "process terminated by signal: {:#?}, stopped_signal: {:#?}, core_dumped: {}",
            status.signal(),
            status.stopped_signal(),
            status.core_dumped()
        )));

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        return Err(error::Error::ExecutionErr(String::from(
            "process terminated by signal",
        )));
    }
}
pub async fn start_child_process(mut cmd: Command, executable: &str) -> Result<Child, Error> {
    return cmd
        .spawn()
        .map_err(|err| tentatively_improve_error(Error::IoErr(err), executable));
}

pub async fn resolve_job_timeout(
    _db: &Pool<Postgres>,
    _w_id: &str,
    _job_id: Uuid,
    custom_timeout_secs: Option<i32>,
) -> (Duration, Option<String>) {
    let mut warn_msg: Option<String> = None;
    #[cfg(feature = "cloud")]
    let cloud_premium_workspace = *CLOUD_HOSTED
        && sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", _w_id)
            .fetch_one(_db)
            .await
            .map_err(|e| {
                tracing::error!(%e, "error getting premium workspace for job {_job_id}: {e:#}");
            })
            .unwrap_or(false);
    #[cfg(not(feature = "cloud"))]
    let cloud_premium_workspace = false;

    // compute global max timeout
    let global_max_timeout_duration = if cloud_premium_workspace {
        *MAX_TIMEOUT_DURATION * 6 //30mins
    } else {
        *MAX_TIMEOUT_DURATION
    };

    match custom_timeout_secs {
        Some(timeout_secs)
            if Duration::from_secs(timeout_secs as u64) < global_max_timeout_duration =>
        {
            (Duration::from_secs(timeout_secs as u64), warn_msg)
        }
        Some(timeout_secs) => {
            warn_msg = Some(format!("WARNING: Custom job timeout of {timeout_secs} seconds was greater than the maximum timeout. It will be ignored and the max timeout will be used instead"));
            tracing::warn!(warn_msg);
            (global_max_timeout_duration, warn_msg)
        }
        None => {
            // fallback to default timeout or max if not set
            let default_timeout = match JOB_DEFAULT_TIMEOUT.read().await.clone() {
                None => global_max_timeout_duration,
                Some(default_timeout_secs)
                    if Duration::from_secs(default_timeout_secs as u64)
                        < global_max_timeout_duration =>
                {
                    Duration::from_secs(default_timeout_secs as u64)
                }
                Some(default_timeout_secs) => {
                    warn_msg = Some(format!("WARNING: Default job timeout of {default_timeout_secs} seconds was greater than the maximum timeout. It will be ignored and the global max timeout will be used instead"));
                    tracing::warn!(warn_msg);
                    global_max_timeout_duration
                }
            };
            (default_timeout, warn_msg)
        }
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

pub fn lines_to_stream<R: tokio::io::AsyncBufRead + Unpin>(
    mut lines: tokio::io::Lines<R>,
) -> impl futures::Stream<Item = io::Result<String>> {
    stream::poll_fn(move |cx| {
        std::pin::Pin::new(&mut lines)
            .poll_next_line(cx)
            .map(|result| result.transpose())
    })
}

lazy_static::lazy_static! {
    static ref RE_00: Regex = Regex::new('\u{00}'.to_string().as_str()).unwrap();
    pub static ref NO_LOGS_AT_ALL: bool = std::env::var("NO_LOGS_AT_ALL").ok().is_some_and(|x| x == "1" || x == "true");
    pub static ref SLOW_LOGS: bool = std::env::var("SLOW_LOGS").ok().is_some_and(|x| x == "1" || x == "true");
}
// as a detail, `BufReader::lines()` removes \n and \r\n from the strings it yields,
// so this pushes \n to thd destination string in each call
fn append_with_limit(dst: &mut String, src: &str, limit: &mut usize) {
    if *NO_LOGS_AT_ALL {
        return;
    }
    let src_str;
    let src = {
        src_str = RE_00.replace_all(src, "");
        src_str.as_ref()
    };
    if !*CLOUD_HOSTED {
        dst.push('\n');
        dst.push_str(&src);
        return;
    } else {
        if *limit > 0 {
            dst.push('\n');
        }
        *limit -= 1;
    }

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

pub async fn hash_args(
    _db: &DB,
    _client: &AuthedClient,
    _workspace_id: &str,
    _job_id: &Uuid,
    v: &Option<sqlx::types::Json<HashMap<String, Box<RawValue>>>>,
) -> String {
    if let Some(vs) = v {
        let mut dh = DefaultHasher::new();
        let hm = &vs.0;
        for k in hm.keys().sorted() {
            k.hash(&mut dh);
            let arg_value = hm.get(k).unwrap();
            #[cfg(feature = "parquet")]
            let (_, arg_additions) =
                arg_value_hash_additions(_db, _client, _workspace_id, hm.get(k).unwrap()).await;
            arg_value.get().hash(&mut dh);
            #[cfg(feature = "parquet")]
            for (_, arg_addition) in arg_additions {
                arg_addition.hash(&mut dh);
            }
        }
        hex::encode(dh.finish().to_be_bytes())
    } else {
        "empty_args".to_string()
    }
}

#[cfg(feature = "parquet")]
async fn get_workspace_s3_resource_path(
    db: &DB,
    client: &AuthedClient,
    workspace_id: &str,
    storage: Option<String>,
) -> windmill_common::error::Result<Option<ObjectStoreResource>> {
    use windmill_common::{
        job_s3_helpers_ee::get_s3_resource_internal, s3_helpers::StorageResourceType,
    };

    let raw_lfs_opt = if let Some(storage) = storage {
        sqlx::query_scalar!(
            "SELECT large_file_storage->'secondary_storage'->$2 FROM workspace_settings WHERE workspace_id = $1",
            workspace_id,
            storage
        )
        .fetch_optional(db)
        .await?
        .flatten()
    } else {
        sqlx::query_scalar!(
            "SELECT large_file_storage FROM workspace_settings WHERE workspace_id = $1",
            workspace_id
        )
        .fetch_optional(db)
        .await?
        .flatten()
    };
    let raw_lfs_opt = raw_lfs_opt
        .map(|val| serde_json::from_value::<LargeFileStorage>(val).ok())
        .flatten();

    let (rt, path) = match raw_lfs_opt {
        Some(LargeFileStorage::S3Storage(s3_storage)) => {
            let resource_path = s3_storage.s3_resource_path.trim_start_matches("$res:");
            (StorageResourceType::S3, resource_path.to_string())
        }
        Some(LargeFileStorage::AzureBlobStorage(azure_blob_storage)) => {
            let resource_path = azure_blob_storage
                .azure_blob_resource_path
                .trim_start_matches("$res:");
            (StorageResourceType::AzureBlob, resource_path.to_string())
        }
        Some(LargeFileStorage::S3AwsOidc(s3_aws_oidc)) => {
            let resource_path = s3_aws_oidc.s3_resource_path.trim_start_matches("$res:");
            (StorageResourceType::S3AwsOidc, resource_path.to_string())
        }
        Some(LargeFileStorage::AzureWorkloadIdentity(azure)) => {
            let resource_path = azure.azure_blob_resource_path.trim_start_matches("$res:");
            (
                StorageResourceType::AzureWorkloadIdentity,
                resource_path.to_string(),
            )
        }
        None => {
            return Ok(None);
        }
    };

    let client2 = client.clone();
    let token_fn = |audience: String| async move {
        client2
            .get_id_token(&audience)
            .await
            .map_err(|e| windmill_common::error::Error::from(e))
    };
    let s3_resource_value_raw = client
        .get_resource_value::<serde_json::Value>(path.as_str())
        .await?;
    get_s3_resource_internal(rt, s3_resource_value_raw, token_fn)
        .await
        .map(Some)
}

#[cfg(feature = "parquet")]
async fn arg_value_hash_additions(
    db: &DB,
    client: &AuthedClient,
    workspace_id: &str,
    raw_value: &Box<RawValue>,
) -> (Option<String>, HashMap<String, String>) {
    let mut result: HashMap<String, String> = HashMap::new();

    let parsed_value = serde_json::from_str::<S3Object>(raw_value.get());

    let mut storage = None;
    if let Ok(s3_object) = parsed_value {
        let s3_resource_opt =
            get_workspace_s3_resource_path(db, client, workspace_id, s3_object.storage.clone())
                .await;
        storage = s3_object.storage.clone();

        if let Some(s3_resource) = s3_resource_opt.ok().flatten() {
            let etag = get_etag_or_empty(&s3_resource, s3_object.clone()).await;
            tracing::warn!("Enriching s3 arg value with etag: {:?}", etag);
            result.insert(s3_object.s3.clone(), etag.unwrap_or_default()); // TODO: maybe inject a random value to invalidate the cache?
        }
    }

    return (storage, result);
}

#[derive(Deserialize, Serialize)]
struct CachedResource {
    expire: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    s3_etags: Option<HashMap<String, String>>,
    value: Box<RawValue>,
    storage: Option<String>,
}

pub async fn get_cached_resource_value_if_valid(
    _db: &DB,
    client: &AuthedClient,
    _job_id: &Uuid,
    _workspace_id: &str,
    cached_res_path: &str,
) -> Option<Box<RawValue>> {
    let resource_opt = client
        .get_resource_value::<CachedResource>(cached_res_path)
        .await
        .ok();
    if let Some(cached_resource) = resource_opt {
        if cached_resource.expire <= chrono::Utc::now().timestamp() {
            // cache expired
            return None;
        }
        #[cfg(feature = "parquet")]
        {
            let s3_etags = cached_resource.s3_etags.unwrap_or_default();
            let object_store_resource_opt: Option<ObjectStoreResource> = if s3_etags.is_empty() {
                None
            } else {
                get_workspace_s3_resource_path(
                    _db,
                    &client,
                    _workspace_id,
                    cached_resource.storage.clone(),
                )
                .await
                .ok()
                .flatten()
            };

            if !s3_etags.is_empty() && object_store_resource_opt.is_none() {
                tracing::warn!("Cached result references s3 files that are not retrievable anymore because the workspace S3 resource can't be fetched. Cache will be invalidated");
                return None;
            }
            for (s3_file_key, s3_file_etag) in s3_etags {
                if let Some(object_store_resource) = object_store_resource_opt.clone() {
                    let etag = get_etag_or_empty(
                        &object_store_resource,
                        S3Object {
                            s3: s3_file_key.clone(),
                            storage: cached_resource.storage.clone(),
                        },
                    )
                    .await;
                    if etag.is_none() || etag.clone().unwrap() != s3_file_etag {
                        tracing::warn!("S3 file etag for '{}' has changed. Value from cache is {:?} while current value from S3 is {:?}. Cache will be invalidated", s3_file_key.clone(), s3_file_etag, etag);
                        return None;
                    }
                }
            }
        }
        return Some(cached_resource.value);
    }
    return None;
}

pub async fn save_in_cache(
    db: &Pool<Postgres>,
    _client: &AuthedClient,
    job: &QueuedJob,
    cached_path: String,
    r: &Box<RawValue>,
) {
    let expire = chrono::Utc::now().timestamp() + job.cache_ttl.unwrap() as i64;

    #[cfg(feature = "parquet")]
    let (storage, s3_etags) =
        arg_value_hash_additions(db, _client, job.workspace_id.as_str(), r).await;

    #[cfg(feature = "parquet")]
    let s3_etags = if s3_etags.is_empty() {
        None
    } else {
        Some(s3_etags)
    };

    #[cfg(not(feature = "parquet"))]
    let s3_etags = None;

    let store_cache_resource = CachedResource { expire, s3_etags, value: r.clone(), storage };
    let raw_json = sqlx::types::Json(store_cache_resource);

    if let Err(e) = sqlx::query!(
        "INSERT INTO resource
    (workspace_id, path, value, resource_type)
    VALUES ($1, $2, $3, $4) ON CONFLICT (workspace_id, path)
    DO UPDATE SET value = $3",
        job.workspace_id,
        cached_path,
        raw_json as sqlx::types::Json<CachedResource>,
        "cache"
    )
    .execute(db)
    .await
    {
        tracing::error!("Error creating cache resource {e:#}")
    }
}

fn tentatively_improve_error(err: Error, executable: &str) -> Error {
    if err
        .to_string()
        .contains("No such file or directory (os error 2)")
    {
        return Error::InternalErr(format!("Executable {executable} not found on worker"));
    }
    return err;
}

pub async fn clean_cache() -> error::Result<()> {
    tracing::info!("Started cleaning cache");
    tokio::fs::remove_dir_all(ROOT_CACHE_DIR).await?;
    tracing::info!("Finished cleaning cache");
    Ok(())
}
