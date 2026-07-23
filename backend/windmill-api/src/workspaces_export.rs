/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use crate::db::ApiAuthed;

use crate::{apps::AppWithLastVersion, db::DB, folders::Folder};

use windmill_api_auth::check_scopes;

#[cfg(any(
    feature = "http_trigger",
    feature = "websocket",
    feature = "postgres_trigger",
    feature = "mqtt_trigger",
    feature = "amqp_trigger",
    all(
        feature = "enterprise",
        any(
            feature = "kafka",
            feature = "sqs_trigger",
            feature = "gcp_trigger",
            feature = "azure_trigger",
            feature = "nats",
            feature = "smtp",
        ),
        feature = "private"
    )
))]
use crate::triggers::TriggerCrud;

use axum::{
    extract::{Extension, Path, Query},
    response::IntoResponse,
};

use http::HeaderName;
use itertools::Itertools;

use windmill_common::runnable_settings::{ConcurrencySettings, DebouncingSettings};
use windmill_common::scripts::ScriptRunnableSettingsHandle;
use windmill_common::utils::require_admin;
use windmill_common::variables::decrypt;
use windmill_common::worker::WINDMILL_DIR;
use windmill_common::{
    db::UserDB,
    error::{to_anyhow, Error, Result},
    flows::Flow,
    schedule::Schedule,
    scripts::{Schema, Script, ScriptLang},
    variables::{build_crypt, ExportableListableVariable},
    workspace_dependencies::WorkspaceDependencies,
};

use hyper::header;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::TempDir;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use windmill_store::resources::{Resource, ResourceType};

#[derive(Serialize)]
struct ScriptMetadata {
    summary: String,
    description: String,
    schema: Option<Schema>,
    lock: Option<String>,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    envs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_ttl: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dedicated_worker: Option<bool>,
    #[serde(skip_serializing_if = "is_none_or_false")]
    ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_after_secs: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_unless_cancelled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_to_runner_only: Option<bool>,
    // auto_kind is intentionally excluded from export — it is auto-detected by the
    // parser at deploy time from the script content (workflow/task patterns for "wac",
    // no main function for "lib").
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    pub auto_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_preprocessor: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_behalf_of_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modules: Option<std::collections::HashMap<String, windmill_common::scripts::ScriptModule>>,
    #[serde(flatten)]
    pub concurrency_settings: ConcurrencySettings,
    #[serde(flatten)]
    pub debouncing_settings: DebouncingSettings,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "is_empty_extra_perms")]
    pub extra_perms: serde_json::Value,
}

fn is_empty_extra_perms(value: &serde_json::Value) -> bool {
    value.as_object().is_some_and(|o| o.is_empty()) || value.is_null()
}

pub fn is_none_or_false(val: &Option<bool>) -> bool {
    match val {
        Some(val) => !val,
        None => true,
    }
}

/// A fork's git-sync export rewrites each trigger's `mode` (and each schedule's
/// `enabled`) to the *parent* workspace's value, instead of emitting the fork's
/// own (clone-disabled / locally-toggled) state. This keeps the fork's synced
/// file byte-identical to the parent on the operational-state field, so a
/// normal-git PR merge has nothing to resolve — no dropped `mode:` line, no
/// flipped parent trigger. Fork-only paths (absent from the parent) keep the
/// fork's own value: there's no parent state to defer to, so the trigger lands
/// with whatever the fork creator set. The write half of the same rule lives in
/// `windmill-trigger::handler::workspace_is_fork`.
///
/// Maps trigger `path` → parent `mode` (as the lowercase enum text that matches
/// `TriggerMode`'s serde representation). Empty when not a fork.
#[cfg(any(
    feature = "http_trigger",
    feature = "websocket",
    feature = "postgres_trigger",
    feature = "mqtt_trigger",
    feature = "amqp_trigger",
    feature = "native_trigger",
    all(
        feature = "enterprise",
        any(
            feature = "kafka",
            feature = "sqs_trigger",
            feature = "gcp_trigger",
            feature = "azure_trigger",
            feature = "nats",
            feature = "smtp",
        ),
        feature = "private"
    )
))]
async fn fork_parent_trigger_modes(
    db: &DB,
    table_name: &str,
    parent_workspace_id: Option<&str>,
) -> Result<HashMap<String, String>> {
    let Some(parent) = parent_workspace_id else {
        return Ok(HashMap::new());
    };
    // Read the parent's rows on the non-RLS pool (like `workspace_is_fork`): the
    // substitution must be complete regardless of the exporter's folder perms,
    // otherwise a parent path the exporter can't read would fall back to the
    // fork's own value and silently re-introduce the divergence we're fixing.
    // No leak: only values for paths the fork already has (it's a clone) are used.
    // SAFETY: `table_name` is a compile-time `TriggerCrud::TABLE_NAME` constant.
    let rows: Vec<(String, String)> = sqlx::query_as(&format!(
        "SELECT path, mode::text FROM {} WHERE workspace_id = $1",
        table_name
    ))
    .bind(parent)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().collect())
}

/// Build the `{ "mode": <parent value> }` override for a single trigger, or
/// `None` (keep the fork's own value) when the path is fork-only.
#[cfg(any(
    feature = "http_trigger",
    feature = "websocket",
    feature = "postgres_trigger",
    feature = "mqtt_trigger",
    feature = "amqp_trigger",
    feature = "native_trigger",
    all(
        feature = "enterprise",
        any(
            feature = "kafka",
            feature = "sqs_trigger",
            feature = "gcp_trigger",
            feature = "azure_trigger",
            feature = "nats",
            feature = "smtp",
        ),
        feature = "private"
    )
))]
fn trigger_mode_override(
    parent_modes: &HashMap<String, String>,
    path: &str,
) -> Option<serde_json::Map<String, Value>> {
    parent_modes.get(path).map(|mode| {
        let mut o = serde_json::Map::new();
        o.insert("mode".to_string(), Value::String(mode.clone()));
        o
    })
}

/// Schedule analog of [`fork_parent_trigger_modes`]: maps schedule `path` →
/// parent `enabled`. Empty when not a fork.
async fn fork_parent_schedule_enabled(
    db: &DB,
    parent_workspace_id: Option<&str>,
) -> Result<HashMap<String, bool>> {
    let Some(parent) = parent_workspace_id else {
        return Ok(HashMap::new());
    };
    // Non-RLS pool, same rationale as `fork_parent_trigger_modes`.
    let rows: Vec<(String, bool)> =
        sqlx::query_as("SELECT path, enabled FROM schedule WHERE workspace_id = $1")
            .bind(parent)
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().collect())
}

enum ArchiveImpl {
    #[cfg(feature = "zip")]
    Zip(async_zip::tokio::write::ZipFileWriter<tokio::fs::File>),
    Tar(tokio_tar::Builder<File>),
}

impl ArchiveImpl {
    async fn write_to_archive(&mut self, content: &str, path: &str) -> Result<()> {
        match self {
            ArchiveImpl::Tar(t) => {
                let bytes = content.as_bytes();
                let mut header = tokio_tar::Header::new_gnu();
                header.set_size(bytes.len() as u64);
                header.set_mtime(0);
                header.set_uid(0);
                header.set_gid(0);
                header.set_mode(0o777);
                header.set_cksum();
                t.append_data(&mut header, path, bytes).await?;
            }
            #[cfg(feature = "zip")]
            ArchiveImpl::Zip(z) => {
                let header =
                    async_zip::ZipEntryBuilder::new(path.into(), async_zip::Compression::Deflate)
                        .last_modification_date(Default::default())
                        .unix_permissions(0o777)
                        .build();
                z.write_entry_whole(header, content.as_bytes())
                    .await
                    .map_err(to_anyhow)?;
            }
        }
        Ok(())
    }
    async fn finish(self) -> Result<()> {
        match self {
            ArchiveImpl::Tar(t) => t.into_inner().await?,
            #[cfg(feature = "zip")]
            ArchiveImpl::Zip(z) => z.close().await.map_err(to_anyhow)?.into_inner(),
        }
        .sync_all()
        .await?;

        Ok(())
    }
}

#[derive(Deserialize)]
pub(crate) struct ArchiveQueryParams {
    archive_type: Option<String>,
    plain_secret: Option<bool>,
    plain_secrets: Option<bool>,
    skip_secrets: Option<bool>,
    skip_variables: Option<bool>,
    skip_resources: Option<bool>,
    skip_resource_types: Option<bool>,
    include_schedules: Option<bool>,
    include_triggers: Option<bool>,
    include_users: Option<bool>,
    include_groups: Option<bool>,
    include_settings: Option<bool>,
    include_key: Option<bool>,
    include_workspace_dependencies: Option<bool>,
    default_ts: Option<String>,
    /// Settings format version: "v1" (default) returns legacy flat format, "v2" returns grouped format
    settings_version: Option<String>,
    /// Opt-in: include `extra_perms` on flow / script / app rows. Default `false`
    /// so cross-workspace tarball imports do not carry over ACLs referring to
    /// identities that may not exist in the target workspace. `wmill sync pull`
    /// passes `true` to surface ACLs in the git-tracked yaml.
    preserve_extra_perms: Option<bool>,
}

/// How to handle `extra_perms` in the serialized output.
///
/// * `Drop`           — strip the field unconditionally (legacy behavior for
///                      types that have never carried ACLs in source).
/// * `KeepEvenEmpty`  — always keep the field, even when `{}`. Matches the
///                      pre-existing serialization for folders and groups so
///                      no customer sees a one-time noisy diff on upgrade.
/// * `KeepIfNonEmpty` — keep when there is at least one entry, drop when `{}`
///                      or null. New surface for flow / script / app, which
///                      never carried ACLs in source before this change.
#[derive(Clone, Copy)]
pub enum ExtraPermsBehavior {
    Drop,
    KeepEvenEmpty,
    KeepIfNonEmpty,
}

#[inline]
pub fn to_string_without_metadata<T>(
    value: &T,
    extra_perms: ExtraPermsBehavior,
    ignore_keys: Option<Vec<&str>>,
) -> Result<String>
where
    T: ?Sized + Serialize,
{
    to_string_without_metadata_inner(value, extra_perms, ignore_keys, None)
}

/// Like [`to_string_without_metadata`] but additionally lets the caller
/// override top-level keys after stripping. Used for fork trigger/schedule
/// exports, where `mode`/`enabled` is rewritten to the *parent* workspace's
/// value so the fork's synced file is byte-identical to the parent on those
/// fields — a clean 3-way git merge instead of a dropped line. See the write
/// half of the rule in `windmill-trigger::handler::workspace_is_fork`.
#[inline]
pub fn to_string_without_metadata_inner<T>(
    value: &T,
    extra_perms: ExtraPermsBehavior,
    ignore_keys: Option<Vec<&str>>,
    overrides: Option<&serde_json::Map<String, Value>>,
) -> Result<String>
where
    T: ?Sized + Serialize,
{
    let mut value = serde_json::to_value(value).map_err(to_anyhow)?;
    value
        .as_object_mut()
        .map(|obj| {
            let keys = [
                vec![
                    "workspace_id",
                    "path",
                    "name",
                    "versions",
                    "id",
                    "created_at",
                    "updated_at",
                    "created_by",
                    "updated_by",
                    "edited_at",
                    "edited_by",
                    "permissioned_as",
                    "archived",
                    "error",
                    "last_server_ping",
                    "server_id",
                    "raw_app",
                ],
                ignore_keys.unwrap_or(vec![]),
            ]
            .concat();

            for key in keys {
                if obj.contains_key(key) {
                    obj.remove(key);
                }
            }

            if let Some(o2) = obj.get_mut("policy").and_then(|x| x.as_object_mut()) {
                o2.remove("on_behalf_of");
                o2.remove("on_behalf_of_email");
            }
            if obj.contains_key("extra_perms") {
                let is_empty_extra_perms = obj
                    .get("extra_perms")
                    .map(|v| v.as_object().is_some_and(|o| o.is_empty()) || v.is_null())
                    .unwrap_or(true);
                let drop = match extra_perms {
                    ExtraPermsBehavior::Drop => true,
                    ExtraPermsBehavior::KeepEvenEmpty => false,
                    ExtraPermsBehavior::KeepIfNonEmpty => is_empty_extra_perms,
                };
                if drop {
                    obj.remove("extra_perms");
                }
            }
            if obj
                .get("default_permissioned_as")
                .and_then(|v| v.as_array())
                .is_some_and(|a| a.is_empty())
            {
                obj.remove("default_permissioned_as");
            }

            if let Some(overrides) = overrides {
                for (k, v) in overrides {
                    obj.insert(k.clone(), v.clone());
                }
            }

            serde_json::to_string_pretty(&obj).ok()
        })
        .flatten()
        .ok_or_else(|| Error::BadRequest("Impossible to serialize value".to_string()))
}

#[derive(Serialize)]
struct SimplifiedUser {
    username: String,
    role: String,
    disabled: bool,
    email: String,
}

#[derive(Serialize)]
struct SimplifiedGroup {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    members: Vec<String>,
    admins: Vec<String>,
}

// V2 format: New grouped format
#[derive(Serialize)]
struct SimplifiedSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_invite: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    webhook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deploy_to: Option<String>,
    // Always serialize (including as `null`) so that `wmill sync pull` emits
    // these fields in settings.yaml unconditionally. Makes round-trip
    // bijective: YAML is the source of truth, absence/null = "clear remote",
    // mirroring every other workspace setting.
    error_handler: Option<Value>,
    success_handler: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ai_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    large_file_storage: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    git_sync: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_scripts: Option<Value>,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    mute_critical_alerts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operator_settings: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    datatable: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slack_team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slack_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slack_command_script: Option<String>,
    // Always serialize (see note above on error_handler / success_handler).
    slack_oauth_client_id: Option<String>,
    slack_oauth_client_secret: Option<String>,
}

// V1 format: Legacy flat format for backward compatibility (matches main branch exactly)
#[derive(Serialize)]
struct SimplifiedSettingsLegacy {
    auto_invite_enabled: bool,
    auto_invite_as: String,
    auto_invite_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    webhook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deploy_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_handler: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_handler_extra_args: Option<Value>,
    error_handler_muted_on_cancel: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    ai_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    large_file_storage: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    git_sync: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_scripts: Option<Value>,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    mute_critical_alerts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operator_settings: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    datatable: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slack_team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slack_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slack_command_script: Option<String>,
}

// Internal struct for querying database
#[derive(sqlx::FromRow)]
struct SettingsRow {
    auto_invite: Option<Value>,
    webhook: Option<String>,
    deploy_to: Option<String>,
    error_handler: Option<Value>,
    success_handler: Option<Value>,
    ai_config: Option<serde_json::Value>,
    large_file_storage: Option<Value>,
    git_sync: Option<Value>,
    default_app: Option<String>,
    default_scripts: Option<Value>,
    name: Option<String>,
    mute_critical_alerts: Option<bool>,
    color: Option<String>,
    operator_settings: Option<serde_json::Value>,
    datatable: Option<Value>,
    slack_team_id: Option<String>,
    slack_name: Option<String>,
    slack_command_script: Option<String>,
    slack_oauth_client_id: Option<String>,
    slack_oauth_client_secret: Option<String>,
}

pub(crate) async fn tarball_workspace(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(ArchiveQueryParams {
        archive_type,
        plain_secret,
        plain_secrets,
        skip_resources,
        skip_resource_types,
        skip_secrets,
        skip_variables,
        include_schedules,
        include_triggers,
        include_users,
        include_groups,
        include_settings,
        include_key,
        include_workspace_dependencies,
        default_ts,
        settings_version,
        preserve_extra_perms,
    }): Query<ArchiveQueryParams>,
) -> Result<([(HeaderName, String); 2], impl IntoResponse)> {
    tracing::info!(
        "tarball_workspace called for workspace {}: include_workspace_dependencies={:?}, skip_variables={:?}, skip_resources={:?}",
        w_id,
        include_workspace_dependencies,
        skip_variables,
        skip_resources
    );

    // The route is gated by workspaces:read, but exporting DECRYPTED secrets is a
    // variable-read capability beyond workspace metadata. Require variables:read
    // only on the plaintext-secret path: ordinary tarball pulls (structure and
    // encrypted-only values) keep working with workspaces:read, and the workspace
    // key itself stays admin-only (include_key). No-op for unscoped tokens.
    if plain_secret.or(plain_secrets).unwrap_or(false)
        && !skip_secrets.unwrap_or(false)
        && !skip_variables.unwrap_or(false)
    {
        check_scopes(&authed, || "variables:read".to_string())?;
    }

    // Opt-in behavior for surfacing per-resource ACLs on flow/app rows.
    // Folder and group rows have always carried `extra_perms` in source and
    // continue to do so unconditionally (`KeepEvenEmpty`) so existing
    // customer git repos see no one-time noisy diff.
    let new_kinds_extra_perms = if preserve_extra_perms.unwrap_or(false) {
        ExtraPermsBehavior::KeepIfNonEmpty
    } else {
        ExtraPermsBehavior::Drop
    };

    // Resolve workspace dependencies on the pool *before* opening the RLS
    // transaction: fetching them mid-transaction would hold a second
    // simultaneous connection while `tx` is still checked out.
    let workspace_dependencies = if include_workspace_dependencies.unwrap_or(false)
        && require_admin(authed.is_admin, &authed.username).is_ok()
    {
        Some(WorkspaceDependencies::list(&w_id, &db).await?)
    } else {
        None
    };

    let mut tx = user_db.begin(&authed).await?;

    // Exporting decrypted secrets in bulk is the same capability as a per-item
    // secret read, so record it for parity with variables.decrypt_secret.
    if plain_secret.or(plain_secrets).unwrap_or(false)
        && !skip_variables.unwrap_or(false)
        && !skip_secrets.unwrap_or(false)
    {
        windmill_audit::audit_oss::audit_log(
            &mut *tx,
            &authed,
            "variables.decrypt_secret",
            windmill_audit::ActionKind::Execute,
            &w_id,
            Some("workspace_tarball_export"),
            None,
        )
        .await?;
    }

    // Source-of-truth for fork-ness: the workspace's parent_workspace_id column.
    // The wm-fork-* prefix is a creation-time naming convention that could in
    // principle drift (rename, manual SQL); the column is the contract that
    // matches what the conflict-warning gates read. The id is also the workspace
    // whose trigger `mode` / schedule `enabled` a fork export defers to.
    let parent_workspace_id: Option<String> = sqlx::query_scalar::<_, Option<String>>(
        "SELECT parent_workspace_id FROM workspace WHERE id = $1",
    )
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    let tmp_dir = TempDir::new_in(&*WINDMILL_DIR)?;

    let name = match archive_type.as_deref() {
        Some("tar") | None => Ok(format!("windmill-{w_id}.tar")),
        Some("zip") => Ok(format!("windmill-{w_id}.zip")),
        Some(t) => Err(Error::BadRequest(format!("Invalid Archive Type {t}"))),
    }?;
    let file_path = tmp_dir.path().join(&name);
    let mut archive = match archive_type.as_deref() {
        Some("tar") | None => {
            let file = File::create(&file_path).await?;
            Ok(ArchiveImpl::Tar(tokio_tar::Builder::new(file)))
        }
        #[cfg(feature = "zip")]
        Some("zip") => {
            let file = tokio::fs::File::create(&file_path).await?;
            Ok(ArchiveImpl::Zip(
                async_zip::tokio::write::ZipFileWriter::with_tokio(file),
            ))
        }
        Some(t) => Err(Error::BadRequest(format!("Invalid Archive Type {t}"))),
    }?;
    {
        let folders = sqlx::query_as::<_, Folder>("SELECT name, workspace_id, display_name, owners, extra_perms, summary, edited_at, created_by, default_permissioned_as, labels FROM folder WHERE workspace_id = $1")
            .bind(&w_id)
            .fetch_all(&mut *tx)
            .await?;

        for folder in folders {
            archive
                .write_to_archive(
                    &to_string_without_metadata(&folder, ExtraPermsBehavior::KeepEvenEmpty, None)
                        .unwrap(),
                    &format!("f/{}/folder.meta.json", folder.name),
                )
                .await?;
        }
    }

    {
        let scripts = sqlx::query_as::<_, Script<ScriptRunnableSettingsHandle>>(&format!(
            "SELECT {} FROM script as o WHERE workspace_id = $1 AND archived = false
                 AND created_at = (select max(created_at) from script where path = o.path AND \
                  workspace_id = $1)",
            windmill_common::scripts::SCRIPT_COLUMNS,
        ))
        .bind(&w_id)
        .fetch_all(&mut *tx)
        .await?;

        for script in scripts {
            let script = windmill_common::scripts::prefetch_cached_script(script, &db).await?;
            let ext = match script.language {
                ScriptLang::Python3 => "py",
                ScriptLang::Deno => {
                    if default_ts.as_ref().is_some_and(|x| x == "bun") {
                        "deno.ts"
                    } else {
                        "ts"
                    }
                }
                ScriptLang::Go => "go",
                ScriptLang::Bash => "sh",
                ScriptLang::Powershell => "ps1",
                ScriptLang::Postgresql => "pg.sql",
                ScriptLang::Mysql => "my.sql",
                ScriptLang::Bigquery => "bq.sql",
                ScriptLang::Snowflake => "sf.sql",
                ScriptLang::Mssql => "ms.sql",
                ScriptLang::DuckDb => "duckdb.sql",
                ScriptLang::Graphql => "gql",
                ScriptLang::Nativets => "fetch.ts",
                ScriptLang::Bun | ScriptLang::Bunnative => {
                    if default_ts.as_ref().is_some_and(|x| x == "bun") {
                        "ts"
                    } else {
                        "bun.ts"
                    }
                }
                ScriptLang::Php => "php",
                ScriptLang::Rust => "rs",
                ScriptLang::Ansible => "playbook.yml",
                ScriptLang::CSharp => "cs",
                ScriptLang::Nu => "nu",
                ScriptLang::OracleDB => "odb.sql",
                ScriptLang::Java => "java",
                ScriptLang::Ruby => "rb",
                ScriptLang::Rlang => "r",
                // for related places search: ADD_NEW_LANG
            };
            archive
                .write_to_archive(&script.content, &format!("{}.{}", script.path, ext))
                .await?;

            let metadata = ScriptMetadata {
                summary: script.summary,
                description: script.description,
                schema: script.schema,
                kind: script.kind.to_string(),
                lock: script.lock,
                envs: script.envs,
                concurrency_settings: script.runnable_settings.concurrency_settings,
                debouncing_settings: script.runnable_settings.debouncing_settings,
                cache_ttl: script.cache_ttl,
                dedicated_worker: script.dedicated_worker,
                ws_error_handler_muted: script.ws_error_handler_muted,
                priority: script.priority,
                tag: script.tag,
                timeout: script.timeout,
                delete_after_secs: script.delete_after_secs,
                restart_unless_cancelled: script.restart_unless_cancelled,
                visible_to_runner_only: script.visible_to_runner_only,
                auto_kind: script.auto_kind,
                codebase: script.codebase,
                has_preprocessor: script.has_preprocessor,
                on_behalf_of_email: script.on_behalf_of_email,
                modules: script.modules,
                labels: script.labels,
                // Same opt-in contract as flow/app: the tarball only surfaces
                // ACLs when `?preserve_extra_perms=true`. Passing `Null` lets the
                // `is_empty_extra_perms` skip-serializer drop the field entirely.
                extra_perms: if matches!(new_kinds_extra_perms, ExtraPermsBehavior::KeepIfNonEmpty)
                {
                    script.extra_perms
                } else {
                    serde_json::Value::Null
                },
            };
            let metadata_str = serde_json::to_string_pretty(&metadata).unwrap();
            archive
                .write_to_archive(&metadata_str, &format!("{}.script.json", script.path))
                .await?;
        }
    }

    if !skip_resources.unwrap_or(false) {
        let resources = sqlx::query_as!(
             Resource,
             "SELECT workspace_id, path, value, description, resource_type, extra_perms, created_by, edited_at, labels FROM resource WHERE workspace_id = $1 AND resource_type != 'state' AND resource_type != 'cache'",
             &w_id
         )
         .fetch_all(&mut *tx)
         .await?;

        for resource in resources {
            let resource_str =
                &to_string_without_metadata(&resource, ExtraPermsBehavior::Drop, None).unwrap();
            archive
                .write_to_archive(&resource_str, &format!("{}.resource.json", resource.path))
                .await?;
        }
    }

    if !skip_resource_types.unwrap_or(false) {
        let resource_types = sqlx::query_as!(
            ResourceType,
            "SELECT workspace_id, name, schema, description, created_by, edited_at, format_extension, is_fileset FROM resource_type WHERE workspace_id = $1",
            &w_id
        )
        .fetch_all(&mut *tx)
        .await?;

        for resource_type in resource_types {
            let resource_str =
                &to_string_without_metadata(&resource_type, ExtraPermsBehavior::Drop, None)
                    .unwrap();
            archive
                .write_to_archive(
                    &resource_str,
                    &format!("{}.resource-type.json", resource_type.name),
                )
                .await?;
        }
    }

    {
        let flows = sqlx::query_as::<_, Flow>(
             "SELECT flow.workspace_id, flow.path, flow.summary, flow.description, flow.archived, flow.extra_perms, flow.dedicated_worker, flow.tag, flow.ws_error_handler_muted, flow.timeout, flow.visible_to_runner_only, flow.on_behalf_of_email, flow.labels, flow_version.schema, flow_version.value, flow_version.created_at as edited_at, flow_version.created_by as edited_by
             FROM flow
             LEFT JOIN flow_version ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
             WHERE flow.workspace_id = $1 AND flow.archived = false",
         )
         .bind(&w_id)
         .fetch_all(&mut *tx)
         .await?;

        for flow in flows {
            let flow_str = &to_string_without_metadata(&flow, new_kinds_extra_perms, None).unwrap();
            archive
                .write_to_archive(&flow_str, &format!("{}.flow.json", flow.path))
                .await?;
        }
    }

    if !skip_variables.unwrap_or(false) {
        let variables =
             sqlx::query_as::<_, ExportableListableVariable>(if !skip_secrets.unwrap_or(false) {
                 "SELECT workspace_id, path, value, is_secret, description, extra_perms, account, is_oauth, expires_at, labels FROM variable WHERE workspace_id = $1 AND expires_at IS NULL"
             } else {
                 "SELECT workspace_id, path, value, is_secret, description, extra_perms, account, is_oauth, expires_at, labels FROM variable WHERE workspace_id = $1 AND is_secret = false AND expires_at IS NULL"
             })
             .bind(&w_id)
             .fetch_all(&mut *tx)
             .await?;

        let mc = build_crypt(&db, &w_id).await?;

        for mut var in variables {
            if plain_secret.or(plain_secrets).unwrap_or(false)
                && var.value.is_some()
                && var.is_secret
            {
                var.value = Some(decrypt(&mc, var.value.unwrap()).map_err(|e| {
                    Error::internal_err(format!("Error decrypting variable {}: {}", var.path, e))
                })?);
            }
            let var_str =
                &to_string_without_metadata(&var, ExtraPermsBehavior::Drop, None).unwrap();
            archive
                .write_to_archive(&var_str, &format!("{}.variable.json", var.path))
                .await?;
        }
    }

    {
        let apps = sqlx::query_as::<_, AppWithLastVersion>(
             "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
             app.extra_perms, app_version.value,
             app_version.created_at, app_version.created_by, app_version.raw_app, app.labels from app, app_version
             WHERE app.workspace_id = $1 AND app_version.id = app.versions[array_upper(app.versions, 1)]",
         )
         .bind(&w_id)
         .fetch_all(&mut *tx)
         .await?;

        for app in apps {
            let app_str = &to_string_without_metadata(&app, new_kinds_extra_perms, None).unwrap();
            let kind = if app.raw_app { "raw_app" } else { "app" };
            archive
                .write_to_archive(&app_str, &format!("{}.{}.json", app.path, kind))
                .await?;
        }
    }

    if let Some(workspace_dependencies) = workspace_dependencies {
        tracing::info!("Including workspace dependencies in tarball export");
        tracing::info!(
            "Found {} workspace dependencies",
            workspace_dependencies.len()
        );
        for dep in workspace_dependencies {
            // let dep_str = &to_string_without_metadata(&dep, ExtraPermsBehavior::Drop, None).unwrap();
            let filename = WorkspaceDependencies::to_path(&dep.name, dep.language)?;
            tracing::info!(
                "Adding workspace dependency: name={:?}, language={:?}, filename={}",
                dep.name,
                dep.language,
                filename
            );
            archive.write_to_archive(&dep.content, &filename).await?;
        }
    } else {
        tracing::info!(
            "Skipping workspace dependencies: include_workspace_dependencies={:?}",
            include_workspace_dependencies
        );
    }

    if include_schedules.unwrap_or(false) {
        // Managed ducklake-maintenance schedules are excluded: they are
        // derived from the workspace ducklake settings (and admins bypass the
        // RLS that hides them), so exporting them would drag unsyncable rows
        // into git.
        let schedules = sqlx::query_as::<_, Schedule>(
            "SELECT workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, args, extra_perms, email, permissioned_as, error, on_failure, on_failure_times, on_failure_exact, on_failure_extra_args, on_recovery, on_recovery_times, on_recovery_extra_args, on_success, on_success_extra_args, ws_error_handler_muted, retry, no_flow_overlap, summary, description, tag, paused_until, cron_version, dynamic_skip, labels FROM schedule
             WHERE workspace_id = $1 AND NOT starts_with(path, $2)",
        )
        .bind(&w_id)
        .bind(windmill_common::workspaces::DUCKLAKE_MAINTENANCE_PATH_PREFIX)
        .fetch_all(&mut *tx)
        .await?;

        // For a fork, defer each schedule's `enabled` to the parent so the
        // synced file matches the parent and the merge doesn't flip it.
        let parent_enabled =
            fork_parent_schedule_enabled(&db, parent_workspace_id.as_deref()).await?;
        for schedule in schedules {
            let enabled_override = parent_enabled.get(&schedule.path).map(|enabled| {
                let mut o = serde_json::Map::new();
                o.insert("enabled".to_string(), Value::Bool(*enabled));
                o
            });
            let app_str = &to_string_without_metadata_inner(
                &schedule,
                ExtraPermsBehavior::Drop,
                None,
                enabled_override.as_ref(),
            )
            .unwrap();
            archive
                .write_to_archive(&app_str, &format!("{}.schedule.json", schedule.path))
                .await?;
        }
    }

    if include_triggers.unwrap_or(false) {
        #[cfg(feature = "http_trigger")]
        {
            use crate::triggers::http::HttpTrigger;
            let handler = HttpTrigger;
            let http_triggers = handler.list_triggers(&mut *tx, &w_id, None, None).await?;
            let parent_modes = fork_parent_trigger_modes(
                &db,
                <HttpTrigger as TriggerCrud>::TABLE_NAME,
                parent_workspace_id.as_deref(),
            )
            .await?;

            for trigger in http_triggers {
                let mode_override = trigger_mode_override(&parent_modes, &trigger.base.path);
                let trigger_str = &to_string_without_metadata_inner(
                    &trigger,
                    ExtraPermsBehavior::Drop,
                    None,
                    mode_override.as_ref(),
                )
                .unwrap();
                archive
                    .write_to_archive(
                        &trigger_str,
                        &format!("{}.http_trigger.json", trigger.base.path),
                    )
                    .await?;
            }
        }

        #[cfg(feature = "websocket")]
        {
            use crate::triggers::websocket::WebsocketTrigger;
            let handler = WebsocketTrigger;
            let websocket_triggers = handler.list_triggers(&mut *tx, &w_id, None, None).await?;
            let parent_modes = fork_parent_trigger_modes(
                &db,
                <WebsocketTrigger as TriggerCrud>::TABLE_NAME,
                parent_workspace_id.as_deref(),
            )
            .await?;

            for trigger in websocket_triggers {
                let mode_override = trigger_mode_override(&parent_modes, &trigger.base.path);
                let trigger_str = &to_string_without_metadata_inner(
                    &trigger,
                    ExtraPermsBehavior::Drop,
                    None,
                    mode_override.as_ref(),
                )
                .unwrap();
                archive
                    .write_to_archive(
                        &trigger_str,
                        &format!("{}.websocket_trigger.json", trigger.base.path),
                    )
                    .await?;
            }
        }

        #[cfg(all(feature = "enterprise", feature = "kafka", feature = "private"))]
        {
            use crate::triggers::kafka::KafkaTrigger;
            let handler = KafkaTrigger;
            let kafka_triggers = handler.list_triggers(&mut *tx, &w_id, None, None).await?;
            let parent_modes = fork_parent_trigger_modes(
                &db,
                <KafkaTrigger as TriggerCrud>::TABLE_NAME,
                parent_workspace_id.as_deref(),
            )
            .await?;

            for trigger in kafka_triggers {
                let mode_override = trigger_mode_override(&parent_modes, &trigger.base.path);
                let trigger_str = &to_string_without_metadata_inner(
                    &trigger,
                    ExtraPermsBehavior::Drop,
                    None,
                    mode_override.as_ref(),
                )
                .unwrap();
                archive
                    .write_to_archive(
                        &trigger_str,
                        &format!("{}.kafka_trigger.json", trigger.base.path),
                    )
                    .await?;
            }
        }

        #[cfg(all(feature = "enterprise", feature = "sqs_trigger", feature = "private"))]
        {
            use crate::triggers::sqs::SqsTrigger;
            let handler = SqsTrigger;
            let sqs_triggers = handler.list_triggers(&mut *tx, &w_id, None, None).await?;
            let parent_modes = fork_parent_trigger_modes(
                &db,
                <SqsTrigger as TriggerCrud>::TABLE_NAME,
                parent_workspace_id.as_deref(),
            )
            .await?;

            for trigger in sqs_triggers {
                let mode_override = trigger_mode_override(&parent_modes, &trigger.base.path);
                let trigger_str = &to_string_without_metadata_inner(
                    &trigger,
                    ExtraPermsBehavior::Drop,
                    None,
                    mode_override.as_ref(),
                )
                .unwrap();
                archive
                    .write_to_archive(
                        &trigger_str,
                        &format!("{}.sqs_trigger.json", trigger.base.path),
                    )
                    .await?;
            }
        }

        #[cfg(all(feature = "enterprise", feature = "gcp_trigger", feature = "private"))]
        {
            use crate::triggers::gcp::GcpTrigger;
            let handler = GcpTrigger;
            let gcp_triggers = handler.list_triggers(&mut *tx, &w_id, None, None).await?;
            let parent_modes = fork_parent_trigger_modes(
                &db,
                <GcpTrigger as TriggerCrud>::TABLE_NAME,
                parent_workspace_id.as_deref(),
            )
            .await?;

            for trigger in gcp_triggers {
                let mode_override = trigger_mode_override(&parent_modes, &trigger.base.path);
                let trigger_str = &to_string_without_metadata_inner(
                    &trigger,
                    ExtraPermsBehavior::Drop,
                    None,
                    mode_override.as_ref(),
                )
                .unwrap();
                archive
                    .write_to_archive(
                        &trigger_str,
                        &format!("{}.gcp_trigger.json", trigger.base.path),
                    )
                    .await?;
            }
        }

        #[cfg(all(feature = "enterprise", feature = "azure_trigger", feature = "private"))]
        {
            use crate::triggers::azure::AzureTrigger;
            let handler = AzureTrigger;
            let azure_triggers = handler.list_triggers(&mut *tx, &w_id, None, None).await?;
            let parent_modes = fork_parent_trigger_modes(
                &db,
                <AzureTrigger as TriggerCrud>::TABLE_NAME,
                parent_workspace_id.as_deref(),
            )
            .await?;

            for trigger in azure_triggers {
                let mode_override = trigger_mode_override(&parent_modes, &trigger.base.path);
                let trigger_str = &to_string_without_metadata_inner(
                    &trigger,
                    ExtraPermsBehavior::Drop,
                    None,
                    mode_override.as_ref(),
                )
                .unwrap();
                archive
                    .write_to_archive(
                        &trigger_str,
                        &format!("{}.azure_trigger.json", trigger.base.path),
                    )
                    .await?;
            }
        }

        #[cfg(all(feature = "enterprise", feature = "nats", feature = "private"))]
        {
            use crate::triggers::nats::NatsTrigger;
            let handler = NatsTrigger;
            let nats_triggers = handler.list_triggers(&mut *tx, &w_id, None, None).await?;
            let parent_modes = fork_parent_trigger_modes(
                &db,
                <NatsTrigger as TriggerCrud>::TABLE_NAME,
                parent_workspace_id.as_deref(),
            )
            .await?;

            for trigger in nats_triggers {
                let mode_override = trigger_mode_override(&parent_modes, &trigger.base.path);
                let trigger_str: &String = &to_string_without_metadata_inner(
                    &trigger,
                    ExtraPermsBehavior::Drop,
                    None,
                    mode_override.as_ref(),
                )
                .unwrap();
                archive
                    .write_to_archive(
                        &trigger_str,
                        &format!("{}.nats_trigger.json", trigger.base.path),
                    )
                    .await?;
            }
        }

        #[cfg(feature = "postgres_trigger")]
        {
            use crate::triggers::postgres::PostgresTrigger;
            let handler = PostgresTrigger;
            let postgres_triggers = handler.list_triggers(&mut *tx, &w_id, None, None).await?;
            let parent_modes = fork_parent_trigger_modes(
                &db,
                <PostgresTrigger as TriggerCrud>::TABLE_NAME,
                parent_workspace_id.as_deref(),
            )
            .await?;

            for trigger in postgres_triggers {
                let mode_override = trigger_mode_override(&parent_modes, &trigger.base.path);
                let trigger_str = &to_string_without_metadata_inner(
                    &trigger,
                    ExtraPermsBehavior::Drop,
                    None,
                    mode_override.as_ref(),
                )
                .unwrap();
                archive
                    .write_to_archive(
                        &trigger_str,
                        &format!("{}.postgres_trigger.json", trigger.base.path),
                    )
                    .await?;
            }
        }

        #[cfg(feature = "mqtt_trigger")]
        {
            use crate::triggers::mqtt::MqttTrigger;
            let handler = MqttTrigger;
            let mqtt_triggers = handler.list_triggers(&mut *tx, &w_id, None, None).await?;
            let parent_modes = fork_parent_trigger_modes(
                &db,
                <MqttTrigger as TriggerCrud>::TABLE_NAME,
                parent_workspace_id.as_deref(),
            )
            .await?;

            for trigger in mqtt_triggers {
                let mode_override = trigger_mode_override(&parent_modes, &trigger.base.path);
                let trigger_str = &to_string_without_metadata_inner(
                    &trigger,
                    ExtraPermsBehavior::Drop,
                    None,
                    mode_override.as_ref(),
                )
                .unwrap();
                archive
                    .write_to_archive(
                        &trigger_str,
                        &format!("{}.mqtt_trigger.json", trigger.base.path),
                    )
                    .await?;
            }
        }

        #[cfg(feature = "amqp_trigger")]
        {
            use crate::triggers::amqp::AmqpTrigger;
            let handler = AmqpTrigger;
            let amqp_triggers = handler.list_triggers(&mut *tx, &w_id, None, None).await?;
            let parent_modes = fork_parent_trigger_modes(
                &db,
                <AmqpTrigger as TriggerCrud>::TABLE_NAME,
                parent_workspace_id.as_deref(),
            )
            .await?;

            for trigger in amqp_triggers {
                let mode_override = trigger_mode_override(&parent_modes, &trigger.base.path);
                let trigger_str = &to_string_without_metadata_inner(
                    &trigger,
                    ExtraPermsBehavior::Drop,
                    None,
                    mode_override.as_ref(),
                )
                .unwrap();
                archive
                    .write_to_archive(
                        &trigger_str,
                        &format!("{}.amqp_trigger.json", trigger.base.path),
                    )
                    .await?;
            }
        }

        #[cfg(all(feature = "enterprise", feature = "smtp", feature = "private"))]
        {
            use crate::triggers::email::EmailTrigger;
            let handler = EmailTrigger;
            let email_triggers = handler.list_triggers(&mut *tx, &w_id, None, None).await?;
            let parent_modes = fork_parent_trigger_modes(
                &db,
                <EmailTrigger as TriggerCrud>::TABLE_NAME,
                parent_workspace_id.as_deref(),
            )
            .await?;

            for trigger in email_triggers {
                let mode_override = trigger_mode_override(&parent_modes, &trigger.base.path);
                let trigger_str = &to_string_without_metadata_inner(
                    &trigger,
                    ExtraPermsBehavior::Drop,
                    None,
                    mode_override.as_ref(),
                )
                .unwrap();
                archive
                    .write_to_archive(
                        &trigger_str,
                        &format!("{}.email_trigger.json", trigger.base.path),
                    )
                    .await?;
            }
        }

        #[cfg(feature = "native_trigger")]
        {
            use crate::native_triggers::{list_native_triggers, ServiceName};
            use strum::IntoEnumIterator;

            for service_name in ServiceName::iter() {
                let native_triggers =
                    list_native_triggers(&mut *tx, &w_id, service_name, None, None, None, None)
                        .await?;

                // Native triggers (Nextcloud, Google Drive, GitHub) are never
                // cloned into a fork — a fork only has one if its owner created
                // it there, so it's always "fork-only" and keeps its own mode.
                // No parent-value substitution applies; we only strip the
                // webhook token hash.
                let native_ignore_keys = vec!["webhook_token_hash"];

                for trigger in native_triggers {
                    let trigger_str = &to_string_without_metadata(
                        &trigger,
                        ExtraPermsBehavior::Drop,
                        Some(native_ignore_keys.clone()),
                    )
                    .unwrap();
                    archive
                        .write_to_archive(
                            &trigger_str,
                            &format!(
                                "{}.{}.{}.{}_native_trigger.json",
                                trigger.script_path,
                                if trigger.is_flow { "flow" } else { "script" },
                                trigger.external_id,
                                service_name.as_str()
                            ),
                        )
                        .await?;
                }
            }
        }
    }

    if include_users.unwrap_or(false) {
        let users = sqlx::query!(
            "SELECT workspace_id, username, email, is_admin, created_at, operator, disabled, role, added_via FROM usr
             WHERE workspace_id = $1",
            &w_id
        )
        .fetch_all(&mut *tx)
        .await?;

        for user in users {
            let user = SimplifiedUser {
                username: user.username,
                role: if user.is_admin {
                    "admin".to_string()
                } else if user.operator {
                    "operator".to_string()
                } else {
                    "developer".to_string()
                },
                disabled: user.disabled,
                email: user.email,
            };
            let user_str =
                &to_string_without_metadata(&user, ExtraPermsBehavior::Drop, Some(vec!["email"]))
                    .unwrap();
            archive
                .write_to_archive(&user_str, &format!("users/{}.user.json", user.email))
                .await?;
        }
    }

    if include_groups.unwrap_or(false) {
        let groups = sqlx::query!(
             r#"SELECT g_.workspace_id, name, summary, extra_perms, array_agg(u2g.usr) filter (where u2g.usr is not null) as members
             FROM usr u
             JOIN usr_to_group u2g ON u2g.usr = u.username AND u2g.workspace_id = u.workspace_id
             RIGHT JOIN group_ g_ ON g_.workspace_id = u.workspace_id AND g_.name = u2g.group_
             WHERE g_.workspace_id = $1 AND g_.name != 'all'
             GROUP BY g_.workspace_id, name, summary, extra_perms"#,
             &w_id
         )
         .fetch_all(&mut *tx)
         .await?;

        for group in groups {
            let extra_perms: HashMap<String, bool> = serde_json::from_value(group.extra_perms)
                .map_err(|e| {
                    Error::internal_err(format!(
                        "Error parsing extra_perms for group {}: {}",
                        group.name, e
                    ))
                })?;
            tracing::info!("{:?}", extra_perms);
            let members = group.members.unwrap_or(vec![]);
            let admins: Vec<String> = extra_perms
                .iter()
                .filter_map(|(k, v)| {
                    // only consider extra_perms that concern actual members of the group
                    if members.contains(&k[2..].to_string()) && *v {
                        Some(k.clone())
                    } else {
                        None
                    }
                })
                .sorted()
                .collect();
            let group = SimplifiedGroup {
                name: group.name,
                summary: group.summary,
                members: members
                    .iter()
                    .filter_map(|x| {
                        // remove members that are also admins as they are already in the admins list
                        let full_name = format!("u/{}", x);
                        if !admins.contains(&full_name) {
                            Some(full_name)
                        } else {
                            None
                        }
                    })
                    .collect(),
                admins,
            };

            let group_str =
                &to_string_without_metadata(&group, ExtraPermsBehavior::KeepEvenEmpty, None)
                    .unwrap();
            archive
                .write_to_archive(&group_str, &format!("groups/{}.group.json", group.name))
                .await?;
        }
    }

    if include_settings.unwrap_or(false) {
        let row = sqlx::query_as::<_, SettingsRow>(
            r#"SELECT
                 auto_invite,
                 webhook,
                 deploy_to,
                 error_handler,
                 success_handler,
                 ai_config,
                 large_file_storage,
                 git_sync,
                 default_app,
                 default_scripts,
                 workspace.name as name,
                 mute_critical_alerts,
                 color,
                 operator_settings,
                 datatable,
                 slack_team_id,
                 slack_name,
                 slack_command_script,
                 slack_oauth_client_id,
                 slack_oauth_client_secret
             FROM workspace_settings
             LEFT JOIN workspace ON workspace.id = workspace_settings.workspace_id
             WHERE workspace_id = $1"#,
        )
        .bind(&w_id)
        .fetch_one(&mut *tx)
        .await?;

        // Use v2 format only if explicitly requested, otherwise use v1 (legacy) for backward compatibility
        // Server-owned auto-pull state (the HMAC webhook secret + hook id/error and
        // the synced-sha / last-pull status) must never leave the server: keep it out
        // of export archives and synced repos, and don't let a re-imported workspace
        // inherit another install's hook/sync state. Mirrors the GET-settings redaction.
        fn redact_git_sync_for_export(git_sync: Option<Value>) -> Option<Value> {
            let mut git_sync = git_sync?;
            if let Some(repos) = git_sync
                .get_mut("repositories")
                .and_then(|r| r.as_array_mut())
            {
                for repo in repos {
                    if let Some(auto_pull) =
                        repo.get_mut("auto_pull").and_then(|a| a.as_object_mut())
                    {
                        for field in [
                            "webhook_secret",
                            "webhook_id",
                            "webhook_error",
                            "last_synced_sha",
                            "last_pull_status",
                        ] {
                            auto_pull.remove(field);
                        }
                    }
                }
            }
            Some(git_sync)
        }

        let settings_str = if settings_version.as_deref() == Some("v2") {
            let settings = SimplifiedSettings {
                auto_invite: row.auto_invite,
                webhook: row.webhook,
                deploy_to: row.deploy_to,
                error_handler: row.error_handler,
                success_handler: row.success_handler,
                ai_config: row.ai_config,
                large_file_storage: row.large_file_storage,
                git_sync: redact_git_sync_for_export(row.git_sync),
                default_app: row.default_app,
                default_scripts: row.default_scripts,
                name: row.name.clone().unwrap_or_default(),
                mute_critical_alerts: row.mute_critical_alerts,
                color: row.color.clone(),
                operator_settings: row.operator_settings.clone(),
                datatable: row.datatable.clone(),
                slack_team_id: row.slack_team_id.clone(),
                slack_name: row.slack_name.clone(),
                slack_command_script: row.slack_command_script.clone(),
                slack_oauth_client_id: row.slack_oauth_client_id.clone(),
                // Mirror the non-admin redaction in `get_settings`: the OAuth
                // client secret is admin-only and must not leak via tarball.
                slack_oauth_client_secret: if authed.is_admin {
                    row.slack_oauth_client_secret.clone()
                } else {
                    None
                },
            };
            serde_json::to_value(settings)
                .map(|v| serde_json::to_string_pretty(&v).ok())
                .ok()
                .flatten()
        } else {
            // V1 (legacy) format: convert JSONB to flat fields (matches main branch exactly)
            let (auto_invite_enabled, auto_invite_as, auto_invite_mode) =
                if let Some(ref ai) = row.auto_invite {
                    let enabled = ai.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                    let operator = ai
                        .get("operator")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let mode = ai.get("mode").and_then(|v| v.as_str()).unwrap_or("invite");
                    (
                        enabled,
                        if operator {
                            "operator".to_string()
                        } else {
                            "developer".to_string()
                        },
                        mode.to_string(),
                    )
                } else {
                    (false, "developer".to_string(), "invite".to_string())
                };

            let (error_handler, error_handler_extra_args, error_handler_muted_on_cancel) =
                if let Some(ref eh) = row.error_handler {
                    let path = eh.get("path").and_then(|v| v.as_str()).map(String::from);
                    let extra_args = eh.get("extra_args").cloned();
                    let muted_on_cancel = eh
                        .get("muted_on_cancel")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    (path, extra_args, muted_on_cancel)
                } else {
                    (None, None, false)
                };

            let settings = SimplifiedSettingsLegacy {
                auto_invite_enabled,
                auto_invite_as,
                auto_invite_mode,
                webhook: row.webhook,
                deploy_to: row.deploy_to,
                error_handler,
                error_handler_extra_args,
                error_handler_muted_on_cancel,
                ai_config: row.ai_config,
                large_file_storage: row.large_file_storage,
                git_sync: redact_git_sync_for_export(row.git_sync),
                default_app: row.default_app,
                default_scripts: row.default_scripts,
                name: row.name.unwrap_or_default(),
                mute_critical_alerts: row.mute_critical_alerts,
                color: row.color,
                operator_settings: row.operator_settings,
                datatable: row.datatable,
                slack_team_id: row.slack_team_id,
                slack_name: row.slack_name,
                slack_command_script: row.slack_command_script,
            };
            serde_json::to_value(settings)
                .map(|v| serde_json::to_string_pretty(&v).ok())
                .ok()
                .flatten()
        }
        .ok_or_else(|| Error::internal_err("Error serializing settings".to_string()))?;

        archive
            .write_to_archive(&settings_str, "settings.json")
            .await?;
    }

    if include_key.unwrap_or(false) {
        require_admin(authed.is_admin, &authed.username)?;

        let key = sqlx::query_scalar!(
            "SELECT key FROM workspace_key WHERE workspace_id = $1",
            &w_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let key_json = serde_json::to_value(key)
            .map(|v| serde_json::to_string_pretty(&v).ok())
            .ok()
            .flatten()
            .ok_or_else(|| Error::internal_err("Error serializing enryption key".to_string()))?;
        archive
            .write_to_archive(&key_json, "encryption_key.json")
            .await?;
    }

    {
        // Data table migrations live in the `datatable_migrations` table; surface
        // them in the export as `migrations/datatable/<datatable>/<version>_<name>`
        // .up.sql (and .down.sql when present) so `wmill sync` treats them like any
        // other workspace item.
        let migrations = sqlx::query!(
            "SELECT datatable, timestamp, name, code_up, code_down FROM datatable_migrations \
             WHERE workspace_id = $1 ORDER BY datatable, timestamp",
            &w_id
        )
        .fetch_all(&mut *tx)
        .await?;
        for m in migrations {
            let base = format!(
                "migrations/datatable/{}/{}_{}",
                m.datatable, m.timestamp, m.name
            );
            archive
                .write_to_archive(&m.code_up, &format!("{base}.up.sql"))
                .await?;
            if let Some(code_down) = m.code_down {
                archive
                    .write_to_archive(&code_down, &format!("{base}.down.sql"))
                    .await?;
            }
        }
    }

    archive.finish().await?;

    let file = tokio::fs::File::open(&file_path).await?;

    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    let headers = [
        (header::CONTENT_TYPE, "application/x-tar".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{name}\""),
        ),
    ];
    Ok((headers, body))
}

#[cfg(test)]
mod fork_export_tests {
    use super::*;
    use serde_json::json;

    /// A fork export rewrites `mode` to the parent's value: the serialized file
    /// carries the parent's state (`enabled`), not the fork's clone-disabled DB
    /// value — so a normal-git merge sees no change on that line.
    #[test]
    fn override_substitutes_parent_mode() {
        let fork_trigger = json!({
            "path": "f/triggers/x",
            "script_path": "f/scripts/x",
            "mode": "disabled", // fork's local (clone-disabled) state
            "is_flow": false,
        });
        let mut overrides = serde_json::Map::new();
        overrides.insert("mode".to_string(), Value::String("enabled".to_string()));

        let out = to_string_without_metadata_inner(
            &fork_trigger,
            ExtraPermsBehavior::Drop,
            None,
            Some(&overrides),
        )
        .unwrap();
        let parsed: Value = serde_json::from_str(&out).unwrap();

        assert_eq!(parsed["mode"], json!("enabled"), "parent mode substituted");
        // `path` is in the metadata strip list, so it should be removed.
        assert!(parsed.get("path").is_none());
    }

    /// A fork-only trigger (no parent counterpart, so no override) keeps the
    /// fork creator's chosen state.
    #[test]
    fn no_override_keeps_fork_value() {
        let fork_only = json!({ "mode": "enabled", "script_path": "f/scripts/x" });
        let out =
            to_string_without_metadata_inner(&fork_only, ExtraPermsBehavior::Drop, None, None)
                .unwrap();
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["mode"], json!("enabled"));
    }

    /// `trigger_mode_override` builds an override only when the parent has the
    /// path; fork-only paths return `None` (keep the fork's own value).
    #[cfg(feature = "http_trigger")]
    #[test]
    fn trigger_mode_override_defers_to_parent_or_self() {
        let mut parent_modes = HashMap::new();
        parent_modes.insert("f/triggers/shared".to_string(), "enabled".to_string());

        let shared = trigger_mode_override(&parent_modes, "f/triggers/shared");
        assert_eq!(
            shared.as_ref().and_then(|o| o.get("mode")),
            Some(&Value::String("enabled".to_string())),
        );

        // Fork-only path: no parent entry → no override → keep fork's own value.
        assert!(trigger_mode_override(&parent_modes, "f/triggers/fork_only").is_none());
    }
}
