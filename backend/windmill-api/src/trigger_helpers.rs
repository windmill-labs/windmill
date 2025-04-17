use quick_cache::sync::Cache;
use serde::Deserialize;
use serde_json::value::RawValue;
use std::collections::HashMap;
use windmill_common::{
    error::Result,
    scripts::{ScriptHash, ScriptLang},
    worker::to_raw_value,
};
use windmill_queue::{PushArgsOwned, TriggerKind};

use crate::db::DB;

lazy_static::lazy_static! {
    pub static ref RUNNABLE_FORMAT_VERSION_CACHE: Cache<(String, RunnableId, TriggerKind), ExpiringRunnableFormat> = Cache::new(1000);
}

const RUNNABLE_FORMAT_VERSION_CACHE_EXPIRATION: std::time::Duration =
    std::time::Duration::from_secs(5);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExpiringRunnableFormat {
    pub format: RunnableFormat,
    pub expires_at: std::time::Instant,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
pub struct RunnableFormat {
    pub version: RunnableFormatVersion,
    pub has_preprocessor: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
pub enum RunnableFormatVersion {
    V1,
    V2,
}

struct ScriptInfo {
    has_preprocessor: Option<bool>,
    language: ScriptLang,
    content: String,
    schema: Option<sqlx::types::Json<PartialSchema>>,
}

struct FlowInfo {
    has_preprocessor: Option<bool>,
    is_v1_preprocessor: Option<bool>,
    schema: Option<sqlx::types::Json<PartialSchema>>,
}

#[derive(Debug, Deserialize)]
struct PropertyDefinition {
    r#type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PartialSchema {
    properties: Option<HashMap<String, PropertyDefinition>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum RunnableId {
    FlowPath(String),
    ScriptId(ScriptId),
}

impl RunnableId {
    pub fn from_script_hash(hash: ScriptHash) -> Self {
        Self::ScriptId(ScriptId::ScriptHash(hash))
    }

    pub fn from_script_path(path: &str) -> Self {
        Self::ScriptId(ScriptId::ScriptPath(path.to_string()))
    }

    pub fn from_flow_path(path: &str) -> Self {
        Self::FlowPath(path.to_string())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ScriptId {
    ScriptPath(String),
    ScriptHash(ScriptHash),
}

impl ScriptId {
    async fn get_script_info(
        self,
        workspace_id: &str,
        db: &DB,
    ) -> std::result::Result<ScriptInfo, sqlx::Error> {
        match self {
            ScriptId::ScriptPath(path) => {
                sqlx::query_as!(ScriptInfo, "SELECT has_preprocessor, language as \"language: _\", content, schema as \"schema: _\" FROM script WHERE workspace_id = $1 AND path = $2 AND created_at = (SELECT max(created_at) FROM script WHERE workspace_id = $1 AND path = $2)", workspace_id, path)
                    .fetch_one(db)
                    .await
            }
            ScriptId::ScriptHash(hash) => {
                sqlx::query_as!(ScriptInfo, "SELECT has_preprocessor, language as \"language: _\", content, schema as \"schema: _\" FROM script WHERE workspace_id = $1 AND hash = $2", workspace_id, hash.0)
                    .fetch_one(db)
                    .await
            }
        }
    }
}

async fn get_runnable_format_inner(
    runnable_id: RunnableId,
    workspace_id: &str,
    db: &DB,
    trigger_kind: &TriggerKind,
) -> Result<RunnableFormat> {
    let (has_preprocessor, schema) = match runnable_id {
        RunnableId::FlowPath(path) => {
            let flow_info = sqlx::query_as!(
            FlowInfo,
            "SELECT
                value->'preprocessor_module' IS NOT NULL as has_preprocessor,
                value->'preprocessor_module'->'value'->'input_transforms'->'wm_trigger' IS NOT NULL as is_v1_preprocessor,
                schema as \"schema: _\"
            FROM flow 
            WHERE workspace_id = $1 
                AND path = $2",
            workspace_id,
            path
        )
        .fetch_one(db)
        .await?;

            let has_preprocessor = flow_info.has_preprocessor.unwrap_or(false);
            let is_v1_preprocessor = flow_info.is_v1_preprocessor.unwrap_or(false);

            if has_preprocessor && is_v1_preprocessor {
                return Ok(RunnableFormat {
                    version: RunnableFormatVersion::V1,
                    has_preprocessor: true,
                });
            }

            (has_preprocessor, flow_info.schema)
        }
        RunnableId::ScriptId(script_id) => {
            let script_info = script_id.get_script_info(workspace_id, db).await?;
            let has_preprocessor = script_info.has_preprocessor.unwrap_or(false);

            if has_preprocessor {
                let args = match script_info.language {
                    ScriptLang::Bun
                    | ScriptLang::Bunnative
                    | ScriptLang::Deno
                    | ScriptLang::Nativets => {
                        let args = windmill_parser_ts::parse_deno_signature(
                            &script_info.content,
                            true,
                            false,
                            Some("preprocessor".to_string()),
                        )?;
                        Some(args.args)
                    }
                    ScriptLang::Python3 => {
                        let args = windmill_parser_py::parse_python_signature(
                            &script_info.content,
                            Some("preprocessor".to_string()),
                            false,
                        )?;
                        Some(args.args)
                    }
                    _ => None,
                };

                if args.is_some_and(|args| args.iter().any(|arg| arg.name == "wm_trigger")) {
                    return Ok(RunnableFormat {
                        version: RunnableFormatVersion::V1,
                        has_preprocessor: true,
                    });
                }
            }

            (has_preprocessor, script_info.schema)
        }
    };

    if matches!(trigger_kind, TriggerKind::Mqtt)
        && schema.as_ref().is_some_and(|schema| {
            schema.properties.as_ref().is_some_and(|properties| {
                properties.iter().any(|(key, def)| {
                    key == "payload" && def.r#type.as_ref().is_some_and(|t| t == "array")
                })
            })
        })
    {
        return Ok(RunnableFormat { version: RunnableFormatVersion::V1, has_preprocessor });
    }

    if matches!(trigger_kind, TriggerKind::Kafka | TriggerKind::Nats)
        && schema.as_ref().is_some_and(|schema| {
            schema.properties.as_ref().is_some_and(|properties| {
                properties.iter().any(|(key, def)| {
                    key == "msg" && def.r#type.as_ref().is_some_and(|t| t == "string")
                })
            })
        })
    {
        return Ok(RunnableFormat { version: RunnableFormatVersion::V1, has_preprocessor });
    }

    Ok(RunnableFormat { version: RunnableFormatVersion::V2, has_preprocessor })
}

pub async fn get_runnable_format(
    runnable_id: RunnableId,
    workspace_id: &str,
    db: &DB,
    trigger_kind: &TriggerKind,
) -> Result<RunnableFormat> {
    let key = (
        workspace_id.to_string(),
        runnable_id.clone(),
        trigger_kind.clone(),
    );
    let cached = RUNNABLE_FORMAT_VERSION_CACHE.get(&key);
    if let Some(cached) = cached {
        if cached.expires_at > std::time::Instant::now() {
            return Ok(cached.format);
        }
    }

    let runnable_format =
        get_runnable_format_inner(runnable_id, workspace_id, db, trigger_kind).await?;
    RUNNABLE_FORMAT_VERSION_CACHE.insert(
        key,
        ExpiringRunnableFormat {
            format: runnable_format,
            expires_at: std::time::Instant::now() + RUNNABLE_FORMAT_VERSION_CACHE_EXPIRATION,
        },
    );
    Ok(runnable_format)
}

pub trait TriggerJobArgs<T: Clone> {
    fn v1_payload_fn(payload: T) -> HashMap<String, Box<RawValue>>;
    fn v2_payload_fn(payload: T) -> HashMap<String, Box<RawValue>> {
        Self::v1_payload_fn(payload)
    }
    fn trigger_kind() -> TriggerKind;

    fn build_job_args_v2(
        has_preprocessor: bool,
        payload: T,
        info: HashMap<String, Box<RawValue>>,
    ) -> PushArgsOwned {
        let trigger_kind = Self::trigger_kind();
        let mut args = Self::v2_payload_fn(payload);
        if has_preprocessor {
            args.insert("kind".to_string(), to_raw_value(&trigger_kind.to_key()));
            args.extend(info);
            let args = HashMap::from([("event".to_string(), to_raw_value(&args))]);
            PushArgsOwned { args, extra: None }
        } else {
            PushArgsOwned { args, extra: None }
        }
    }

    fn build_job_args_v1(
        has_preprocessor: bool,
        payload: T,
        info: HashMap<String, Box<RawValue>>,
    ) -> PushArgsOwned {
        let trigger_kind = Self::trigger_kind();
        let trigger_key = trigger_kind.to_key();
        let args = Self::v1_payload_fn(payload);
        let extra = if has_preprocessor {
            Some(HashMap::from([(
                "wm_trigger".to_string(),
                to_raw_value(&serde_json::json!({
                    "kind": trigger_key,
                    trigger_key: info
                })),
            )]))
        } else {
            None
        };

        PushArgsOwned { args, extra }
    }

    async fn build_job_args(
        runnable_path: &str,
        is_flow: bool,
        w_id: &str,
        db: &DB,
        payload: T,
        info: HashMap<String, Box<RawValue>>,
    ) -> Result<PushArgsOwned> {
        let runnable_id = if is_flow {
            RunnableId::from_flow_path(runnable_path)
        } else {
            RunnableId::from_script_path(runnable_path)
        };
        Self::build_job_args_from_runnable_id(runnable_id, w_id, db, payload, info).await
    }

    async fn build_job_args_from_runnable_id(
        runnable_id: RunnableId,
        w_id: &str,
        db: &DB,
        payload: T,
        info: HashMap<String, Box<RawValue>>,
    ) -> Result<PushArgsOwned> {
        let runnable_format =
            get_runnable_format(runnable_id, w_id, db, &Self::trigger_kind()).await?;

        match runnable_format {
            RunnableFormat { version: RunnableFormatVersion::V1, has_preprocessor } => {
                Ok(Self::build_job_args_v1(has_preprocessor, payload, info))
            }
            RunnableFormat { version: RunnableFormatVersion::V2, has_preprocessor } => {
                Ok(Self::build_job_args_v2(has_preprocessor, payload, info))
            }
        }
    }

    fn build_capture_payloads(
        payload: T,
        info: HashMap<String, Box<RawValue>>,
    ) -> (PushArgsOwned, PushArgsOwned) {
        let main_args = Self::build_job_args_v2(false, payload.clone(), info.clone());
        let preprocessor_args = Self::build_job_args_v2(true, payload, info);
        (main_args, preprocessor_args)
    }
}
