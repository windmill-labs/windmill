use std::{collections::HashMap, sync::Arc};

use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{error, global_settings::CUSTOM_TAGS_SETTING, DB};

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
        "graphql".to_string(),
        "dependency".to_string(),
        "flow".to_string(),
        "hub".to_string(),
        "other".to_string()];


    pub static ref WORKER_CONFIG: Arc<RwLock<WorkerConfig>> = Arc::new(RwLock::new(WorkerConfig {
        worker_tags: Default::default(),
        dedicated_worker: Default::default(),
    }));


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

    {
        let l = CUSTOM_TAGS_PER_WORKSPACE.read().await;
        if l.clone() == custom_tags {
            tracing::info!("Custom tags setting unchanged, skipping update");
            return Ok(());
        } else {
            tracing::info!("Custom tags setting changed, updating");
        }
    }
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
    // pub static ref CUSTOM_TAGS_PER_WORKSPACE: (Vec<String>, HashMap<String, Vec<String>>) =  process_custom_tags(std::env::var("CUSTOM_TAGS")
    //     .ok());

    // pub static ref ALL_TAGS: Vec<String> = [CUSTOM_TAGS_PER_WORKSPACE.0.clone(), CUSTOM_TAGS_PER_WORKSPACE.1.keys().map(|x| x.to_string()).collect_vec()].concat();
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
    let wc = WORKER_CONFIG.read().await;
    let tags = wc.worker_tags.as_slice();
    sqlx::query!(
        "INSERT INTO worker_ping (worker_instance, worker, ip, custom_tags, worker_group) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (worker) DO UPDATE set ip = $3, custom_tags = $4, worker_group = $5",
        worker_instance,
        worker_name,
        ip,
        tags,
        *WORKER_GROUP
    )
    .execute(db)
    .await
    .expect("insert worker_ping initial value");
}

pub async fn load_worker_config(db: &DB) -> error::Result<WorkerConfig> {
    let config: WorkerConfigOpt = sqlx::query_scalar!(
        "SELECT config FROM worker_group_config WHERE name = $1",
        *WORKER_GROUP
    )
    .fetch_optional(db)
    .await?
    .flatten()
    .map(|x| serde_json::from_value(x).ok())
    .flatten()
    .unwrap_or_default();
    let dedicated_worker = std::env::var("DEDICATED_WORKER").ok().map(|x| {
        let splitted = x.split(':').to_owned().collect_vec();
        if splitted.len() != 2 {
            panic!("DEDICATED_WORKER should be in the form of <workspace>:<script_path>")
        } else {
            let workspace = splitted[0];
            let script_path = splitted[1];
            WorkspacedPath { workspace_id: workspace.to_string(), path: script_path.to_string() }
        }
    });
    Ok(WorkerConfig {
        worker_tags: config
            .worker_tags
            .or_else(|| {
                if let Some(ref dedicated_worker) = dedicated_worker.as_ref() {
                    Some(vec![format!(
                        "{}:{}",
                        dedicated_worker.workspace_id, dedicated_worker.path
                    )])
                } else {
                    std::env::var("WORKER_TAGS")
                        .ok()
                        .map(|x| x.split(',').map(|x| x.to_string()).collect())
                }
            })
            .unwrap_or_else(|| DEFAULT_TAGS.clone()),
        dedicated_worker,
    })
}

#[derive(Clone, PartialEq)]
pub struct WorkspacedPath {
    pub workspace_id: String,
    pub path: String,
}
#[derive(Serialize, Deserialize)]
pub struct WorkerConfigOpt {
    pub worker_tags: Option<Vec<String>>,
    pub dedicated_worker: Option<String>,
}

impl Default for WorkerConfigOpt {
    fn default() -> Self {
        Self { worker_tags: Default::default(), dedicated_worker: Default::default() }
    }
}

#[derive(PartialEq)]
pub struct WorkerConfig {
    pub worker_tags: Vec<String>,
    pub dedicated_worker: Option<WorkspacedPath>,
}
