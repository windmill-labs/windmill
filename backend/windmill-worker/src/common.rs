use async_recursion::async_recursion;

use itertools::Itertools;
use lazy_static::lazy_static;
use process_wrap::tokio::TokioChildWrapper;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{json, Value};
use sha2::Digest;
use sqlx::types::Json;
use sqlx::{Pool, Postgres};
use tokio::process::Command;
use tokio::sync::{RwLock, Semaphore};
use tokio::{fs::File, io::AsyncReadExt};

#[cfg(feature = "parquet")]
use windmill_common::s3_helpers::{
    get_etag_or_empty, LargeFileStorage, ObjectStoreResource, S3Object,
};
use windmill_common::variables::{build_crypt_with_key_suffix, decrypt};
use windmill_common::worker::{
    to_raw_value, update_ping_for_failed_init_script_query, write_file, Connection, Ping, PingType,
    CLOUD_HOSTED, ROOT_CACHE_DIR, WORKER_CONFIG,
};
use windmill_common::{
    cache::{Cache, RawData},
    error::{self, Error},
    scripts::ScriptHash,
    variables::ContextualVariable,
};

use anyhow::{anyhow, bail, Result};
use windmill_parser_sql::{s3_mode_extension, S3ModeArgs, S3ModeFormat};
use windmill_queue::MiniPulledJob;

use std::ops::AsyncFn;
use std::path::Path;
use std::{collections::HashMap, sync::Arc, time::Duration};

use uuid::Uuid;
use windmill_common::{variables, DB};

use tokio::{io::AsyncWriteExt, time::Instant};

use crate::agent_workers::UPDATE_PING_URL;
use crate::{DISABLE_NSJAIL, JOB_DEFAULT_TIMEOUT, MAX_RESULT_SIZE, MAX_TIMEOUT_DURATION, PATH_ENV};
use windmill_common::client::AuthedClient;

pub async fn build_args_map<'a>(
    job: &'a MiniPulledJob,
    client: &AuthedClient,
    conn: &Connection,
) -> error::Result<Option<HashMap<String, Box<RawValue>>>> {
    if let Some(args) = &job.args {
        return transform_json(client, &job.workspace_id, &args.0, &job, conn).await;
    }
    return Ok(None);
}

pub fn check_executor_binary_exists(
    executor: &str,
    executor_path: &str,
    language: &str,
) -> Result<(), Error> {
    if !Path::new(executor_path).exists() {
        #[cfg(feature = "enterprise")]
        let msg = format!("Couldn't find {executor} at {}. This probably means that you are not using the windmill-ee-full image. Please use the image `ghcr.io/windmill-labs/windmill-ee-full` for your instance in order to run {language} jobs.", executor_path);

        #[cfg(not(feature = "enterprise"))]
        let msg = format!("Couldn't find {executor} at {}. This probably means that you are not using the windmill-full image. Please use the image `ghcr.io/windmill-labs/windmill-full` for your instance in order to run {language} jobs.", executor_path);
        return Err(Error::NotFound(msg));
    }

    Ok(())
}

pub async fn build_args_values(
    job: &MiniPulledJob,
    client: &AuthedClient,
    conn: &Connection,
) -> error::Result<HashMap<String, serde_json::Value>> {
    if let Some(args) = &job.args {
        transform_json_as_values(client, &job.workspace_id, &args.0, job, conn).await
    } else {
        Ok(HashMap::new())
    }
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn create_args_and_out_file(
    client: &AuthedClient,
    job: &MiniPulledJob,
    job_dir: &str,
    conn: &Connection,
) -> Result<(), Error> {
    if let Some(args) = job.args.as_ref() {
        if let Some(x) = transform_json(client, &job.workspace_id, &args.0, job, conn).await? {
            write_file(
                job_dir,
                "args.json",
                &serde_json::to_string(&x).unwrap_or_else(|_| "{}".to_string()),
            )?;
        } else {
            write_file(
                job_dir,
                "args.json",
                &serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string()),
            )?;
        }
    } else {
        write_file(job_dir, "args.json", "{}")?;
    };

    write_file(job_dir, "result.json", "")?;
    Ok(())
}

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
    client: &AuthedClient,
    workspace: &str,
    vs: &'a HashMap<String, Box<RawValue>>,
    job: &MiniPulledJob,
    db: &Connection,
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
                error::Error::internal_err(format!("Error while parsing inner arg: {e:#}"))
            })?;
            let transformed = transform_json_value(&k, &client, workspace, value, job, db).await?;
            let as_raw = serde_json::from_value(transformed).map_err(|e| {
                error::Error::internal_err(format!("Error while parsing inner arg: {e:#}"))
            })?;
            r.insert(k.to_string(), as_raw);
        } else {
            r.insert(k.to_string(), v.to_owned());
        }
    }
    Ok(Some(r))
}

pub async fn transform_json_as_values<'a>(
    client: &AuthedClient,
    workspace: &str,
    vs: &'a HashMap<String, Box<RawValue>>,
    job: &MiniPulledJob,
    db: &Connection,
) -> error::Result<HashMap<String, serde_json::Value>> {
    let mut r: HashMap<String, serde_json::Value> = HashMap::new();
    for (k, v) in vs {
        let inner_vs = v.get();
        if (*RE_RES_VAR).is_match(inner_vs) {
            let value = serde_json::from_str(inner_vs).map_err(|e| {
                error::Error::internal_err(format!("Error while parsing inner arg: {e:#}"))
            })?;
            let transformed = transform_json_value(&k, &client, workspace, value, job, db).await?;
            let as_raw = serde_json::from_value(transformed).map_err(|e| {
                error::Error::internal_err(format!("Error while parsing inner arg: {e:#}"))
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
pub async fn get_root_job_id(job: &Uuid, db: &Pool<Postgres>) -> anyhow::Result<Uuid> {
    let njob = sqlx::query_scalar!(
        "SELECT flow_innermost_root_job FROM v2_job WHERE id = $1",
        job
    )
    .fetch_optional(db)
    .await?
    .flatten();
    if let Some(root_job) = njob {
        if root_job == *job {
            return Ok(job.to_owned());
        }
        get_root_job_id(&root_job, db).await
    } else {
        Ok(job.to_owned())
    }
}

#[async_recursion]
pub async fn transform_json_value(
    name: &str,
    client: &AuthedClient,
    workspace: &str,
    v: Value,
    job: &MiniPulledJob,
    conn: &Connection,
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
                return Err(Error::internal_err(format!(
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
            match conn {
                Connection::Sql(db) => {
                    let encrypted = y.strip_prefix("$encrypted:").unwrap();

                    let root_job_id =
                        get_root_job_id(&job.flow_innermost_root_job.unwrap_or_else(|| job.id), db)
                            .await?;
                    let mc = build_crypt_with_key_suffix(
                        &db,
                        &job.workspace_id,
                        &root_job_id.to_string(),
                    )
                    .await?;
                    decrypt(&mc, encrypted.to_string()).and_then(|x| {
                        serde_json::from_str(&x).map_err(|e| Error::internal_err(e.to_string()))
                    })
                }
                Connection::Http(_) => {
                    Err(Error::NotFound("Http connection not supported".to_string()))
                }
            }

            // let path = y.strip_prefix("$res:").unwrap();
        }
        Value::String(y) if y.starts_with("$") => {
            let variables = get_reserved_variables(job, &client.token, conn, None).await?;

            let name = y.strip_prefix("$").unwrap();

            let value = variables
                .iter()
                .find(|x| x.0 == name)
                .map(|x| x.1.clone())
                .unwrap_or_else(|| y);
            Ok(json!(value))
        }
        Value::Object(mut m) => {
            for (a, b) in m.clone().into_iter() {
                m.insert(
                    a.clone(),
                    transform_json_value(&a, client, workspace, b, job, conn).await?,
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

fn check_result_too_big(size: usize) -> error::Result<()> {
    if *CLOUD_HOSTED && size > MAX_RESULT_SIZE {
        return Err(error::Error::ExecutionErr("Result is too large for the cloud app (limit 2MB).
We highly recommend using object to store and pass heavy data (https://www.windmill.dev/docs/core_concepts/object_storage_in_windmill#read-a-file-from-s3-within-a-script)
Alternatively, if using this script as part of a flow, activate shared folder and use the shared folder to pass heavy data between steps.".to_owned()));
    };
    Ok(())
}

/// This function assumes that the file contains valid json and will result in UB if it isn't. If
/// the file is user generated or otherwise not guaranteed to be valid used the
/// `read_and_check_file` instead
pub async fn read_file(path: &str) -> error::Result<Box<RawValue>> {
    let content = read_file_content(path).await?;

    check_result_too_big(content.len())?;

    let r = unsafe_raw(content);
    return Ok(r);
}

/// Read the `result.json` file. This function assumes that the file contains valid json and will
/// result in undefined behaviour if it isn't. If the result.json is user generated or otherwise
/// not guaranteed to be valid, use `read_and_check_result`
pub async fn read_result(job_dir: &str) -> error::Result<Box<RawValue>> {
    return read_file(&format!("{job_dir}/result.json")).await;
}

pub async fn read_and_check_file(path: &str) -> error::Result<Box<RawValue>> {
    let content = read_file_content(path).await?;

    check_result_too_big(content.len())?;

    let raw_value: Box<RawValue> =
        serde_json::from_str(&content).map_err(|e| anyhow!("{} is not valid json: {}", path, e))?;
    Ok(raw_value)
}

/// Use this to read `result.json` that were user-generated
pub async fn read_and_check_result(job_dir: &str) -> error::Result<Box<RawValue>> {
    let result_path = format!("{job_dir}/result.json");

    if let Ok(metadata) = tokio::fs::metadata(&result_path).await {
        if metadata.len() > 0 {
            return read_and_check_file(&result_path)
                .await
                .map_err(|e| anyhow!("Failed to read result: {}", e).into());
        }
    }
    Ok(to_raw_value(&json!("null")))
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
    job: &MiniPulledJob,
    token: &str,
    db: &Connection,
    parent_runnable_path: Option<String>,
) -> Result<HashMap<String, String>, Error> {
    let flow_path = if parent_runnable_path.is_some() {
        parent_runnable_path
    } else if let Some(uuid) = job.parent_job {
        match db {
            Connection::Sql(db) => {
                sqlx::query_scalar!("SELECT runnable_path FROM v2_job WHERE id = $1", uuid)
                    .fetch_optional(db)
                    .await?
                    .flatten()
            }
            Connection::Http(_) => None,
        }
    } else {
        None
    };

    let variables = variables::get_reserved_variables(
        db,
        &job.workspace_id,
        token,
        &job.permissioned_as_email,
        &job.created_by,
        &job.id.to_string(),
        &job.permissioned_as,
        job.runnable_path.clone(),
        job.parent_job.map(|x| x.to_string()),
        flow_path,
        job.schedule_path(),
        job.flow_step_id.clone(),
        job.flow_innermost_root_job.clone().map(|x| x.to_string()),
        Some(job.scheduled_for.clone()),
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

pub async fn update_worker_ping_for_failed_init_script(
    conn: &Connection,
    worker_name: &str,
    last_job_id: Uuid,
) {
    match conn {
        Connection::Sql(db) => {
            if let Err(e) =
                update_ping_for_failed_init_script_query(worker_name, last_job_id, db).await
            {
                tracing::error!("Error updating worker ping for failed init script: {e:?}");
            }
        }
        Connection::Http(client) => {
            if let Err(e) = client
                .post::<_, ()>(
                    UPDATE_PING_URL,
                    None,
                    &Ping {
                        last_job_executed: Some(last_job_id),
                        last_job_workspace_id: None,
                        worker_instance: None,
                        ip: None,
                        tags: None,
                        dw: None,
                        jobs_executed: None,
                        occupancy_rate: None,
                        occupancy_rate_15s: None,
                        occupancy_rate_5m: None,
                        occupancy_rate_30m: None,
                        version: None,
                        vcpus: None,
                        memory: None,
                        memory_usage: None,
                        wm_memory_usage: None,
                        ping_type: PingType::InitScript,
                    },
                )
                .await
            {
                tracing::error!("Error updating worker ping for failed init script: {e:?}");
            }
        }
    }
}

pub fn error_to_value(err: Error) -> serde_json::Value {
    match err {
        Error::JsonErr(err) => err,
        _ => json!({"message": err.to_string(), "name": "InternalErr"}),
    }
}

pub struct OccupancyMetrics {
    pub running_job_started_at: Option<Instant>,
    pub total_duration_of_running_jobs: f32,
    pub worker_occupancy_rate_history: Vec<(f32, f32)>,
    pub start_time: Instant,
}

pub struct OccupancyResult {
    pub occupancy_rate: f32,
    pub occupancy_rate_15s: Option<f32>,
    pub occupancy_rate_5m: Option<f32>,
    pub occupancy_rate_30m: Option<f32>,
}

impl OccupancyMetrics {
    pub fn new(start_time: Instant) -> Self {
        OccupancyMetrics {
            running_job_started_at: None,
            total_duration_of_running_jobs: 0.0,
            worker_occupancy_rate_history: Vec::new(),
            start_time,
        }
    }

    pub fn update_occupancy_metrics(&mut self) -> OccupancyResult {
        let metrics = self;
        let current_occupied_duration = metrics
            .running_job_started_at
            .map(|started_at| started_at.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        let total_occupation = metrics.total_duration_of_running_jobs + current_occupied_duration;

        let elapsed = metrics.start_time.elapsed().as_secs_f32();

        let (occupancy_rate_15s, occupancy_rate_5m, occupancy_rate_30m) =
            if !metrics.worker_occupancy_rate_history.is_empty() {
                let mut total_occupation_15s = 0.0;
                let mut total_occupation_5m = 0.0;
                let mut total_occupation_30m = 0.0;
                let mut index30m = 0;
                for (i, (past_total_occupation, time)) in
                    metrics.worker_occupancy_rate_history.iter().enumerate()
                {
                    let diff = elapsed - time;
                    if diff < 1800.0 && total_occupation_30m == 0.0 {
                        total_occupation_30m = (total_occupation - past_total_occupation) / diff;
                        index30m = i;
                    }
                    if diff < 300.0 && total_occupation_5m == 0.0 {
                        total_occupation_5m = (total_occupation - past_total_occupation) / diff;
                    }
                    if diff < 15.0 {
                        total_occupation_15s = (total_occupation - past_total_occupation) / diff;
                        break;
                    }
                }

                //drop all elements before the oldest one in 30m windows
                metrics.worker_occupancy_rate_history.drain(..index30m);

                (
                    Some(total_occupation_15s),
                    Some(total_occupation_5m),
                    Some(total_occupation_30m),
                )
            } else {
                (None, None, None)
            };
        let occupancy_rate = total_occupation / elapsed;

        //push the current occupancy rate and the timestamp
        metrics
            .worker_occupancy_rate_history
            .push((total_occupation, elapsed));

        OccupancyResult {
            occupancy_rate,
            occupancy_rate_15s,
            occupancy_rate_5m,
            occupancy_rate_30m,
        }
    }
}

pub async fn start_child_process(
    cmd: Command,
    executable: &str,
) -> Result<Box<dyn TokioChildWrapper>, Error> {
    use process_wrap::tokio::*;
    let mut cmd = TokioCommandWrap::from(cmd);
    #[cfg(unix)]
    {
        use process_wrap::tokio::ProcessGroup;

        cmd.wrap(ProcessGroup::leader());
    }
    #[cfg(windows)]
    {
        cmd.wrap(JobObject);
    }
    return cmd
        .spawn()
        .map_err(|err| tentatively_improve_error(err.into(), executable));
}

pub async fn resolve_job_timeout(
    _conn: &Connection,
    _w_id: &str,
    _job_id: Uuid,
    custom_timeout_secs: Option<i32>,
) -> (Duration, Option<String>, bool) {
    let mut warn_msg: Option<String> = None;
    #[cfg(feature = "cloud")]
    let cloud_premium_workspace = *CLOUD_HOSTED
        && windmill_common::workspaces::is_premium_workspace(
            _conn.as_sql().expect("cloud cannot use http connection"),
            _w_id,
        )
        .await;
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
            (Duration::from_secs(timeout_secs as u64), warn_msg, true)
        }
        Some(timeout_secs) => {
            warn_msg = Some(format!("WARNING: Custom job timeout of {timeout_secs} seconds was greater than the maximum timeout. It will be ignored and the max timeout will be used instead"));
            tracing::warn!(warn_msg);
            (global_max_timeout_duration, warn_msg, false)
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
            (default_timeout, warn_msg, false)
        }
    }
}

async fn hash_args(
    _db: &DB,
    _client: &AuthedClient,
    _workspace_id: &str,
    v: &Option<Json<HashMap<String, Box<RawValue>>>>,
    hasher: &mut sha2::Sha256,
) {
    if let Some(Json(hm)) = v {
        for k in hm.keys().sorted() {
            hasher.update(k.as_bytes());
            let arg_value = hm.get(k).unwrap();
            #[cfg(feature = "parquet")]
            let (_, arg_additions) =
                arg_value_hash_additions(_db, _client, _workspace_id, hm.get(k).unwrap()).await;
            hasher.update(arg_value.get().as_bytes());
            #[cfg(feature = "parquet")]
            for (_, arg_addition) in arg_additions {
                hasher.update(arg_addition.as_bytes());
            }
        }
    }
}

pub async fn cached_result_path(
    db: &DB,
    client: &AuthedClient,
    job: &MiniPulledJob,
    raw_data: Option<&RawData>,
) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(&[job.kind as u8]);
    if let Some(ScriptHash(hash)) = job.runnable_id {
        hasher.update(&hash.to_le_bytes())
    } else {
        job.runnable_path
            .as_ref()
            .inspect(|x| hasher.update(x.as_bytes()));
        match raw_data {
            Some(RawData::Flow(data)) => hasher.update(data.raw_flow.get()),
            Some(RawData::Script(data)) => hasher.update(&data.code),
            _ => {}
        }
    }
    hash_args(db, client, &job.workspace_id, &job.args, &mut hasher).await;
    format!("g/results/{:064x}", hasher.finalize())
}

#[cfg(feature = "parquet")]
async fn get_workspace_s3_resource_path(
    db: &DB,
    client: &AuthedClient,
    workspace_id: &str,
    storage: Option<&String>,
) -> windmill_common::error::Result<Option<ObjectStoreResource>> {
    use windmill_common::{
        job_s3_helpers_oss::get_s3_resource_internal, s3_helpers::StorageResourceType,
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
        Some(LargeFileStorage::GoogleCloudStorage(gcs)) => {
            let resource_path = gcs.gcs_resource_path.trim_start_matches("$res:");
            (
                StorageResourceType::GoogleCloudStorage,
                resource_path.to_string(),
            )
        }
        None => {
            return Ok(None);
        }
    };

    let s3_resource_value_raw = client
        .get_resource_value::<serde_json::Value>(path.as_str())
        .await?;
    get_s3_resource_internal(
        rt,
        s3_resource_value_raw,
        windmill_common::job_s3_helpers_oss::TokenGenerator::AsClient(client),
        db,
    )
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
            get_workspace_s3_resource_path(db, client, workspace_id, s3_object.storage.as_ref())
                .await;
        storage = s3_object.storage.clone();

        if let Some(mut s3_resource) = s3_resource_opt.ok().flatten() {
            let etag = get_etag_or_empty(&mut s3_resource, s3_object.clone()).await;
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
    value: Arc<Box<RawValue>>,
    storage: Option<String>,
}

impl CachedResource {
    fn expired(&self) -> bool {
        self.expire <= chrono::Utc::now().timestamp()
    }
}

lazy_static! {
    /// In-memory cache for resources.
    static ref CACHED_RESULTS: Cache<String, Arc<CachedResource>> = Cache::new(1000);
}

pub async fn get_cached_resource_value_if_valid(
    _db: &DB,
    client: &AuthedClient,
    _job_id: &Uuid,
    _workspace_id: &str,
    cached_res_path: &str,
) -> Option<Arc<Box<RawValue>>> {
    let resource = match CACHED_RESULTS
        .get_value_or_guard_async(cached_res_path)
        .await
    {
        // check for cache expiration.
        Ok(resource) if resource.expired() => {
            let _ = CACHED_RESULTS.remove(cached_res_path);
            return None;
        }
        // resource is in cache and not expired, return it.
        Ok(resource) => resource,
        // resource is not in cache, fetch it from the database.
        Err(entry) => match client
            .get_resource_value::<CachedResource>(cached_res_path)
            .await
            .ok()
        {
            None => return None,
            // check for cache expiration.
            Some(resource) if resource.expired() => return None,
            Some(resource) => {
                let resource = Arc::new(resource);
                let _ = entry.insert(resource.clone());
                resource
            }
        },
    };

    #[cfg(feature = "parquet")]
    {
        let empty_etags = HashMap::new();
        let s3_etags = resource.s3_etags.as_ref().unwrap_or(&empty_etags);
        let object_store_resource_opt: Option<ObjectStoreResource> = if s3_etags.is_empty() {
            None
        } else {
            get_workspace_s3_resource_path(_db, &client, _workspace_id, resource.storage.as_ref())
                .await
                .ok()
                .flatten()
        };

        if !s3_etags.is_empty() && object_store_resource_opt.is_none() {
            tracing::warn!("Cached result references s3 files that are not retrievable anymore because the workspace S3 resource can't be fetched. Cache will be invalidated");
            return None;
        }
        for (s3_file_key, s3_file_etag) in s3_etags {
            if let Some(object_store_resource) = object_store_resource_opt.as_ref() {
                let etag = get_etag_or_empty(
                    object_store_resource,
                    S3Object {
                        s3: s3_file_key.clone(),
                        storage: resource.storage.clone(),
                        ..Default::default()
                    },
                )
                .await;
                if etag.as_ref() != Some(s3_file_etag) {
                    tracing::warn!("S3 file etag for '{}' has changed. Value from cache is {:?} while current value from S3 is {:?}. Cache will be invalidated", s3_file_key.clone(), s3_file_etag, etag);
                    return None;
                }
            }
        }
    }

    Some(resource.value.clone())
}

pub async fn save_in_cache(
    db: &Pool<Postgres>,
    _client: &AuthedClient,
    job: &MiniPulledJob,
    cached_path: String,
    r: Arc<Box<RawValue>>,
) {
    let expire = chrono::Utc::now().timestamp() + job.cache_ttl.unwrap() as i64;

    #[cfg(feature = "parquet")]
    let (storage, s3_etags) =
        arg_value_hash_additions(db, _client, job.workspace_id.as_str(), &r).await;

    #[cfg(feature = "parquet")]
    let s3_etags = if s3_etags.is_empty() {
        None
    } else {
        Some(s3_etags)
    };

    #[cfg(not(feature = "parquet"))]
    let (storage, s3_etags) = (None, None);

    let store_cache_resource = CachedResource { expire, s3_etags, value: r, storage };
    let raw_json = Json(&store_cache_resource);

    if let Err(e) = sqlx::query!(
        "INSERT INTO resource
        (workspace_id, path, value, resource_type, created_by, edited_at)
        VALUES ($1, $2, $3, $4, $5, now()) ON CONFLICT (workspace_id, path)
        DO UPDATE SET value = $3, edited_at = now()",
        job.workspace_id,
        &cached_path,
        raw_json as Json<&CachedResource>,
        "cache",
        job.created_by
    )
    .execute(db)
    .await
    {
        tracing::error!("Error creating cache resource {e:#}")
    }

    // Cache result in-memory.
    CACHED_RESULTS.insert(cached_path, Arc::new(store_cache_resource));
}

fn tentatively_improve_error(err: Error, executable: &str) -> Error {
    #[cfg(unix)]
    let err_msgs = vec!["os error 2", "os error 3", "No such file or directory"];

    #[cfg(windows)]
    let err_msgs = vec!["program not found", "os error 2", "os error 3"];

    if err_msgs.iter().any(|msg| err.to_string().contains(msg)) {
        return Error::internal_err(format!(
            "Executable {executable} not found on worker. PATH: {}",
            *PATH_ENV
        ));
    }
    return Error::ExecutionErr(format!("Error executing {executable}: {err:#}"));
}

pub async fn clean_cache() -> error::Result<()> {
    tracing::info!("Started cleaning cache");
    tokio::fs::remove_dir_all(ROOT_CACHE_DIR).await?;
    tracing::info!("Finished cleaning cache");
    Ok(())
}

lazy_static::lazy_static! {
    static ref RE_FLOW_ROOT: Regex = Regex::new(r"(?i)(.*?)(?:/branchone-\d+/|/branchall-\d+/|/loop-\d+|/forloop-\d+/)").unwrap();

}

pub fn use_flow_root_path(flow_path: &str) -> String {
    if let Some(captures) = RE_FLOW_ROOT.captures(flow_path) {
        return captures
            .get(1)
            .map(|m| format!("{}/flow", m.as_str()))
            .unwrap_or_else(|| flow_path.to_string());
    } else {
        return flow_path.to_string();
    }
}

pub fn build_http_client(timeout_duration: std::time::Duration) -> error::Result<Client> {
    reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .timeout(timeout_duration)
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| Error::internal_err(format!("Error building http client: {e:#}")))
}

#[derive(Clone)]
pub struct RequiredDependency {
    /// Expected directory of dependency in cache
    /// For example:
    /// /tmp/windmill/cache/python_311/rich==0.0.0
    /// IMPORTANT!: path should not end with '/'
    pub path: String,
    /// Name to use for S3 tars
    /// If not specified will use top level directory of path.
    pub custom_name: Option<String>,
    /// Display name
    /// Name that will be used for console output and logging
    /// If not specified will either use custom_name or top level directory of path.
    pub short_name: Option<String>,
}

pub enum InstallStrategy {
    /// Will invoke callback to install single dependency
    Single(Arc<dyn Fn(RequiredDependency) -> Result<Command, error::Error> + Send + Sync>),
    /// Will try to pull S3 first and will invoke closure to install the rest
    AllAtOnce(Arc<dyn Fn(Vec<RequiredDependency>) -> Result<Command, error::Error> + Send + Sync>),
}
/// # General
///
/// Languages that compile usually include dependencies in final executable.
/// When dynamic languages do not and runtime dependencies are provided separately.
///
/// This helper assumes that the language is dynamic.
/// Python, Ruby, Java are dynamic and they can use this helper.
///
/// # Features
///
/// This helper will install all specified dependencies in parallel and if it is EE, cache to S3
/// It has atomic success file, allowing to distinguish failed installations from succesfull.
///
/// Besides that it provides console output and does logging.
///
/// # Usage
///
/// Most important arguments in this helper are `deps` and `install_fn`
///
/// In `deps` you specify all dependencies that are needed to be on worker in order to execute script.
/// You don't know which are actually installed and which are not.
///
/// `deps` is a vector of RequiredDependency. Check [RequiredDependency] for more context.
///
/// After `deps` are provided helper will check each dependency and check if it is in cache, if not it will try to pull from S3
/// and if it does not work either, it will invoke `install_fn` closure.
/// Closure arguments has dependency name as well as it`s expected path in cache.
/// Closure should return Command that will install dependency to asked place.
pub async fn par_install_language_dependencies<'a>(
    deps: Vec<RequiredDependency>,
    language_name: &'a str,
    installer_executable_name: &'a str,
    platform_agnostic: bool,
    concurrent_downloads: usize,
    stdout_on_err: bool,
    install_fn: InstallStrategy,
    postinstall_cb: impl AsyncFn(Vec<RequiredDependency>) -> Result<(), error::Error>,
    job_id: &'a Uuid,
    w_id: &'a str,
    worker_name: &'a str,
    conn: &Connection,
) -> anyhow::Result<()> {
    #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
    let _ = (platform_agnostic, language_name);

    let total_time = std::time::Instant::now();

    // Total to install
    let mut not_installed = vec![];
    let total_to_install;
    // let mut not_installed = vec![];
    let counter_arc = Arc::new(tokio::sync::Mutex::new(0));
    // Append logs with line like this:
    // [9/21]   +  requests==2.32.3            << (S3) |  in 57ms
    #[allow(unused_assignments)]
    async fn print_success(
        mut s3_pull: bool,
        mut s3_push: bool,
        job_id: &Uuid,
        w_id: &str,
        req: &str,
        req_tl: usize,
        counter_arc: Arc<tokio::sync::Mutex<usize>>,
        total_to_install: usize,
        instant: std::time::Instant,
        conn: &Connection,
    ) {
        #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
        {
            (s3_pull, s3_push) = (false, false);
        }

        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if windmill_common::s3_helpers::OBJECT_STORE_SETTINGS
            .read()
            .await
            .is_none()
        {
            (s3_pull, s3_push) = (false, false);
        }

        let mut counter = counter_arc.lock().await;
        *counter += 1;

        windmill_queue::append_logs(
            job_id,
            w_id,
            format!(
                "\n{}+  {}{}{}|  in {}ms",
                windmill_common::worker::pad_string(
                    &format!("[{}/{total_to_install}]", counter),
                    9
                ),
                // Because we want to align to max len [999/999] we take 9
                //                                     123456789
                windmill_common::worker::pad_string(&req, req_tl + 1),
                // Margin to the right    ^
                if s3_pull { "<< (S3) " } else { "" },
                if s3_push { " > (S3) " } else { "" },
                instant.elapsed().as_millis(),
            ),
            conn,
        )
        .await;
        // Drop lock, so next print success can fire
    }

    let mut name_tl = 0;
    struct NotInstalledDependency {
        path: String,
        custom_name: Option<String>,
        short_name: Option<String>,
        display_name: String,
    }
    {
        let mut to_be_installed_is_used = false;
        for RequiredDependency {
            path, //
            custom_name,
            short_name,
        } in deps.into_iter()
        {
            if path.ends_with("/") {
                anyhow::bail!("Internal error: path should not end with '/'")
            }
            let display_name = short_name
            .as_ref()
            .or(custom_name.as_ref())
            .or(path.split("/").last().map(|e| e.to_owned()).as_ref())
            .unwrap_or_else(|| {
                tracing::warn!(
                    workspace_id = %w_id,
                    job_id = %job_id,
                    "failed to parse top level directory name for {path}, fallback to full path.",
                );

                &path
            })
            .to_owned();
            {
                // Later will help us align text in log console
                if display_name.len() > name_tl {
                    name_tl = display_name.len();
                }
            }
            // Will look like: /tmp/windmill/cache/lang/dependency.valid.windmill
            if tokio::fs::metadata(path.clone() + ".valid.windmill")
                .await
                .is_err()
            {
                if !to_be_installed_is_used {
                    windmill_queue::append_logs(
                        job_id,
                        w_id,
                        format!("\n--- INSTALLATION ---\n\nTo be installed:\n\n"),
                        conn,
                    )
                    .await;
                    to_be_installed_is_used = true;
                }
                windmill_queue::append_logs(job_id, w_id, format!("- {display_name}\n"), conn)
                    .await;
                not_installed.push(NotInstalledDependency {
                    path,
                    custom_name,
                    short_name,
                    display_name,
                });
            }
        }
    }
    total_to_install = not_installed.len();
    if total_to_install == 0 {
        return Ok(());
    }
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    let is_not_pro = !matches!(
        windmill_common::ee_oss::get_license_plan().await,
        windmill_common::ee_oss::LicensePlan::Pro
    );
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if is_not_pro && matches!(install_fn, InstallStrategy::AllAtOnce(_)) {
        windmill_queue::append_logs(
            job_id,
            w_id,
            format!("\nLooking for packages on S3:\n"),
            conn,
        )
        .await;
    }

    // Parallelism level (N)
    let parallel_limit = // Semaphore will panic if value less then 1
            concurrent_downloads.clamp(1, 30);

    tracing::info!(
        workspace_id = %w_id,
        "Install parallel limit: {}, job: {}",
        parallel_limit,
        job_id
    );

    let mut handles = vec![];
    let semaphore = Arc::new(Semaphore::new(parallel_limit));
    let for_all_at_once = Arc::new(RwLock::new(vec![]));
    for NotInstalledDependency {
        //
        path,
        custom_name,
        short_name,
        display_name,
    } in not_installed
    {
        let permit = semaphore.clone().acquire_owned().await; // Acquire a permit

        if let Err(_) = permit {
            tracing::error!(
                workspace_id = %w_id,
                "Cannot acquire permit on semaphore, that can only mean that semaphore has been closed."
            );
            break;
        }

        let permit = permit.unwrap();

        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        let s3_pull_future = if is_not_pro {
            if let Some(os) = windmill_common::s3_helpers::get_object_store().await {
                Some(crate::global_cache::pull_from_tar(
                    os,
                    path.clone(),
                    language_name.to_owned(),
                    custom_name.clone(),
                    platform_agnostic,
                ))
            } else {
                None
            }
        } else {
            None
        };
        let child = {
            if let InstallStrategy::Single(ref callback, ..) = install_fn {
                let cmd = callback(RequiredDependency {
                    path: path.clone(),
                    custom_name: custom_name.clone(),
                    short_name: short_name.clone(),
                })?;
                tracing::debug!("{:?}", &cmd);
                Some(start_child_process(cmd, &installer_executable_name).await?)
            } else {
                None
            }
        };

        let (
            worker_name_2,
            path_2,
            display_name_2,
            custom_name,
            job_id_2,
            w_id_2,
            conn_2,
            counter_arc,
            language_name,
            installer_executable_name,
            for_all_at_once,
        ) = (
            worker_name.to_owned(),
            path.clone(),
            display_name.clone(),
            custom_name.clone(),
            job_id.clone(),
            w_id.to_owned(),
            conn.clone(),
            counter_arc.clone(),
            language_name.to_owned(),
            installer_executable_name.to_owned(),
            for_all_at_once.clone(),
        );
        #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
        let _ = language_name;

        let start = std::time::Instant::now();
        let handle = tokio::spawn(async move {
            let _permit = permit;

            #[cfg(all(feature = "enterprise", feature = "parquet"))]
            if let Some(s3_pull_future) = s3_pull_future {
                if let Err(e) = s3_pull_future.await {
                    tracing::info!(
                        workspace_id = %w_id_2,
                        "No tarball was found for {:?} on S3 or different problem occured {job_id_2}:\n{e}",
                        &custom_name.clone().unwrap_or(path)
                    );
                } else {
                    // TODO: Refactor
                    // Create a file to indicate that installation was successfull
                    let valid_path = path_2.clone() + ".valid.windmill";
                    // This is atomic operation, meaning, that it either completes and dependency is valid,
                    // or it does not and dependency is invalid and will be reinstalled next run
                    if let Err(e) = File::create(&valid_path).await {
                        tracing::error!(
                            workspace_id = %w_id_2,
                            job_id = %job_id_2,
                            "Failed to create {}!\n{e}\n
                                This file needed for jobs to function",
                            valid_path
                        );
                    };
                    print_success(
                        true,
                        false,
                        &job_id_2,
                        &w_id_2,
                        &display_name_2,
                        name_tl,
                        counter_arc,
                        total_to_install,
                        start,
                        &conn_2,
                    )
                    .await;
                    return;
                }
            }

            let Some(child) = child else
            // Happense only if All At Once mode is selected
            {
                let mut lock = for_all_at_once.write().await;
                lock.push(RequiredDependency {
                    path: path_2.clone(),
                    custom_name: custom_name.clone(),
                    short_name: short_name.clone(),
                });
                return;
            };
            if let Err(e) = crate::handle_child::handle_child(
                &job_id_2,
                &conn_2,
                // TODO: Return mem_peak
                &mut 0,
                // TODO: Return canceld_by_ref
                &mut None,
                child,
                !*DISABLE_NSJAIL,
                &worker_name_2,
                &w_id_2,
                &installer_executable_name,
                None,
                false,
                &mut None,
                None,
            )
            .await
            {
                windmill_queue::append_logs(
                    &job_id_2,
                    &w_id_2,
                    format!("error while installing {}: {e:?}", &display_name_2),
                    &conn_2,
                )
                .await;
            } else {
                // if let Some(cb) = postinstall_cb {
                //     if let Err(e) = cb(vec![RequiredDependency {
                //         path: path_2.clone(),
                //         custom_name: custom_name.clone(),
                //         short_name: short_name.clone(),
                //     }])
                //     .await
                //     {
                //         tracing::error!(
                //             workspace_id = %w_id_2,
                //             job_id = %job_id_2,
                //             "Postinstall callback failed!\n{e}\n
                //                 This might affect execution",
                //         );
                //     }
                // }
                print_success(
                    false,
                    true,
                    &job_id_2,
                    &w_id_2,
                    &display_name_2,
                    name_tl,
                    counter_arc,
                    total_to_install,
                    start,
                    &conn_2,
                )
                .await;
                // TODO: Refactor
                // Create a file to indicate that installation was successfull
                let valid_path = path_2.clone() + ".valid.windmill";
                // This is atomic operation, meaning, that it either completes and dependency is valid,
                // or it does not and dependency is invalid and will be reinstalled next run
                if let Err(e) = File::create(&valid_path).await {
                    tracing::error!(
                        workspace_id = %w_id_2,
                        job_id = %job_id_2,
                        "Failed to create {}!\n{e}\n
                            This file needed for jobs to function",
                        valid_path
                    );
                };
                #[cfg(all(feature = "enterprise", feature = "parquet"))]
                {
                    if let Some(os) = windmill_common::s3_helpers::get_object_store().await {
                        tokio::spawn(async move {
                            if let Err(e) = crate::global_cache::build_tar_and_push(
                                os,
                                path_2,
                                language_name,
                                custom_name,
                                platform_agnostic,
                            )
                            .await
                            {
                                tracing::warn!("failed to build tar and push: {e:?}");
                            }
                        });
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        if let Err(e) = handle.await {
            tracing::error!("Error joining handles: {e:?}");
        }
    }
    if !for_all_at_once.read().await.is_empty() {
        if let InstallStrategy::AllAtOnce(ref callback, ..) = install_fn {
            let for_all_at_once_copy = for_all_at_once.read().await.clone();
            windmill_queue::append_logs(
                job_id,
                w_id,
                format!("\n\nFetching {} packages...\n", for_all_at_once_copy.len()),
                &conn,
            )
            .await;
            let cmd = callback(for_all_at_once_copy.clone())?;
            tracing::debug!("{:?}", &cmd);
            let child = start_child_process(cmd, &installer_executable_name).await?;
            let mut buf = "".to_owned();
            let pipe_stdout = if stdout_on_err { Some(&mut buf) } else { None };
            if let Err(e) = crate::handle_child::handle_child(
                &(if pipe_stdout.is_some() {
                    Uuid::nil()
                } else {
                    *job_id
                }),
                conn,
                // TODO: Return mem_peak
                &mut 0,
                // TODO: Return canceld_by_ref
                &mut None,
                child,
                !*DISABLE_NSJAIL,
                &worker_name,
                &w_id,
                &installer_executable_name,
                None,
                false,
                &mut None,
                pipe_stdout,
            )
            .await
            {
                bail!(format!(
                    "error while installing dependencies: {e:?}\n{}",
                    buf
                ));
            }
            {
                postinstall_cb(for_all_at_once_copy.clone()).await?;
            }
            for RequiredDependency { path, custom_name: _custom_name, .. } in
                for_all_at_once_copy.into_iter()
            {
                // TODO: Refactor
                // Create a file to indicate that installation was successfull
                let valid_path = path.clone() + ".valid.windmill";
                // This is atomic operation, meaning, that it either completes and dependency is valid,
                // or it does not and dependency is invalid and will be reinstalled next run
                if let Err(e) = File::create(&valid_path).await {
                    tracing::error!(
                        workspace_id = %w_id,
                        job_id = %job_id,
                        "Failed to create {}!\n{e}\n
                            This file needed for jobs to function",
                        valid_path
                    );
                };
                #[cfg(all(feature = "enterprise", feature = "parquet"))]
                {
                    if let Some(os) = windmill_common::s3_helpers::get_object_store().await {
                        let language_name = language_name.to_owned();
                        tokio::spawn(async move {
                            if let Err(e) = crate::global_cache::build_tar_and_push(
                                os,
                                path,
                                language_name,
                                _custom_name,
                                platform_agnostic,
                            )
                            .await
                            {
                                tracing::warn!("failed to build tar and push: {e:?}");
                            }
                        });
                    }
                }
            }
        }
    }
    {
        let total_time = total_time.elapsed().as_millis();
        windmill_queue::append_logs(
            &job_id,
            w_id,
            format!(
                "\nDone. Time spent on installation phase: {}ms\n",
                total_time
            ),
            conn,
        )
        .await;
    }
    Ok(())
}

#[derive(Clone)]
pub struct S3ModeWorkerData {
    pub client: AuthedClient,
    pub object_key: String,
    pub format: S3ModeFormat,
    pub storage: Option<String>,
    pub workspace_id: String,
}

impl S3ModeWorkerData {
    pub async fn upload<S>(&self, stream: S) -> anyhow::Result<()>
    where
        S: futures::stream::TryStream + Send + 'static,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        bytes::Bytes: From<S::Ok>,
    {
        self.client
            .upload_s3_file(
                self.workspace_id.as_str(),
                self.object_key.clone(),
                self.storage.clone(),
                stream,
            )
            .await
    }

    pub fn to_return_s3_obj(&self) -> windmill_common::s3_helpers::S3Object {
        windmill_common::s3_helpers::S3Object {
            s3: self.object_key.clone(),
            storage: self.storage.clone(),
            ..Default::default()
        }
    }
}

pub fn s3_mode_args_to_worker_data(
    s3: S3ModeArgs,
    client: AuthedClient,
    job: &MiniPulledJob,
) -> S3ModeWorkerData {
    S3ModeWorkerData {
        client,
        storage: s3.storage,
        format: s3.format,
        object_key: format!(
            "{}/{}.{}",
            s3.prefix.unwrap_or_else(|| format!(
                "wmill_datalake/{}",
                job.runnable_path
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("unknown_script")
            )),
            job.id,
            s3_mode_extension(s3.format)
        ),
        workspace_id: job.workspace_id.clone(),
    }
}
