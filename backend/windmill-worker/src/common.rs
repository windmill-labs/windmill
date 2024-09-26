use async_recursion::async_recursion;

use itertools::Itertools;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{json, Value};
use sqlx::types::Json;
use sqlx::{Pool, Postgres};
use tokio::process::Command;
use tokio::{fs::File, io::AsyncReadExt};
use windmill_common::jobs::ENTRYPOINT_OVERRIDE;

#[cfg(feature = "parquet")]
use windmill_common::s3_helpers::{
    get_etag_or_empty, LargeFileStorage, ObjectStoreResource, S3Object,
};
use windmill_common::variables::{build_crypt_with_key_suffix, decrypt_value_with_mc};
use windmill_common::worker::{
    to_raw_value, write_file, CLOUD_HOSTED, ROOT_CACHE_DIR, WORKER_CONFIG,
};
use windmill_common::{
    error::{self, Error},
    jobs::QueuedJob,
    variables::ContextualVariable,
};

use anyhow::{anyhow, Result};

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    time::Duration,
};

use uuid::Uuid;
use windmill_common::{variables, DB};

use tokio::{io::AsyncWriteExt, process::Child, time::Instant};

use crate::{
    AuthedClient, AuthedClientBackgroundTask, JOB_DEFAULT_TIMEOUT, MAX_RESULT_SIZE,
    MAX_TIMEOUT_DURATION,
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
    if let Some(args) = job.args.as_ref() {
        if let Some(x) = transform_json(client, &job.workspace_id, &args.0, job, db).await? {
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
            let mc =
                build_crypt_with_key_suffix(&db, &job.workspace_id, &job.id.to_string()).await?;
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
                Some(job.scheduled_for.clone()),
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

fn check_result_too_big(size: usize) -> error::Result<()> {
    if *CLOUD_HOSTED && size > MAX_RESULT_SIZE {
        return Err(error::Error::ExecutionErr("Result is too large for the cloud app (limit 2MB).
        If using this script as part of the flow, use the shared folder to pass heavy data between steps.".to_owned()));
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

pub fn get_main_override(args: Option<&Json<HashMap<String, Box<RawValue>>>>) -> Option<String> {
    return args
        .map(|x| {
            x.0.get(ENTRYPOINT_OVERRIDE)
                .map(|x| x.get().to_string().replace("\"", ""))
        })
        .flatten();
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
    db: &DB,
    worker_name: &str,
    last_job_id: Uuid,
) {
    if let Err(e) = sqlx::query!(
        "UPDATE worker_ping SET 
        ping_at = now(), 
        jobs_executed = 1, 
        current_job_id = $1, 
        current_job_workspace_id = 'admins' 
        WHERE worker = $2",
        last_job_id,
        worker_name
    )
    .execute(db)
    .await
    {
        tracing::error!("Error updating worker ping for failed init script: {e:?}");
    }
}
pub struct OccupancyMetrics {
    pub running_job_started_at: Option<Instant>,
    pub total_duration_of_running_jobs: f32,
    pub worker_occupancy_rate_history: Vec<(f32, f32)>,
    pub start_time: Instant,
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

    pub fn update_occupancy_metrics(&mut self) -> (f32, Option<f32>, Option<f32>, Option<f32>) {
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

        (
            occupancy_rate,
            occupancy_rate_15s,
            occupancy_rate_5m,
            occupancy_rate_30m,
        )
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
    let (storage, s3_etags) = (None, None);

    let store_cache_resource = CachedResource { expire, s3_etags, value: r.clone(), storage };
    let raw_json = sqlx::types::Json(store_cache_resource);

    if let Err(e) = sqlx::query!(
        "INSERT INTO resource
    (workspace_id, path, value, resource_type, created_by, edited_at)
    VALUES ($1, $2, $3, $4, $5, now()) ON CONFLICT (workspace_id, path)
    DO UPDATE SET value = $3, edited_at = now()",
        job.workspace_id,
        cached_path,
        raw_json as sqlx::types::Json<CachedResource>,
        "cache",
        job.created_by
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

lazy_static::lazy_static! {
    static ref RE_FLOW_ROOT: Regex = Regex::new(r"(?i)(.*?)(?:/branchone-\d+/|/branchall-\d+/|/loop-\d+/)").unwrap();

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
