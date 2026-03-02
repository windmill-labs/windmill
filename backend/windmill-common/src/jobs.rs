use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use bytes::Bytes;
use futures_core::Stream;
use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use serde_json::value::RawValue;
use tokio::io::AsyncReadExt;

pub use windmill_types::jobs::*;

use crate::{
    auth::is_super_admin_email,
    client::AuthedClient,
    db::{AuthedRef, UserDbWithAuthed, DB},
    error::{self, to_anyhow, Error},
    get_latest_deployed_hash_for_path, get_latest_flow_version_info_for_path,
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    users::username_to_permissioned_as,
    utils::{StripPath, HTTP_CLIENT},
    worker::{to_raw_value, CUSTOM_TAGS_PER_WORKSPACE, TMP_DIR},
    FlowVersionInfo, ScriptHashInfo, Tag,
};

pub fn get_has_preprocessor_from_content_and_lang(
    content: &str,
    language: &ScriptLang,
) -> error::Result<bool> {
    let has_preprocessor = match language {
        ScriptLang::Bun | ScriptLang::Bunnative | ScriptLang::Deno | ScriptLang::Nativets => {
            let args = windmill_parser_ts::parse_deno_signature(&content, true, true, None)?;
            args.has_preprocessor.unwrap_or(false)
        }
        #[cfg(feature = "python")]
        ScriptLang::Python3 => {
            let args = windmill_parser_py::parse_python_signature(&content, None, true)?;
            args.has_preprocessor.unwrap_or(false)
        }
        _ => false,
    };

    Ok(has_preprocessor)
}

pub async fn script_path_to_payload<'e>(
    script_path: &str,
    db_authed: Option<UserDbWithAuthed<'e, AuthedRef<'e>>>,
    db: DB,
    w_id: &str,
    skip_preprocessor: Option<bool>,
) -> error::Result<(
    JobPayload,
    Option<Tag>,
    Option<bool>,
    Option<i32>,
    Option<OnBehalfOf>,
)> {
    let (job_payload, tag, delete_after_use, script_timeout, on_behalf_of) = if script_path
        .starts_with("hub/")
    {
        let hub_script =
            get_full_hub_script_by_path(StripPath(script_path.to_string()), &HTTP_CLIENT, None)
                .await?;

        let has_preprocessor =
            get_has_preprocessor_from_content_and_lang(&hub_script.content, &hub_script.language)?;

        (
            JobPayload::ScriptHub {
                path: script_path.to_owned(),
                apply_preprocessor: has_preprocessor && !skip_preprocessor.unwrap_or(false),
            },
            None,
            None,
            None,
            None,
        )
    } else {
        let ScriptHashInfo {
            hash,
            tag,
            runnable_settings:
                super::scripts::ScriptRunnableSettingsInline {
                    concurrency_settings,
                    debouncing_settings,
                },
            cache_ttl,
            cache_ignore_s3_path,
            language,
            dedicated_worker,
            priority,
            delete_after_use,
            timeout,
            has_preprocessor,
            on_behalf_of_email,
            created_by,
            ..
        } = get_latest_deployed_hash_for_path(db_authed, db.clone(), w_id, script_path)
            .await?
            .prefetch_cached(&db)
            .await?;

        let on_behalf_of = if let Some(email) = on_behalf_of_email {
            Some(OnBehalfOf {
                email,
                permissioned_as: username_to_permissioned_as(created_by.as_str()),
            })
        } else {
            None
        };

        (
            JobPayload::ScriptHash {
                hash: ScriptHash(hash),
                path: script_path.to_owned(),
                cache_ttl,
                cache_ignore_s3_path,
                language,
                dedicated_worker,
                priority,
                apply_preprocessor: !skip_preprocessor.unwrap_or(false)
                    && has_preprocessor.unwrap_or(false),
                debouncing_settings,
                concurrency_settings,
            },
            tag,
            delete_after_use,
            timeout,
            on_behalf_of,
        )
    };
    Ok((
        job_payload,
        tag,
        delete_after_use,
        script_timeout,
        on_behalf_of,
    ))
}

pub async fn get_payload_tag_from_prefixed_path(
    path: &str,
    db: &DB,
    w_id: &str,
) -> Result<(JobPayload, Option<String>, Option<OnBehalfOf>), Error> {
    let (payload, tag, _, _, on_behalf_of) = if path.starts_with("script/") {
        script_path_to_payload(
            path.strip_prefix("script/").unwrap(),
            None,
            db.clone(),
            w_id,
            Some(true),
        )
        .await?
    } else if path.starts_with("flow/") {
        let path = path.strip_prefix("flow/").unwrap().to_string();
        let FlowVersionInfo { dedicated_worker, tag, version, .. } =
            get_latest_flow_version_info_for_path(None, &db, w_id, &path, true).await?;
        (
            JobPayload::Flow { path, dedicated_worker, apply_preprocessor: false, version },
            tag,
            None,
            None,
            None,
        )
    } else {
        return Err(Error::BadRequest(format!(
            "path must start with script/ or flow/ (got {})",
            path
        )));
    };
    Ok((payload, tag, on_behalf_of))
}

pub fn order_columns(
    rows: Option<Vec<Box<RawValue>>>,
    column_order: &Vec<String>,
) -> Option<Box<RawValue>> {
    if let Some(mut rows) = rows {
        if let Some(first_row) = rows.get(0) {
            let first_row = serde_json::from_str::<HashMap<String, Box<RawValue>>>(first_row.get());
            if let Ok(first_row) = first_row {
                let mut new_first_row = IndexMap::new();
                for col in column_order {
                    if let Some(val) = first_row.get(col) {
                        new_first_row.insert(col.clone(), val.clone());
                    }
                }
                let new_row_as_raw_value = to_raw_value(&new_first_row);

                rows[0] = new_row_as_raw_value;

                return Some(to_raw_value(&rows));
            }
        }
    }

    None
}

pub fn format_result(
    result_columns: Option<&Vec<String>>,
    result: Option<&mut sqlx::types::Json<Box<RawValue>>>,
) -> () {
    if let Some(result_columns) = result_columns {
        if let Some(result) = result {
            let rows = serde_json::from_str::<Vec<Box<RawValue>>>(result.get()).ok();
            if let Some(ordered_result) = order_columns(rows, result_columns) {
                *result = sqlx::types::Json(ordered_result);
            }
        }
    }
}

pub fn format_completed_job_result(mut cj: CompletedJob) -> CompletedJob {
    format_result(cj.result_columns.as_ref(), cj.result.as_mut());
    cj
}

pub async fn get_logs_from_disk(
    log_offset: i32,
    logs: &str,
    log_file_index: &Option<Vec<String>>,
) -> Option<impl Stream<Item = Result<Bytes, anyhow::Error>>> {
    if log_offset > 0 {
        if let Some(file_index) = log_file_index.clone() {
            for file_p in &file_index {
                if !tokio::fs::metadata(format!("{TMP_DIR}/{file_p}"))
                    .await
                    .is_ok()
                {
                    return None;
                }
            }

            let logs = logs.to_string();
            let stream = async_stream::stream! {
                for file_p in file_index.clone() {
                    let mut file = tokio::fs::File::open(format!("{TMP_DIR}/{file_p}")).await.map_err(to_anyhow)?;
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer).await.map_err(to_anyhow)?;
                    yield Ok(bytes::Bytes::from(buffer)) as anyhow::Result<bytes::Bytes>;
                }

                yield Ok(bytes::Bytes::from(logs))
            };
            return Some(stream);
        }
    }
    return None;
}

lazy_static::lazy_static! {
    pub static ref TAGS_ARE_SENSITIVE: bool = std::env::var("TAGS_ARE_SENSITIVE").map(
        |v| v.parse().unwrap()
    ).unwrap_or(false);
    pub static ref HIDE_WORKERS_FOR_NON_ADMINS: bool = std::env::var("HIDE_WORKERS_FOR_NON_ADMINS").map(
        |v| v.parse().unwrap()
    ).unwrap_or(false);
}

pub async fn check_tag_available_for_workspace_internal(
    db: &DB,
    w_id: &str,
    tag: &str,
    email: &str,
    scope_tags: Option<Vec<&str>>,
) -> error::Result<()> {
    let mut is_tag_in_scope_tags = None;
    let mut is_tag_in_workspace_custom_tags = false;

    if let Some(scope_tags) = scope_tags.as_ref() {
        is_tag_in_scope_tags = Some(scope_tags.contains(&tag));
    }

    let custom_tags_per_w = CUSTOM_TAGS_PER_WORKSPACE.read().await;
    if custom_tags_per_w.global.contains(&tag.to_string()) {
        is_tag_in_workspace_custom_tags = true;
    } else if let Some(specific_tag) = custom_tags_per_w.specific.get(tag) {
        is_tag_in_workspace_custom_tags = specific_tag.applies_to_workspace(w_id);
    }

    match is_tag_in_scope_tags {
        Some(true) | None => {
            if is_tag_in_workspace_custom_tags {
                return Ok(());
            }
        }
        _ => {}
    }

    if !is_super_admin_email(db, email).await? {
        if scope_tags.is_some() && is_tag_in_scope_tags.is_some() {
            return Err(Error::BadRequest(format!(
                "Tag {tag} is not available in your scope"
            )));
        }

        if *TAGS_ARE_SENSITIVE {
            return Err(Error::BadRequest(format!("{tag} is not available to you")));
        } else {
            return Err(error::Error::BadRequest(format!(
            "Only super admins are allowed to use tags that are not included in the allowed CUSTOM_TAGS: {:?}",
            custom_tags_per_w
        )));
        }
    }

    return Ok(());
}

pub struct RunInlinePreviewScriptFnParams {
    pub workspace_id: String,
    pub content: String,
    pub lang: ScriptLang,
    pub args: Option<HashMap<String, Box<RawValue>>>,
    pub created_by: String,
    pub permissioned_as: String,
    pub permissioned_as_email: String,
    pub base_internal_url: String,
    pub worker_name: String,
    pub conn: crate::worker::Connection,
    pub client: AuthedClient,
    pub job_dir: String,
    pub worker_dir: String,
    pub killpill_rx: tokio::sync::broadcast::Receiver<()>,
}

#[derive(Clone)]
pub struct WorkerInternalServerInlineUtils {
    pub killpill_rx: Arc<tokio::sync::broadcast::Receiver<()>>,
    pub base_internal_url: String,
    pub run_inline_preview_script: Arc<
        dyn Fn(
                RunInlinePreviewScriptFnParams,
            ) -> Pin<Box<dyn Future<Output = error::Result<Box<RawValue>>> + Send>>
            + Send
            + Sync,
    >,
}
// To run a script inline, bypassing the db and job queue, windmill-api uses these functions.
// They should only be called by the internal server of a worker.
// main() sets the global on startup.
// The server cannot call the worker functions directly because they are independent crates
pub static WORKER_INTERNAL_SERVER_INLINE_UTILS: OnceCell<WorkerInternalServerInlineUtils> =
    OnceCell::new();
