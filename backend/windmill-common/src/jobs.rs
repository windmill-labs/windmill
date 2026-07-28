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
    flows::get_full_hub_flow_by_path,
    get_latest_deployed_hash_for_path, get_latest_flow_version_info_for_path,
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    users::username_to_permissioned_as,
    utils::{StripPath, HTTP_CLIENT},
    worker::{to_raw_value, CUSTOM_TAGS_PER_WORKSPACE, WINDMILL_DIR},
    workspaces::workspace_with_fork_ancestors,
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

pub async fn schedule_job_deletion(
    db: &DB,
    job_id: uuid::Uuid,
    w_id: &str,
    delete_after_secs: i32,
) -> crate::error::Result<()> {
    sqlx::query!(
        "INSERT INTO job_delete_schedule (job_id, workspace_id, delete_at) \
         VALUES ($1, $2, now() + make_interval(secs => $3::double precision)) \
         ON CONFLICT (job_id) DO NOTHING",
        job_id,
        w_id,
        delete_after_secs as f64,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Resolve effective delete behavior from delete_after_use (bool) and delete_after_secs.
/// Returns Some(secs) if deletion should happen, None otherwise.
pub fn resolve_delete_after_secs(
    delete_after_use: Option<bool>,
    delete_after_secs: Option<i32>,
) -> Option<i32> {
    match (delete_after_use, delete_after_secs) {
        (_, Some(secs)) if secs >= 0 => Some(secs),
        (_, Some(_)) => None,          // reject negative values
        (Some(true), None) => Some(0), // backward compat: immediate
        _ => None,
    }
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
    Option<i32>,
    Option<OnBehalfOf>,
)> {
    let (job_payload, tag, delete_after_use, delete_after_secs, script_timeout, on_behalf_of) =
        if script_path.starts_with("hub/") {
            let hub_script =
                get_full_hub_script_by_path(StripPath(script_path.to_string()), &HTTP_CLIENT, None)
                    .await?;

            let has_preprocessor = get_has_preprocessor_from_content_and_lang(
                &hub_script.content,
                &hub_script.language,
            )?;

            (
                JobPayload::ScriptHub {
                    path: script_path.to_owned(),
                    apply_preprocessor: has_preprocessor && !skip_preprocessor.unwrap_or(false),
                },
                None,
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
                delete_after_secs,
                timeout,
                has_preprocessor,
                on_behalf_of_email,
                created_by,
                labels,
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
                    labels,
                },
                tag,
                delete_after_use,
                delete_after_secs,
                timeout,
                on_behalf_of,
            )
        };
    Ok((
        job_payload,
        tag,
        delete_after_use,
        delete_after_secs,
        script_timeout,
        on_behalf_of,
    ))
}

pub async fn get_payload_tag_from_prefixed_path(
    path: &str,
    db: &DB,
    w_id: &str,
) -> Result<(JobPayload, Option<String>, Option<OnBehalfOf>), Error> {
    let (payload, tag, _, _, _, on_behalf_of) = if path.starts_with("script/") {
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
        if path.starts_with("hub/flows/") {
            let hub_flow =
                get_full_hub_flow_by_path(StripPath(path.clone()), &HTTP_CLIENT, Some(db)).await?;
            (
                JobPayload::RawFlow {
                    value: hub_flow.value,
                    path: Some(path),
                    restarted_from: None,
                },
                None,
                None,
                None,
                None,
                None,
            )
        } else {
            let FlowVersionInfo { dedicated_worker, tag, version, labels, .. } =
                get_latest_flow_version_info_for_path(None, &db, w_id, &path, true).await?;
            (
                JobPayload::Flow {
                    path,
                    dedicated_worker,
                    apply_preprocessor: false,
                    version,
                    labels,
                },
                tag,
                None,
                None,
                None,
                None,
            )
        }
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

/// `log_file_index` is normally written by the worker as job-id-scoped relative
/// paths under the windmill log directory. Any code path that lets a request
/// control this value (e.g. job import) must reject entries that could escape
/// that directory, otherwise the log-reading endpoints become an arbitrary file
/// read primitive. Rejects path traversal (`..`) and absolute paths; on-disk
/// readers additionally refuse symlinks (see `get_logs_from_disk`).
pub fn is_safe_log_file_path(file_p: &str) -> bool {
    !file_p.is_empty()
        && !file_p.starts_with('/')
        && !file_p.starts_with('\\')
        && !file_p.split(['/', '\\']).any(|c| c == "..")
}

pub async fn get_logs_from_disk(
    log_offset: i32,
    logs: &str,
    log_file_index: &Option<Vec<String>>,
) -> Option<impl Stream<Item = Result<Bytes, anyhow::Error>>> {
    if log_offset > 0 {
        if let Some(file_index) = log_file_index.clone() {
            for file_p in &file_index {
                if !is_safe_log_file_path(file_p) {
                    return None;
                }
                let local_file = format!("{}/{file_p}", *WINDMILL_DIR);
                // Defense in depth: refuse to read through a symlink so a planted
                // symlink under the log directory cannot exfiltrate arbitrary files.
                match tokio::fs::symlink_metadata(&local_file).await {
                    Ok(meta) if meta.file_type().is_symlink() => return None,
                    Ok(_) => {}
                    Err(_) => return None,
                }
            }

            let logs = logs.to_string();
            let stream = async_stream::stream! {
                for file_p in file_index.clone() {
                    let mut file = tokio::fs::File::open(format!("{}/{file_p}", *WINDMILL_DIR)).await.map_err(to_anyhow)?;
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

    let custom_tags_per_w = CUSTOM_TAGS_PER_WORKSPACE.load();
    if custom_tags_per_w.global.contains(&tag.to_string()) {
        is_tag_in_workspace_custom_tags = true;
    } else if let Some(specific_tag) = custom_tags_per_w.specific.get(tag) {
        // Only a fork-scoped tag can match through the lineage, so every other tag keeps the
        // ancestor lookup off the push path entirely.
        let chain = if specific_tag.is_fork_scoped() {
            workspace_with_fork_ancestors(db, w_id).await?
        } else {
            vec![w_id.to_string()]
        };
        is_tag_in_workspace_custom_tags = specific_tag.applies_to_workspace(&chain);
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

pub enum InlineScriptTarget {
    Path(String),
    Hash(i64),
}

pub struct RunInlineScriptFnParams {
    pub workspace_id: String,
    pub target: InlineScriptTarget,
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
    pub user_db: Option<(crate::db::UserDB, crate::db::Authed)>,
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
    pub run_inline_script: Arc<
        dyn Fn(
                RunInlineScriptFnParams,
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

/// Deletes the given jobs from `v2_job` together with the side tables that reference it
/// without an `ON DELETE CASCADE` foreign key.
///
/// **Authorization contract:** this helper does NO authorization and NO workspace scoping —
/// it deletes exactly the `ids` passed, regardless of which workspace they belong to. Callers
/// MUST ensure `ids` only contains jobs the caller is allowed to delete (either a trusted
/// internal id set, e.g. a retention batch, or ids already filtered by `workspace_id`).
/// Passing user-supplied, unvalidated ids would reintroduce the cross-workspace side-row
/// deletion this centralizes. It is deliberately not workspace-scoped at the signature level
/// because its primary caller — retention — deletes expired jobs across every workspace at
/// once; a `workspace_id` parameter cannot express that. (This is the same trust model as the
/// `ON DELETE CASCADE` FK it replaces: given a job id, the row and its side rows go.)
///
/// Those FKs were removed (migration `drop_v2_job_side_table_cascades`) because they turned
/// every bulk retention delete into a per-row RI trigger; for the unindexed
/// `flow_conversation_message.job_id` that was a sequential scan per deleted row. The
/// set-based deletes below cost one scan per table per call instead. Because the cascade no
/// longer fires, every code path that deletes from `v2_job` by id must go through this helper
/// (or delete these tables itself) or it will leave orphan rows behind.
pub async fn delete_jobs(conn: &mut sqlx::PgConnection, ids: &[uuid::Uuid]) -> error::Result<()> {
    sqlx::query!(
        "DELETE FROM dispatch_event WHERE producer_job_id = ANY($1)",
        ids
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "DELETE FROM flow_conversation_message WHERE job_id = ANY($1)",
        ids
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!("DELETE FROM zombie_job_counter WHERE job_id = ANY($1)", ids)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM job_resolution WHERE job_id = ANY($1)", ids)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM v2_job WHERE id = ANY($1)", ids)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_safe_log_file_path;

    #[test]
    fn safe_log_file_paths_are_accepted() {
        // Legit worker-written entries are job-id-scoped relative paths.
        assert!(is_safe_log_file_path(
            "0190d3e2-0000-7000-8000-000000000000/0.txt"
        ));
        assert!(is_safe_log_file_path("logs/abc/chunk1.log"));
        assert!(is_safe_log_file_path("file..with..dots.txt"));
    }

    #[test]
    fn traversal_and_absolute_paths_are_rejected() {
        assert!(!is_safe_log_file_path(""));
        assert!(!is_safe_log_file_path("../../../../etc/passwd"));
        assert!(!is_safe_log_file_path("a/../../etc/passwd"));
        assert!(!is_safe_log_file_path(".."));
        assert!(!is_safe_log_file_path("/etc/passwd"));
        assert!(!is_safe_log_file_path("/proc/self/environ"));
        assert!(!is_safe_log_file_path("\\windows\\path"));
        assert!(!is_safe_log_file_path("a\\..\\..\\b"));
    }
}
