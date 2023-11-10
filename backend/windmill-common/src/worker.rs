use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

use crate::{error, global_settings::CUSTOM_TAGS_SETTING, server::ServerConfig, DB};

lazy_static::lazy_static! {
    pub static ref WORKER_GROUP: String = std::env::var("WORKER_GROUP").unwrap_or_else(|_| "default".to_string());

    pub static ref DEFAULT_TAGS : Vec<String> = vec![
        "deno".to_string(),
        "python3".to_string(),
        "go".to_string(),
        "bash".to_string(),
        "powershell".to_string(),
        "nativets".to_string(),
        "mysql".to_string(),
        "graphql".to_string(),
        "bun".to_string(),
        "postgresql".to_string(),
        "bigquery".to_string(),
        "snowflake".to_string(),
        "mssql".to_string(),
        "graphql".to_string(),
        "dependency".to_string(),
        "flow".to_string(),
        "hub".to_string(),
        "other".to_string()];


    pub static ref WORKER_CONFIG: Arc<RwLock<WorkerConfig>> = Arc::new(RwLock::new(WorkerConfig {
        worker_tags: Default::default(),
        priority_tags_sorted: Default::default(),
        dedicated_worker: Default::default(),
        cache_clear: Default::default(),
        init_bash: Default::default(),
        additional_python_paths: Default::default(),
        pip_local_dependencies: Default::default()
    }));

    pub static ref SERVER_CONFIG: Arc<RwLock<ServerConfig>> = Arc::new(RwLock::new(ServerConfig { smtp: Default::default(), timeout_wait_result: 20 }));



    pub static ref CLOUD_HOSTED: bool = std::env::var("CLOUD_HOSTED").is_ok();

    pub static ref CUSTOM_TAGS: Vec<String> = std::env::var("CUSTOM_TAGS")
        .ok()
        .map(|x| x.split(',').map(|x| x.to_string()).collect::<Vec<_>>()).unwrap_or_default();


    pub static ref CUSTOM_TAGS_PER_WORKSPACE: Arc<RwLock<(Vec<String>, HashMap<String, Vec<String>>)>> = Arc::new(RwLock::new((vec![], HashMap::new())));

    pub static ref ALL_TAGS: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(vec![]));

    static ref CUSTOM_TAG_REGEX: Regex =  Regex::new(r"^(\w+)\(((?:\w+)\+?)+\)$").unwrap();

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
        "Loaded setting custom tags, common: {:?}, per-workspace: {:?}",
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
    sqlx::query!(
        "INSERT INTO worker_ping (worker_instance, worker, ip, custom_tags, worker_group, dedicated_worker, wm_version) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (worker) DO UPDATE set ip = $3, custom_tags = $4, worker_group = $5",
        worker_instance,
        worker_name,
        ip,
        tags.as_slice(),
        *WORKER_GROUP,
        dw,
        crate::utils::GIT_VERSION
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
