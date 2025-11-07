use anyhow::Context;
use axum::response::IntoResponse;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::types::Json;
use std::collections::HashMap;
use std::future::Future;
use uuid::Uuid;
use windmill_common::{
    db::{UserDB, UserDbWithAuthed},
    error::Result,
    flows::{FlowModuleValue, Retry},
    get_latest_deployed_hash_for_path, get_latest_flow_version_info_for_path,
    jobs::{get_has_preprocessor_from_content_and_lang, script_path_to_payload, JobPayload},
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    triggers::{
        HubOrWorkspaceId, RunnableFormat, RunnableFormatVersion, TriggerKind,
        RUNNABLE_FORMAT_VERSION_CACHE,
    },
    users::username_to_permissioned_as,
    utils::StripPath,
    worker::to_raw_value,
    FlowVersionInfo,
};
use windmill_queue::{push, PushArgs, PushArgsOwned, PushIsolationLevel};

#[cfg(feature = "enterprise")]
use crate::jobs::check_license_key_valid;
use crate::{
    db::{ApiAuthed, DB},
    jobs::{
        check_tag_available_for_workspace, delete_job_metadata_after_use, result_to_response,
        run_flow_by_path_inner, run_script_by_path_inner, run_wait_result_internal, RunJobQuery,
    },
    utils::check_scopes,
    HTTP_CLIENT,
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
    FlowPath(String),
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
        Self::FlowPath(path.to_string())
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
        RunnableId::FlowPath(path) => {
            let FlowVersionInfo { version, .. } =
                get_latest_flow_version_info_for_path(None, &db, workspace_id, &path, true).await?;

            let key = (
                HubOrWorkspaceId::WorkspaceId(workspace_id.to_string()),
                version,
                trigger_kind.clone(),
            );

            let runnable_format = RUNNABLE_FORMAT_VERSION_CACHE.get(&key);

            if let Some(runnable_format) = runnable_format {
                tracing::debug!("Using cached runnable format for flow {path}");
                return Ok(runnable_format);
            }

            let flow_info = sqlx::query_as!(
                FlowInfo,
                "SELECT
                    value->'preprocessor_module'->'value' as \"preprocessor_module: _\",
                    schema as \"schema: _\"
                FROM flow_version
                WHERE
                    path = $1
                    AND workspace_id = $2
                ORDER BY created_at DESC
                LIMIT 1",
                path,
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
pub async fn trigger_runnable_inner(
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
) -> Result<(Uuid, Option<bool>, Option<String>)> {
    let error_handler_args = error_handler_args.map(|args| {
        let args = args
            .0
            .iter()
            .map(|(key, value)| (key.to_owned(), to_raw_value(&value)))
            .collect::<HashMap<String, Box<RawValue>>>();
        Json(args)
    });

    let user_db = user_db.unwrap_or_else(|| UserDB::new(db.clone()));
    let (uuid, delete_after_use, early_return) = if is_flow {
        let run_query = RunJobQuery { job_id, ..Default::default() };
        let path = StripPath(runnable_path.to_string());
        let (uuid, early_return) = run_flow_by_path_inner(
            authed,
            db.clone(),
            user_db,
            workspace_id.to_string(),
            path,
            run_query,
            args,
        )
        .await?;
        (uuid, None, early_return)
    } else {
        let (uuid, delete_after_use) = trigger_script_internal(
            db,
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
        )
        .await?;
        (uuid, delete_after_use, None)
    };

    Ok((uuid, delete_after_use, early_return))
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
) -> Result<axum::response::Response> {
    let (uuid, _, _) = trigger_runnable_inner(
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
        job_id,
    )
    .await?;
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
) -> Result<axum::response::Response> {
    let username = authed.username.clone();
    let (uuid, delete_after_use, early_return) = trigger_runnable_inner(
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
        None,
    )
    .await?;
    let (result, success) =
        run_wait_result_internal(db, uuid, workspace_id.to_string(), early_return, &username)
            .await?;

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
) -> Result<(Box<RawValue>, bool)> {
    let username = authed.username.clone();
    let (uuid, delete_after_use, early_return) = trigger_runnable_inner(
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
        None,
    )
    .await?;

    let (result, success) =
        run_wait_result_internal(db, uuid, workspace_id.to_string(), early_return, &username)
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

async fn trigger_script_internal(
    db: &DB,
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
) -> Result<(Uuid, Option<bool>)> {
    if retry.is_none() && error_handler_path.is_none() {
        let run_query = RunJobQuery { job_id, ..Default::default() };
        let path = StripPath(script_path.to_string());
        run_script_by_path_inner(
            authed,
            db.clone(),
            user_db,
            workspace_id.to_string(),
            path,
            run_query,
            args,
        )
        .await
    } else {
        trigger_script_with_retry_and_error_handler(
            db,
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
        )
        .await
    }
}

async fn trigger_script_with_retry_and_error_handler(
    db: &DB,
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
) -> Result<(Uuid, Option<bool>)> {
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

    let (email, permissioned_as, push_authed, tx) =
        if let Some(on_behalf_of) = on_behalf_of.as_ref() {
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
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            custom_debounce_key,
            debounce_delay_s,
            cache_ttl,
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
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            priority,
            tag_override: tag.clone(),
            apply_preprocessor,
            trigger_path: Some(trigger_path),
            custom_debounce_key,
            debounce_delay_s,
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
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((uuid, delete_after_use))
}

// Queue mode and suspend number utilities

/// Generates a random suspend number for queue mode (always > 100)
pub fn generate_trigger_suspend_number() -> i32 {
    use rand::Rng;
    let mut rng = rand::rng();
    // Generate number between 101 and 999999 to stay well above the reserved range
    rng.random_range(101..1000000)
}

/// Generates suspend number based on queue_mode setting
/// Returns Some(suspend_number) if queue_mode is true, None otherwise
pub fn get_suspend_number_for_queue_mode(queue_mode: Option<bool>) -> Option<i32> {
    if queue_mode.unwrap_or(false) {
        Some(generate_trigger_suspend_number())
    } else {
        None
    }
}

/// Updates the suspend number for a trigger in the database
pub async fn update_trigger_suspend_number(
    db: &DB,
    workspace_id: &str,
    trigger_path: &str,
    trigger_type: &str,
    suspend_number: i32,
) -> Result<()> {
    match trigger_type {
        "postgres" => {
            sqlx::query(
                "UPDATE postgres_trigger SET suspend_number = $1 WHERE workspace_id = $2 AND path = $3"
            )
            .bind(suspend_number)
            .bind(workspace_id)
            .bind(trigger_path)
            .execute(db)
            .await?;
        }
        "websocket" => {
            sqlx::query(
                "UPDATE websocket_trigger SET suspend_number = $1 WHERE workspace_id = $2 AND path = $3"
            )
            .bind(suspend_number)
            .bind(workspace_id)
            .bind(trigger_path)
            .execute(db)
            .await?;
        }
        "http" => {
            sqlx::query(
                "UPDATE http_trigger SET suspend_number = $1 WHERE workspace_id = $2 AND path = $3"
            )
            .bind(suspend_number)
            .bind(workspace_id)
            .bind(trigger_path)
            .execute(db)
            .await?;
        }
        "mqtt" => {
            sqlx::query(
                "UPDATE mqtt_trigger SET suspend_number = $1 WHERE workspace_id = $2 AND path = $3"
            )
            .bind(suspend_number)
            .bind(workspace_id)
            .bind(trigger_path)
            .execute(db)
            .await?;
        }
        _ => {
            return Err(windmill_common::error::Error::BadRequest(format!(
                "Unsupported trigger type for queue mode: {}",
                trigger_type
            )));
        }
    }
    
    Ok(())
}

/// Gets all suspended jobs for a specific suspend number
pub async fn get_jobs_by_suspend_number(
    db: &DB,
    workspace_id: &str,
    suspend_number: i32,
) -> Result<Vec<uuid::Uuid>> {
    let job_ids = sqlx::query_scalar!(
        "SELECT id FROM v2_job_queue WHERE workspace_id = $1 AND suspend = $2 ORDER BY created_at ASC",
        workspace_id,
        suspend_number
    )
    .fetch_all(db)
    .await?;

    Ok(job_ids)
}

/// Resumes all jobs with a specific suspend number
pub async fn resume_suspended_jobs(
    db: &DB,
    workspace_id: &str,
    suspend_number: i32,
) -> Result<i64> {
    let result = sqlx::query!(
        "UPDATE v2_job_queue SET suspend = 0, suspend_until = NULL WHERE workspace_id = $1 AND suspend = $2",
        workspace_id,
        suspend_number
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected() as i64)
}

/// Cancels all jobs with a specific suspend number
pub async fn cancel_suspended_jobs(
    db: &DB,
    workspace_id: &str,
    suspend_number: i32,
    canceled_by: &str,
    cancel_reason: &str,
) -> Result<i64> {
    let result = sqlx::query!(
        r#"
        UPDATE v2_job_queue 
        SET canceled_by = $3, canceled_reason = $4, running = false
        WHERE workspace_id = $1 AND suspend = $2
        "#,
        workspace_id,
        suspend_number,
        canceled_by,
        cancel_reason
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected() as i64)
}

/// Wrapper function that handles queue mode logic for trigger execution
pub async fn trigger_runnable_with_queue_mode(
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
    queue_mode: bool,
    suspend_number: Option<i32>,
    trigger_type: &str,
    job_id: Option<Uuid>,
) -> Result<Uuid> {
    if queue_mode {
        // Generate suspend number if not provided
        let suspend_num = suspend_number.unwrap_or_else(|| generate_trigger_suspend_number());
        
        // Update trigger with the suspend number if needed
        update_trigger_suspend_number(db, workspace_id, &trigger_path, trigger_type, suspend_num).await?;
        
        // Execute the runnable with suspend mode
        let uuid = trigger_runnable_with_suspend(
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
            suspend_num,
            job_id,
        ).await?;
        
        Ok(uuid)
    } else {
        // Execute normally without suspend
        let (uuid, _, _) = trigger_runnable_inner(
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
            job_id,
        ).await?;
        
        Ok(uuid)
    }
}

/// Executes a runnable with suspend functionality for queue mode
async fn trigger_runnable_with_suspend(
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
    suspend_number: i32,
    job_id: Option<Uuid>,
) -> Result<Uuid> {
    if is_flow {
        // For flows, use direct queue push with suspend number
        use windmill_queue::{push, PushIsolationLevel};
        use windmill_common::users::username_to_permissioned_as;
        use windmill_common::db::UserDbWithAuthed;
        use windmill_common::get_latest_flow_version_info_for_path;
        
        let user_db = user_db.unwrap_or_else(|| UserDB::new(db.clone()));
        let db_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
        
        // Get flow information
        let FlowVersionInfo { version, .. } = 
            get_latest_flow_version_info_for_path(Some(db_authed), &db, workspace_id, runnable_path, false).await?;
        
        let (email, permissioned_as, push_authed, tx) = (
            authed.email.as_str(),
            username_to_permissioned_as(&authed.username),
            Some(authed.clone().into()),
            PushIsolationLevel::Isolated(user_db, authed.clone().into()),
        );
        
        let push_args = windmill_queue::PushArgs { args: &args.args, extra: args.extra };
        
        // Create flow job payload with suspend number
        let flow_payload = windmill_common::jobs::JobPayload::Flow {
            path: runnable_path.to_string(),
            version: version,
            dedicated_worker: None,
            apply_preprocessor: false,
        };
        
        let (uuid, tx) = push(
            db,
            tx,
            workspace_id,
            flow_payload,
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
            None,
            None,
            None,
            Some(suspend_number as i16), // Set the suspend number (convert to i16)
            push_authed.as_ref(),
            false,
            None,
            None,
        ).await?;
        
        tx.commit().await?;
        Ok(uuid)
    } else {
        // For scripts, create suspended job manually using queue push
        use windmill_queue::{push, PushIsolationLevel};
        use windmill_common::users::username_to_permissioned_as;
        use windmill_common::jobs::script_path_to_payload;
        use crate::jobs::check_tag_available_for_workspace;
        use windmill_common::db::UserDbWithAuthed;
        
        let error_handler_args = error_handler_args.map(|args| {
            args.0
                .iter()
                .map(|(key, value)| (key.to_owned(), to_raw_value(&value)))
                .collect::<HashMap<String, Box<RawValue>>>()
        });
        
        let user_db = user_db.unwrap_or_else(|| UserDB::new(db.clone()));
        let db_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
        let (job_payload, tag, _delete_after_use, timeout, on_behalf_of) = 
            script_path_to_payload(runnable_path, Some(db_authed), db.clone(), workspace_id, Some(false)).await?;
        
        check_tag_available_for_workspace(db, workspace_id, &tag, &authed).await?;
        
        let (email, permissioned_as, push_authed, tx) =
            if let Some(on_behalf_of) = on_behalf_of.as_ref() {
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
        
        let push_args = windmill_queue::PushArgs { args: &args.args, extra: args.extra };
        
        // Create job payload for script execution with retry and error handling
        let retryable_job_payload = match job_payload {
            windmill_common::jobs::JobPayload::ScriptHash {
                hash,
                path,
                custom_concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                custom_debounce_key,
                debounce_delay_s,
                cache_ttl,
                priority,
                apply_preprocessor,
                ..
            } => windmill_common::jobs::JobPayload::SingleStepFlow {
                path,
                hash: Some(hash),
                flow_version: None,
                args: HashMap::from(&push_args),
                retry: retry.map(|r| r.0.clone()),
                error_handler_path: error_handler_path.map(|s| s.to_string()),
                error_handler_args,
                skip_handler: None,
                custom_concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl,
                priority,
                tag_override: tag.clone(),
                apply_preprocessor,
                trigger_path: Some(trigger_path),
                custom_debounce_key,
                debounce_delay_s,
            },
            _ => {
                return Err(windmill_common::error::Error::internal_err(format!(
                    "Unsupported job payload for suspended execution"
                )))
            }
        };
        
        let (uuid, tx) = push(
            db,
            tx,
            workspace_id,
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
            Some(suspend_number as i16), // Set the suspend number (convert to i16)
            push_authed.as_ref(),
            false,
            None,
            None,
        ).await?;
        
        tx.commit().await?;
        Ok(uuid)
    }
}

// API endpoint structures for queue management
#[derive(Debug, Deserialize, Serialize)]
pub struct QueuedJobsResponse {
    pub job_ids: Vec<uuid::Uuid>,
    pub count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResumeJobsRequest {
    pub suspend_number: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CancelJobsRequest {
    pub suspend_number: i32,
    pub cancel_reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JobsOperationResponse {
    pub affected_jobs: i64,
    pub message: String,
}

// API endpoint handlers for queue management  
use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Router,
};

/// API endpoint to list all queued jobs for a specific suspend number
pub async fn list_queued_jobs(
    _authed: crate::db::ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    Query(query): Query<ResumeJobsRequest>,
) -> windmill_common::error::JsonResult<QueuedJobsResponse> {
    let job_ids = get_jobs_by_suspend_number(&db, &workspace_id, query.suspend_number).await?;
    let count = job_ids.len();
    
    Ok(axum::Json(QueuedJobsResponse { job_ids, count }))
}

/// API endpoint to resume all jobs with a specific suspend number
pub async fn resume_queued_jobs(
    _authed: crate::db::ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    axum::Json(request): axum::Json<ResumeJobsRequest>,
) -> windmill_common::error::JsonResult<JobsOperationResponse> {
    let affected_jobs = resume_suspended_jobs(&db, &workspace_id, request.suspend_number).await?;
    
    Ok(axum::Json(JobsOperationResponse {
        affected_jobs,
        message: format!("Resumed {} queued jobs with suspend number {}", affected_jobs, request.suspend_number),
    }))
}

/// API endpoint to cancel all jobs with a specific suspend number  
pub async fn cancel_queued_jobs(
    authed: crate::db::ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    axum::Json(request): axum::Json<CancelJobsRequest>,
) -> windmill_common::error::JsonResult<JobsOperationResponse> {
    let cancel_reason = request.cancel_reason.unwrap_or_else(|| "Canceled by user".to_string());
    let affected_jobs = cancel_suspended_jobs(
        &db, 
        &workspace_id, 
        request.suspend_number,
        &authed.username,
        &cancel_reason
    ).await?;
    
    Ok(axum::Json(JobsOperationResponse {
        affected_jobs,
        message: format!("Canceled {} queued jobs with suspend number {}", affected_jobs, request.suspend_number),
    }))
}

/// Creates the queue management service router
pub fn queue_management_service() -> Router {
    Router::new()
        .route("/list/:workspace_id", get(list_queued_jobs))
        .route("/resume/:workspace_id", post(resume_queued_jobs))
        .route("/cancel/:workspace_id", post(cancel_queued_jobs))
}
