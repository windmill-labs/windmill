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
use windmill_common::global_settings::NSJAIL_TMP_BACKING_DISK;
use windmill_common::variables::{build_crypt_with_key_suffix, decrypt};
use windmill_common::worker::{
    to_raw_value, update_ping_for_failed_init_script_query, write_file, Connection, Ping, PingType,
    CLOUD_HOSTED, ROOT_CACHE_DIR, WORKER_CONFIG,
};
use windmill_common::workspace_dependencies::WorkspaceDependenciesPrefetched;
use windmill_common::{
    cache::{Cache, RawData},
    error::{self, Error},
    scripts::ScriptHash,
    utils::configure_client,
    variables::ContextualVariable,
};
#[cfg(feature = "parquet")]
use windmill_object_store::get_etag_or_empty;
#[cfg(feature = "parquet")]
use windmill_types::s3::{LargeFileStorage, ObjectStoreResource, S3Object};

use anyhow::{anyhow, Result};
use windmill_parser_sql::{s3_mode_extension, S3ModeArgs, S3ModeFormat};
use windmill_queue::{MiniCompletedJob, MiniPulledJob};

use std::collections::HashSet;
use std::path::Path;
use std::{collections::HashMap, sync::Arc, time::Duration};

use uuid::Uuid;
use windmill_common::{variables, DB};

use tokio::{io::AsyncWriteExt, time::Instant};

use crate::agent_workers::UPDATE_PING_URL;
use crate::{
    JOB_DEFAULT_TIMEOUT, MAX_RESULT_SIZE, MAX_TIMEOUT_DURATION, NSJAIL_TMPFS_SIZE_MB,
    NSJAIL_TMP_BACKING, PATH_ENV,
};
use windmill_common::client::AuthedClient;

/// Additional nsjail config for development. Currently used for nix flake.
#[cfg(debug_assertions)]
pub const DEV_CONF_NSJAIL: &str = r#"
mount {
    src: "/nix/store"
    dst: "/nix/store"
    is_bind: true
    mandatory: false
}
"#;

#[cfg(not(debug_assertions))]
pub const DEV_CONF_NSJAIL: &str = "";

/// Turn a JSON value into the string a shell/CLI arg should receive: a JSON string
/// becomes its inner value, anything else is re-serialized compactly.
pub(crate) fn raw_to_string(x: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(x) {
        Ok(serde_json::Value::String(x)) => x,
        Ok(x) => serde_json::to_string(&x).unwrap_or_else(|_| String::new()),
        _ => String::new(),
    }
}

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
        if let Some(mut x) = transform_json(client, &job.workspace_id, &args.0, job, conn).await? {
            x.remove("_MODULES");
            x.remove("_TEMP_SCRIPT_REFS");
            write_file(
                job_dir,
                "args.json",
                &serde_json::to_string(&x).unwrap_or_else(|_| "{}".to_string()),
            )?;
        } else {
            let mut filtered = args.0.clone();
            filtered.remove("_MODULES");
            filtered.remove("_TEMP_SCRIPT_REFS");
            write_file(
                job_dir,
                "args.json",
                &serde_json::to_string(&filtered).unwrap_or_else(|_| "{}".to_string()),
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
    static ref RE_RES_VAR: Regex = Regex::new(r#"\$(?:var|jsonvar|res|encrypted)\:|"\$interpolate""#).unwrap();
}

/// Returns true if any value in `vs` contains a `$var:`/`$jsonvar:`/`$res:`/`$encrypted:`
/// reference or an `$interpolate` object that would need interpolation by
/// `transform_json`. Cheap pre-check that callers can use to skip the DB
/// roundtrip + clone path when nothing requires resolution.
pub(crate) fn map_needs_resolution(vs: &HashMap<String, Box<RawValue>>) -> bool {
    vs.values().any(|v| (*RE_RES_VAR).is_match(v.get()))
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
            let transformed =
                transform_json_value(&k, &client, workspace, value, job, db, 0).await?;
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
            let transformed =
                transform_json_value(&k, &client, workspace, value, job, db, 0).await?;
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

// Defense-in-depth bound on worker-side interpolation recursion. `$res:`
// resolution is delegated to the API (which enforces its own
// MAX_RESOURCE_INTERPOLATION_DEPTH), so this only bounds nested
// object/array structure here, but a finite cap guards against pathological
// inputs regardless of the API-side guard.
const MAX_INTERPOLATION_DEPTH: u8 = 50;

/// Resolve the parts of an `$interpolate` arg into the final string. Returns
/// `Ok(None)` when a part has a shape the transform compiler never emits, so
/// the caller falls back to treating the object as plain data.
async fn resolve_interpolate_parts(
    name: &str,
    client: &AuthedClient,
    parts: &[Value],
) -> error::Result<Option<String>> {
    enum Part<'a> {
        Literal(&'a str),
        Var(&'a str),
    }
    // Validate the full shape before fetching anything.
    let mut validated = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            Value::String(s) => validated.push(Part::Literal(s)),
            Value::Object(o) if o.len() == 1 => {
                let Some(Value::String(path)) = o.get("$var") else {
                    return Ok(None);
                };
                validated.push(Part::Var(path));
            }
            _ => return Ok(None),
        }
    }
    let mut out = String::new();
    for part in validated {
        match part {
            Part::Literal(s) => out.push_str(s),
            Part::Var(path) => {
                let value = client.get_variable_value(path).await.map_err(|e| {
                    Error::NotFound(format!("Variable {path} not found for `{name}`: {e:#}"))
                })?;
                out.push_str(&value);
            }
        }
    }
    Ok(Some(out))
}

#[async_recursion]
pub async fn transform_json_value(
    name: &str,
    client: &AuthedClient,
    workspace: &str,
    v: Value,
    job: &MiniPulledJob,
    conn: &Connection,
    depth: u8,
) -> error::Result<Value> {
    if depth >= MAX_INTERPOLATION_DEPTH {
        return Err(Error::internal_err(format!(
            "Maximum resource/variable interpolation depth ({MAX_INTERPOLATION_DEPTH}) exceeded for `{name}`; this usually indicates a circular `$res:` or `$var:` reference"
        )));
    }
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
        Value::String(y) if y.starts_with("$jsonvar:") => {
            let path = y.strip_prefix("$jsonvar:").unwrap();
            let v = client.get_variable_value(path).await.map_err(|e| {
                Error::NotFound(format!("Variable {path} not found for `{name}`: {e:#}"))
            })?;
            serde_json::from_str::<serde_json::Value>(&v).map_err(|e| {
                Error::internal_err(format!("Failed to parse $jsonvar value as JSON: {e}"))
            })
        }
        Value::String(y) if y.starts_with("$res:") => {
            let path = y.strip_prefix("$res:").unwrap();

            if path.split("/").count() < 2 && !path.starts_with("CUSTOM_INSTANCE_DB/") {
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
                        // Register the raw decrypted string for log masking.
                        // This covers both string values and their JSON representations
                        // (numbers, objects, etc.) that could appear in logs.
                        windmill_common::sensitive_log_masks::register_secret_for_job(job.id, &x);
                        if let serde_json::Value::String(ref s) =
                            serde_json::from_str::<serde_json::Value>(&x).unwrap_or_default()
                        {
                            // Also register the inner string value (without JSON quotes)
                            windmill_common::sensitive_log_masks::register_secret_for_job(
                                job.id, s,
                            );
                        }
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
        Value::Array(mut arr) if depth <= 2 && arr.len() <= 1000 => {
            for i in 0..arr.len() {
                let val = std::mem::take(&mut arr[i]);
                arr[i] = transform_json_value(name, client, workspace, val, job, conn, depth + 1)
                    .await?;
            }
            Ok(Value::Array(arr))
        }
        Value::Array(arr) => {
            if arr.len() > 1000 {
                tracing::warn!(
                    "Array with {} items exceeds 1000 item limit for variable/resource resolution, skipping",
                    arr.len()
                );
            }
            Ok(Value::Array(arr))
        }
        Value::Object(mut m) => {
            // `{"$interpolate": ["literal", {"$var": "path"}, ...]}` args are produced
            // by flow input transforms that embed `variable()` calls in a template
            // literal (see `transform_var_defer`): the variable is fetched here, on the
            // job's own worker, so it gets registered for log masking and its raw value
            // never appears in the job's stored args. Shapes the compiler never emits
            // (non-string/non-`$var` parts) are treated as plain data below instead.
            if m.len() == 1 {
                if let Some(Value::Array(parts)) = m.get("$interpolate") {
                    if let Some(out) = resolve_interpolate_parts(name, client, parts).await? {
                        return Ok(Value::String(out));
                    }
                }
            }
            for (a, b) in m.clone().into_iter() {
                m.insert(
                    a.clone(),
                    transform_json_value(&a, client, workspace, b, job, conn, depth + 1).await?,
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

pub use windmill_common::utils::unsafe_raw;

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

    let tested_runnable = match (&job.trigger_kind, &job.trigger) {
        (Some(windmill_common::jobs::JobTriggerKind::CiTest), Some(t)) => Some(t.clone()),
        _ => None,
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
        tested_runnable,
    )
    .await
    .to_vec();

    Ok(build_envs_map(variables).await)
}

pub async fn build_envs_map(context: Vec<ContextualVariable>) -> HashMap<String, String> {
    let mut r: HashMap<String, String> =
        context.into_iter().map(|rv| (rv.name, rv.value)).collect();

    let envs = (**WORKER_CONFIG.load()).clone().env_vars;
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
                        dws: None,
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
                        job_isolation: None,
                        native_mode: None,
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

                // Only report occupancy for a window if the worker has been running
                // long enough to have meaningful data for that window. Otherwise,
                // short-lived workers would report misleadingly high occupancy rates.
                (
                    if elapsed >= 15.0 {
                        Some(total_occupation_15s)
                    } else {
                        None
                    },
                    if elapsed >= 300.0 {
                        Some(total_occupation_5m)
                    } else {
                        None
                    },
                    if elapsed >= 1800.0 {
                        Some(total_occupation_30m)
                    } else {
                        None
                    },
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

/// 2 GB memory limit in bytes for LIMIT_WINDOWS_TO_1CU
#[cfg(windows)]
const MEMORY_LIMIT_1CU: usize = 2 * 1024 * 1024 * 1024;

/// Wrapper that holds a Windows Job Object handle alongside the child process.
/// The job object carries KILL_ON_JOB_CLOSE and/or a memory limit. With
/// KILL_ON_JOB_CLOSE, the worker process dying closes the handle and the OS reaps the
/// whole child tree — preventing orphans when the worker is force-killed (e.g. a second
/// CTRL_BREAK_EVENT or Nomad's kill_timeout) before a job has drained.
#[cfg(windows)]
struct WindowsJobChild {
    inner: Box<dyn TokioChildWrapper>,
    _job_handle: Win32JobHandle,
}

#[cfg(windows)]
impl std::fmt::Debug for WindowsJobChild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsJobChild").finish()
    }
}

/// RAII wrapper for a raw Win32 HANDLE that closes it on drop.
#[cfg(windows)]
struct Win32JobHandle(windows::Win32::Foundation::HANDLE);

// SAFETY: Win32 HANDLEs are plain pointer-sized values with no thread affinity;
// the kernel ref-counts the underlying object, so sending/sharing the handle is safe.
#[cfg(windows)]
unsafe impl Send for Win32JobHandle {}
#[cfg(windows)]
unsafe impl Sync for Win32JobHandle {}

#[cfg(windows)]
impl Drop for Win32JobHandle {
    fn drop(&mut self) {
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(self.0) };
    }
}

#[cfg(windows)]
impl process_wrap::tokio::TokioChildWrapper for WindowsJobChild {
    fn inner(&self) -> &tokio::process::Child {
        self.inner.inner()
    }
    fn inner_mut(&mut self) -> &mut tokio::process::Child {
        self.inner.inner_mut()
    }
    fn into_inner(self: Box<Self>) -> tokio::process::Child {
        // Leak the job handle: with KILL_ON_JOB_CLOSE, closing the last handle reaps
        // the (still-running) child, so extracting the inner Child must not close it.
        let WindowsJobChild { inner, _job_handle } = *self;
        std::mem::forget(_job_handle);
        inner.into_inner()
    }
    fn start_kill(&mut self) -> std::io::Result<()> {
        self.inner.start_kill()
    }
    fn wait(
        &mut self,
    ) -> Box<dyn std::future::Future<Output = std::io::Result<std::process::ExitStatus>> + Send + '_>
    {
        self.inner.wait()
    }
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.inner.try_wait()
    }
}

/// Resume all threads of a process created with CREATE_SUSPENDED. We create the child
/// suspended so it can be assigned to its job object before running any code; otherwise
/// a child that forks a helper at startup could create it before the assignment and
/// leave it outside the job (escaping KILL_ON_JOB_CLOSE reaping / the memory cap).
/// (Ported from process-wrap's resume_threads.)
#[cfg(windows)]
fn resume_process(pid: u32) -> Result<(), std::io::Error> {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("CreateToolhelp32Snapshot: {e}"),
            )
        })?;
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            cntUsage: 0,
            th32ThreadID: 0,
            th32OwnerProcessID: 0,
            tpBasePri: 0,
            tpDeltaPri: 0,
            dwFlags: 0,
        };
        let mut res = Thread32First(snapshot, &mut entry);
        while res.is_ok() {
            if entry.th32OwnerProcessID == pid {
                if let Ok(thread) = OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID) {
                    ResumeThread(thread);
                    let _ = windows::Win32::Foundation::CloseHandle(thread);
                }
            }
            res = Thread32Next(snapshot, &mut entry);
        }
        let _ = windows::Win32::Foundation::CloseHandle(snapshot);
    }
    Ok(())
}

/// Create a Windows Job Object and assign the process to it, optionally with
/// KILL_ON_JOB_CLOSE (so the child tree is reaped when the worker drops the handle /
/// dies) and/or a memory limit. Assigning post-spawn (rather than via process-wrap's
/// suspend/resume JobObject wrap) keeps the dotnet-safe behavior that csharp relies on.
#[cfg(windows)]
fn assign_job_object(
    pid: u32,
    memory_limit: Option<usize>,
    kill_on_close: bool,
) -> Result<Win32JobHandle, std::io::Error> {
    use windows::Win32::System::JobObjects::*;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    unsafe {
        let job = CreateJobObjectW(None, None).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("CreateJobObjectW: {e}"))
        })?;

        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        if kill_on_close {
            info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        }
        if let Some(memory_limit) = memory_limit {
            info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_JOB_MEMORY;
            info.JobMemoryLimit = memory_limit;
        }

        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .map_err(|e| {
            let _ = windows::Win32::Foundation::CloseHandle(job);
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("SetInformationJobObject: {e}"),
            )
        })?;

        let process_handle = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
            .map_err(|e| {
                let _ = windows::Win32::Foundation::CloseHandle(job);
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("OpenProcess({pid}): {e}"),
                )
            })?;

        let assign_result = AssignProcessToJobObject(job, process_handle);
        let _ = windows::Win32::Foundation::CloseHandle(process_handle);
        assign_result.map_err(|e| {
            let _ = windows::Win32::Foundation::CloseHandle(job);
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("AssignProcessToJobObject: {e}"),
            )
        })?;

        Ok(Win32JobHandle(job))
    }
}

pub fn build_command_with_isolation(program: &str, args: &[&str]) -> Command {
    use tokio::process::Command;

    if crate::is_unshare_enabled() {
        if let Some(unshare_path) = crate::UNSHARE_PATH.as_ref() {
            let mut cmd = Command::new(unshare_path);

            let flags = crate::UNSHARE_ISOLATION_FLAGS.as_str();
            for flag in flags.split_whitespace() {
                cmd.arg(flag);
            }

            // If tini is available, use it for proper PID 1 signal handling
            // (especially OOM exit codes which return 137 instead of sigprocmask errors).
            // Note: --fork should already be in the flags for proper namespace setup.
            if let Some(tini_path) = crate::TINI_AVAILABLE.as_ref() {
                cmd.arg("--");
                cmd.arg(tini_path);
                cmd.arg("-s");
                cmd.arg("--");
            } else {
                // Without tini, just run the command directly (--fork is in flags)
                cmd.arg("--");
            }

            cmd.arg(program);
            cmd.args(args);
            cmd
        } else {
            tracing::error!(
                "unshare isolation is enabled but UNSHARE_PATH is not available. \
                Running job without isolation. Check Instance Settings > Job Isolation."
            );
            let mut cmd = Command::new(program);
            cmd.args(args);
            cmd
        }
    } else {
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd
    }
}

pub async fn start_child_process(
    cmd: Command,
    executable: &str,
    disable_process_group: bool,
) -> Result<Box<dyn TokioChildWrapper>, Error> {
    use process_wrap::tokio::*;
    let mut cmd = TokioCommandWrap::from(cmd);

    // On Windows, put the child in its own console process group so a CTRL_BREAK_EVENT
    // sent to the worker's group (e.g. by Nomad on scale-in) is not delivered to it.
    // Otherwise the whole child tree dies instantly with STATUS_CONTROL_C_EXIT
    // (0xC000013A) before the worker can drain running jobs.
    //
    // This deliberately does NOT use process-wrap's JobObject wrap: its pre_spawn calls
    // command.creation_flags(CREATE_SUSPENDED | get_wrap::<CreationFlags>()), but
    // get_wrap returns None mid-spawn (process-wrap 8.2.1 takes the wrappers out of
    // `self` before running pre_spawn), so it silently overwrites and drops
    // CREATE_NEW_PROCESS_GROUP. Job-object grouping is done post-spawn below instead,
    // which also matches the dotnet-safe assignment csharp already relies on.
    #[cfg(windows)]
    if !*DISABLE_PROCESS_GROUP {
        use process_wrap::tokio::CreationFlags;
        use windows::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_SUSPENDED};
        // Also create the child suspended so it can be placed in its job object (below)
        // before it runs any code; otherwise a child that forks a helper at startup could
        // create it before the assignment and leave it outside the job. Resumed right
        // after the job assignment.
        cmd.wrap(CreationFlags(CREATE_NEW_PROCESS_GROUP | CREATE_SUSPENDED));
    }

    if !*DISABLE_PROCESS_GROUP && !disable_process_group {
        #[cfg(unix)]
        {
            use process_wrap::tokio::ProcessGroup;

            cmd.wrap(ProcessGroup::leader());
        }
    }

    let child: Box<dyn TokioChildWrapper> = cmd
        .spawn()
        .map_err(|err| tentatively_improve_error(err.into(), executable))?;

    // On Windows, assign the child to a job object. KILL_ON_JOB_CLOSE makes the OS reap
    // the whole child tree when the worker drops the handle or dies, so jobs aren't
    // orphaned if the worker is force-killed (second CTRL_BREAK_EVENT, or Nomad exceeding
    // kill_timeout) before they drain; it is gated on the DISABLE_PROCESS_GROUP escape
    // hatch alongside CREATE_NEW_PROCESS_GROUP above. The LIMIT_WINDOWS_TO_1CU memory cap
    // is independent of that hatch (it's its own opt-in), so it's applied whenever set.
    // Assigning post-spawn does not touch the creation flags (so it composes with
    // CREATE_NEW_PROCESS_GROUP) and is the dotnet-safe path csharp already uses.
    #[cfg(windows)]
    {
        let kill_on_close = !*DISABLE_PROCESS_GROUP;
        let memory_limit =
            (*windmill_common::worker::LIMIT_WINDOWS_TO_1CU).then_some(MEMORY_LIMIT_1CU);
        if kill_on_close || memory_limit.is_some() {
            if let Some(pid) = child.inner().id() {
                let assigned = assign_job_object(pid, memory_limit, kill_on_close);
                // When kill_on_close, the child was created suspended (CREATE_SUSPENDED
                // above) so it could be placed in the job before running. Resume it now —
                // unconditionally of assign success, so a failed assign never leaves it hung.
                if kill_on_close {
                    if let Err(e) = resume_process(pid) {
                        tracing::error!("Failed to resume child process {pid}: {e}");
                    }
                }
                match assigned {
                    Ok(job_handle) => {
                        if memory_limit.is_some() {
                            tracing::info!(
                                "Applied 2GB memory limit (LIMIT_WINDOWS_TO_1CU) to child process {pid}"
                            );
                        }
                        return Ok(Box::new(WindowsJobChild {
                            inner: child,
                            _job_handle: job_handle,
                        }));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to assign child process {pid} to job object: {e}");
                    }
                }
            }
        }
    }

    Ok(child)
}

pub async fn resolve_job_timeout(
    _conn: &Connection,
    _w_id: &str,
    _job_id: Uuid,
    custom_timeout_secs: Option<i32>,
) -> (Duration, Option<String>, bool) {
    let mut warn_msg: Option<String> = None;
    #[cfg(feature = "cloud")]
    let cloud_premium_workspace =
        *CLOUD_HOSTED
        && windmill_common::workspaces::get_team_plan_status(
            _conn.as_sql().expect("cloud cannot use http connection"),
            _w_id,
        )
        .await
        .inspect_err(|err| tracing::error!("Failed to get team plan status to resolve job timeout for workspace {_w_id}: {err:#}"))
        .map(|s| s.premium)
        .unwrap_or(true);
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

/// Compute the nsjail timeout (in seconds) with a 15s buffer so handle_child fires first.
pub async fn resolve_nsjail_timeout(
    conn: &Connection,
    w_id: &str,
    job_id: Uuid,
    custom_timeout: Option<i32>,
) -> String {
    let (duration, _, _) = resolve_job_timeout(conn, w_id, job_id, custom_timeout).await;
    (duration.as_secs() + 15).to_string()
}

/// Default size (in bytes) of the `/tmp` tmpfs mount inside nsjail sandboxes,
/// used when the `nsjail_tmpfs_size_mb` instance setting is unset.
pub const DEFAULT_NSJAIL_TMPFS_SIZE_BYTES: u64 = 800_000_000;

/// Resolve the tmpfs `size=` value (in bytes, formatted for the nsjail proto)
/// for the `/tmp` tmpfs mount. When the `nsjail_tmpfs_size_mb` instance setting
/// is `None`, `Some(0)`, or negative, falls back to
/// [`DEFAULT_NSJAIL_TMPFS_SIZE_BYTES`].
pub async fn resolve_nsjail_tmpfs_size_bytes() -> String {
    match *NSJAIL_TMPFS_SIZE_MB.read().await {
        Some(mb) if mb > 0 => ((mb as u64).saturating_mul(1_000_000)).to_string(),
        _ => DEFAULT_NSJAIL_TMPFS_SIZE_BYTES.to_string(),
    }
}

/// Sub-directory inside each job dir used as the disk-backed `/tmp` when
/// `nsjail_tmp_disk_backed` is enabled. Kept under `{JOB_DIR}` so existing
/// job-dir cleanup removes it for free.
const NSJAIL_TMP_BIND_SUBDIR: &str = "jail_tmp";

fn tmpfs_mount_block(size_bytes: &str) -> String {
    format!(
        "mount {{\n    dst: \"/tmp\"\n    fstype: \"tmpfs\"\n    rw: true\n    options: \"size={size_bytes}\"\n}}"
    )
}

fn bind_mount_block(jail_tmp: &str) -> String {
    format!(
        "mount {{\n    src: \"{jail_tmp}\"\n    dst: \"/tmp\"\n    is_bind: true\n    rw: true\n}}"
    )
}

/// Build the nsjail `mount { ... }` block that backs `/tmp` inside the
/// sandbox.
///
/// **Caller contract**: `job_dir` must be a trusted, worker-allocated job
/// directory (typically `{worker_dir}/{job_id}`). In disk-backed mode this
/// function creates `{job_dir}/jail_tmp` and bind-mounts it as `/tmp` with
/// `rw: true`. Callers must not pass user-controlled paths.
///
/// Some executors (e.g. the bun codebase path) extract user-supplied archives
/// into `job_dir` before this resolver runs, so the resolver actively refuses
/// any pre-existing entry at `{job_dir}/jail_tmp` (including symlinks) to
/// avoid bind-mounting an attacker-controlled host directory as `/tmp`.
///
/// When the `nsjail_tmp_backing` instance setting is `"disk"`, returns a
/// disk-backed bind mount of `{job_dir}/jail_tmp` after creating the
/// directory. If creation or the pre-existence check fails, logs an error and
/// falls back to the historical tmpfs block so the job can still start. For
/// any other value (including unset, `"tmpfs"`, or unrecognized), returns the
/// historical RAM-backed tmpfs mount sized via `nsjail_tmpfs_size_mb`.
pub(crate) async fn resolve_nsjail_tmp_mount_block(job_dir: &str) -> String {
    let disk_backed = NSJAIL_TMP_BACKING
        .read()
        .await
        .as_deref()
        .map(|v| v.eq_ignore_ascii_case(NSJAIL_TMP_BACKING_DISK))
        .unwrap_or(false);
    let size_bytes = resolve_nsjail_tmpfs_size_bytes().await;
    if !disk_backed {
        return tmpfs_mount_block(&size_bytes);
    }
    let jail_tmp = format!("{job_dir}/{NSJAIL_TMP_BIND_SUBDIR}");

    // SECURITY: never bind-mount a symlinked (or otherwise non-directory)
    // entry at jail_tmp. `symlink_metadata` returns the link's own metadata
    // without following it, so `is_dir()` is true only for a real directory.
    // User-controlled archives extracted into job_dir could otherwise plant
    // `jail_tmp` as a symlink to an arbitrary host directory, which nsjail
    // would then expose as a writable /tmp.
    //
    // A pre-existing real directory at this path is legitimate: several
    // executors (python_executor, ruby_executor, rust_executor) invoke nsjail
    // more than once per job_dir (e.g. dep install, then run), and the first
    // invocation will have created it via the `create_dir` below.
    match tokio::fs::symlink_metadata(&jail_tmp).await {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if let Err(e) = tokio::fs::create_dir(&jail_tmp).await {
                tracing::error!(
                    "Failed to create nsjail disk-backed /tmp at {jail_tmp}: {e:?}; \
                     falling back to tmpfs for this job."
                );
                return tmpfs_mount_block(&size_bytes);
            }
        }
        Ok(meta) if meta.is_dir() => {
            // Real directory left over from an earlier nsjail invocation in
            // this same job_dir — safe to reuse.
        }
        Ok(_) => {
            tracing::error!(
                "Refusing to bind-mount nsjail disk-backed /tmp: {jail_tmp} \
                 exists but is not a regular directory (possibly a symlink \
                 planted by a user-controlled archive). Falling back to \
                 RAM-backed tmpfs for this job."
            );
            return tmpfs_mount_block(&size_bytes);
        }
        Err(e) => {
            tracing::error!(
                "Failed to stat nsjail disk-backed /tmp at {jail_tmp}: {e:?}; \
                 falling back to tmpfs for this job."
            );
            return tmpfs_mount_block(&size_bytes);
        }
    }
    bind_mount_block(&jail_tmp)
}

#[cfg(test)]
mod nsjail_tmp_mount_tests {
    use super::*;

    #[test]
    fn tmpfs_block_renders_size() {
        let block = tmpfs_mount_block("800000000");
        assert!(block.contains("dst: \"/tmp\""));
        assert!(block.contains("fstype: \"tmpfs\""));
        assert!(block.contains("options: \"size=800000000\""));
        assert!(!block.contains("is_bind"));
    }

    #[test]
    fn bind_block_renders_source_path() {
        let block = bind_mount_block("/var/lib/windmill/jobs/abc/jail_tmp");
        assert!(block.contains("src: \"/var/lib/windmill/jobs/abc/jail_tmp\""));
        assert!(block.contains("dst: \"/tmp\""));
        assert!(block.contains("is_bind: true"));
        assert!(block.contains("rw: true"));
        assert!(!block.contains("fstype"));
    }

    /// Serializes tests that mutate the process-global `NSJAIL_TMP_BACKING`
    /// so they don't race when cargo runs them in parallel.
    static SETTING_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    async fn with_tmp_backing<F, Fut, T>(value: Option<String>, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let _serial = SETTING_GUARD.lock().await;
        let prev = NSJAIL_TMP_BACKING.read().await.clone();
        *NSJAIL_TMP_BACKING.write().await = value;
        let res = f().await;
        *NSJAIL_TMP_BACKING.write().await = prev;
        res
    }

    #[tokio::test]
    async fn tmpfs_mode_returns_tmpfs_block_for_any_job_dir() {
        let block = with_tmp_backing(Some("tmpfs".to_string()), || async {
            resolve_nsjail_tmp_mount_block("/anything").await
        })
        .await;
        assert!(block.contains("fstype: \"tmpfs\""));
        assert!(block.contains("options: \"size="));
        assert!(!block.contains("is_bind"));
    }

    #[tokio::test]
    async fn unset_defaults_to_tmpfs() {
        let block = with_tmp_backing(None, || async {
            resolve_nsjail_tmp_mount_block("/anything").await
        })
        .await;
        assert!(block.contains("fstype: \"tmpfs\""));
        assert!(!block.contains("is_bind"));
    }

    /// Disk-backed branch: the resolver must create `{job_dir}/jail_tmp` and
    /// emit a bind block pointing at it.
    #[tokio::test]
    async fn disk_backed_creates_jail_tmp_and_returns_bind_block() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let job_dir = tmp.path().to_str().expect("utf8 path").to_string();

        let block = with_tmp_backing(Some("disk".to_string()), || async {
            resolve_nsjail_tmp_mount_block(&job_dir).await
        })
        .await;

        let expected_dir = format!("{job_dir}/{NSJAIL_TMP_BIND_SUBDIR}");
        assert!(
            std::path::Path::new(&expected_dir).is_dir(),
            "jail_tmp dir should have been created at {expected_dir}"
        );
        assert!(block.contains("is_bind: true"));
        assert!(block.contains(&format!("src: \"{expected_dir}\"")));
    }

    /// Disk-backed branch fallback: if `create_dir_all` fails, we must emit
    /// the tmpfs block instead of returning an invalid bind config.
    #[tokio::test]
    async fn disk_backed_falls_back_to_tmpfs_on_mkdir_error() {
        // /proc is a kernel filesystem that disallows directory creation,
        // so create_dir_all on a subpath returns EPERM/EACCES.
        let job_dir = "/proc/win1967_should_not_exist";

        let block = with_tmp_backing(Some("disk".to_string()), || async {
            resolve_nsjail_tmp_mount_block(job_dir).await
        })
        .await;

        assert!(block.contains("fstype: \"tmpfs\""));
        assert!(!block.contains("is_bind"));
    }

    /// Unknown values fall through to the tmpfs branch instead of crashing.
    #[tokio::test]
    async fn unknown_value_defaults_to_tmpfs() {
        let block = with_tmp_backing(Some("bogus".to_string()), || async {
            resolve_nsjail_tmp_mount_block("/anything").await
        })
        .await;
        assert!(block.contains("fstype: \"tmpfs\""));
        assert!(!block.contains("is_bind"));
    }

    /// Security regression: if a pre-existing symlink sits at the jail_tmp
    /// path (e.g. planted by a user-controlled tarball extracted into
    /// `job_dir` before the resolver runs), the resolver must refuse the
    /// bind mount and fall back to tmpfs — never bind-mount the symlink
    /// target into the sandbox as /tmp.
    #[cfg(unix)]
    #[tokio::test]
    async fn disk_backed_refuses_preexisting_symlink_at_jail_tmp() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let job_dir = tmp.path().to_str().expect("utf8 path").to_string();

        // Plant a symlink at {job_dir}/jail_tmp pointing at an arbitrary host
        // path. Target doesn't have to exist — what matters is that the
        // resolver doesn't follow it.
        let jail_tmp_path = format!("{job_dir}/{NSJAIL_TMP_BIND_SUBDIR}");
        std::os::unix::fs::symlink("/etc", &jail_tmp_path).expect("plant symlink");
        assert!(std::path::Path::new(&jail_tmp_path).is_symlink());

        let block = with_tmp_backing(Some("disk".to_string()), || async {
            resolve_nsjail_tmp_mount_block(&job_dir).await
        })
        .await;

        // Fell back to tmpfs — no bind-mount of the attacker-controlled path.
        assert!(
            block.contains("fstype: \"tmpfs\""),
            "expected tmpfs fallback, got: {block}"
        );
        assert!(
            !block.contains("is_bind"),
            "must not emit bind block, got: {block}"
        );
        assert!(
            !block.contains("/etc"),
            "must not leak the symlink target into the proto, got: {block}"
        );
    }

    /// Sequential resolver calls in the same `job_dir` (e.g. Python uv install
    /// → Python run, Ruby install → run, Rust build → run) must keep using
    /// the bind mount instead of silently falling back to tmpfs on the
    /// second call. The first call creates `jail_tmp`; subsequent calls see
    /// it as a pre-existing real directory and must accept it.
    #[tokio::test]
    async fn disk_backed_reuses_jail_tmp_across_sequential_calls() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let job_dir = tmp.path().to_str().expect("utf8 path").to_string();
        let expected_dir = format!("{job_dir}/{NSJAIL_TMP_BIND_SUBDIR}");

        let (first, second) = with_tmp_backing(Some("disk".to_string()), || async {
            let first = resolve_nsjail_tmp_mount_block(&job_dir).await;
            // Simulate an executor that completes its first nsjail invocation
            // (e.g. uv install) leaving jail_tmp on disk, then invokes nsjail
            // again for the main run.
            assert!(std::path::Path::new(&expected_dir).is_dir());
            let second = resolve_nsjail_tmp_mount_block(&job_dir).await;
            (first, second)
        })
        .await;

        assert!(first.contains("is_bind: true"), "first call: {first}");
        assert!(
            second.contains("is_bind: true"),
            "second call regressed to tmpfs: {second}"
        );
        assert!(second.contains(&format!("src: \"{expected_dir}\"")));
    }
}

async fn hash_args(
    #[allow(unused)] db: &DB,
    #[allow(unused)] client: &AuthedClient,
    #[allow(unused)] workspace_id: &str,
    v: &Option<Json<HashMap<String, Box<RawValue>>>>,
    hasher: &mut sha2::Sha256,
    #[allow(unused)] job_id: &Uuid,
    #[allow(unused)] ignore_s3_path: bool,
) {
    if let Some(Json(hm)) = v {
        for k in hm.keys().sorted() {
            hasher.update(k.as_bytes());
            let arg_value = hm.get(k).unwrap();

            #[cfg(feature = "parquet")]
            let etag = match serde_json::from_str::<S3Object>(arg_value.get()).ok() {
                Some(s3_object) => {
                    let s3_resource = get_workspace_s3_resource_path(
                        db,
                        client,
                        workspace_id,
                        s3_object.storage.as_ref(),
                        job_id,
                    )
                    .await
                    .ok()
                    .flatten();
                    match s3_resource {
                        Some(s3_resource) => get_etag_or_empty(&s3_resource, s3_object).await,
                        None => None,
                    }
                }
                None => None,
            };

            #[cfg(feature = "parquet")]
            if let Some(etag) = etag {
                hasher.update(etag.as_bytes());
                if ignore_s3_path {
                    continue;
                }
            }
            hasher.update(arg_value.get().as_bytes());
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
    hash_args(
        db,
        client,
        &job.workspace_id,
        &job.args,
        &mut hasher,
        &job.id,
        job.cache_ignore_s3_path.unwrap_or(false),
    )
    .await;
    format!("g/results/{:064x}", hasher.finalize())
}

#[cfg(feature = "parquet")]
pub(crate) async fn get_workspace_s3_resource_path(
    db: &DB,
    client: &AuthedClient,
    workspace_id: &str,
    storage: Option<&String>,
    job_id: &Uuid,
) -> windmill_common::error::Result<Option<ObjectStoreResource>> {
    use windmill_object_store::job_s3_helpers_oss::get_s3_resource_internal;
    use windmill_types::s3::StorageResourceType;

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
        Some(LargeFileStorage::FilesystemStorage(fs)) => {
            return Ok(Some(
                windmill_object_store::ObjectStoreResource::Filesystem(
                    windmill_object_store::FilesystemSettings { root_path: fs.root_path.clone() },
                ),
            ));
        }
        None => {
            return Ok(None);
        }
    };

    let s3_resource_value_raw = client
        .get_resource_value_interpolated::<serde_json::Value>(
            path.as_str(),
            Some(job_id.to_string()),
        )
        .await?;
    let object_store_resource = get_s3_resource_internal(
        rt,
        s3_resource_value_raw,
        windmill_object_store::job_s3_helpers_oss::TokenGenerator::AsClient(client),
        db,
    )
    .await?;

    // Check bucket workspace restrictions
    use windmill_object_store::ObjectStoreResource;
    let bucket_name = match &object_store_resource {
        ObjectStoreResource::S3(s3_resource) => Some(&s3_resource.bucket),
        ObjectStoreResource::Azure(azure_resource) => Some(&azure_resource.container_name),
        ObjectStoreResource::Gcs(gcs_resource) => Some(&gcs_resource.bucket),
        ObjectStoreResource::Filesystem(_) => None,
    };

    if let Some(bucket) = bucket_name {
        windmill_object_store::check_bucket_workspace_restriction(bucket, workspace_id)?;
    }

    // Check Azure account name workspace restrictions
    if let ObjectStoreResource::Azure(azure_resource) = &object_store_resource {
        windmill_object_store::check_az_account_name_workspace_restriction(
            &azure_resource.account_name,
            workspace_id,
        )?;
    }

    Ok(Some(object_store_resource))
}

#[derive(Deserialize, Serialize)]
struct CachedResource {
    expire: i64,
    value: Arc<Box<RawValue>>,
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

    Some(resource.value.clone())
}

pub async fn save_in_cache(
    db: &Pool<Postgres>,
    _client: &AuthedClient,
    job: &MiniCompletedJob,
    cached_path: String,
    r: Arc<Box<RawValue>>,
) {
    let expire = chrono::Utc::now().timestamp() + job.cache_ttl.unwrap() as i64;

    let store_cache_resource = CachedResource { expire, value: r };
    let raw_json = Json(&store_cache_resource);

    if let Err(e) = sqlx::query!(
        "INSERT INTO resource
        (workspace_id, path, value, resource_type, created_by, edited_at)
        VALUES ($1, $2, $3, $4, $5, now()) ON CONFLICT (workspace_id, path)
        DO UPDATE SET value = EXCLUDED.value, edited_at = now()",
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
    tokio::fs::remove_dir_all(&*ROOT_CACHE_DIR).await?;
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

/// Extract the flow root path by stripping /branchone-N/, /branchall-N/, /forloop-N/, /loop-N/
/// and everything after. Returns None if no nesting segments are found.
pub fn extract_flow_root(runnable_path: &str) -> Option<&str> {
    RE_FLOW_ROOT
        .captures(runnable_path)
        .and_then(|c| c.get(1).map(|m| m.as_str()))
}

pub fn build_http_client(timeout_duration: std::time::Duration) -> error::Result<Client> {
    configure_client(
        reqwest::ClientBuilder::new()
            .user_agent("windmill/beta")
            .timeout(timeout_duration)
            .connect_timeout(std::time::Duration::from_secs(10)),
    )
    .build()
    .map_err(|e| Error::internal_err(format!("Error building http client: {e:#}")))
}

pub fn get_root_job_id(job: &MiniPulledJob) -> uuid::Uuid {
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
    flow_step_id: Option<String>,
}

// Helper struct to hold parent job information
#[derive(Debug)]
struct JobInfo {
    flow_step_id: Option<String>,
    step: Option<i32>,
    len: Option<i32>,
    is_branch_one: Option<bool>,
    next_parent: Option<Uuid>,
}

// Shared helper function to fetch parent job info with flow status
async fn get_job_info(db: &DB, job_id: Uuid) -> error::Result<JobInfo> {
    sqlx::query_as!(
        JobInfo,
        r#"SELECT
            flow_step_id,
            (flow_status->'step')::integer as step,
            jsonb_array_length(flow_status->'modules') as len,
            runnable_path ~ '/branchone-\d+$' as is_branch_one,
            parent_job as next_parent
        FROM v2_job
            LEFT JOIN v2_job_status USING (id)
        WHERE v2_job.id = $1"#,
        job_id
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::internal_err(format!("fetching parent job info: {e:#}")))
}

async fn check_if_last_step(
    db: &DB,
    mut next_parent: Option<Uuid>,
    root_job: Uuid,
) -> error::Result<bool> {
    let mut visited = HashSet::new();
    loop {
        // Get parent of current job
        let Some(parent_job) = next_parent else {
            return Ok(false);
        };

        // Check for cycles
        if !visited.insert(parent_job) {
            return Ok(false);
        }

        let parent_info = get_job_info(db, parent_job).await?;

        if let Some(step) = parent_info.step {
            let step = Step::from_i32_and_len(step, parent_info.len.unwrap_or(0) as usize);
            if step.is_last_step() {
                if parent_job == root_job {
                    return Ok(true);
                } else if parent_info.is_branch_one.unwrap_or(false) {
                    next_parent = parent_info.next_parent;
                    continue;
                }
            }
        }

        return Ok(false);
    }
}

// Iterative implementation to avoid stack overflow from async_recursion
// Checks if we're at last step in nested branches AND if any parent's flow_step_id matches early_return_id
async fn check_if_early_return_or_last_in_early_return_parent(
    db: &DB,
    mut next_parent: Option<Uuid>,
    mut step_id: Option<String>,
    early_return_id: &str,
    root_job: Uuid,
) -> error::Result<bool> {
    let mut visited = HashSet::new();
    loop {
        if step_id.is_none() {
            return Ok(false);
        }

        let Some(parent_job) = next_parent else {
            return Ok(false);
        };

        // Check for cycles
        if !visited.insert(parent_job) {
            return Ok(false);
        }

        // If the parent's flow_step_id matches early_return_id, we found it!
        if step_id.as_deref() == Some(early_return_id) && parent_job == root_job {
            return Ok(true);
        } else {
            let parent_info = get_job_info(db, parent_job).await?;
            if let Some(step) = parent_info.step {
                let step = Step::from_i32_and_len(step, parent_info.len.unwrap_or(0) as usize);
                // we only continue if we are at the last step and the parent is a branch one
                if step.is_last_step() && parent_info.is_branch_one.unwrap_or(false) {
                    next_parent = parent_info.next_parent;
                    step_id = parent_info.flow_step_id;
                    continue;
                }
            }
            return Ok(false);
        }
    }
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
                    flow_step_id: job.flow_step_id.clone(),
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
        flow_step_id: Option<String>,
    ) -> Result<(), Error> {
        // Check if early_return is set at the flow level
        let early_return_node_id = sqlx::query_scalar!(
            r#"
            SELECT fv.value->>'early_return' as "early_return"
            FROM v2_job j
            INNER JOIN flow_version fv ON fv.id = j.runnable_id
            WHERE j.id = $1
            "#,
            root_job
        )
        .fetch_optional(&db)
        .await?
        .flatten();

        let should_set_stream_job = if let Some(ref early_return_id) = early_return_node_id {
            check_if_early_return_or_last_in_early_return_parent(
                &db,
                Some(parent_job),
                flow_step_id,
                early_return_id,
                root_job,
            )
            .await?
        } else {
            check_if_last_step(&db, Some(parent_job), root_job).await?
        };

        if should_set_stream_job {
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
        let flow_step_id = self.flow_step_id.clone();
        tokio::spawn(async move {
            if let Err(err) = Self::update_flow_status_with_stream_job_inner(
                db,
                parent_job,
                job_id,
                root_job,
                flow_step_id,
            )
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

    pub fn to_return_s3_obj(&self) -> windmill_types::s3::S3Object {
        windmill_types::s3::S3Object {
            s3: self.object_key.clone(),
            storage: self.storage.clone(),
            ..Default::default()
        }
    }
}

/// Stream rows to S3 (via `convert_json_line_stream` + `s3.upload`) while appending
/// periodic progress, ingest-done, and upload-done logs to the job output.
///
/// `db_name` is a human-readable prefix for the log lines (e.g. "MSSQL", "PostgreSQL").
pub async fn s3_stream_and_upload_with_logs<S, V, E>(
    db_name: &str,
    rows_stream: S,
    s3: &S3ModeWorkerData,
    job_id: Uuid,
    workspace_id: &str,
    conn: &Connection,
) -> anyhow::Result<()>
where
    S: futures::TryStreamExt<Item = Result<V, E>> + Unpin,
    V: serde::Serialize,
    E: Into<anyhow::Error>,
{
    let s3_format = s3.format;
    let (progress_tx, mut progress_rx) =
        tokio::sync::mpsc::channel::<windmill_object_store::IngestStats>(8);
    let progress_conn = conn.clone();
    let progress_workspace = workspace_id.to_string();
    let progress_db_name = db_name.to_string();
    let progress_task = tokio::spawn(async move {
        while let Some(stats) = progress_rx.recv().await {
            let logs = format!(
                "\n{} s3 stream progress: {} rows, {:.1} MB | elapsed {:.1}s (db-fetch {:.1}s, write {:.1}s)",
                progress_db_name,
                stats.rows,
                stats.bytes as f64 / 1_048_576.0,
                stats.elapsed.as_secs_f64(),
                stats.fetch_wait.as_secs_f64(),
                stats.write_time.as_secs_f64(),
            );
            windmill_queue::append_logs(&job_id, &progress_workspace, logs, &progress_conn).await;
        }
    });

    let (stream, ingest_stats) =
        windmill_object_store::convert_json_line_stream(rows_stream, s3_format, Some(progress_tx))
            .await?;
    let _ = progress_task.await;

    let ingest_log = format!(
        "\n{} s3 stream ingest done ({:?}): {} rows, {:.1} MB in {:.2}s (db-fetch {:.2}s, write {:.2}s, first row after {:.2}s)",
        db_name,
        s3_format,
        ingest_stats.rows,
        ingest_stats.bytes as f64 / 1_048_576.0,
        ingest_stats.elapsed.as_secs_f64(),
        ingest_stats.fetch_wait.as_secs_f64(),
        ingest_stats.write_time.as_secs_f64(),
        ingest_stats.first_row_latency.unwrap_or_default().as_secs_f64(),
    );
    windmill_queue::append_logs(&job_id, workspace_id, ingest_log, conn).await;

    let upload_start = std::time::Instant::now();
    s3.upload(stream).await?;
    let upload_elapsed = upload_start.elapsed();

    let final_log = format!(
        "\n{} s3 stream upload+transcode done: {:.2}s | total {:.2}s",
        db_name,
        upload_elapsed.as_secs_f64(),
        (ingest_stats.elapsed + upload_elapsed).as_secs_f64(),
    );
    windmill_queue::append_logs(&job_id, workspace_id, final_log, conn).await;

    Ok(())
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

#[derive(Debug)]
pub enum MaybeLock {
    /// Deployed Scripts
    Resolved { lock: String },
    /// Previews
    Unresolved { workspace_dependencies: WorkspaceDependenciesPrefetched },
}

impl MaybeLock {
    pub fn map_unresolved<B, F>(&self, mut f: F) -> Option<B>
    where
        Self: Sized,
        F: FnMut(&WorkspaceDependenciesPrefetched) -> B,
    {
        self.get_workspace_dependencies().map(|wd| f(wd))
    }

    pub fn get_workspace_dependencies(&self) -> Option<&WorkspaceDependenciesPrefetched> {
        match self {
            MaybeLock::Resolved { .. } => None,
            MaybeLock::Unresolved { ref workspace_dependencies } => Some(workspace_dependencies),
        }
    }

    pub fn get_lock(&self) -> Option<&String> {
        match self {
            MaybeLock::Resolved { ref lock } => Some(lock),
            MaybeLock::Unresolved { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use windmill_common::client::AuthedClient;

    fn test_client() -> AuthedClient {
        AuthedClient::new(
            "http://localhost:0".to_string(),
            "test".to_string(),
            "test-token".to_string(),
            None,
        )
    }

    fn test_job() -> MiniPulledJob {
        MiniPulledJob {
            workspace_id: "test".to_string(),
            id: uuid::Uuid::nil(),
            args: None,
            parent_job: None,
            created_by: "test".to_string(),
            scheduled_for: chrono::Utc::now(),
            started_at: None,
            runnable_path: None,
            kind: windmill_common::jobs::JobKind::Noop,
            runnable_id: None,
            canceled_reason: None,
            canceled_by: None,
            permissioned_as: "test".to_string(),
            permissioned_as_email: "test@test.com".to_string(),
            flow_status: None,
            tag: "test".to_string(),
            script_lang: None,
            same_worker: false,
            pre_run_error: None,
            concurrent_limit: None,
            concurrency_time_window_s: None,
            flow_innermost_root_job: None,
            root_job: None,
            timeout: None,
            flow_step_id: None,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            priority: None,
            preprocessed: None,
            script_entrypoint_override: None,
            trigger: None,
            trigger_kind: None,
            visible_to_owner: false,
            permissioned_as_end_user_email: None,
            runnable_settings_handle: None,
        }
    }

    #[tokio::test]
    async fn test_transform_array_over_1000_passthrough() {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or("postgres://postgres:changeme@localhost:5432/windmill".to_string());
        let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
        let conn = Connection::Sql(pool);
        let client = test_client();
        let job = test_job();

        let arr: Vec<Value> = (0..1001).map(|i| json!(format!("$var:x/{i}"))).collect();
        let input = Value::Array(arr.clone());

        let result = transform_json_value("test", &client, "test", input, &job, &conn, 0)
            .await
            .unwrap();

        assert_eq!(result, Value::Array(arr));
    }

    #[tokio::test]
    async fn test_transform_array_non_matching_strings_passthrough() {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or("postgres://postgres:changeme@localhost:5432/windmill".to_string());
        let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
        let conn = Connection::Sql(pool);
        let client = test_client();
        let job = test_job();

        let input = json!(["hello", "world", 42, true, null, {"key": "val"}]);

        let result = transform_json_value("test", &client, "test", input.clone(), &job, &conn, 0)
            .await
            .unwrap();

        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn test_transform_array_resolved_inside_object() {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or("postgres://postgres:changeme@localhost:5432/windmill".to_string());
        let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
        let conn = Connection::Sql(pool);
        let client = test_client();
        let job = test_job();

        let input = json!({"urls": ["$var:u/test/nonexistent", "plain"]});

        let result = transform_json_value("test", &client, "test", input, &job, &conn, 0).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transform_array_attempts_matching_items() {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or("postgres://postgres:changeme@localhost:5432/windmill".to_string());
        let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
        let conn = Connection::Sql(pool);
        let client = test_client();
        let job = test_job();

        let input = json!(["$var:u/test/nonexistent", "plain"]);

        let result = transform_json_value("test", &client, "test", input, &job, &conn, 0).await;

        assert!(result.is_err());
    }
}
