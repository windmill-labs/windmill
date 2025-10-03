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
use tokio::{fs::File, io::AsyncReadExt};

use windmill_common::flows::Step;
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

use anyhow::{anyhow, Result};
use windmill_parser_sql::{s3_mode_extension, S3ModeArgs, S3ModeFormat};
use windmill_queue::flow_status::get_step_of_flow_status;
use windmill_queue::MiniPulledJob;

use std::collections::HashSet;
use std::path::Path;
use std::{collections::HashMap, sync::Arc, time::Duration};

use uuid::Uuid;
use windmill_common::{variables, DB};

use tokio::{io::AsyncWriteExt, time::Instant};

use crate::agent_workers::UPDATE_PING_URL;
use crate::{JOB_DEFAULT_TIMEOUT, MAX_RESULT_SIZE, MAX_TIMEOUT_DURATION, PATH_ENV};
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

                    let root_job_id = get_root_job_id(&job);
                    let mc = build_crypt_with_key_suffix(
                        &db,
                        &job.workspace_id,
                        &root_job_id.to_string(),
                    )
                    .await?;
                    decrypt(&mc, encrypted.to_string()).and_then(|x| {
                        serde_json::from_str(&x).map_err(|e| {
                            Error::internal_err(format!(
                                "Failed to decrypt '$encrypted:' value: {e}"
                            ))
                        })
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

pub async fn merge_result_stream(
    result: error::Result<Box<RawValue>>,
    result_stream: Option<String>,
) -> error::Result<Box<RawValue>> {
    if let Some(result_stream) = result_stream {
        result.and_then(|x| {
            let mut value: Value = serde_json::from_str(x.get())?;

            // Insert the string at the "wm_stream" field
            if let Value::Object(ref mut map) = value {
                map.insert("wm_stream".to_string(), Value::String(result_stream));
            } else if value.is_null() {
                // return Ok(unsafe_raw(json))
                return Ok(to_raw_value(&json!(result_stream)));
            } else {
                return Ok(x);
            }

            // Convert back to RawValue
            let json_string = serde_json::to_string(&value)?;
            Ok(RawValue::from_string(json_string)?)
        })
    } else {
        result
    }
}
/// Read the `result.json` file. This function assumes that the file contains valid json and will
/// result in undefined behaviour if it isn't. If the result.json is user generated or otherwise
/// not guaranteed to be valid, use `read_and_check_result`
pub async fn read_result(
    job_dir: &str,
    result_stream: Option<String>,
) -> error::Result<Box<RawValue>> {
    let rf = read_file(&format!("{job_dir}/result.json")).await;
    merge_result_stream(rf, result_stream).await
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
        Some(get_root_job_id(job).to_string()),
        Some(job.scheduled_for.clone()),
        job.runnable_id,
        job.permissioned_as_end_user_email.clone(),
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

pub fn error_to_value(err: &Error) -> serde_json::Value {
    match err {
        Error::JsonErr(err) => err.clone(),
        _ => json!({"message": err.to_string(), "name": err.name()}),
    }
}

#[derive(Clone)]
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

lazy_static! {
    static ref DISABLE_PROCESS_GROUP: bool = std::env::var("DISABLE_PROCESS_GROUP").is_ok();
}

pub async fn start_child_process(
    cmd: Command,
    executable: &str,
    disable_process_group: bool,
) -> Result<Box<dyn TokioChildWrapper>, Error> {
    use process_wrap::tokio::*;
    let mut cmd = TokioCommandWrap::from(cmd);

    if !*DISABLE_PROCESS_GROUP && !disable_process_group {
        #[cfg(unix)]
        {
            use process_wrap::tokio::ProcessGroup;

            cmd.wrap(ProcessGroup::leader());
        }
        #[cfg(windows)]
        {
            cmd.wrap(JobObject);
        }
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
        && windmill_common::workspaces::get_team_plan_status(
            _conn.as_sql().expect("cloud cannot use http connection"),
            _w_id,
        )
        .await
        .premium;
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

pub fn get_root_job_id(job: &MiniPulledJob) -> uuid::Uuid {
    // fallback to flow_innermost_root_job and parent_job as root_job is not set if equal to innermost root job or parent job
    job.root_job
        .or(job.flow_innermost_root_job)
        .or(job.parent_job)
        .unwrap_or(job.id)
}

#[derive(Clone)]
pub struct StreamNotifier {
    db: DB,
    job_id: uuid::Uuid,
    parent_job: uuid::Uuid,
    root_job: uuid::Uuid,
}

#[async_recursion]
async fn check_if_nested_step_is_last(
    db: &DB,
    parent_job: Uuid,
    parent_of_parent_job: Option<Uuid>,
    root_job: Uuid,
    visited: Option<HashSet<Uuid>>,
) -> error::Result<bool> {
    // Initialize or use the provided visited set for cycle detection
    let mut visited = visited.unwrap_or_else(HashSet::new);

    // Check for cycles - if we've already visited this job, return false to break the recursion
    if !visited.insert(parent_job) {
        return Ok(false);
    }

    // get parent of parent job to get step of parent job
    let parent_of_parent_job = parent_of_parent_job.or(sqlx::query_scalar!(
        "SELECT parent_job FROM v2_job WHERE id = $1",
        parent_job
    )
    .fetch_one(db)
    .await?);
    if let Some(parent_of_parent_job) = parent_of_parent_job {
        // Check for cycles again with the parent_of_parent_job
        if !visited.insert(parent_of_parent_job) {
            return Ok(false);
        }

        let r = sqlx::query!(
            r#"SELECT 
                (flow_status->'step')::integer as step,
                jsonb_array_length(flow_status->'modules') as len,
                flow_status->'modules'->-1->>'branch_chosen' IS NOT NULL as is_branch_one,
                parent_job as ppp_job
            FROM v2_job 
                LEFT JOIN v2_job_status USING (id)
            WHERE v2_job.id = $1"#,
            parent_of_parent_job
        )
        .fetch_one(db)
        .await
        .map_err(|e| Error::internal_err(format!("fetching step flow status: {e:#}")))?;

        if let Some(step) = r.step {
            let step = Step::from_i32_and_len(step, r.len.unwrap_or(0) as usize);

            // if parent job is last and a branch one and
            // - root_job is equal to parent of parent job, return true
            // - root job is not equal to parent of parent job, recursively check if the parent of parent job is a branch one and last
            if step.is_last_step() && r.is_branch_one.unwrap_or(false) {
                if parent_of_parent_job == root_job {
                    return Ok(true);
                } else {
                    return check_if_nested_step_is_last(
                        db,
                        parent_of_parent_job,
                        r.ppp_job,
                        root_job,
                        Some(visited),
                    )
                    .await;
                }
            }
        }
    }

    Ok(false)
}

impl StreamNotifier {
    pub fn new(conn: &Connection, job: &MiniPulledJob) -> Option<Self> {
        let root_job = get_root_job_id(job);
        if job.is_flow_step() && job.parent_job.is_some() {
            match conn {
                Connection::Sql(db) => Some(Self {
                    db: db.clone(),
                    parent_job: job.parent_job.unwrap(),
                    job_id: job.id,
                    root_job,
                }),
                Connection::Http(_) => {
                    tracing::warn!(
                        "Flow job streaming is only supported for workers connected to a database"
                    );
                    None
                }
            }
        } else {
            None
        }
    }

    async fn update_flow_status_with_stream_job_inner(
        db: DB,
        parent_job: Uuid,
        job_id: Uuid,
        root_job: Uuid,
    ) -> Result<(), Error> {
        let step = get_step_of_flow_status(&db, parent_job).await?;

        if step.is_last_step()
            && (parent_job == root_job
                || check_if_nested_step_is_last(&db, parent_job, None, root_job, None).await?)
        {
            sqlx::query!(r#"
                    UPDATE v2_job_status
                    SET flow_status = jsonb_set(flow_status, array['stream_job'], to_jsonb($1::UUID::TEXT))
                    WHERE id = $2"#,
                    job_id,
                    root_job
                )
                .execute(&db)
                .await?;
        }

        Ok(())
    }

    pub fn update_flow_status_with_stream_job(&self) -> () {
        let db = self.db.clone();
        let parent_job = self.parent_job;
        let job_id = self.job_id;
        let root_job = self.root_job;
        tokio::spawn(async move {
            if let Err(err) =
                Self::update_flow_status_with_stream_job_inner(db, parent_job, job_id, root_job)
                    .await
            {
                tracing::error!("Could not notify about stream job {}: {err:#?}", parent_job);
            }
        });
    }
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
