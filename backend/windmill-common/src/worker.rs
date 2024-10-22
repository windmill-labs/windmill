use const_format::concatcp;
use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    fs::File,
    io::Write,
    path::{Component, Path, PathBuf},
    str::FromStr,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::RwLock;
use windmill_macros::annotations;

use crate::{error, global_settings::CUSTOM_TAGS_SETTING, server::Smtp, DB};

lazy_static::lazy_static! {
    pub static ref WORKER_GROUP: String = std::env::var("WORKER_GROUP").unwrap_or_else(|_| "default".to_string());
    pub static ref NO_LOGS: bool = std::env::var("NO_LOGS").ok().is_some_and(|x| x == "1" || x == "true");

    pub static ref CGROUP_V2_PATH_RE: Regex = Regex::new(r#"(?m)^0::(/.*)$"#).unwrap();
    pub static ref CGROUP_V2_CPU_RE: Regex = Regex::new(r#"(?m)^(\d+) \S+$"#).unwrap();
    pub static ref CGROUP_V1_INACTIVE_FILE_RE: Regex = Regex::new(r#"(?m)^total_inactive_file (\d+)$"#).unwrap();
    pub static ref CGROUP_V2_INACTIVE_FILE_RE: Regex = Regex::new(r#"(?m)^inactive_file (\d+)$"#).unwrap();

    pub static ref DEFAULT_TAGS: Vec<String> = vec![
        "deno".to_string(),
        "python3".to_string(),
        "go".to_string(),
        "bash".to_string(),
        "powershell".to_string(),
        "nativets".to_string(),
        "mysql".to_string(),
        "bun".to_string(),
        "postgresql".to_string(),
        "bigquery".to_string(),
        "snowflake".to_string(),
        "mssql".to_string(),
        "graphql".to_string(),
        "php".to_string(),
        "rust".to_string(),
        "ansible".to_string(),
        "dependency".to_string(),
        "flow".to_string(),
        "other".to_string()
    ];

    pub static ref DEFAULT_TAGS_PER_WORKSPACE: AtomicBool = AtomicBool::new(false);
    pub static ref DEFAULT_TAGS_WORKSPACES: Arc<RwLock<Option<Vec<String>>>> = Arc::new(RwLock::new(None));


    pub static ref WORKER_CONFIG: Arc<RwLock<WorkerConfig>> = Arc::new(RwLock::new(WorkerConfig {
        worker_tags: Default::default(),
        priority_tags_sorted: Default::default(),
        dedicated_worker: Default::default(),
        cache_clear: Default::default(),
        init_bash: Default::default(),
        additional_python_paths: Default::default(),
        pip_local_dependencies: Default::default(),
        env_vars: Default::default(),
    }));

    pub static ref WORKER_PULL_QUERIES: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(vec![]));
    pub static ref WORKER_SUSPENDED_PULL_QUERY: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));


    pub static ref SMTP_CONFIG: Arc<RwLock<Option<Smtp>>> = Arc::new(RwLock::new(None));


    pub static ref CLOUD_HOSTED: bool = std::env::var("CLOUD_HOSTED").is_ok();

    pub static ref CUSTOM_TAGS: Vec<String> = std::env::var("CUSTOM_TAGS")
        .ok()
        .map(|x| x.split(',').map(|x| x.to_string()).collect::<Vec<_>>()).unwrap_or_default();


    pub static ref CUSTOM_TAGS_PER_WORKSPACE: Arc<RwLock<(Vec<String>, HashMap<String, Vec<String>>)>> = Arc::new(RwLock::new((vec![], HashMap::new())));

    pub static ref ALL_TAGS: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(vec![]));

    static ref CUSTOM_TAG_REGEX: Regex =  Regex::new(r"^(\w+)\(((?:\w+)\+?)+\)$").unwrap();

    pub static ref DISABLE_BUNDLING: bool = std::env::var("DISABLE_BUNDLING")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(false);

}

pub async fn make_suspended_pull_query(wc: &WorkerConfig) {
    if wc.worker_tags.len() == 0 {
        tracing::error!("Empty tags in worker tags, skipping");
        return;
    }
    let query = format!(
        "UPDATE queue
            SET running = true
              , started_at = coalesce(started_at, now())
              , last_ping = now()
              , suspend_until = null
            WHERE id = (
                SELECT id
                FROM queue
                WHERE suspend_until IS NOT NULL AND (suspend <= 0 OR suspend_until <= now()) AND tag IN ({})
                ORDER BY priority DESC NULLS LAST, created_at
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING  id,  workspace_id,  parent_job,  created_by,  created_at,  started_at,  scheduled_for,
            running,  script_hash,  script_path,  args,   null as logs,  raw_code,  canceled,  canceled_by,
            canceled_reason,  last_ping,  job_kind, schedule_path,  permissioned_as,
            flow_status,  raw_flow,  is_flow_step,  language,  suspend,  suspend_until,
            same_worker,  raw_lock,  pre_run_error,  email,  visible_to_owner,  mem_peak,
             root_job,  leaf_jobs,  tag,  concurrent_limit,  concurrency_time_window_s,
             timeout,  flow_step_id,  cache_ttl, priority", wc.worker_tags.iter().map(|x| format!("'{x}'")).join(", "));
    let mut l = WORKER_SUSPENDED_PULL_QUERY.write().await;
    *l = query;
}

pub async fn make_pull_query(wc: &WorkerConfig) {
    let mut queries = vec![];
    for tags in wc.priority_tags_sorted.iter() {
        if tags.tags.len() == 0 {
            tracing::error!("Empty tags in priority tags, skipping");
            continue;
        }
        let query = format!("UPDATE queue
        SET running = true
        , started_at = coalesce(started_at, now())
        , last_ping = now()
        , suspend_until = null
        WHERE id = (
            SELECT id
            FROM queue
            WHERE running = false AND tag IN ({}) AND scheduled_for <= now()
            ORDER BY priority DESC NULLS LAST, scheduled_for
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        )
        RETURNING  id,  workspace_id,  parent_job,  created_by,  created_at,  started_at,  scheduled_for,
        running,  script_hash,  script_path,  args,  null as logs,  raw_code,  canceled,  canceled_by,
        canceled_reason,  last_ping,  job_kind,  schedule_path,  permissioned_as,
        flow_status,  raw_flow,  is_flow_step,  language,  suspend,  suspend_until,
        same_worker,  raw_lock,  pre_run_error,  email,  visible_to_owner,  mem_peak,
         root_job,  leaf_jobs,  tag,  concurrent_limit,  concurrency_time_window_s,
         timeout,  flow_step_id,  cache_ttl, priority", tags.tags.iter().map(|x| format!("'{x}'")).join(", "));

        queries.push(query);
    }

    let mut l = WORKER_PULL_QUERIES.write().await;
    *l = queries;
}

pub const TMP_DIR: &str = "/tmp/windmill";
pub const HUB_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "hub");

pub const ROOT_CACHE_DIR: &str = concatcp!(TMP_DIR, "/cache/");

pub fn write_file(dir: &str, path: &str, content: &str) -> error::Result<File> {
    let path = format!("{}/{}", dir, path);
    let mut file = File::create(&path)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(file)
}

/// from : https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}
pub fn write_file_at_user_defined_location(
    job_dir: &str,
    user_defined_path: &str,
    content: &str,
) -> error::Result<PathBuf> {
    let job_dir = Path::new(job_dir);
    let user_path = PathBuf::from(user_defined_path);

    let full_path = job_dir.join(&user_path);

    // let normalized_job_dir = std::fs::canonicalize(job_dir)?;
    // let normalized_full_path = std::fs::canonicalize(&full_path)?;
    let normalized_job_dir = normalize_path(job_dir);
    let normalized_full_path = normalize_path(&full_path);

    if !normalized_full_path.starts_with(&normalized_job_dir) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Path is outside the allowed job directory.",
        )
        .into());
    }

    let full_path = normalized_full_path.as_path();
    if let Some(parent_dir) = full_path.parent() {
        std::fs::create_dir_all(parent_dir)?;
    }

    let mut file = File::create(full_path)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(normalized_full_path)
}

pub async fn reload_custom_tags_setting(db: &DB) -> error::Result<()> {
    let q = sqlx::query!(
        "SELECT value FROM global_settings WHERE name = $1",
        CUSTOM_TAGS_SETTING
    )
    .fetch_optional(db)
    .await?;

    let tags = if let Some(q) = q {
        if let Ok(v) = serde_json::from_value::<Vec<String>>(q.value.clone()) {
            v
        } else {
            tracing::error!(
                "Could not parse custom tags setting as vec of strings, found: {:#?}",
                &q.value
            );
            vec![]
        }
    } else {
        CUSTOM_TAGS.clone()
    };

    let custom_tags = process_custom_tags(tags);

    tracing::info!(
        "Loaded setting custom_tags, common: {:?}, per-workspace: {:?}",
        custom_tags.0,
        custom_tags.1,
    );

    {
        let mut l = CUSTOM_TAGS_PER_WORKSPACE.write().await;
        *l = custom_tags.clone()
    }
    {
        let mut l = ALL_TAGS.write().await;
        *l = [
            custom_tags.0.clone(),
            custom_tags.1.keys().map(|x| x.to_string()).collect_vec(),
        ]
        .concat();
    }
    Ok(())
}

fn process_custom_tags(tags: Vec<String>) -> (Vec<String>, HashMap<String, Vec<String>>) {
    let mut global = vec![];
    let mut specific: HashMap<String, Vec<String>> = HashMap::new();
    for e in tags {
        if let Some(cap) = CUSTOM_TAG_REGEX.captures(&e) {
            let tag = cap.get(1).unwrap().as_str().to_string();
            let workspaces = cap.get(2).unwrap().as_str().split("+");
            specific.insert(tag, workspaces.map(|x| x.to_string()).collect_vec());
        } else {
            global.push(e.to_string());
        }
    }
    (global, specific)
}

fn parse_file<T: FromStr>(path: &str) -> Option<T> {
    std::process::Command::new("cat")
        .args([path])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .to_string()
                .trim()
                .parse::<T>()
                .ok()
        })
        .flatten()
}

#[annotations("#")]
pub struct PythonAnnotations {
    pub no_cache: bool,
    pub no_uv: bool,
    pub no_uv_install: bool,
    pub no_uv_compile: bool,
}

#[annotations("//")]
pub struct TypeScriptAnnotations {
    pub npm: bool,
    pub nodejs: bool,
    pub native: bool,
    pub nobundling: bool,
}

#[annotations("--")]
pub struct SqlAnnotations {
    pub return_last_result: bool,
}

pub async fn load_cache(bin_path: &str, _remote_path: &str) -> (bool, String) {
    if tokio::fs::metadata(&bin_path).await.is_ok() {
        (true, format!("loaded from local cache: {}\n", bin_path))
    } else {
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(os) = crate::s3_helpers::OBJECT_STORE_CACHE_SETTINGS
            .read()
            .await
            .clone()
        {
            let started = std::time::Instant::now();
            use crate::s3_helpers::attempt_fetch_bytes;

            if let Ok(mut x) = attempt_fetch_bytes(os, _remote_path).await {
                if let Err(e) = write_binary_file(bin_path, &mut x) {
                    tracing::error!("could not write bundle/bin file locally: {e:?}");
                    return (
                        false,
                        "error writing bundle/bin file from object store".to_string(),
                    );
                }
                tracing::info!("loaded from object store {}", bin_path);
                return (
                    true,
                    format!(
                        "loaded bin/bundle from object store {} in {}ms",
                        bin_path,
                        started.elapsed().as_millis()
                    ),
                );
            }
        }
        (false, "".to_string())
    }
}

pub async fn exists_in_cache(bin_path: &str, _remote_path: &str) -> bool {
    if tokio::fs::metadata(&bin_path).await.is_ok() {
        return true;
    } else {
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(os) = crate::s3_helpers::OBJECT_STORE_CACHE_SETTINGS
            .read()
            .await
            .clone()
        {
            return os
                .get(&object_store::path::Path::from(_remote_path))
                .await
                .is_ok();
        }
        return false;
    }
}

pub async fn save_cache(
    local_cache_path: &str,
    _remote_cache_path: &str,
    origin: &str,
) -> crate::error::Result<String> {
    let mut _cached_to_s3 = false;
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if let Some(os) = crate::s3_helpers::OBJECT_STORE_CACHE_SETTINGS
        .read()
        .await
        .clone()
    {
        use object_store::path::Path;

        if let Err(e) = os
            .put(
                &Path::from(_remote_cache_path),
                std::fs::read(origin)?.into(),
            )
            .await
        {
            tracing::error!(
                "Failed to put go bin to object store: {_remote_cache_path}. Error: {:?}",
                e
            );
        } else {
            _cached_to_s3 = true;
        }
    }

    // if !*CLOUD_HOSTED {
    if true {
        std::fs::copy(origin, local_cache_path)?;
        Ok(format!(
            "\nwrote cached binary: {} (backed by EE distributed object store: {_cached_to_s3})\n",
            local_cache_path
        ))
    } else if _cached_to_s3 {
        Ok(format!(
            "wrote cached binary to object store {}\n",
            local_cache_path
        ))
    } else {
        Ok("".to_string())
    }
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
fn write_binary_file(main_path: &str, byts: &mut bytes::Bytes) -> error::Result<()> {
    use std::fs::{File, Permissions};
    use std::io::Write;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let mut file = File::create(main_path)?;
    file.write_all(byts)?;
    #[cfg(unix)]
    file.set_permissions(Permissions::from_mode(0o755))?;
    file.flush()?;
    Ok(())
}

fn get_cgroupv2_path() -> Option<String> {
    let cgroup_path: String = parse_file("/proc/self/cgroup")?;

    CGROUP_V2_PATH_RE
        .captures(&cgroup_path)
        .map(|x| format!("/sys/fs/cgroup{}", x.get(1).unwrap().as_str()))
}

pub fn get_vcpus() -> Option<i64> {
    if Path::new("/sys/fs/cgroup/cpu/cpu.cfs_quota_us").exists() {
        // cgroup v1
        parse_file("/sys/fs/cgroup/cpu/cpu.cfs_quota_us")
            .map(|x: i64| if x > 0 { Some(x) } else { None })
            .flatten()
    } else {
        // cgroup v2
        let cgroup_path = get_cgroupv2_path()?;

        let cpu_max_path = format!("{cgroup_path}/cpu.max");

        parse_file(&cpu_max_path)
            .map(|x: String| {
                CGROUP_V2_CPU_RE
                    .captures(&x)
                    .map(|x| x.get(1).unwrap().as_str().parse::<i64>().ok())
                    .flatten()
            })
            .flatten()
            .map(|x| if x > 0 { Some(x) } else { None })
            .flatten()
    }
}

pub fn get_memory() -> Option<i64> {
    if Path::new("/sys/fs/cgroup/memory/memory.limit_in_bytes").exists() {
        // cgroup v1
        parse_file("/sys/fs/cgroup/memory/memory.limit_in_bytes")
    } else {
        // cgroup v2
        let cgroup_path = get_cgroupv2_path()?;
        let memory_max_path = format!("{cgroup_path}/memory.max");

        parse_file(&memory_max_path)
    }
}

pub fn get_worker_memory_usage() -> Option<i64> {
    if Path::new("/sys/fs/cgroup/memory/memory.usage_in_bytes").exists() {
        // cgroup v1
        let total_memory_usage: i64 = parse_file("/sys/fs/cgroup/memory/memory.usage_in_bytes")?;

        let inactive_file = parse_file("/sys/fs/cgroup/memory/memory.stat")
            .map(|x: String| {
                CGROUP_V1_INACTIVE_FILE_RE
                    .captures(&x)
                    .map(|x| x.get(1).unwrap().as_str().parse::<i64>().ok())
                    .flatten()
            })
            .flatten()?;

        Some(total_memory_usage - inactive_file)
    } else {
        // cgroup v2
        let cgroup_path = get_cgroupv2_path()?;
        let memory_current_path = format!("{cgroup_path}/memory.current");

        let total_memory_usage: i64 = parse_file(&memory_current_path)?;

        let memory_stat_path = format!("{cgroup_path}/memory.stat");

        let inactive_file = parse_file(&memory_stat_path)
            .map(|x: String| {
                CGROUP_V2_INACTIVE_FILE_RE
                    .captures(&x)
                    .map(|x| x.get(1).unwrap().as_str().parse::<i64>().ok())
                    .flatten()
            })
            .flatten()?;

        Some(total_memory_usage - inactive_file)
    }
}

pub fn get_windmill_memory_usage() -> Option<i64> {
    #[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
    {
        match tikv_jemalloc_ctl::epoch::advance() {
            Ok(_) => match tikv_jemalloc_ctl::stats::resident::read() {
                Ok(resident) => i64::try_from(resident).ok(),
                Err(e) => {
                    tracing::error!("jemalloc resident memory read failed: {:?}", e);
                    None
                }
            },
            Err(e) => {
                tracing::error!("jemalloc epoch advance failed: {:?}", e);
                None
            }
        }
    }

    #[cfg(any(target_env = "msvc", not(feature = "jemalloc")))]
    {
        None
    }
}

pub async fn update_ping(worker_instance: &str, worker_name: &str, ip: &str, db: &DB) {
    let (tags, dw) = {
        let wc = WORKER_CONFIG.read().await.clone();
        (
            wc.worker_tags,
            wc.dedicated_worker
                .as_ref()
                .map(|x| format!("{}:{}", x.workspace_id, x.path)),
        )
    };

    let vcpus = get_vcpus();
    let memory = get_memory();

    sqlx::query!(
        "INSERT INTO worker_ping (worker_instance, worker, ip, custom_tags, worker_group, dedicated_worker, wm_version, vcpus, memory) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT (worker) DO UPDATE set ip = $3, custom_tags = $4, worker_group = $5",
        worker_instance,
        worker_name,
        ip,
        tags.as_slice(),
        *WORKER_GROUP,
        dw,
        crate::utils::GIT_VERSION,
        vcpus,
        memory
    )
    .execute(db)
    .await
    .expect("insert worker_ping initial value");
}

pub async fn load_worker_config(
    db: &DB,
    killpill_tx: tokio::sync::broadcast::Sender<()>,
) -> error::Result<WorkerConfig> {
    tracing::info!("Loading config from WORKER_GROUP: {}", *WORKER_GROUP);
    let mut config: WorkerConfigOpt = sqlx::query_scalar!(
        "SELECT config FROM config WHERE name = $1",
        format!("worker__{}", *WORKER_GROUP)
    )
    .fetch_optional(db)
    .await?
    .flatten()
    .map(|x| serde_json::from_value(x).ok())
    .flatten()
    .unwrap_or_default();

    if config.dedicated_worker.is_none() {
        let dw = std::env::var("DEDICATED_WORKER").ok();
        if dw.is_some() {
            tracing::info!(
                "DEDICATED_WORKER set from env variable: {}",
                dw.as_ref().unwrap()
            );
            config.dedicated_worker = dw;
        }
    } else {
        tracing::info!(
            "DEDICATED_WORKER set from config: {}",
            config.dedicated_worker.as_ref().unwrap()
        );
    }
    let dedicated_worker = config
        .dedicated_worker
        .map(|x| {
            let splitted = x.split(':').to_owned().collect_vec();
            if splitted.len() != 2 {
                killpill_tx.send(()).expect("send");
                return Err(anyhow::anyhow!(
                    "Invalid dedicated_worker format. Got {x}, expects <workspace_id>:<path>"
                ));
            } else {
                let workspace = splitted[0];
                let script_path = splitted[1];
                Ok(WorkspacedPath {
                    workspace_id: workspace.to_string(),
                    path: script_path.to_string(),
                })
            }
        })
        .transpose()?;
    if *WORKER_GROUP == "default" && dedicated_worker.is_none() {
        let mut all_tags = config
            .worker_tags
            .unwrap_or_default()
            .into_iter()
            .chain(
                std::env::var("WORKER_TAGS")
                    .ok()
                    .map(|x| x.split(',').map(|x| x.to_string()).collect_vec())
                    .unwrap_or_default(),
            )
            .sorted()
            .collect_vec();
        all_tags.dedup();
        config.worker_tags = Some(all_tags);
    }

    if let Some(force_worker_tags) = std::env::var("FORCE_WORKER_TAGS")
        .ok()
        .filter(|x| !x.is_empty())
        .map(|x| x.split(',').map(|x| x.to_string()).collect::<Vec<String>>())
    {
        tracing::info!(
            "Detected FORCE_WORKER_TAGS, forcing worker tags to: {:#?}",
            force_worker_tags
        );
        config.worker_tags = Some(force_worker_tags);
    }

    // set worker_tags using default if none. If priority tags is set, compute the sorted priority tags as well
    let worker_tags = config
        .worker_tags
        .or_else(|| {
            if let Some(ref dedicated_worker) = dedicated_worker.as_ref() {
                let mut dedi_tags = vec![format!(
                    "{}:{}",
                    dedicated_worker.workspace_id, dedicated_worker.path
                )];
                if std::env::var("ADD_FLOW_TAG").is_ok() {
                    dedi_tags.push("flow".to_string());
                }
                Some(dedi_tags)
            } else {
                std::env::var("WORKER_TAGS")
                    .ok()
                    .map(|x| x.split(',').map(|x| x.to_string()).collect())
            }
        })
        .unwrap_or_else(|| DEFAULT_TAGS.clone());

    let mut priority_tags_sorted: Vec<PriorityTags> = Vec::new();
    let priority_tags_map = config.priority_tags.unwrap_or_else(HashMap::new);
    if priority_tags_map.len() > 0 {
        let mut all_tags_set: HashSet<String> = HashSet::from_iter(worker_tags.clone());

        let mut tags_by_priority: HashMap<u8, Vec<String>> = HashMap::new();

        for (tag, priority) in priority_tags_map.iter() {
            if *priority == 0 {
                // ignore tags with no priority as they will be added at the end from the `all_tags` set
                continue;
            }
            match tags_by_priority.get_mut(priority) {
                Some(tags) => {
                    tags.push(tag.clone());
                }
                None => {
                    let mut t: Vec<String> = Vec::new();
                    t.push(tag.clone());
                    tags_by_priority.insert(*priority, t);
                }
            };
            all_tags_set.remove(tag);
        }
        priority_tags_sorted = tags_by_priority
            .iter()
            .map(|(priority, tags)| PriorityTags { priority: priority.clone(), tags: tags.clone() })
            .collect();
        priority_tags_sorted.push(PriorityTags { priority: 0, tags: Vec::from_iter(all_tags_set) }); // push the tags that were not listed as high priority with a priority = 0
        priority_tags_sorted.sort_by_key(|elt| Reverse(elt.priority)); // sort by priority DESC
    } else {
        // if no priority is used, push all tags with a priority to 0
        priority_tags_sorted.push(PriorityTags { priority: 0, tags: worker_tags.clone() });
    }
    tracing::debug!("Custom tags priority set: {:?}", priority_tags_sorted);

    let env_vars_static = config.env_vars_static.unwrap_or_default().clone();
    let resolved_env_vars: HashMap<String, String> = env_vars_static
        .keys()
        .map(|x| x.to_string())
        .chain(config.env_vars_allowlist.unwrap_or_default())
        .chain(
            std::env::var("WHITELIST_ENVS")
                .ok()
                .map(|x| x.split(',').map(|x| x.to_string()).collect_vec())
                .unwrap_or_default()
                .into_iter(),
        )
        .sorted()
        .unique()
        .map(|envvar_name| {
            (
                envvar_name.clone(),
                env_vars_static
                    .get::<String>(&envvar_name)
                    .map(|v| v.to_owned())
                    .unwrap_or_else(|| {
                        std::env::var(envvar_name.clone()).unwrap_or("".to_string())
                    }),
            )
        })
        .collect();

    Ok(WorkerConfig {
        worker_tags,
        priority_tags_sorted,
        dedicated_worker,
        init_bash: config
            .init_bash
            .or_else(|| std::env::var("INIT_SCRIPT").ok())
            .and_then(|x| if x.is_empty() { None } else { Some(x) }),
        cache_clear: config.cache_clear,
        pip_local_dependencies: config.pip_local_dependencies.or_else(|| {
            let pip_local_dependencies = std::env::var("PIP_LOCAL_DEPENDENCIES")
                .ok()
                .map(|x| x.split(',').map(|x| x.to_string()).collect());
            if pip_local_dependencies == Some(vec!["".to_string()]) {
                None
            } else {
                pip_local_dependencies
            }
        }),
        additional_python_paths: config.additional_python_paths.or_else(|| {
            std::env::var("ADDITIONAL_PYTHON_PATHS")
                .ok()
                .map(|x| x.split(':').map(|x| x.to_string()).collect())
        }),
        env_vars: resolved_env_vars,
    })
}

#[derive(Clone, PartialEq, Debug)]
pub struct WorkspacedPath {
    pub workspace_id: String,
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct WorkerConfigOpt {
    pub worker_tags: Option<Vec<String>>,
    pub priority_tags: Option<HashMap<String, u8>>,
    pub dedicated_worker: Option<String>,
    pub init_bash: Option<String>,
    pub cache_clear: Option<u32>,
    pub additional_python_paths: Option<Vec<String>>,
    pub pip_local_dependencies: Option<Vec<String>>,
    pub env_vars_static: Option<HashMap<String, String>>,
    pub env_vars_allowlist: Option<Vec<String>>,
}

impl Default for WorkerConfigOpt {
    fn default() -> Self {
        Self {
            worker_tags: Default::default(),
            priority_tags: Default::default(),
            dedicated_worker: Default::default(),
            init_bash: Default::default(),
            cache_clear: Default::default(),
            additional_python_paths: Default::default(),
            pip_local_dependencies: Default::default(),
            env_vars_static: Default::default(),
            env_vars_allowlist: Default::default(),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct WorkerConfig {
    pub worker_tags: Vec<String>,
    pub priority_tags_sorted: Vec<PriorityTags>,
    pub dedicated_worker: Option<WorkspacedPath>,
    pub init_bash: Option<String>,
    pub cache_clear: Option<u32>,
    pub additional_python_paths: Option<Vec<String>>,
    pub pip_local_dependencies: Option<Vec<String>>,
    pub env_vars: HashMap<String, String>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct PriorityTags {
    pub priority: u8,
    pub tags: Vec<String>,
}

pub fn to_raw_value<T: Serialize>(result: &T) -> Box<RawValue> {
    serde_json::value::to_raw_value(result)
        .unwrap_or_else(|_| RawValue::from_string("{}".to_string()).unwrap())
}

pub fn to_raw_value_owned(result: serde_json::Value) -> Box<RawValue> {
    serde_json::value::to_raw_value(&result)
        .unwrap_or_else(|_| RawValue::from_string("{}".to_string()).unwrap())
}
