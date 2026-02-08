use anyhow::Context;
use axum::response::IntoResponse;
use http::StatusCode;
use serde::Deserialize;
use serde_json::value::RawValue;
use sqlx::types::Json;
use std::collections::HashMap;
use std::future::Future;
use uuid::Uuid;
use windmill_common::{
    db::{UserDB, UserDbWithAuthed, DB},
    error::Result,
    flows::{FlowModuleValue, Retry},
    get_latest_deployed_hash_for_path, get_latest_flow_version_info_for_path,
    jobs::{get_has_preprocessor_from_content_and_lang, script_path_to_payload, JobPayload},
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    triggers::{
        HubOrWorkspaceId, RunnableFormat, RunnableFormatVersion, TriggerKind, TriggerMetadata,
        RUNNABLE_FORMAT_VERSION_CACHE,
    },
    users::username_to_permissioned_as,
    utils::{StripPath, HTTP_CLIENT},
    worker::to_raw_value,
};
use windmill_queue::{push, PushArgs, PushArgsOwned, PushIsolationLevel};

use windmill_api_auth::{check_scopes, ApiAuthed};
#[cfg(feature = "enterprise")]
use windmill_api_jobs::execution::check_license_key_valid;
use windmill_api_jobs::{
    execution::{
        check_tag_available_for_workspace, delete_job_metadata_after_use,
        push_flow_job_by_path_into_queue, push_script_job_by_path_into_queue, result_to_response,
        run_wait_result_internal,
    },
    types::RunJobQuery,
};

struct ScriptInfo {
    has_preprocessor: Option<bool>,
    language: ScriptLang,
    content: String,
    schema: Option<sqlx::types::Json<PartialSchema>>,
}

#[derive(Debug, Deserialize)]
struct PropertyDefinition {
    r#type: Option<Box<RawValue>>,
}

#[derive(Debug, Deserialize)]
struct PartialSchema {
    properties: Option<HashMap<String, PropertyDefinition>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum RunnableId {
    FlowId(FlowId),
    ScriptId(ScriptId),
    HubScript(String),
}

impl RunnableId {
    pub fn from_script_hash(hash: ScriptHash) -> Self {
        Self::ScriptId(ScriptId::ScriptHash(hash))
    }

    pub fn from_script_path(path: &str) -> Self {
        if path.starts_with("hub/") {
            Self::HubScript(path.to_string())
        } else {
            Self::ScriptId(ScriptId::ScriptPath(path.to_string()))
        }
    }

    pub fn from_flow_path(path: &str) -> Self {
        Self::FlowId(FlowId::FlowPath(path.to_string()))
    }

    pub fn from_flow_version(version: i64) -> Self {
        Self::FlowId(FlowId::FlowVersion(version))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum FlowId {
    FlowPath(String),
    FlowVersion(i64),
}

impl FlowId {
    async fn get_flow_version_id(self, workspace_id: &str, db: &DB) -> Result<i64> {
        let version_id = match self {
            FlowId::FlowPath(path) => {
                let info =
                    get_latest_flow_version_info_for_path(None, db, workspace_id, &path, true)
                        .await?;
                info.version
            }
            FlowId::FlowVersion(version) => version,
        };

        Ok(version_id)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ScriptId {
    ScriptPath(String),
    ScriptHash(ScriptHash),
}

impl ScriptId {
    async fn get_script_hash(self, workspace_id: &str, db: &DB) -> Result<i64> {
        let hash = match self {
            ScriptId::ScriptPath(path) => {
                let info = get_latest_deployed_hash_for_path(None, db.clone(), workspace_id, &path)
                    .await?;
                info.hash
            }
            ScriptId::ScriptHash(hash) => hash.0,
        };

        Ok(hash)
    }
}

async fn get_script_info(
    db: &DB,
    workspace_id: &str,
    hash: i64,
) -> std::result::Result<ScriptInfo, sqlx::Error> {
    sqlx::query_as!(ScriptInfo, "SELECT has_preprocessor, language as \"language: _\", content, schema as \"schema: _\" FROM script WHERE workspace_id = $1 AND hash = $2", workspace_id, hash)
        .fetch_one(db)
        .await
}

fn runnable_format_from_schema_without_preprocessor(
    trigger_kind: &TriggerKind,
    schema: Option<sqlx::types::Json<PartialSchema>>,
) -> RunnableFormat {
    match trigger_kind {
        TriggerKind::Mqtt
            if schema.as_ref().is_some_and(|schema| {
                schema.properties.as_ref().is_some_and(|properties| {
                    properties.iter().any(|(key, def)| {
                        key == "payload"
                            && def.r#type.as_ref().is_some_and(|t| {
                                let typ = t.get().trim();
                                typ == "array" || (typ.starts_with('[') && typ.ends_with(']'))
                            })
                    })
                })
            }) =>
        {
            RunnableFormat { version: RunnableFormatVersion::V1, has_preprocessor: false }
        }
        TriggerKind::Kafka | TriggerKind::Nats
            if schema.as_ref().is_some_and(|schema| {
                schema
                    .properties
                    .as_ref()
                    .is_some_and(|properties| properties.keys().any(|key| key == "msg"))
            }) =>
        {
            RunnableFormat { version: RunnableFormatVersion::V1, has_preprocessor: false }
        }
        _ => RunnableFormat { version: RunnableFormatVersion::V2, has_preprocessor: false },
    }
}

fn runnable_format_from_preprocessor_args(
    args: Option<Vec<windmill_parser::Arg>>,
) -> RunnableFormat {
    if let Some(args) = args {
        if args.iter().any(|arg| arg.name == "wm_trigger")
            || (args.len() > 0 && args.iter().all(|arg| arg.name != "event"))
        {
            RunnableFormat { version: RunnableFormatVersion::V1, has_preprocessor: true }
        } else {
            RunnableFormat { version: RunnableFormatVersion::V2, has_preprocessor: true }
        }
    } else {
        RunnableFormat { version: RunnableFormatVersion::V2, has_preprocessor: true }
    }
}

enum PreprocessorInfo {
    Preprocessor { content: String, language: ScriptLang },
    NoPreprocessor { schema: Option<sqlx::types::Json<PartialSchema>> },
}

#[derive(Debug, Deserialize)]
struct FlowInfo {
    preprocessor_module: Option<sqlx::types::Json<FlowModuleValue>>,
    schema: Option<sqlx::types::Json<PartialSchema>>,
}

fn get_preprocessor_args_from_content_and_language(
    content: &str,
    language: &ScriptLang,
) -> Result<Option<Vec<windmill_parser::Arg>>> {
    let args = match language {
        ScriptLang::Bun | ScriptLang::Bunnative | ScriptLang::Deno | ScriptLang::Nativets => {
            let args = windmill_parser_ts::parse_deno_signature(
                &content,
                true,
                false,
                Some("preprocessor".to_string()),
            )?;
            Some(args.args)
        }
        #[cfg(feature = "python")]
        ScriptLang::Python3 => {
            let args = windmill_parser_py::parse_python_signature(
                &content,
                Some("preprocessor".to_string()),
                false,
            )?;
            Some(args.args)
        }
        _ => None,
    };
    Ok(args)
}

pub async fn get_runnable_format(
    runnable_id: RunnableId,
    workspace_id: &str,
    db: &DB,
    trigger_kind: &TriggerKind,
) -> Result<RunnableFormat> {
    let (key, preprocessor_info) = match runnable_id {
        RunnableId::HubScript(path) => {
            let Some(version) = path.split("/").nth(1) else {
                return Err(windmill_common::error::Error::internal_err(
                    "Invalid hub script path".to_string(),
                ));
            };

            let version = match version.parse::<i64>() {
                Ok(version) => version,
                Err(_) => {
                    return Err(windmill_common::error::Error::internal_err(
                        "Invalid hub script version".to_string(),
                    ));
                }
            };

            let key = (HubOrWorkspaceId::Hub, version, trigger_kind.clone());

            let runnable_format = RUNNABLE_FORMAT_VERSION_CACHE.get(&key);

            if let Some(runnable_format) = runnable_format {
                tracing::debug!("Using cached runnable format for hub script {path}");
                return Ok(runnable_format);
            }

            let hub_script =
                get_full_hub_script_by_path(StripPath(path.to_string()), &HTTP_CLIENT, Some(db))
                    .await?;

            let has_preprocessor = get_has_preprocessor_from_content_and_lang(
                &hub_script.content,
                &hub_script.language,
            )?;

            let partial_schema = serde_json::from_str(hub_script.schema.get())?;

            (
                key,
                if has_preprocessor {
                    PreprocessorInfo::Preprocessor {
                        content: hub_script.content,
                        language: hub_script.language,
                    }
                } else {
                    PreprocessorInfo::NoPreprocessor {
                        schema: Some(sqlx::types::Json(partial_schema)),
                    }
                },
            )
        }
        RunnableId::FlowId(flow_id) => {
            let version = flow_id.get_flow_version_id(workspace_id, db).await?;

            let key = (
                HubOrWorkspaceId::WorkspaceId(workspace_id.to_string()),
                version,
                trigger_kind.clone(),
            );

            let runnable_format = RUNNABLE_FORMAT_VERSION_CACHE.get(&key);

            if let Some(runnable_format) = runnable_format {
                tracing::debug!("Using cached runnable format for flow version {version}");
                return Ok(runnable_format);
            }

            let flow_info = sqlx::query_as!(
                FlowInfo,
                "SELECT
                    value->'preprocessor_module'->'value' as \"preprocessor_module: _\",
                    schema as \"schema: _\"
                FROM flow_version
                WHERE
                    id = $1
                    AND workspace_id = $2",
                version,
                workspace_id,
            )
            .fetch_one(db)
            .await?;

            if let Some(preprocessor_module) = flow_info.preprocessor_module {
                match preprocessor_module.0 {
                    FlowModuleValue::RawScript { content, language, .. } => {
                        (key, PreprocessorInfo::Preprocessor { content, language })
                    }
                    FlowModuleValue::Script { path, hash, .. } => {
                        let hash = if let Some(hash) = hash {
                            hash.0
                        } else {
                            let script_hash = get_latest_deployed_hash_for_path(
                                None,
                                db.clone(),
                                workspace_id,
                                &path,
                            )
                            .await?;
                            script_hash.hash
                        };
                        let script_info = get_script_info(db, workspace_id, hash).await?;
                        (
                            key,
                            PreprocessorInfo::Preprocessor {
                                content: script_info.content,
                                language: script_info.language,
                            },
                        )
                    }
                    _ => {
                        return Err(windmill_common::error::Error::internal_err(
                            "Unsupported preprocessor module".to_string(),
                        ));
                    }
                }
            } else {
                (
                    key,
                    PreprocessorInfo::NoPreprocessor { schema: flow_info.schema },
                )
            }
        }
        RunnableId::ScriptId(script_id) => {
            let hash = script_id.get_script_hash(workspace_id, db).await?;
            let key = (
                HubOrWorkspaceId::WorkspaceId(workspace_id.to_string()),
                hash,
                trigger_kind.clone(),
            );
            let runnable_format = RUNNABLE_FORMAT_VERSION_CACHE.get(&key);

            if let Some(runnable_format) = runnable_format {
                tracing::debug!("Using cached runnable format for script {hash}");
                return Ok(runnable_format);
            }

            let script_info = get_script_info(db, workspace_id, hash).await?;

            if script_info.has_preprocessor.unwrap_or(false) {
                (
                    key,
                    PreprocessorInfo::Preprocessor {
                        content: script_info.content,
                        language: script_info.language,
                    },
                )
            } else {
                (
                    key,
                    PreprocessorInfo::NoPreprocessor { schema: script_info.schema },
                )
            }
        }
    };

    let runnable_format = match preprocessor_info {
        PreprocessorInfo::Preprocessor { content, language } => {
            let args = get_preprocessor_args_from_content_and_language(&content, &language)?;
            runnable_format_from_preprocessor_args(args)
        }
        PreprocessorInfo::NoPreprocessor { schema } => {
            runnable_format_from_schema_without_preprocessor(trigger_kind, schema)
        }
    };

    RUNNABLE_FORMAT_VERSION_CACHE.insert(key, runnable_format);

    Ok(runnable_format)
}

#[allow(dead_code)]

pub trait TriggerJobArgs {
    type Payload: Send + Sync;
    const TRIGGER_KIND: TriggerKind;

    fn v1_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>>;
    fn v2_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        Self::v1_payload_fn(payload)
    }

    fn build_job_args_v2(
        has_preprocessor: bool,
        payload: &Self::Payload,
        info: HashMap<String, Box<RawValue>>,
    ) -> PushArgsOwned {
        let mut args = Self::v2_payload_fn(payload);
        if has_preprocessor {
            args.insert(
                "kind".to_string(),
                to_raw_value(&Self::TRIGGER_KIND.to_key()),
            );
            args.extend(info);
            let args = HashMap::from([("event".to_string(), to_raw_value(&args))]);
            PushArgsOwned { args, extra: None }
        } else {
            PushArgsOwned { args, extra: None }
        }
    }

    fn build_job_args_v1(
        has_preprocessor: bool,
        payload: &Self::Payload,
        info: HashMap<String, Box<RawValue>>,
    ) -> PushArgsOwned {
        let trigger_key = Self::TRIGGER_KIND.to_key();
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

    fn build_job_args(
        runnable_path: &str,
        is_flow: bool,
        w_id: &str,
        db: &DB,
        payload: Self::Payload,
        info: HashMap<String, Box<RawValue>>,
    ) -> impl Future<Output = Result<PushArgsOwned>> + Send {
        async move {
            let runnable_id = if is_flow {
                RunnableId::from_flow_path(runnable_path)
            } else {
                RunnableId::from_script_path(runnable_path)
            };
            Self::build_job_args_from_runnable_id(runnable_id, w_id, db, payload, info).await
        }
    }

    fn build_job_args_from_runnable_id(
        runnable_id: RunnableId,
        w_id: &str,
        db: &DB,
        payload: Self::Payload,
        trigger_info: HashMap<String, Box<RawValue>>,
    ) -> impl Future<Output = Result<PushArgsOwned>> + Send {
        async move {
            tracing::debug!("Building job args for {runnable_id:?}");
            let runnable_format =
                get_runnable_format(runnable_id, w_id, db, &Self::TRIGGER_KIND).await?;
            let job_args = match runnable_format {
                RunnableFormat { version: RunnableFormatVersion::V1, has_preprocessor } => {
                    Self::build_job_args_v1(has_preprocessor, &payload, trigger_info)
                }
                RunnableFormat { version: RunnableFormatVersion::V2, has_preprocessor } => {
                    Self::build_job_args_v2(has_preprocessor, &payload, trigger_info)
                }
            };

            Ok(job_args)
        }
    }

    fn build_capture_payloads(
        payload: &Self::Payload,
        info: HashMap<String, Box<RawValue>>,
    ) -> (PushArgsOwned, PushArgsOwned) {
        let main_args = Self::build_job_args_v2(false, payload, info.clone());
        let preprocessor_args = Self::build_job_args_v2(true, payload, info);
        (main_args, preprocessor_args)
    }
}

#[allow(dead_code)]
pub async fn trigger_runnable_inner<'c>(
    db: &DB,
    tx_o: Option<sqlx::Transaction<'c, sqlx::Postgres>>,
    user_db: Option<UserDB>,
    authed: ApiAuthed,
    workspace_id: &str,
    runnable_path: &str,
    is_flow: bool,
    args: PushArgsOwned,
    retry: Option<&sqlx::types::Json<Retry>>,
    error_handler_path: Option<&str>,
    error_handler_args: Option<&sqlx::types::Json<HashMap<String, serde_json::Value>>>,
    trigger_path: String,
    job_id: Option<Uuid>,
    trigger: TriggerMetadata,
    suspended_mode: Option<bool>,
) -> Result<(
    Uuid,
    Option<bool>,
    Option<String>,
    Option<sqlx::Transaction<'c, sqlx::Postgres>>,
)> {
    let error_handler_args = error_handler_args.map(|args| {
        let args = args
            .0
            .iter()
            .map(|(key, value)| (key.to_owned(), to_raw_value(&value)))
            .collect::<HashMap<String, Box<RawValue>>>();
        Json(args)
    });

    let user_db = user_db.unwrap_or_else(|| UserDB::new(db.clone()));
    let (uuid, delete_after_use, early_return, tx_out) = if is_flow {
        let run_query = RunJobQuery { job_id, suspended_mode, ..Default::default() };
        let path = StripPath(runnable_path.to_string());
        let (uuid, early_return, tx_out) = push_flow_job_by_path_into_queue(
            authed,
            db.clone(),
            tx_o,
            user_db,
            workspace_id.to_string(),
            path,
            run_query,
            args,
            Some(trigger),
        )
        .await?;
        (uuid, None, early_return, tx_out)
    } else {
        let (uuid, delete_after_use, tx_out) = trigger_script_internal(
            db,
            tx_o,
            user_db,
            authed,
            workspace_id,
            runnable_path,
            args,
            retry,
            error_handler_path,
            error_handler_args.as_ref(),
            trigger_path,
            job_id,
            trigger,
            suspended_mode,
        )
        .await?;
        (uuid, delete_after_use, None, tx_out)
    };

    Ok((uuid, delete_after_use, early_return, tx_out))
}

#[allow(dead_code)]
pub async fn trigger_runnable(
    db: &DB,
    user_db: Option<UserDB>,
    authed: ApiAuthed,
    workspace_id: &str,
    runnable_path: &str,
    is_flow: bool,
    args: PushArgsOwned,
    retry: Option<&sqlx::types::Json<Retry>>,
    error_handler_path: Option<&str>,
    error_handler_args: Option<&sqlx::types::Json<HashMap<String, serde_json::Value>>>,
    trigger_path: String,
    job_id: Option<Uuid>,
    suspended_mode: bool,
    trigger: TriggerMetadata,
) -> Result<axum::response::Response> {
    let uuid = trigger_runnable_inner(
        db,
        None,
        user_db,
        authed,
        workspace_id,
        runnable_path,
        is_flow,
        args,
        retry,
        error_handler_path,
        error_handler_args,
        trigger_path,
        job_id,
        trigger,
        Some(suspended_mode),
    )
    .await?
    .0;
    Ok((StatusCode::CREATED, uuid.to_string()).into_response())
}

#[allow(dead_code)]
pub async fn trigger_runnable_and_wait_for_result(
    db: &DB,
    user_db: Option<UserDB>,
    authed: ApiAuthed,
    workspace_id: &str,
    runnable_path: &str,
    is_flow: bool,
    args: PushArgsOwned,
    retry: Option<&sqlx::types::Json<Retry>>,
    error_handler_path: Option<&str>,
    error_handler_args: Option<&sqlx::types::Json<HashMap<String, serde_json::Value>>>,
    trigger_path: String,
    trigger: TriggerMetadata,
) -> Result<axum::response::Response> {
    let username = authed.username.clone();
    let (uuid, delete_after_use, early_return, _) = trigger_runnable_inner(
        db,
        None,
        user_db,
        authed,
        workspace_id,
        runnable_path,
        is_flow,
        args,
        retry,
        error_handler_path,
        error_handler_args,
        trigger_path,
        None,
        trigger,
        None,
    )
    .await?;
    let (result, success) =
        run_wait_result_internal(db, uuid, &workspace_id, early_return, &username).await?;

    if delete_after_use.unwrap_or(false) {
        delete_job_metadata_after_use(&db, uuid).await?;
    }

    result_to_response(result, success)
}

#[allow(dead_code)]
pub async fn trigger_runnable_and_wait_for_raw_result(
    db: &DB,
    user_db: Option<UserDB>,
    authed: ApiAuthed,
    workspace_id: &str,
    runnable_path: &str,
    is_flow: bool,
    args: PushArgsOwned,
    retry: Option<&sqlx::types::Json<Retry>>,
    error_handler_path: Option<&str>,
    error_handler_args: Option<&sqlx::types::Json<HashMap<String, serde_json::Value>>>,
    trigger_path: String,
    trigger: TriggerMetadata,
) -> Result<(Box<RawValue>, bool)> {
    let username = authed.username.clone();
    let (uuid, delete_after_use, early_return, _) = trigger_runnable_inner(
        db,
        None,
        user_db,
        authed,
        workspace_id,
        runnable_path,
        is_flow,
        args,
        retry,
        error_handler_path,
        error_handler_args,
        trigger_path,
        None,
        trigger,
        None,
    )
    .await?;

    let (result, success) =
        run_wait_result_internal(db, uuid, &workspace_id, early_return, &username)
            .await
            .with_context(|| {
                format!(
                    "Error fetching job result for {} {}",
                    if is_flow { "flow" } else { "script" },
                    runnable_path
                )
            })?;

    if delete_after_use.unwrap_or(false) {
        delete_job_metadata_after_use(&db, uuid).await?;
    }

    Ok((result, success))
}

pub async fn trigger_runnable_and_wait_for_raw_result_with_error_ctx(
    db: &DB,
    user_db: Option<UserDB>,
    authed: ApiAuthed,
    workspace_id: &str,
    runnable_path: &str,
    is_flow: bool,
    args: PushArgsOwned,
    retry: Option<&sqlx::types::Json<Retry>>,
    error_handler_path: Option<&str>,
    error_handler_args: Option<&sqlx::types::Json<HashMap<String, serde_json::Value>>>,
    trigger_path: String,
    trigger: TriggerMetadata,
) -> Result<Box<RawValue>> {
    let (result, success) = trigger_runnable_and_wait_for_raw_result(
        db,
        user_db,
        authed,
        workspace_id,
        runnable_path,
        is_flow,
        args,
        retry,
        error_handler_path,
        error_handler_args,
        trigger_path,
        trigger,
    )
    .await?;

    if !success {
        Err(windmill_common::error::Error::internal_err(format!(
            "{} {runnable_path} failed: {:?}",
            if is_flow { "Flow" } else { "Script" },
            result
        )))
    } else {
        Ok(result)
    }
}

async fn trigger_script_internal<'c>(
    db: &DB,
    tx_o: Option<sqlx::Transaction<'c, sqlx::Postgres>>,
    user_db: UserDB,
    authed: ApiAuthed,
    workspace_id: &str,
    script_path: &str,
    args: PushArgsOwned,
    retry: Option<&sqlx::types::Json<Retry>>,
    error_handler_path: Option<&str>,
    error_handler_args: Option<&sqlx::types::Json<HashMap<String, Box<RawValue>>>>,
    trigger_path: String,
    job_id: Option<Uuid>,
    trigger: TriggerMetadata,
    suspended_mode: Option<bool>,
) -> Result<(
    Uuid,
    Option<bool>,
    Option<sqlx::Transaction<'c, sqlx::Postgres>>,
)> {
    if retry.is_none() && error_handler_path.is_none() {
        let run_query = RunJobQuery { job_id, suspended_mode, ..Default::default() };
        let path = StripPath(script_path.to_string());
        let (uuid, delete_after_use, tx_out) = push_script_job_by_path_into_queue(
            authed,
            db.clone(),
            tx_o,
            user_db,
            workspace_id.to_string(),
            path,
            run_query,
            args,
            Some(trigger),
        )
        .await?;
        Ok((uuid, delete_after_use, tx_out))
    } else {
        let (uuid, delete_after_use, tx_out) = trigger_script_with_retry_and_error_handler(
            db,
            tx_o,
            user_db,
            authed,
            workspace_id,
            script_path,
            args,
            retry,
            error_handler_path,
            error_handler_args,
            trigger_path,
            job_id,
            trigger,
            suspended_mode,
        )
        .await?;
        Ok((uuid, delete_after_use, tx_out))
    }
}

async fn trigger_script_with_retry_and_error_handler<'c>(
    db: &DB,
    tx_o: Option<sqlx::Transaction<'c, sqlx::Postgres>>,
    user_db: UserDB,
    authed: ApiAuthed,
    workspace_id: &str,
    script_path: &str,
    args: PushArgsOwned,
    retry: Option<&sqlx::types::Json<Retry>>,
    error_handler_path: Option<&str>,
    error_handler_args: Option<&sqlx::types::Json<HashMap<String, Box<RawValue>>>>,
    trigger_path: String,
    job_id: Option<Uuid>,
    trigger: TriggerMetadata,
    suspended_mode: Option<bool>,
) -> Result<(
    Uuid,
    Option<bool>,
    Option<sqlx::Transaction<'c, sqlx::Postgres>>,
)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    check_scopes(&authed, || format!("jobs:run:scripts:{script_path}"))?;

    let retry = retry.map(|r| r.0.clone());
    let error_handler_path = error_handler_path.map(|p| p.to_string());
    let error_handler_args = error_handler_args.map(|args| args.0.clone());

    let (job_payload, tag, delete_after_use, timeout, on_behalf_of) = {
        let db_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
        script_path_to_payload(
            script_path,
            Some(db_authed),
            db.clone(),
            &workspace_id,
            Some(false),
        )
        .await?
    };

    check_tag_available_for_workspace(&db, &workspace_id, &tag, &authed).await?;

    let return_tx = tx_o.is_some();

    let (email, permissioned_as, push_authed, tx) = if let Some(tx) = tx_o {
        (
            authed.email.as_str(),
            username_to_permissioned_as(&authed.username),
            Some(authed.clone().into()),
            PushIsolationLevel::Transaction(tx),
        )
    } else if let Some(on_behalf_of) = on_behalf_of.as_ref() {
        (
            on_behalf_of.email.as_str(),
            on_behalf_of.permissioned_as.clone(),
            None,
            PushIsolationLevel::IsolatedRoot(db.clone()),
        )
    } else {
        (
            authed.email.as_str(),
            username_to_permissioned_as(&authed.username),
            Some(authed.clone().into()),
            PushIsolationLevel::Isolated(user_db, authed.clone().into()),
        )
    };

    let push_args = PushArgs { args: &args.args, extra: args.extra };

    let retryable_job_payload = match job_payload {
        JobPayload::ScriptHash {
            hash,
            path,
            concurrency_settings,
            debouncing_settings,
            cache_ttl,
            cache_ignore_s3_path,
            priority,
            apply_preprocessor,
            ..
        } => JobPayload::SingleStepFlow {
            path,
            hash: Some(hash),
            flow_version: None,
            args: HashMap::from(&push_args),
            retry,
            error_handler_path,
            error_handler_args,
            skip_handler: None,
            cache_ttl,
            cache_ignore_s3_path,
            priority,
            tag_override: tag.clone(),
            apply_preprocessor,
            trigger_path: Some(trigger_path.clone()),
            concurrency_settings,
            debouncing_settings,
        },
        _ => {
            return Err(windmill_common::error::Error::internal_err(format!(
                "Unsupported job payload: {:?}",
                job_payload
            )))
        }
    };
    let (uuid, tx) = push(
        &db,
        tx,
        &workspace_id,
        retryable_job_payload,
        push_args,
        authed.display_username(),
        email,
        permissioned_as,
        authed.token_prefix.as_deref(),
        None,
        None,
        None,
        None,
        None,
        job_id,
        false,
        false,
        None,
        true,
        tag,
        timeout,
        None,
        None,
        push_authed.as_ref(),
        false,
        None,
        Some(trigger),
        suspended_mode,
    )
    .await?;

    // If we were given a transaction, return it; otherwise commit it
    if return_tx {
        Ok((uuid, delete_after_use, Some(tx)))
    } else {
        tx.commit().await?;
        Ok((uuid, delete_after_use, None))
    }
}
