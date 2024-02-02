use async_recursion::async_recursion;
use itertools::Itertools;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use tokio::process::Command;
use tokio::{fs::File, io::AsyncReadExt};
use windmill_common::s3_helpers::{
    get_etag_or_empty, AzureBlobResource, LargeFileStorage, ObjectStoreResource, S3Object,
    S3Resource,
};
use windmill_common::worker::{CLOUD_HOSTED, WORKER_CONFIG};
use windmill_common::{
    error::{self, Error},
    jobs::QueuedJob,
    variables::ContextualVariable,
};

use anyhow::Result;
use windmill_queue::CanceledBy;

use std::{
    borrow::Borrow,
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    io,
    os::unix::process::ExitStatusExt,
    panic,
    time::Duration,
};

use tracing::{trace_span, Instrument};
use uuid::Uuid;
use windmill_common::{job_metrics, variables, DB};

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
    static ref RE_RES_VAR: Regex = Regex::new(r#"\$(?:var|res)\:"#).unwrap();
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
                error::Error::InternalErr(format!("Error while parsing inner arg: {e}"))
            })?;
            let transformed =
                transform_json_value(&k, &client.get_authed().await, workspace, value, job, db)
                    .await?;
            let as_raw = serde_json::from_value(transformed).map_err(|e| {
                error::Error::InternalErr(format!("Error while parsing inner arg: {e}"))
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
                error::Error::InternalErr(format!("Error while parsing inner arg: {e}"))
            })?;
            let transformed =
                transform_json_value(&k, &client.get_authed().await, workspace, value, job, db)
                    .await?;
            let as_raw = serde_json::from_value(transformed).map_err(|e| {
                error::Error::InternalErr(format!("Error while parsing inner arg: {e}"))
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
                    Error::NotFound(format!("Variable {path} not found for `{name}`: {e}"))
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
                    Error::NotFound(format!("Resource {path} not found for `{name}`: {e}"))
                })
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

#[tracing::instrument(level = "trace", skip_all)]
pub async fn set_logs(logs: &str, id: &uuid::Uuid, db: &Pool<Postgres>) {
    if sqlx::query!(
        "UPDATE queue SET logs = $1 WHERE id = $2",
        logs.to_owned(),
        id
    )
    .execute(db)
    .await
    .is_err()
    {
        tracing::error!(%id, "error updating logs for id {id}")
    };
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
        if job_id == Uuid::nil() {
            return;
        }
        let db = db.clone();

        let mut interval = interval(update_job_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let mut i = 0;
        let mut memory_metric_id: Result<String, Error> =
            Err(Error::NotFound("not yet initialized".to_string()));

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
                    let current_mem = get_mem_peak(pid, nsjail).await;
                    if current_mem > *mem_peak {
                        *mem_peak = current_mem
                    }
                    tracing::info!("{worker_name}/{job_id} in {w_id} still running.  mem: {current_mem}kB, peak mem: {mem_peak}kB");

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

                    let (canceled, canceled_by, canceled_reason) = sqlx::query_as::<_, (bool, Option<String>, Option<String>)>("UPDATE queue SET mem_peak = $1, last_ping = now() WHERE id = $2 RETURNING canceled, canceled_by, canceled_reason")
                        .bind(*mem_peak)
                        .bind(job_id)
                        .fetch_optional(&db)
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!(%e, "error updating job {job_id}: {e}");
                            Some((false, None, None))
                        })
                        .unwrap_or((false, None, None));
                    if canceled {
                        canceled_by_ref.replace(CanceledBy {
                            username: canceled_by.clone(),
                            reason: canceled_reason.clone(),
                        });
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

    let (timeout_duration, timeout_warn_msg) =
        resolve_job_timeout(&db, w_id, job_id, custom_timeout).await;
    if let Some(msg) = timeout_warn_msg {
        logs.push_str(msg.as_str());
        append_logs(job_id, msg.as_str(), db).await;
    }

    /* a future that completes when the child process exits */
    let wait_on_child = async {
        let db = db.clone();

        let kill_reason = tokio::select! {
            biased;
            result = child.wait() => return result.map(Ok),
            Ok(()) = too_many_logs.changed() => KillReason::TooManyLogs,
            _ = sleep(timeout_duration) => KillReason::Timeout,
            _ = update_job, if job_id != Uuid::nil() => KillReason::Cancelled,
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
            max_log_size.saturating_sub(logs.chars().count())
        } else {
            usize::MAX
        };
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

    tracing::info!(%job_id, "child process '{child_name}' for {worker_name}/{job_id} took {}ms, mem_peak: {:?}", start.elapsed().as_millis(), mem_peak);
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

pub async fn start_child_process(mut cmd: Command, executable: &str) -> Result<Child, Error> {
    return cmd
        .spawn()
        .map_err(|err| tentatively_improve_error(Error::IoErr(err), executable));
}

async fn resolve_job_timeout(
    db: &Pool<Postgres>,
    w_id: &str,
    job_id: Uuid,
    custom_timeout_secs: Option<i32>,
) -> (Duration, Option<String>) {
    let mut warn_msg: Option<String> = None;
    #[cfg(feature = "enterprise")]
    let cloud_premium_workspace = *CLOUD_HOSTED
        && sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", w_id)
            .fetch_one(db)
            .await
            .map_err(|e| {
                tracing::error!(%e, "error getting premium workspace for job {job_id}: {e}");
            })
            .unwrap_or(false);
    #[cfg(not(feature = "enterprise"))]
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
}
// as a detail, `BufReader::lines()` removes \n and \r\n from the strings it yields,
// so this pushes \n to thd destination string in each call
fn append_with_limit(dst: &mut String, src: &str, limit: &mut usize) {
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
    db: &DB,
    client: &AuthedClient,
    workspace_id: &str,
    job_id: &Uuid,
    v: &Option<sqlx::types::Json<HashMap<String, Box<RawValue>>>>,
) -> String {
    if let Some(vs) = v {
        let mut dh = DefaultHasher::new();
        let hm = &vs.0;
        for k in hm.keys().sorted() {
            k.hash(&mut dh);
            let arg_value = hm.get(k).unwrap();
            let arg_additions =
                arg_value_hash_additions(db, client, workspace_id, job_id, hm.get(k).unwrap())
                    .await;
            arg_value.get().hash(&mut dh);
            for (_, arg_addition) in arg_additions {
                arg_addition.hash(&mut dh);
            }
        }
        hex::encode(dh.finish().to_be_bytes())
    } else {
        "empty_args".to_string()
    }
}

async fn get_workspace_s3_resource_path(
    db: &DB,
    client: &AuthedClient,
    workspace_id: &str,
    job_id: &Uuid,
) -> Option<ObjectStoreResource> {
    let raw_lfs_opt = sqlx::query_scalar!(
        "SELECT large_file_storage FROM workspace_settings WHERE workspace_id = $1",
        workspace_id
    )
    .fetch_one(db)
    .await
    .ok()
    .flatten()
    .map(|val| serde_json::from_value::<LargeFileStorage>(val).ok())
    .flatten();

    match raw_lfs_opt {
        Some(LargeFileStorage::S3Storage(s3_storage)) => {
            let resource_path = s3_storage.s3_resource_path.trim_start_matches("$res:");
            let s3_resource = client
                .get_resource_value_interpolated::<S3Resource>(
                    &resource_path,
                    Some(job_id.to_string()),
                )
                .await
                .ok();
            s3_resource.map(|resource| ObjectStoreResource::S3Resource(resource))
        }
        Some(LargeFileStorage::AzureBlobStorage(azure_blob_storage)) => {
            let resource_path = azure_blob_storage
                .azure_blob_resource_path
                .trim_start_matches("$res:");
            let azure_blob_resource = client
                .get_resource_value_interpolated::<AzureBlobResource>(
                    &resource_path,
                    Some(job_id.to_string()),
                )
                .await
                .ok();
            azure_blob_resource.map(|resource| ObjectStoreResource::AzureBlobResource(resource))
        }
        None => None,
    }
}

async fn arg_value_hash_additions(
    db: &DB,
    client: &AuthedClient,
    workspace_id: &str,
    job_id: &Uuid,
    raw_value: &Box<RawValue>,
) -> HashMap<String, String> {
    let mut result: HashMap<String, String> = HashMap::new();
    let parsed_s3_values: &mut Vec<S3Object> = &mut Vec::new();
    extract_all_s3_object_from_raw_value(raw_value, parsed_s3_values);
    if parsed_s3_values.is_empty() {
        // no s3 object, nothing to return
        return result;
    }

    let s3_resource_opt = get_workspace_s3_resource_path(db, client, workspace_id, job_id).await;
    if let Some(s3_resource) = s3_resource_opt {
        for s3_object in parsed_s3_values {
            let etag = get_etag_or_empty(&s3_resource, s3_object.clone()).await;
            tracing::warn!("Enriching s3 arg value with etag: {:?}", etag);
            result.insert(s3_object.s3.clone(), etag.unwrap_or_default()); // TODO: maybe inject a random value to invalidate the cache?
        }
    }

    return result;
}

fn extract_all_s3_object_from_raw_value(raw_value: &Box<RawValue>, result: &mut Vec<S3Object>) {
    let parsed_value = serde_json::from_str::<S3Object>(raw_value.get());
    if let Ok(parsed_value) = parsed_value {
        result.push(parsed_value);
    } else {
        let parsed_value = serde_json::from_str::<HashMap<String, Box<RawValue>>>(raw_value.get());
        if let Ok(parsed_value) = parsed_value {
            for (_, v) in parsed_value {
                extract_all_s3_object_from_raw_value(&v, result);
            }
        } else {
            let parsed_value = serde_json::from_str::<Vec<Box<RawValue>>>(raw_value.get());
            if let Ok(parsed_value) = parsed_value {
                for v in parsed_value {
                    extract_all_s3_object_from_raw_value(&v, result);
                }
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
struct CachedResource {
    expire: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    s3_etags: Option<HashMap<String, String>>,
    value: Box<RawValue>,
}

pub async fn get_cached_resource_value_if_valid(
    db: &DB,
    client: &AuthedClient,
    job_id: &Uuid,
    workspace_id: &str,
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
        let s3_etags = cached_resource.s3_etags.unwrap_or_default();
        let object_store_resource_opt: Option<ObjectStoreResource> = if s3_etags.is_empty() {
            None
        } else {
            get_workspace_s3_resource_path(db, &client, workspace_id, job_id).await
        };
        if !s3_etags.is_empty() && object_store_resource_opt.is_none() {
            tracing::warn!("Cached result references s3 files that are not retrievable anymore because the workspace S3 resource can't be fetched. Cache will be invalidated");
            return None;
        }
        for (s3_file_key, s3_file_etag) in s3_etags {
            if let Some(object_store_resource) = object_store_resource_opt.clone() {
                let etag =
                    get_etag_or_empty(&object_store_resource, S3Object { s3: s3_file_key.clone() })
                        .await;
                if etag.is_none() || etag.clone().unwrap() != s3_file_etag {
                    tracing::warn!("S3 file etag for '{}' has changed. Value from cache is {:?} while current value from S3 is {:?}. Cache will be invalidated", s3_file_key.clone(), s3_file_etag, etag);
                    return None;
                }
            }
        }
        return Some(cached_resource.value);
    }
    return None;
}

pub async fn save_in_cache(
    db: &Pool<Postgres>,
    client: &AuthedClient,
    job: &QueuedJob,
    cached_path: String,
    r: &Box<RawValue>,
) {
    let expire = chrono::Utc::now().timestamp() + job.cache_ttl.unwrap() as i64;

    let s3_etags =
        arg_value_hash_additions(db, client, job.workspace_id.as_str(), &job.id, r).await;

    let store_cache_resource = CachedResource {
        expire,
        s3_etags: if s3_etags.is_empty() {
            None
        } else {
            Some(s3_etags)
        },
        value: r.clone(),
    };
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
        tracing::error!("Error creating cache resource {e}")
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
