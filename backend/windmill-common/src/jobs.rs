use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use bytes::Bytes;
use futures_core::Stream;
use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use serde_json::value::RawValue;
use tokio::io::AsyncReadExt;

pub use windmill_types::jobs::*;

use crate::{
    client::AuthedClient,
    db::{AuthedRef, UserDbWithAuthed, DB},
    error::{self, to_anyhow, Error},
    flows::get_full_hub_flow_by_path,
    get_latest_deployed_hash_for_path, get_latest_flow_version_info_for_path,
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    utils::{strip_json_nul, StripPath, HTTP_CLIENT},
    worker::{custom_tag_matches, to_raw_value, CUSTOM_TAGS_PER_WORKSPACE, WINDMILL_DIR},
    workspaces::workspace_with_fork_ancestors,
    FlowVersionInfo, ScriptHashInfo, Tag,
};

/// Whether `args` already reads the way `push` will store it, i.e. holds no NUL escape in a
/// name or a value. Cheap (one SIMD substring scan per value) so the normalization below can
/// be a no-op on the overwhelmingly common clean case.
pub fn args_contain_nul(args: &HashMap<String, Box<RawValue>>) -> bool {
    args.iter().any(|(k, v)| {
        k.contains('\0') || memchr::memmem::find(v.get().as_bytes(), b"\\u0000").is_some()
    })
}

/// Rewrite `args` the way `push` stores it: `push` runs [`strip_json_nul`] over the
/// serialized args just before the INSERT, which removes `\u0000` escapes from **names as
/// well as values** (jsonb rejects them with 22P05). Any decision taken on a name before
/// that — `push`'s own removal of a caller [`MODULES_ARG`], an app policy's `static_inputs`
/// and `one_of_inputs`, the `PushArgs` serializer's "`extra` wins this name" rule — would
/// otherwise compare a name nobody knows against the name that actually lands in the job:
/// an arg sent as `_MODULES\0` is stored as `_MODULES`, and a viewer's `tenant\0` is stored
/// as `tenant`, after the publisher's pinned value, which jsonb then discards as the earlier
/// of two equal keys.
///
/// Normalizing first makes every such check see the stored name. Values are normalized too,
/// so a check on a value sees what the job will receive.
///
/// Two names that collide once normalized keep a single value, deterministically: the
/// spelling without NULs wins, then the lexicographically smallest original name — never the
/// one that `HashMap` iteration happened to serialize last.
pub fn normalize_nul_in_args(args: &mut HashMap<String, Box<RawValue>>) {
    if !args_contain_nul(args) {
        return;
    }
    let mut entries = args.drain().collect::<Vec<_>>();
    entries.sort_by(|(a, _), (b, _)| {
        a.contains('\0')
            .cmp(&b.contains('\0'))
            .then_with(|| a.cmp(b))
    });
    for (key, value) in entries {
        let key = if key.contains('\0') {
            key.replace('\0', "")
        } else {
            key
        };
        let stripped = match strip_json_nul(value.get()) {
            std::borrow::Cow::Owned(stripped) => RawValue::from_string(stripped).ok(),
            std::borrow::Cow::Borrowed(_) => None,
        };
        args.entry(key).or_insert(stripped.unwrap_or(value));
    }
}

/// Drop every [`RESERVED_CONTROL_ARGS`] key from args that a third party supplied for a job
/// running under somebody else's identity (an HTTP route body or native trigger message run
/// as the trigger's owner, a public app viewer's inputs run as the app's publisher). Those
/// keys pick the function the worker calls instead of `main`, the module files written into
/// the job directory, and where workspace imports resolve from: the third party must not
/// choose any of them, while a caller running its own code under its own identity still may.
///
/// Normalizes first, so a key spelled `_ENTRYPOINT_OVERRIDE\0` — which `push` stores as
/// `_ENTRYPOINT_OVERRIDE` — cannot slip past the comparison.
pub fn sanitize_untrusted_args(args: &mut HashMap<String, Box<RawValue>>) {
    normalize_nul_in_args(args);
    for key in RESERVED_CONTROL_ARGS {
        args.remove(key);
    }
}

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

/// A job of a completed flow's run, as returned by [`completed_flow_run_jobs`].
pub struct FlowRunJob {
    pub id: uuid::Uuid,
    pub flow_step_id: Option<String>,
    /// Inside a nested flow, whose step ids are its own rather than the run's.
    pub in_subflow: bool,
}

/// The jobs of a completed flow's run, at any depth. Walks `parent_job`, because `root_job`
/// is stored NULL whenever it equals the job's parent or innermost root, which is the case
/// for most steps. Descends through completed jobs only: once the flow has completed, a job
/// still queued or running below it was launched by a step through the API, and that job and
/// everything under it are a separate run.
pub async fn completed_flow_run_jobs(
    db: &DB,
    flow_id: uuid::Uuid,
) -> crate::error::Result<Vec<FlowRunJob>> {
    Ok(sqlx::query_as!(
        FlowRunJob,
        r#"WITH RECURSIVE run AS (
            SELECT j.id, j.flow_step_id, j.kind = 'flow' AS is_flow, false AS in_subflow
            FROM v2_job j JOIN v2_job_completed c ON c.id = j.id
            WHERE j.parent_job = $1
            UNION ALL
            SELECT j.id, j.flow_step_id, j.kind = 'flow', r.in_subflow OR r.is_flow
            FROM v2_job j JOIN v2_job_completed c ON c.id = j.id JOIN run r ON j.parent_job = r.id
        )
        SELECT id AS "id!", flow_step_id, in_subflow AS "in_subflow!" FROM run"#,
        flow_id
    )
    .fetch_all(db)
    .await?)
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
            let script_info =
                get_latest_deployed_hash_for_path(db_authed, db.clone(), w_id, script_path)
                    .await?
                    .prefetch_cached(&db)
                    .await?;
            let on_behalf_of = script_info.on_behalf_of(w_id, &db).await?;
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
                labels,
                job_token_scopes,
                ..
            } = script_info;

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
                    job_token_scopes,
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
            let FlowVersionInfo {
                dedicated_worker, tag, version, labels, job_token_scopes, ..
            } = get_latest_flow_version_info_for_path(None, &db, w_id, &path, true).await?;
            (
                JobPayload::Flow {
                    path,
                    dedicated_worker,
                    apply_preprocessor: false,
                    version,
                    labels,
                    job_token_scopes,
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

// `is_super_admin` is passed in (not derived from an email here) so callers can
// make it job-token-aware: a job's WM_TOKEN must never count as superadmin
// (GHSA-hfh4-cx4h-3fcr). See `is_super_admin_authed` at the request wrapper.
//
// Custom tags judge `resolved_tag`, the queue the job actually lands on, and `tag` only as the
// job's author wrote it: a dynamic tag names any queue its values do. Callers go through
// `windmill_queue::check_tag_available_for_push`, which resolves it the way `push` does.
// `resolved_tag` is `None` when the values are not known yet: only an entry spelled like `tag`
// admits it then. `tag_workspace` resolves to what the job's `$workspace` stands for.
pub async fn check_tag_available_for_workspace_internal(
    db: &DB,
    w_id: &str,
    tag: &str,
    resolved_tag: Option<&str>,
    tag_workspace: impl Future<Output = String>,
    is_super_admin: bool,
    scope_tags: Option<Vec<&str>>,
) -> error::Result<()> {
    let is_tag_in_scope_tags = scope_tags.as_ref().map(|scope_tags| {
        scope_tags.contains(&tag) || resolved_tag.is_some_and(|r| scope_tags.contains(&r))
    });

    let custom_tags_per_w = CUSTOM_TAGS_PER_WORKSPACE.load();
    // Only an entry reading `$workspace` needs it, so every other tag keeps its lookup off the
    // push path.
    let tag_workspace = if resolved_tag.is_some()
        && custom_tags_per_w
            .global
            .iter()
            .chain(custom_tags_per_w.specific.keys())
            .any(|t| t.contains("$workspace"))
    {
        tag_workspace.await
    } else {
        w_id.to_string()
    };
    // A job whose tag is written exactly as an entry resolves inside what that entry admits, which
    // is the only way in for an entry `custom_tag_matches` cannot turn into a pattern.
    let admits = |entry: &str| {
        entry == tag || resolved_tag.is_some_and(|r| custom_tag_matches(entry, r, &tag_workspace))
    };
    let scoped = custom_tags_per_w
        .specific
        .iter()
        .filter(|(name, _)| admits(name))
        .collect::<Vec<_>>();
    // Only a fork-scoped tag can match through the lineage, so every other tag keeps the
    // ancestor lookup off the push path entirely.
    let chain = if scoped.iter().any(|(_, t)| t.is_fork_scoped()) {
        workspace_with_fork_ancestors(db, w_id).await?
    } else {
        vec![w_id.to_string()]
    };
    // A tag listed by its own name with a scope stays inside that scope: a pattern it fits does
    // not open it to other workspaces. Only listing it by name without a scope does.
    let lands_on = resolved_tag.unwrap_or(tag);
    let confined_by = custom_tags_per_w
        .specific
        .get_key_value(lands_on)
        .filter(|(name, t)| {
            !name.contains('$')
                && !t.applies_to_workspace(&chain)
                && !custom_tags_per_w.global.iter().any(|e| e == lands_on)
        });
    let is_tag_in_workspace_custom_tags = confined_by.is_none()
        && (custom_tags_per_w.global.iter().any(|entry| admits(entry))
            || scoped.iter().any(|(_, t)| t.applies_to_workspace(&chain)));

    match is_tag_in_scope_tags {
        Some(true) | None => {
            if is_tag_in_workspace_custom_tags {
                return Ok(());
            }
        }
        _ => {}
    }

    if !is_super_admin {
        let tag = match resolved_tag {
            Some(resolved_tag) if resolved_tag != tag => {
                format!("{tag} (resolved to {resolved_tag})")
            }
            _ => tag.to_string(),
        };
        if scope_tags.is_some() && is_tag_in_scope_tags.is_some() {
            return Err(Error::BadRequest(format!(
                "Tag {tag} is not available in your scope"
            )));
        }

        if *TAGS_ARE_SENSITIVE {
            return Err(Error::BadRequest(format!("{tag} is not available to you")));
        } else if let Some((name, confining)) = confined_by {
            let overridden = custom_tags_per_w
                .global
                .iter()
                .filter(|entry| admits(entry))
                .cloned()
                .chain(
                    scoped
                        .iter()
                        .filter(|(_, t)| t.applies_to_workspace(&chain))
                        .map(|(n, t)| t.authored(n)),
                )
                .collect::<Vec<_>>();
            let overridden = if overridden.is_empty() {
                String::new()
            } else {
                format!(
                    ", which takes precedence over the custom tags it also fits: {}",
                    overridden.join(", ")
                )
            };
            return Err(Error::BadRequest(format!(
                "Tag {tag} is not available in workspace {w_id}: the custom tag {} restricts it \
                 to other workspaces{overridden}",
                confining.authored(name)
            )));
        } else {
            return Err(error::Error::BadRequest(format!(
            "Tag {tag} is not included in the allowed CUSTOM_TAGS, which only super admins can go beyond: {:?}",
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
    /// The caller's recorded end user: `job_perms.end_user_email` of the calling job.
    pub end_user_email: Option<String>,
    /// `job_perms.viewer_data_access` of the calling job: the code is the caller's own, so its
    /// data table connections are made the way the caller's are.
    pub viewer_data_access: bool,
    pub base_internal_url: String,
    pub worker_name: String,
    pub conn: crate::worker::Connection,
    pub client: AuthedClient,
    pub worker_dir: String,
    pub killpill_rx: tokio::sync::broadcast::Receiver<()>,
}

pub enum InlineScriptTarget {
    /// A script addressed by path, pinned to the version the caller resolved and checked.
    Path {
        path: String,
        hash: i64,
    },
    Hash(i64),
}

pub struct RunInlineScriptFnParams {
    pub workspace_id: String,
    pub target: InlineScriptTarget,
    pub args: Option<HashMap<String, Box<RawValue>>>,
    pub created_by: String,
    pub permissioned_as: String,
    pub permissioned_as_email: String,
    /// The caller's recorded end user: `job_perms.end_user_email` of the calling job.
    pub end_user_email: Option<String>,
    pub base_internal_url: String,
    pub worker_name: String,
    pub conn: crate::worker::Connection,
    pub client: AuthedClient,
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
/// **Transaction contract:** call this inside a transaction. The conversation cleanup below
/// locks rows to serialise itself against a concurrent delete, and on an autocommit
/// connection that lock is released at statement end, silently restoring the race.
/// A message row goes with the flow run its turn started (`turn_job_id`), not with the step
/// or tool job in `job_id`: those complete, and so expire, before the run, and deleting by
/// them would drop a turn's answer while keeping its question. `job_id` is the fallback for
/// rows written by a worker that predates `turn_job_id`. A conversation is collected once
/// every message row of it has gone.
pub async fn delete_jobs(conn: &mut sqlx::PgConnection, ids: &[uuid::Uuid]) -> error::Result<()> {
    sqlx::query!(
        "DELETE FROM dispatch_event WHERE producer_job_id = ANY($1)",
        ids
    )
    .execute(&mut *conn)
    .await?;
    let mut conversation_ids: Vec<uuid::Uuid> = sqlx::query_scalar!(
        "DELETE FROM flow_conversation_message WHERE COALESCE(turn_job_id, job_id) = ANY($1)
         RETURNING conversation_id",
        ids
    )
    .fetch_all(&mut *conn)
    .await?;
    conversation_ids.sort_unstable();
    conversation_ids.dedup();
    if !conversation_ids.is_empty() {
        // A conversation is a view over its messages: once the last one goes with its job,
        // the row and the agent's memory for it are all that is left, and nothing else
        // collects them — `ai_agent_memory` carries no job id for retention to match on.
        // Two statements rather than one CTE: a data-modifying CTE reads the snapshot from
        // before the delete above, so every conversation would still look non-empty.
        // Two calls each deleting one of a conversation's last messages would each still see
        // the other's row — uncommitted deletes are invisible across transactions — so
        // neither would collect it and nothing would try again. Taking the conversation row
        // first serialises them: the second reads the first's delete and finds it empty.
        sqlx::query_scalar!(
            "SELECT id FROM flow_conversation WHERE id = ANY($1) ORDER BY id FOR UPDATE",
            &conversation_ids
        )
        .fetch_all(&mut *conn)
        .await?;
        // Memory first, since it reads the conversation row for its workspace.
        sqlx::query!(
            "DELETE FROM ai_agent_memory a
               USING flow_conversation c
              WHERE c.id = ANY($1)
                AND a.conversation_id = c.id
                AND a.workspace_id = c.workspace_id
                AND NOT EXISTS (
                    SELECT 1 FROM flow_conversation_message m WHERE m.conversation_id = c.id
                )",
            &conversation_ids
        )
        .execute(&mut *conn)
        .await?;
        sqlx::query!(
            "DELETE FROM flow_conversation c
              WHERE c.id = ANY($1)
                AND NOT EXISTS (
                    SELECT 1 FROM flow_conversation_message m WHERE m.conversation_id = c.id
                )",
            &conversation_ids
        )
        .execute(&mut *conn)
        .await?;
    }
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
mod reserved_args_tests {
    use super::{normalize_nul_in_args, sanitize_untrusted_args};
    use serde_json::value::RawValue;
    use std::collections::HashMap;

    fn raw(json: &str) -> Box<RawValue> {
        RawValue::from_string(json.to_string()).unwrap()
    }

    fn args(entries: &[(&str, &str)]) -> HashMap<String, Box<RawValue>> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), raw(v)))
            .collect()
    }

    fn as_json(args: &HashMap<String, Box<RawValue>>) -> serde_json::Value {
        serde_json::to_value(args).unwrap()
    }

    /// A clean map must come out byte for byte the same: this runs on every push.
    #[test]
    fn normalization_leaves_clean_args_alone() {
        let mut map = args(&[
            ("tenant", r#""acme""#),
            // A legitimately escaped backslash followed by the literal text `u0000`
            // (minified JS regexes contain these) is not a NUL and must survive.
            ("src", r#""[^\\u0000]+""#),
        ]);
        let before = as_json(&map);
        normalize_nul_in_args(&mut map);
        assert_eq!(as_json(&map), before);
    }

    /// `push` stores the args with their NUL escapes removed, names included, so a name
    /// that only differs by NULs is the same name and must be compared as such.
    #[test]
    fn normalization_renames_nul_spelled_names_and_cleans_values() {
        let mut map = args(&[
            ("_MODULES\u{0}", r#"{"m.py": "evil"}"#),
            ("_MOD\u{0}ULES", r#"{"m.py": "evil"}"#),
            ("greeting\u{0}", r#""anything""#),
            ("note", r#""a\u0000b""#),
        ]);
        normalize_nul_in_args(&mut map);
        let mut names = map.keys().cloned().collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, vec!["_MODULES", "greeting", "note"]);
        assert_eq!(map["note"].get(), r#""ab""#);
    }

    /// Which of two names that collide once normalized survives may not depend on
    /// `HashMap` iteration order: the spelling the publisher/server used wins.
    #[test]
    fn normalization_keeps_the_nul_free_spelling_on_collision() {
        for _ in 0..16 {
            let mut map = args(&[
                ("tenant", r#""acme""#),
                ("tenant\u{0}", r#""globex""#),
                ("\u{0}tenant", r#""initech""#),
            ]);
            normalize_nul_in_args(&mut map);
            assert_eq!(map.len(), 1);
            assert_eq!(map["tenant"].get(), r#""acme""#);
        }
    }

    /// Every reserved control key goes, in every spelling, and nothing else does.
    #[test]
    fn untrusted_args_lose_every_reserved_control_key() {
        let mut map = args(&[
            ("_ENTRYPOINT_OVERRIDE", r#""apply""#),
            ("_ENTRYPOINT_OVERRIDE\u{0}", r#""apply""#),
            ("_ENTRY\u{0}POINT_OVERRIDE", r#""apply""#),
            ("_MODULES", r#"{"m.py": "evil"}"#),
            ("_MODULES\u{0}", r#"{"m.py": "evil"}"#),
            ("_TEMP_SCRIPT_REFS", r#"{"f/a/b": "deadbeef"}"#),
            ("_TEMP_SCRIPT_REFS\u{0}", r#"{"f/a/b": "deadbeef"}"#),
            // Neighbours that must not be caught by a prefix/contains match.
            ("_TEMP_SCRIPT_REFS_", r#"1"#),
            ("entrypoint", r#""main""#),
            ("action", r#""refund""#),
        ]);
        sanitize_untrusted_args(&mut map);
        let mut names = map.keys().cloned().collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, vec!["_TEMP_SCRIPT_REFS_", "action", "entrypoint"]);
    }
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
