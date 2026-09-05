use std::{borrow::Cow, collections::HashMap, sync::Arc};

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
#[cfg(all(feature = "parquet", not(feature = "enterprise")))]
use crate::job_helpers_oss::{
    bump_storage_usage, ce_upload_budget, reject_reserved_volume_key,
    spawn_storage_usage_recount_floored,
};
use crate::{
    apps_raw_bundle,
    auth::{get_end_user_email, AuthCache, OptTokened},
    db::{ApiAuthed, DB},
    jobs::RunJobQuery,
    users::{require_owner_of_path, require_path_read_access_for_preview, OptAuthed},
    utils::{build_scope_path_predicate, check_scopes},
    webhook_util::{WebhookMessage, WebhookShared},
    HTTP_CLIENT,
};
#[cfg(feature = "parquet")]
use crate::{
    job_helpers_oss::{
        download_s3_file_internal, get_large_file_storage, get_random_file_name, get_s3_resource,
        get_workspace_s3_resource_and_check_paths, upload_file_from_req, DownloadFileQuery,
        LoadCountQuery, LoadFileMetadataQuery, LoadFilePreviewQuery, LoadPreviewQuery,
    },
    users::fetch_api_authed_from_permissioned_as,
};
use axum::response::Response;
use axum::{
    body::Body,
    extract::{Extension, Json, Multipart, Path, Query},
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use futures::future::{FutureExt, TryFutureExt};
use hyper::StatusCode;
#[cfg(feature = "parquet")]
use itertools::Itertools;
use lazy_static::lazy_static;
use magic_crypt::MagicCryptTrait;
#[cfg(feature = "parquet")]
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};
use sha2::{Digest, Sha256};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{types::Uuid, FromRow};
use std::str;
use windmill_audit::audit_oss::{audit_log, AuditAuthorable};
use windmill_audit::ActionKind;
use windmill_common::{
    apps::{AppScriptId, ListAppQuery, APP_WORKSPACED_ROUTE},
    auth::TOKEN_PREFIX_LEN,
    cache::{self, future::FutureCachedExt},
    db::{DbWithOptAuthed, UserDB},
    error::{to_anyhow, Error, JsonResult, Result},
    jobs::{
        get_payload_tag_from_prefixed_path, resolve_delete_after_secs, schedule_job_deletion,
        JobPayload, JobTriggerKind, RawCode,
    },
    triggers::TriggerMetadata,
    user_drafts::{overlay_or_draft_only, DraftUserRef, UserDraftItemKind, WithDraftOverlay},
    users::username_to_permissioned_as,
    utils::{
        http_get_from_hub, not_found_if_none, paginate, query_elems_from_hub, require_admin,
        strip_json_nul, Pagination, RunnableKind, StripPath,
    },
    variables::{build_crypt, build_crypt_with_key_suffix, encrypt},
    worker::{to_raw_value, CLOUD_HOSTED},
    workspaces::{
        check_deploy_rules, check_user_against_rule, ProtectionRuleKind, RuleCheckResult,
    },
    HUB_BASE_URL,
};
#[cfg(feature = "parquet")]
use windmill_object_store::object_store_reexports::{Attribute, Attributes};
use windmill_store::resources::get_resource_value_interpolated_internal;

use windmill_api_auth::{
    create_token_internal, ensure_scopes_within_caller, forbid_elevated_job_token, NewToken,
    OptJobAuthed,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_queue::{push, PushArgs, PushArgsOwned, PushIsolationLevel};

#[cfg(feature = "parquet")]
use hmac::Mac;
#[cfg(feature = "parquet")]
use windmill_common::{jwt, oauth2::HmacSha256, variables::get_workspace_key};
#[cfg(feature = "parquet")]
use windmill_types::s3::{S3Object, S3Permission};

pub fn workspaced_service(raw_app_body_limit: usize) -> Router {
    Router::new()
        .route("/list", get(list_apps))
        .route("/list_search", get(list_search_apps))
        .route("/get/p/{*path}", get(get_app))
        .route("/embed_token/p/{*path}", get(get_app_embed_token_for_path))
        .route("/preview_sdk_token", post(mint_preview_sdk_token))
        .route("/get/lite/{*path}", get(get_app_lite))
        .route("/secret_of/{*path}", get(get_secret_id))
        .route(
            "/secret_of_latest_version/{*path}",
            get(get_latest_version_secret_id),
        )
        .route("/get/v/{*id}", get(get_app_by_id))
        .route("/get_data/v/{*id}", get(get_raw_app_data))
        .route("/exists/{*path}", get(exists_app))
        .route("/update/{*path}", post(update_app))
        .route(
            "/update_raw/{*path}",
            post(update_app_raw).layer(axum::extract::DefaultBodyLimit::max(raw_app_body_limit)),
        )
        .route(
            "/create_raw_source",
            post(create_app_raw_source)
                .layer(axum::extract::DefaultBodyLimit::max(raw_app_body_limit)),
        )
        .route(
            "/update_raw_source/{*path}",
            post(update_app_raw_source)
                .layer(axum::extract::DefaultBodyLimit::max(raw_app_body_limit)),
        )
        .route("/delete/{*path}", delete(delete_app))
        .route("/create", post(create_app))
        .route(
            "/create_raw",
            post(create_app_raw).layer(axum::extract::DefaultBodyLimit::max(raw_app_body_limit)),
        )
        .route("/history/p/{*path}", get(get_app_history))
        .route("/get_latest_version/{*path}", get(get_latest_version))
        .route(
            "/history_update/a/{id}/v/{version}",
            post(update_app_history),
        )
        .route(
            "/list_paths_from_workspace_runnable/{runnable_kind}/{*path}",
            get(list_paths_from_workspace_runnable),
        )
        .route(
            "/custom_path_exists/{*custom_path}",
            get(custom_path_exists),
        )
        .route("/sign_s3_objects", post(sign_s3_objects))
}

pub fn unauthed_service() -> Router {
    Router::new()
        .route("/execute_component/{*path}", post(execute_component))
        .route("/upload_s3_file/{*path}", post(upload_s3_file_from_app))
        .route("/delete_s3_file", delete(delete_s3_file_from_app))
        .route("/download_s3_file/{*path}", get(download_s3_file_from_app))
        .route(
            "/download_s3_parquet_file_as_csv/{*path}",
            get(app_download_s3_parquet_file_as_csv),
        )
        .route("/load_file_metadata/{*path}", get(app_load_file_metadata))
        .route("/load_file_preview/{*path}", get(app_load_file_preview))
        .route("/load_table_count/{*path}", get(app_load_table_count))
        .route(
            "/load_parquet_preview/{*path}",
            get(app_load_parquet_preview),
        )
        .route("/load_csv_preview/{*path}", get(app_load_csv_preview))
        .route("/public_app/{secret}", get(get_public_app_by_secret))
        .route("/guest_entry/{secret}", get(get_guest_entry))
        .route("/embed_token/{secret}", get(get_app_embed_token))
        .route("/public_resource/{*path}", get(get_public_resource))
        .route("/get_data/v/{*id}", get(get_raw_app_data))
}
pub fn global_service() -> Router {
    Router::new()
        .route("/hub/list", get(list_hub_apps))
        .route("/hub/get/{id}", get(get_hub_app_by_id))
        .route("/hub/get_raw/{id}", get(get_hub_raw_app_by_id))
}

#[derive(FromRow, Deserialize, Serialize)]
pub struct ListableApp {
    pub id: i64,
    pub workspace_id: String,
    pub path: String,
    pub summary: String,
    pub version: i64,
    pub extra_perms: serde_json::Value,
    pub execution_mode: String,
    pub starred: bool,
    pub edited_at: Option<chrono::DateTime<chrono::Utc>>,
    /// `Some(true)` only on rows synthesised from the `draft` table; `None` for
    /// deployed rows. See ListableScript in windmill-types/src/scripts.rs.
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_msg: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub raw_app: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    /// True when the authed user has a draft for this app (draft-only or layered
    /// over the deployed row). See ListableScript in windmill-types/src/scripts.rs.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_draft: bool,
    /// User-typed staged path from the draft JSON's `draft_path`; `None` = unchanged.
    /// See ListableScript in windmill-types/src/scripts.rs.
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_path: Option<String>,
    /// Per-path draft owners driving the home-page avatar circles.
    /// See ListableScript in windmill-types/src/scripts.rs.
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_users: Option<sqlx::types::Json<Vec<windmill_types::user_drafts::DraftUserRef>>>,
    /// Labels inherited from the parent folder, computed at read time.
    #[sqlx(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherited_labels: Option<Vec<String>>,
}

fn is_false(b: &bool) -> bool {
    !b
}

// #[derive(FromRow, Serialize, Deserialize)]
// pub struct AppVersion {
//     pub id: i64,
//     pub app_id: Uuid,
//     pub value: sqlx::types::Json<Box<RawValue>>,
//     pub created_by: String,
//     pub created_at: chrono::DateTime<chrono::Utc>,
// }

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AppWithLastVersion {
    pub id: i64,
    pub path: String,
    pub summary: String,
    pub policy: sqlx::types::Json<Box<RawValue>>,
    pub versions: Vec<i64>,
    pub value: sqlx::types::Json<Box<RawValue>>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub extra_perms: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_path: Option<String>,
    pub raw_app: bool,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

#[derive(Serialize, FromRow)]
pub struct AppWithLastVersionAndStarred {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub app: AppWithLastVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<bool>,
}

#[derive(Serialize)]
pub struct AppHistory {
    pub app_id: i64,
    pub version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_msg: Option<String>,
}

#[derive(Deserialize)]
pub struct AppHistoryUpdate {
    pub deployment_msg: Option<String>,
}

pub type StaticFields = HashMap<String, Box<RawValue>>;
pub type OneOfFields = HashMap<String, Vec<Box<RawValue>>>;
pub type AllowUserResources = Vec<String>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Anonymous,
    /// Login required, workspace membership not: anyone the instance's identity
    /// provider authenticates may open the app, and the runnables execute as the
    /// publisher exactly as in [`ExecutionMode::Publisher`]. Such a viewer holds a
    /// guest session: an identity with no account at all (no `password` row, no `usr`
    /// row anywhere), which is what keeps it off every row-based seat counter; what a
    /// guest costs instead is the allowance in `windmill_common::workspaces`. Honored
    /// only where `workspace_settings.guest_access_enabled` is on, re-read at the auth
    /// door on every guest request.
    Guest,
    /// Default for a policy that omits `execution_mode`. It MUST stay a mode
    /// that requires an authenticated viewer: an omitted field must never be
    /// able to publish an app anonymously (publicly executable).
    #[default]
    Publisher,
    Viewer,
}

impl ExecutionMode {
    /// The serialized form, matching this enum's `rename_all = "lowercase"`.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionMode::Anonymous => "anonymous",
            ExecutionMode::Guest => "guest",
            ExecutionMode::Publisher => "publisher",
            ExecutionMode::Viewer => "viewer",
        }
    }
}

/// The protection rule gating a *transition into* `mode`, if any. Anonymous and
/// guest each widen who may open an app past the workspace's own members, so each
/// carries its own rule; the two member-only modes are ungated.
fn deployment_rule_for_mode(mode: ExecutionMode) -> Option<ProtectionRuleKind> {
    match mode {
        ExecutionMode::Anonymous => Some(ProtectionRuleKind::RestrictAnonymousAppDeployment),
        ExecutionMode::Guest => Some(ProtectionRuleKind::RestrictGuestAppDeployment),
        ExecutionMode::Publisher | ExecutionMode::Viewer => None,
    }
}

/// A guest session is scoped to its app by path, so an app whose path the scope
/// grammar cannot hold as one literal (`is_scope_literal_path`) can never admit a
/// guest; refuse the mode at deploy time rather than advertise an app nobody enters.
/// `path` is where the app ends up: on a rename, the destination.
fn refuse_unscopable_guest_app(path: &str, mode: ExecutionMode) -> Result<()> {
    if matches!(mode, ExecutionMode::Guest) && !windmill_common::auth::is_scope_literal_path(path) {
        return Err(Error::BadRequest(format!(
            "app {path} cannot be set to Guests: a path with `:`, `,` or `*`, or a leading `/`, \
             cannot be scoped"
        )));
    }
    Ok(())
}

/// Gate a viewer on the app's `execution_mode`, as far as can be decided without an
/// ACL probe. `Ok(true)` means already authorized — anonymous admits anyone, guest
/// admits anyone signed in; `Ok(false)` means the caller is a member and still owes
/// the read-access check its caller performs.
///
/// A guest is authorized by its token's scope and never by an ACL probe: it holds no
/// `usr` row, so RLS finds nothing for it and every guest would read as having no
/// access. That scope is also what keeps a guest session to the one app it was minted
/// for, even though the mode itself admits anyone signed in. The workspace's guest
/// switch is not checked here: `AuthCache` enforces it for every guest request.
pub fn authorize_non_member_viewer(
    mode: ExecutionMode,
    app_path: &str,
    opt_authed: &Option<ApiAuthed>,
) -> Result<bool> {
    if matches!(mode, ExecutionMode::Anonymous) {
        return Ok(true);
    }
    let Some(authed) = opt_authed.as_ref() else {
        return Err(Error::NotAuthorized(
            "App visibility does not allow public access and you are not logged in".to_string(),
        ));
    };
    let is_guest = windmill_api_auth::scopes::has_guest_sentinel(authed.scopes.as_deref());
    if matches!(mode, ExecutionMode::Guest) {
        if is_guest {
            check_scopes(authed, || format!("apps:read:{}", app_path))?;
        }
        return Ok(true);
    }
    if is_guest {
        return Err(Error::PermissionDenied(format!(
            "app {app_path} is not open to guests"
        )));
    }
    Ok(false)
}

/// Confines a guest to its app once the app's mode is known; a no-op for every other
/// caller. An anonymous app is open to anyone, so the guest stays the caller there,
/// as itself: the run and the reads that follow it (job results, S3 provenance) must
/// carry one identity. Anywhere else a mismatch is refused.
fn guest_caller_for_mode(
    opt_authed: Option<ApiAuthed>,
    mode: ExecutionMode,
    app_path: &str,
) -> Result<Option<ApiAuthed>> {
    let Some(authed) = opt_authed.as_ref() else {
        return Ok(None);
    };
    if !windmill_api_auth::scopes::has_guest_sentinel(authed.scopes.as_deref())
        || matches!(mode, ExecutionMode::Anonymous)
    {
        return Ok(opt_authed);
    }
    check_scopes(authed, || format!("apps:run:{app_path}"))
        .or_else(|_| check_scopes(authed, || format!("apps:read:{app_path}")))?;
    Ok(opt_authed)
}

/// [`authorize_non_member_viewer`] plus the member read-access probe, for the
/// entry points that address an app by id.
async fn authorize_app_viewer(
    mode: ExecutionMode,
    app_path: &str,
    app_id: i64,
    w_id: &str,
    user_db: &UserDB,
    opt_authed: &Option<ApiAuthed>,
) -> Result<()> {
    if authorize_non_member_viewer(mode, app_path, opt_authed)? {
        return Ok(());
    }
    let authed = opt_authed
        .as_ref()
        .ok_or_else(|| Error::internal_err("authorize_app_viewer: unauthenticated".to_string()))?;
    let mut tx = user_db.clone().begin(authed).await?;
    let is_visible = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM app WHERE id = $1 AND workspace_id = $2)",
        app_id,
        w_id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    if !is_visible.unwrap_or(false) {
        return Err(Error::NotAuthorized(
            "App visibility does not allow public access and you are logged in but you have no read-access to that app".to_string(),
        ));
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PolicyTriggerableInputs {
    static_inputs: StaticFields,
    one_of_inputs: OneOfFields,
    #[serde(default)]
    allow_user_resources: AllowUserResources,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    delete_after_secs: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sensitive_inputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct S3Input {
    allowed_resources: Vec<String>,
    allow_user_resources: bool,
    allow_workspace_resource: bool,
    file_key_regex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct S3Key {
    s3_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Policy {
    pub on_behalf_of: Option<String>,
    pub on_behalf_of_email: Option<String>,
    //paths:
    // - script/<path>
    // - flow/<path>
    // - rawscript/<sha256>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggerables: Option<HashMap<String, StaticFields>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggerables_v2: Option<HashMap<String, PolicyTriggerableInputs>>,
    /// `None` when the policy states no mode, which the OpenAPI schema (and so
    /// the MCP tools generated from it) allows. Create resolves that to
    /// [`ExecutionMode::default`]; update keeps the deployed app's mode, so a
    /// partial policy cannot silently re-permission an app. Read the effective
    /// mode with [`Policy::execution_mode`], never this field.
    #[serde(default, rename = "execution_mode")]
    stated_execution_mode: Option<ExecutionMode>,
    pub s3_inputs: Option<Vec<S3Input>>,
    pub allowed_s3_keys: Option<Vec<S3Key>>,
    // WIN-2006: publisher opt-in to iframe sandbox isolation (alpha). When true the
    // app is isolated from each viewer's Windmill session: low-code renders in an
    // opaque-origin iframe with a scoped embed token, raw renders its bundle in an
    // opaque iframe. Default/absent means unsandboxed — the app runs same-origin
    // with the viewer's full session, the pre-isolation behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<bool>,
    /// Raw apps: scopes a viewer may mint a token for, so the bundle's
    /// `windmill-client` calls run as them. Subset of
    /// `FRONTEND_SDK_ALLOWED_SCOPES`; absent means no credential (the default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontend_sdk_scopes: Option<Vec<String>>,
}

impl Policy {
    /// The mode this policy runs under. A policy that states none resolves to
    /// [`ExecutionMode::default`], never to `anonymous`, so an app can only be
    /// publicly executable because someone said so.
    pub fn execution_mode(&self) -> ExecutionMode {
        self.stated_execution_mode.unwrap_or_default()
    }

    /// What the policy states, or `None` for "not stated". Only the write paths
    /// need this: everything else wants [`Policy::execution_mode`].
    fn stated_execution_mode(&self) -> Option<ExecutionMode> {
        self.stated_execution_mode
    }

    pub fn set_execution_mode(&mut self, execution_mode: ExecutionMode) {
        self.stated_execution_mode = Some(execution_mode);
    }
}

#[derive(Deserialize)]
pub struct CreateApp {
    pub path: String,
    pub summary: String,
    pub value: sqlx::types::Json<Box<RawValue>>,
    pub policy: Policy,
    pub deployment_message: Option<String>,
    pub custom_path: Option<String>,
    pub preserve_on_behalf_of: Option<bool>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    /// Caller-intent flag (set by the CLI / git sync): when true, deploying
    /// this app must NOT delete an existing user draft at the same path.
    /// Transient — never persisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_draft_deletion: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct EditApp {
    pub path: Option<String>,
    pub summary: Option<String>,
    pub value: Option<sqlx::types::Json<Box<RawValue>>>,
    pub policy: Option<Policy>,
    pub deployment_message: Option<String>,
    pub custom_path: Option<String>,
    pub preserve_on_behalf_of: Option<bool>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    /// Caller-intent flag (set by the CLI / git sync): when true, deploying
    /// this app must NOT delete an existing user draft at the same path.
    /// Transient — never persisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_draft_deletion: Option<bool>,
    /// Caller-intent flag: when true this deploy may switch the app between
    /// low-code and raw (see `update_app_internal`). Transient — never
    /// persisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_kind_change: Option<bool>,
}

#[derive(Serialize, FromRow)]
pub struct SearchApp {
    path: String,
    value: sqlx::types::Json<Box<RawValue>>,
}
async fn list_search_apps(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<SearchApp>> {
    // Require domain-level read: this returns every visible app's full value (code).
    // The route layer treats `apps:run` as satisfying read, so without this handler
    // check a scoped embed token (apps:run + apps:read:<one path>) could read all
    // apps' definitions. `check_scopes` uses ScopeDefinition::includes, where run
    // does NOT include read, so it correctly denies such tokens.
    check_scopes(&authed, || "apps:read".to_string())?;
    let n = 1000;
    let mut tx = user_db.begin(&authed).await?;

    let allowed = build_scope_path_predicate(&authed, "apps", "read");

    let rows = sqlx::query_as::<_, SearchApp>(
        "SELECT path, app_version.value from app LEFT JOIN app_version ON app_version.id = versions[array_upper(versions, 1)]  WHERE workspace_id = $1 LIMIT $2",
    )
    .bind(&w_id)
    .bind(n)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .filter(|r| allowed(&r.path))
    .collect::<Vec<_>>();
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_apps(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListAppQuery>,
) -> JsonResult<Vec<ListableApp>> {
    // Domain-level read (see list_search_apps): keeps a scoped embed token, whose
    // `apps:run` only satisfies read at the route layer, from listing all apps.
    check_scopes(&authed, || "apps:read".to_string())?;
    let (per_page, offset) = paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("app")
        .fields(&[
            "app.id",
            "app.workspace_id",
            "app.path",
            "app.summary",
            "app.versions[array_upper(app.versions, 1)] as version",
            "app.policy->>'execution_mode' as execution_mode",
            "app_version.created_at as edited_at",
            "app.extra_perms",
            "favorite.path IS NOT NULL as starred",
            "app_version.raw_app",
            "app.labels",
            "draft.path IS NOT NULL as is_draft",
            // Per-path draft owners as a JSON array; see scripts.rs for the rationale
            // (non-member superadmin identity fallback via `password`, legacy NULL-email row).
            // `app`/`raw_app` are separate draft kinds over one `app` table — match
            // either (like the `is_draft` join below), else a deployed raw app's draft
            // owners are dropped and the row shows "Draft" with no user badge.
            "(SELECT json_agg(json_build_object('username', COALESCE(u.username, p.username, CASE WHEN p.email IS NOT NULL THEN d.email END)) ORDER BY COALESCE(u.username, p.username, CASE WHEN p.email IS NOT NULL THEN d.email END) NULLS LAST) \
              FROM draft d \
              LEFT JOIN usr u ON u.workspace_id = d.workspace_id AND u.email = d.email \
              LEFT JOIN password p ON p.email = d.email AND p.super_admin = true \
              WHERE d.workspace_id = app.workspace_id AND d.path = app.path AND d.typ IN ('app', 'raw_app') \
                AND (d.email IS NULL OR u.username IS NOT NULL OR p.email IS NOT NULL)) as draft_users",
            "folder_labels(app.workspace_id, app.path) as inherited_labels",
        ])
        .left()
        .join("favorite")
        .on(
            "favorite.favorite_kind = 'app' AND favorite.workspace_id = app.workspace_id AND favorite.path = app.path AND favorite.usr = ?"
                .bind(&authed.username),
        )
        .left()
        // `app`/`raw_app` are separate draft kinds over one `app` table — match either
        // for `is_draft`. DISTINCT in the subquery: a path with both kinds for the same
        // user would otherwise fan the deployed row into two identical entries.
        .join(
            "(SELECT DISTINCT path, workspace_id FROM draft WHERE typ IN ('app', 'raw_app') AND email = ?) draft"
                .bind(&authed.email),
        )
        .on("draft.path = app.path AND draft.workspace_id = app.workspace_id")
        .left()
        .join("app_version")
        .on(
            "app_version.id = versions[array_upper(versions, 1)]"
        )
        .order_desc("favorite.path IS NOT NULL")
        .order_by("app_version.created_at", true)
        .and_where("app.workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    if lq.starred_only.unwrap_or(false) {
        sqlb.and_where_is_not_null("favorite.path");
    }

    if let Some(path_start) = &lq.path_start {
        sqlb.and_where_like_left("app.path", path_start);
    }

    if let Some(path_exact) = &lq.path_exact {
        sqlb.and_where_eq("app.path", "?".bind(path_exact));
    }

    if let Some(label) = &lq.label {
        for l in label.split(',') {
            sqlb.and_where(
                "(app.labels @> ARRAY[?] OR folder_labels(app.workspace_id, app.path) @> ARRAY[?])"
                    .bind(&l.trim())
                    .bind(&l.trim()),
            );
        }
    }

    if lq.with_deployment_msg.unwrap_or(false) {
        sqlb.join("deployment_metadata dm")
            .left()
            .on("dm.app_version = app.versions[array_upper(app.versions, 1)]")
            .fields(&["dm.deployment_msg"]);
    }

    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let mut rows = sqlx::query_as::<_, ListableApp>(&sql)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    // Append the authed user's `app`/`raw_app` drafts at paths with no deployed app;
    // see scripts.rs.
    if lq.include_draft_only.unwrap_or(false)
        && !authed.is_operator
        && offset == 0
        && lq.path_start.is_none()
        && lq.path_exact.is_none()
        && lq.label.is_none()
        && !lq.starred_only.unwrap_or(false)
    {
        // DISTINCT ON (path), newest first: collapse a path holding both `app` and
        // `raw_app` drafts to one row (the home list keyed by `type/path` would crash
        // on duplicates). `(email IS NULL)` last keeps the owned row over the legacy one.
        let draft_only_rows = sqlx::query!(
            r#"SELECT DISTINCT ON (path)
                      path,
                      value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>",
                      created_at,
                      typ::text as "typ!"
               FROM draft
               WHERE workspace_id = $1
                 AND typ IN ('app', 'raw_app')
                 AND (email = $2 OR email IS NULL)
                 AND NOT EXISTS (
                     SELECT 1 FROM app a
                     WHERE a.workspace_id = draft.workspace_id
                       AND a.path = draft.path
                 )
               ORDER BY path, (email IS NULL), created_at DESC"#,
            &w_id,
            &authed.email,
        )
        .fetch_all(&db)
        .await?;

        for row in draft_only_rows {
            let v: serde_json::Value =
                serde_json::from_str(row.value.0.get()).unwrap_or(serde_json::Value::Null);
            // App/raw-app drafts are the bare editor value with no `path`, so the editor
            // writes a separate `draft_path` only when it differs from deployed; see flows.rs.
            let draft_path = v
                .get("draft_path")
                .and_then(|s| s.as_str())
                .filter(|s| !s.is_empty() && *s != row.path.as_str())
                .map(|s| s.to_string());
            rows.push(ListableApp {
                id: 0,
                workspace_id: w_id.clone(),
                path: row.path,
                summary: v
                    .get("summary")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                version: 0,
                extra_perms: serde_json::Value::Object(serde_json::Map::new()),
                execution_mode: String::new(),
                starred: false,
                edited_at: Some(row.created_at),
                draft_only: Some(true),
                deployment_msg: None,
                raw_app: row.typ == "raw_app",
                labels: None,
                // No deployed row to inherit folder labels from.
                inherited_labels: None,
                is_draft: true,
                draft_path,
                // Synthesized rows are the authed user's own draft.
                draft_users: Some(sqlx::types::Json(vec![DraftUserRef {
                    username: Some(authed.username.clone()),
                }])),
            });
        }
    }

    let allowed = build_scope_path_predicate(&authed, "apps", "read");
    rows.retain(|r| allowed(&r.path));

    Ok(Json(rows))
}

async fn get_raw_app_data(
    Path((w_id, secret_with_ext)): Path<(String, String)>,
    Query(query): Query<std::collections::HashMap<String, String>>,
    Extension(db): Extension<DB>,
) -> Result<Response> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    let object_store = windmill_object_store::get_object_store().await;

    // tracing::info!("secret_with_ext: {}", secret_with_ext);
    let mut splitted = secret_with_ext.split('.');
    let secret_id = splitted.next().unwrap_or("");

    if secret_id.is_empty() {
        return Err(Error::BadRequest("Invalid secret".to_string()));
    }

    let id = get_id_from_secret(
        &db,
        &w_id,
        secret_id.to_string(),
        Some(BUNDLE_SECRET_PREFIX),
    )
    .await?;

    let file_type = splitted.next().unwrap_or("");

    // Sandboxed wrapper document that hosts the bundle. Served from a real URL
    // (not blob:/srcdoc) so we can attach `CSP: sandbox` as a response header,
    // which forces an opaque origin even on direct navigation — a raw-app
    // bundle can then never reach the authenticated Windmill origin (WIN-2006).
    // The `.js`/`.css` are loaded as same-path subresources by this document.
    if file_type == "html" {
        // ALWAYS served with `CSP: sandbox`, which forces an opaque origin even on
        // direct top-level navigation — so this real-origin URL can never be used
        // to run a raw-app bundle with the viewer's session (WIN-2006). The
        // unsandboxed (default) render is NOT applied here: it is handled entirely
        // on the viewer side, which builds its own same-origin wrapper. Relaxing
        // this header from a policy flag would let anyone with the share secret
        // hand a logged-in victim a same-origin URL that runs the bundle with
        // their session — so the standalone document stays sandboxed no matter how
        // it is reached.
        let html = raw_app_wrapper_html(secret_id);
        let mut builder = Response::builder()
            .header(http::header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header("X-Content-Type-Options", "nosniff")
            .header("Cross-Origin-Resource-Policy", "cross-origin")
            // This URL carries the frontend-SDK handshake nonce, so it must not
            // travel to whatever the bundle navigates to or links out to.
            .header("Referrer-Policy", "no-referrer")
            .header(
                http::header::CONTENT_SECURITY_POLICY,
                "sandbox allow-scripts allow-forms allow-popups \
                 allow-popups-to-escape-sandbox allow-downloads allow-modals \
                 allow-top-navigation",
            );
        // When the public app page is embedded in a cross-origin-isolated page
        // (`wm_coep` opt-in, COEP `require-corp`), this nested wrapper document
        // must itself assert COEP to be allowed to load. Opt-in only — COEP
        // restricts the bundle's own subresources to CORP'd/same-origin ones
        // (e.g. external images would break), so it must not be always-on. The
        // viewer propagates the flag from the page URL (see RawAppPreview).
        if query.contains_key("wm_coep") {
            builder = builder.header("Cross-Origin-Embedder-Policy", "require-corp");
        }
        return Ok(builder.body(Body::from(html)).unwrap());
    }

    let file_type = if file_type == "css" {
        "css"
    } else if file_type == "js" {
        "js"
    } else {
        return Err(Error::BadRequest(
            "Invalid file type, only .css, .js and .html are supported".to_string(),
        ));
    };
    // tracing::info!("file_type: {}", file_type);

    #[allow(unused_assignments)]
    let mut body: Option<Body> = None;
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if let Some(os) = object_store {
        let path = format!("/app_bundles/{}/{}.{}", w_id, id, file_type);
        match os
            .get(&windmill_object_store::object_store_reexports::Path::from(
                path,
            ))
            .await
        {
            Ok(result) => {
                let stream = result
                    .bytes()
                    .await
                    .map_err(windmill_object_store::object_store_error_to_error)?;
                tracing::info!("stream: {}", stream.len());
                body = Some(Body::from(stream));
            }
            Err(windmill_object_store::object_store_reexports::ObjectStoreError::NotFound {
                ..
            }) => {
                // S3 key not found, fall through to DB lookup below
            }
            Err(e) => {
                return Err(windmill_object_store::object_store_error_to_error(e));
            }
        }
    }

    if body.is_none() {
        let get_raw_app_file = sqlx::query_scalar!(
            "SELECT data FROM app_bundles WHERE app_version_id = $1 AND file_type = $2 AND w_id = $3",
            id,
            file_type,
            &w_id,
        )
        .fetch_optional(&db)
        .await?;
        if let Some(file) = get_raw_app_file {
            body = Some(Body::from(file));
        }
    }

    // A bundle with no styles has no `css` row at all: the write side stores that
    // blob only when the client sends the field. Absent means empty, not broken —
    // 404ing here makes such an app permanently un-deployable, because a
    // cross-workspace deploy re-fetches both parts and treats the 404 as fatal.
    // A missing `js` is a genuinely broken bundle and must still 404.
    //
    // Logged at debug, not warn: for a style-less app this is the normal path and
    // fires on every page load. It is the only trace distinguishing that case from
    // a css blob lost by an object-store migration, which lands here identically.
    if body.is_none() && file_type == "css" {
        tracing::debug!(w_id = %w_id, app_version_id = %id, "no css bundle stored, serving empty stylesheet");
        body = Some(Body::empty());
    }

    if let Some(body) = body {
        // let stream = tokio_util::io::ReaderStream::new(file);
        let res = Response::builder()
            .header(
                http::header::CONTENT_TYPE,
                if file_type == "css" {
                    "text/css"
                } else {
                    "text/javascript"
                },
            )
            // nosniff + CORP so the bundle loads correctly as a subresource of
            // the opaque, sandboxed wrapper (incl. under a cross-origin-isolated
            // / COEP `require-corp` embedder).
            .header("X-Content-Type-Options", "nosniff")
            .header("Cross-Origin-Resource-Policy", "cross-origin");
        Ok(res.body(body).unwrap())
    } else {
        return Err(Error::NotFound("File not found".to_string()));
    }
}

/// HTML wrapper that hosts a raw-app bundle inside a sandboxed, opaque-origin
/// iframe. Served by [`get_raw_app_data`] for the `.html` "file type". It loads
/// the bundle `.js`/`.css` as same-path subresources, shims web storage (which
/// an opaque origin disallows), and waits for the embedder's handshake before
/// evaluating the bundle, so `window.ctx` — and `window.process.env` when the
/// viewer approved SDK scopes — is set synchronously when it runs. That token is
/// the bundle's only credential, gated on both handshake proofs below.
fn raw_app_wrapper_html(secret: &str) -> String {
    const TEMPLATE: &str = r##"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8" />
<title>App</title>
<link rel="stylesheet" href="./__SECRET__.css" />
<script>
(function () {
  // Storage shim: an opaque-origin (sandboxed) document has no localStorage and
  // accessing it throws. Provide an in-memory implementation so apps that use
  // web storage keep working within the session.
  try {
    window.localStorage.getItem('__wm_probe__');
  } catch (e) {
    function makeShim(onOp) {
      var mem = {};
      return {
        getItem: function (k) { k = String(k); return Object.prototype.hasOwnProperty.call(mem, k) ? mem[k] : null; },
        setItem: function (k, v) { mem[String(k)] = String(v); if (onOp) onOp({ op: 'set', key: String(k), value: String(v) }); },
        removeItem: function (k) { delete mem[String(k)]; if (onOp) onOp({ op: 'remove', key: String(k) }); },
        clear: function () { for (var k in mem) { delete mem[k]; } if (onOp) onOp({ op: 'clear' }); },
        key: function (i) { var ks = Object.keys(mem); return i < ks.length ? ks[i] : null; },
        get length() { return Object.keys(mem).length; },
        __hydrate: function (obj) { if (obj) { for (var k in obj) { mem[k] = String(obj[k]); } } }
      };
    }
    // localStorage relays each mutation up to the parent (RawAppPreview), which
    // backs a single store shared across all apps; sessionStorage stays session-only.
    function relayOp(o) { try { window.parent.postMessage({ type: 'wm_ls_op', op: o.op, key: o.key, value: o.value }, '*'); } catch (_) {} }
    var ls = makeShim(relayOp);
    var ss = makeShim(null);
    try { Object.defineProperty(window, 'localStorage', { value: ls, configurable: true }); } catch (_) {}
    try { Object.defineProperty(window, 'sessionStorage', { value: ss, configurable: true }); } catch (_) {}
    window.__wmStorageShim = { local: ls, session: ss };
    // document.cookie also throws in an opaque origin; back it with an in-memory
    // jar so reads don't crash apps. This is NOT the real session cookie (which
    // is unreachable here) — just an isolated client-side store.
    try {
      var jar = {};
      Object.defineProperty(Document.prototype, 'cookie', {
        configurable: true,
        get: function () { return Object.keys(jar).map(function (k) { return k + '=' + jar[k]; }).join('; '); },
        set: function (v) { var p = String(v).split(';')[0]; var i = p.indexOf('='); if (i > -1) { jar[p.slice(0, i).trim()] = p.slice(i + 1).trim(); } }
      });
    } catch (_) {}
  }

  // Keep the iframe hash in sync with the parent so app URLs stay shareable.
  function notifyParent() {
    try { if (window.parent !== window) { window.parent.postMessage({ type: 'windmill:hashchange', hash: window.location.hash }, '*'); } } catch (_) {}
  }
  window.addEventListener('hashchange', notifyParent);
  var _ps = history.pushState, _rs = history.replaceState;
  history.pushState = function () { _ps.apply(this, arguments); notifyParent(); };
  history.replaceState = function () { _rs.apply(this, arguments); notifyParent(); };

  // ctx handshake: the embedding parent hands us the user context (and any
  // persisted storage) before we evaluate the bundle, so `window.ctx` is set
  // synchronously when the bundle runs. The bundle <script> is injected only
  // after this, never inline, so it always observes a ready context.
  var loaded = false;
  function loadBundle() {
    if (loaded) return; loaded = true;
    var s = document.createElement('script');
    s.src = './__SECRET__.js';
    document.body.appendChild(s);
  }
  function applyCtx(d) {
    if (!d || d.type !== 'windmill:ctx') return;
    window.ctx = d.ctx;
    if (window.__wmStorageShim && d.storage) {
      window.__wmStorageShim.local.__hydrate(d.storage.local);
      window.__wmStorageShim.session.__hydrate(d.storage.session);
    }
    // A bundled `windmill-client` reads window.process.env at module load, so the
    // env must exist before the bundle script is injected below. BASE_URL comes
    // from the embedder — location.origin is "null" on this opaque origin.
    if (d.sdk && d.sdk.token) {
      window.process = { env: { WM_RAW_APP: 'true', WM_TOKEN: d.sdk.token, BASE_URL: d.sdk.baseUrl, WM_WORKSPACE: d.sdk.workspace } };
    }
    if (d.initialHash && d.initialHash !== '#' && !window.location.hash) {
      try { history.replaceState(null, '', d.initialHash); } catch (_) {}
    }
    loadBundle();
  }
  // The embedder parts with the viewer's SDK token only if both hold: the nonce
  // from our URL says which document is asking (one navigated in later cannot
  // read it), and our own port says the answer reaches only this document
  // (replacing the document discards the port).
  try {
    var hs = new URLSearchParams(window.location.search).get('wm_hs') || undefined;
    var ch = new MessageChannel();
    ch.port1.onmessage = function (e) { applyCtx(e.data); };
    window.parent.postMessage({ type: 'windmill:ready', nonce: hs }, '*', [ch.port2]);
  } catch (_) {}
  // Fallback for contexts that never send ctx (e.g. ctx-less rendering).
  setTimeout(loadBundle, 1500);
})();
</script>
</head>
<body>
<div id="root"></div>
</body>
</html>
"##;
    TEMPLATE.replace("__SECRET__", secret)
}

// async fn get_app_version(
//     authed: ApiAuthed,
//     Extension(user_db): Extension<UserDB>,
//     Path((w_id, path)): Path<(String, StripPath)>,
// ) -> JsonResult<i64> {
//     let path = path.to_path();
//     let mut tx = user_db.begin(&authed).await?;

//     let version_o = sqlx::query_scalar!(
//         "SELECT app.versions[array_upper(app.versions, 1)] as version FROM app
//             WHERE app.path = $1 AND app.workspace_id = $2",
//         path,
//         &w_id,
//     )
//     .fetch_optional(&mut *tx)
//     .await?
//     .flatten();
//     tx.commit().await?;

//     let version = not_found_if_none(version_o, "App", path)?;
//     Ok(Json(version))
// }

// Fields inlined rather than flattened (axum query bool quirk); see GetScriptByPathQuery in scripts.rs.
#[derive(Deserialize)]
struct GetAppQuery {
    with_starred_info: Option<bool>,
    #[serde(default)]
    get_draft: bool,
    /// Picks the draft kind for a draft-only lookup (`/apps_raw/...` → true).
    /// Ignored when a deployed row exists — its own `raw_app` column wins.
    #[serde(default)]
    raw_app: Option<bool>,
}

async fn get_app(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<GetAppQuery>,
) -> JsonResult<WithDraftOverlay> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let app_o = if query.with_starred_info.unwrap_or(false) {
        sqlx::query_as::<_, AppWithLastVersionAndStarred>(
            "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
            app.extra_perms, app_version.value,
            app_version.created_at, app_version.created_by, favorite.path IS NOT NULL as starred, app_version.raw_app, app.labels
            FROM app
            JOIN app_version
            ON app_version.id = app.versions[array_upper(app.versions, 1)]
            LEFT JOIN favorite
            ON favorite.favorite_kind = 'app'
                AND favorite.workspace_id = app.workspace_id
                AND favorite.path = app.path
                AND favorite.usr = $3
            WHERE app.path = $1 AND app.workspace_id = $2",
        )
        .bind(path.to_owned())
        .bind(&w_id)
        .bind(&authed.username)
        .fetch_optional(&mut *tx)
        .await?
    } else {
        sqlx::query_as::<_, AppWithLastVersionAndStarred>(
            "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
            app.extra_perms, app_version.value,
            app_version.created_at, app_version.created_by, NULL as starred, app_version.raw_app, app.labels
            FROM app, app_version
            WHERE app.path = $1 AND app.workspace_id = $2 AND app_version.id = app.versions[array_upper(app.versions, 1)]",
        )
        .bind(path.to_owned())
        .bind(&w_id)
        .fetch_optional(&mut *tx)
        .await?
    };
    tx.commit().await?;

    // No deployed row + `get_draft`: fall back to the draft table; see scripts.rs.
    // Draft kind comes from the deployed row's `raw_app` flag, or for a draft-only
    // path from the caller's `raw_app` query param.
    let kind = match &app_o {
        Some(app) if app.app.raw_app => UserDraftItemKind::RawApp,
        Some(_) => UserDraftItemKind::App,
        None if query.raw_app.unwrap_or(false) => UserDraftItemKind::RawApp,
        None => UserDraftItemKind::App,
    };
    let overlay = overlay_or_draft_only(
        &db,
        &w_id,
        &authed.email,
        kind,
        path,
        query.get_draft,
        app_o,
        || windmill_common::error::Error::NotFound(format!("App not found at path {path}")),
    )
    .await?;
    Ok(Json(overlay))
}

async fn get_app_lite(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<AppWithLastVersion> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let app_o = sqlx::query_as::<_, AppWithLastVersion>(
        "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
        app.extra_perms, coalesce(app_version_lite.value::json, app_version.value) as value,
        app_version.created_at, app_version.created_by, NULL as starred, app_version.raw_app, app.labels
        FROM app, app_version
        LEFT JOIN app_version_lite ON app_version_lite.id = app_version.id
        WHERE app.path = $1 AND app.workspace_id = $2 AND app_version.id = app.versions[array_upper(app.versions, 1)]",
    )
    .bind(path.to_owned())
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", path)?;
    Ok(Json(app))
}

async fn get_app_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Vec<AppHistory>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", &path))?;
    let mut tx = user_db.begin(&authed).await?;
    let query_result = sqlx::query!(
        "SELECT a.id as app_id, av.id as version_id, dm.deployment_msg as deployment_msg
        FROM app a LEFT JOIN app_version av ON a.id = av.app_id LEFT JOIN deployment_metadata dm ON av.id = dm.app_version
        WHERE a.workspace_id = $1 AND a.path = $2
        ORDER BY created_at DESC",
        w_id,
        path,
    ).fetch_all(&mut *tx).await?;
    tx.commit().await?;

    let result: Vec<AppHistory> = query_result
        .into_iter()
        .map(|row| AppHistory {
            app_id: row.app_id,
            version: row.version_id,
            deployment_msg: row.deployment_msg,
        })
        .collect();
    return Ok(Json(result));
}

async fn get_latest_version(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Option<AppHistory>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;
    let row = sqlx::query!(
        "SELECT a.id as app_id, av.id as version_id, dm.deployment_msg as deployment_msg
        FROM app a LEFT JOIN app_version av ON a.id = av.app_id LEFT JOIN deployment_metadata dm ON av.id = dm.app_version
        WHERE a.workspace_id = $1 AND a.path = $2
        ORDER BY created_at DESC",
        w_id,
        path,
    ).fetch_optional(&mut *tx).await?;
    tx.commit().await?;

    if let Some(row) = row {
        let result = AppHistory {
            app_id: row.app_id,
            version: row.version_id,
            deployment_msg: row.deployment_msg,
        };

        return Ok(Json(Some(result)));
    } else {
        return Ok(Json(None));
    }
}

async fn update_app_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, app_id, app_version)): Path<(String, i64, i64)>,
    Json(app_history_update): Json<AppHistoryUpdate>,
) -> Result<()> {
    let mut tx = user_db.begin(&authed).await?;
    let app_path = sqlx::query_scalar!("SELECT path FROM app WHERE id = $1", app_id)
        .fetch_optional(&mut *tx)
        .await?;

    let Some(app_path) = app_path else {
        tx.commit().await?;
        return Err(Error::NotFound(
            format!("App with ID {app_id} not found").to_string(),
        ));
    };

    check_scopes(&authed, || format!("apps:write:{}", &app_path))?;

    sqlx::query!(
        "INSERT INTO deployment_metadata (workspace_id, path, app_version, deployment_msg) VALUES ($1, $2, $3, $4) ON CONFLICT (workspace_id, path, app_version) WHERE app_version IS NOT NULL DO UPDATE SET deployment_msg = EXCLUDED.deployment_msg",
        w_id,
        app_path,
        app_version,
        app_history_update.deployment_msg,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(());
}

async fn custom_path_exists(
    Extension(db): Extension<DB>,
    Path((w_id, custom_path)): Path<(String, String)>,
) -> JsonResult<bool> {
    let as_workspaced_route = APP_WORKSPACED_ROUTE.load(std::sync::atomic::Ordering::Relaxed);

    let exists =
        sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM app WHERE custom_path = $1 AND ($2::TEXT IS NULL OR workspace_id = $2))",
            custom_path,
            if *CLOUD_HOSTED || as_workspaced_route { Some(&w_id) } else { None }
        )
        .fetch_one(&db)
        .await?.unwrap_or(false);
    Ok(Json(exists))
}

async fn get_app_by_id(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, i64)>,
) -> JsonResult<AppWithLastVersion> {
    let mut tx = user_db.begin(&authed).await?;

    let app_o = sqlx::query_as::<_, AppWithLastVersion>(
        "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
        app.extra_perms, app_version.value,
        app_version.created_at, app_version.created_by, app_version.raw_app, app.labels
        FROM app, app_version
        WHERE app_version.id = $1 AND app.id = app_version.app_id AND app.workspace_id = $2",
    )
    .bind(&id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let app = not_found_if_none(app_o, "App", id.to_string())?;

    check_scopes(&authed, || format!("apps:read:{}", &app.path))?;

    Ok(Json(app))
}

async fn get_public_app_by_secret(
    OptAuthed(opt_authed): OptAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, secret)): Path<(String, String)>,
) -> JsonResult<AppWithLastVersion> {
    let id = get_id_from_secret(&db, &w_id, secret, None).await?;

    let app_o = sqlx::query_as::<_, AppWithLastVersion>(
        "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
        null as extra_perms, coalesce(app_version_lite.value::json, app_version.value::json) as value,
        app_version.created_at, app_version.created_by, app_version.raw_app, app.labels
        FROM app, app_version
        LEFT JOIN app_version_lite ON app_version_lite.id = app_version.id
        WHERE app.id = $1 AND app.workspace_id = $2 AND app_version.id = app.versions[array_upper(app.versions, 1)]")
        .bind(&id)
        .bind(&w_id)
    .fetch_optional(&db)
    .await?;

    let mut app = not_found_if_none(app_o, "App", id.to_string())?;

    // Confine the app embed token (the only credential handed to untrusted app JS,
    // carrying the viewer's identity + `apps:read:<own path>`) to the app the secret
    // resolves to: without this, app JS could reuse the viewer's identity to read any
    // app it can see by secret via the RLS check below. Scoped to embed tokens only —
    // other callers (anonymous, cookie, plain external JWT) keep their existing access.
    if let Some(authed) = opt_authed.as_ref() {
        if windmill_api_auth::scopes::has_app_embed_sentinel(authed.scopes.as_deref()) {
            check_scopes(authed, || format!("apps:read:{}", app.path))?;
        }
    }

    let policy = serde_json::from_str::<Policy>(app.policy.0.get()).map_err(to_anyhow)?;

    authorize_app_viewer(
        policy.execution_mode(),
        &app.path,
        id,
        &w_id,
        &user_db,
        &opt_authed,
    )
    .await?;

    // Compute bundle_secret for raw apps
    if app.raw_app {
        app.bundle_secret = Some(compute_bundle_secret(&db, &w_id, &app.versions).await?);
    }

    Ok(Json(app))
}

/// Scopes granted to a short-lived "app embed token". This is the token the
/// app-embedder page hands the (opaque-origin) app iframe at startup so the app
/// never receives the viewer's session cookie. Instead of restricting which
/// routes a *domain* may hit, we restrict which routes the *token* may hit, so
/// that even a malicious or compromised app document can only reach the
/// endpoints an app legitimately needs. The `app_embed` sentinel turns each of
/// these into a strict route allowlist (`app_embed_route_denied`):
/// - `jobs:read`     → by-id job poll/cancel only; enumeration, counts, exports,
///                      and `job_signature`/`resume_urls` are denied, and by-id
///                      reads are confined to the app's own runs.
/// - `app_embed`     → sentinel tagging this as an app embed token (grants nothing).
/// - `resources:run` → resource metadata only (pickers, type schemas), never values.
/// - `users:read`    → `users/whoami` only.
/// - `folders:read`  → `folders/listnames` only.
/// Plus two path-scoped scopes minted per app (see `mint_app_embed_token`):
/// - `apps:read:<path>` → the app's own definition (`apps/get/p/<path>`); no
///                      `apps:write`, so management routes are unreachable.
/// - `apps:run:<path>`  → run THIS app's components (`execute_component`, which
///                      re-checks the path); `apps_u/*` public-serving routes.
pub const APP_EMBED_SCOPES: [&str; 5] = [
    "jobs:read",
    windmill_api_auth::scopes::APP_EMBED_SENTINEL,
    "resources:run",
    "users:read",
    "folders:read",
];

/// How long an app embed token stays valid. The embedder re-mints on demand
/// (e.g. after a `401` from the iframe) so this can stay short.
const APP_EMBED_TOKEN_VALIDITY_HOURS: i64 = 12;

/// Scopes an app author may declare in `Policy::frontend_sdk_scopes`. No `apps:*`
/// scope, so the token cannot reach the mint endpoints and renew itself; the
/// `raw_app_sdk` sentinel narrows the rest (see `scopes.rs`).
pub const FRONTEND_SDK_ALLOWED_SCOPES: [&str; 5] = [
    "jobs:run",
    "jobs:read",
    "users:read",
    "resources:read",
    "variables:read",
];

/// Reject a policy declaring frontend SDK scopes outside the curated list.
/// Enforced on every policy write AND re-checked at mint time, so a policy
/// written by an older/foreign client can't broaden what gets minted.
fn validate_frontend_sdk_scopes_list(scopes: &[String]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for s in scopes {
        if !FRONTEND_SDK_ALLOWED_SCOPES.contains(&s.as_str()) {
            return Err(Error::BadRequest(format!(
                "Invalid frontend SDK scope '{}'. Allowed scopes: {}",
                s,
                FRONTEND_SDK_ALLOWED_SCOPES.join(", ")
            )));
        }
        // The viewer's permission prompt renders these as a keyed list, which a
        // repeat would break. The editor can't produce one; the API/CLI can.
        if !seen.insert(s.as_str()) {
            return Err(Error::BadRequest(format!(
                "Duplicate frontend SDK scope '{}'",
                s
            )));
        }
    }
    Ok(())
}

fn validate_frontend_sdk_scopes(policy: &Policy) -> Result<()> {
    if let Some(scopes) = &policy.frontend_sdk_scopes {
        validate_frontend_sdk_scopes_list(scopes)?;
    }
    Ok(())
}

/// Mint the viewer-identity token a raw app's bundle uses for `windmill-client`.
///
/// The CALLER MUST establish that `authed` may hold this app's credential: the
/// viewer endpoints verify read access, `mint_preview_sdk_token` requires
/// `apps:write:<path>`.
async fn mint_raw_app_sdk_token(
    db: &DB,
    w_id: &str,
    app_path: &str,
    authed: &ApiAuthed,
    scopes: &[String],
    job_id: Option<uuid::Uuid>,
) -> Result<(String, chrono::DateTime<chrono::Utc>)> {
    // This credential outlives the request, so an ephemeral job token must not be
    // able to launder itself into one — the reason `users/tokens/create` refuses.
    // The minted scopes do not contain it: `users/tokens/update_scopes` can widen
    // any token of the same email.
    forbid_elevated_job_token(db, &authed.email, job_id).await?;
    // An embed token represents untrusted app JS; it must not bootstrap a
    // broader SDK credential (same guard as `mint_app_embed_token`).
    if windmill_api_auth::scopes::has_app_embed_sentinel(authed.scopes.as_deref()) {
        return Err(Error::NotAuthorized(
            "App embed tokens cannot mint SDK tokens".to_string(),
        ));
    }
    validate_frontend_sdk_scopes_list(scopes)?;
    // A scope-restricted caller token must not bootstrap a broader-scoped SDK
    // token. No-op for unscoped browser sessions. Checked on the declared scopes
    // only — the sentinel grants nothing and exists to *narrow* the token.
    ensure_scopes_within_caller(authed, Some(scopes))?;
    let mut scopes = scopes.to_vec();
    scopes.push(windmill_api_auth::scopes::RAW_APP_SDK_SENTINEL.to_string());
    let requested_exp =
        chrono::Utc::now() + chrono::Duration::hours(APP_EMBED_TOKEN_VALIDITY_HOURS);
    let (label, expiration) =
        match guest_derived_token_constraints(db, authed, requested_exp).await? {
            Some((label, exp)) => {
                scopes.push(windmill_api_auth::scopes::GUEST_SENTINEL.to_string());
                (label, exp)
            }
            None => (format!("sdk_app:{app_path}"), requested_exp),
        };
    let token_config = NewToken::new(
        Some(label),
        Some(expiration),
        None,
        Some(scopes),
        Some(w_id.to_string()),
        // Never let an SDK token gain write capability the caller's own
        // session lacks.
        Some(authed.read_only),
    );
    let mut tx = db.begin().await?;
    let token = create_token_internal(&mut *tx, db, authed, token_config).await?;
    tx.commit().await?;
    Ok((token, expiration))
}

/// Label and expiry a token minted *by* a guest session must carry, or `None` for a
/// non-guest minter. The label is what lets it resolve; the caller pushes the `guest`
/// sentinel so every guest control still applies; the expiry is capped at the parent's,
/// since that expiry is a guest's only revocation short of logging out.
async fn guest_derived_token_constraints(
    db: &DB,
    authed: &ApiAuthed,
    requested: chrono::DateTime<chrono::Utc>,
) -> Result<Option<(String, chrono::DateTime<chrono::Utc>)>> {
    if !windmill_api_auth::scopes::has_guest_sentinel(authed.scopes.as_deref()) {
        return Ok(None);
    }
    // A guest JWT carries its own expiry and has no token row to look up; a signed-in
    // guest session is a row found by prefix (MIN is the conservative side of a
    // theoretical prefix collision). Either way the derived token caps on it, never on
    // a fresh interval.
    let parent_exp = if let Some(exp) = authed.credential_expiry {
        exp
    } else {
        let parent: Option<Option<chrono::DateTime<chrono::Utc>>> = sqlx::query_scalar(
            "SELECT MIN(expiration) FROM token WHERE token_prefix = $1 AND email = $2 AND label = $3",
        )
        .bind(authed.token_prefix.as_deref().unwrap_or(""))
        .bind(&authed.email)
        .bind(windmill_common::auth::GUEST_SESSION_LABEL)
        .fetch_optional(db)
        .await?;
        parent.flatten().ok_or_else(|| {
            Error::NotAuthorized("guest session not found or has no expiry".to_string())
        })?
    };
    Ok(Some((
        windmill_common::auth::GUEST_SESSION_LABEL.to_string(),
        requested.min(parent_exp),
    )))
}

/// Shared tail of the three embed-token endpoints: which credential the viewer
/// gets. Sandboxed low-code gets the embed token; a sandboxed raw app declaring
/// `frontend_sdk_scopes` gets the SDK token once `sdk_consent` is set — the
/// viewer's answer, not a boundary; the boundary is the scope set.
///
/// The CALLER MUST verify that `opt_authed` may view `app_path`.
pub async fn build_embed_token_response(
    db: &DB,
    w_id: &str,
    app_path: &str,
    raw_app: bool,
    policy: &EmbedPolicyView,
    opt_authed: Option<&ApiAuthed>,
    sdk_consent: bool,
    job_id: Option<uuid::Uuid>,
) -> Result<EmbedTokenResponse> {
    // Only advertise scopes where a token could actually be minted, so the viewer
    // is never shown a permission prompt that can grant nothing: an anonymous
    // visitor has no identity to mint against.
    // Frontend SDK access is sandbox-only: an unsandboxed bundle already runs with
    // the viewer's session, so a prompt there would suggest a boundary that isn't
    // one. A policy still carrying scopes with isolation off is simply inert.
    let sdk_scopes = if raw_app
        && policy.sandbox
        && opt_authed.is_some()
        && !policy.frontend_sdk_scopes.is_empty()
    {
        // An SDK token runs as the viewer, and a guest's session is the ceiling on what
        // it may delegate — the mint enforces that. Advertise only what a guest can
        // actually be granted, so the consent prompt never promises a scope the mint
        // would then refuse.
        let declared = policy.frontend_sdk_scopes.clone();
        let offered = match opt_authed {
            Some(a) if windmill_api_auth::scopes::has_guest_sentinel(a.scopes.as_deref()) => {
                let held = a.scopes.as_deref().unwrap_or_default();
                declared
                    .into_iter()
                    .filter(|sc| held.iter().any(|h| h == sc))
                    .collect::<Vec<_>>()
            }
            _ => declared,
        };
        (!offered.is_empty()).then_some(offered)
    } else {
        None
    };
    let (token, expiration) = if raw_app {
        match (&sdk_scopes, opt_authed) {
            (Some(scopes), Some(authed)) if sdk_consent => {
                let (t, e) =
                    mint_raw_app_sdk_token(db, w_id, app_path, authed, scopes, job_id).await?;
                (Some(t), Some(e))
            }
            _ => (None, None),
        }
    } else if policy.sandbox {
        let resp = mint_app_embed_token(db, w_id, app_path, opt_authed, job_id).await?;
        (resp.token, resp.expiration)
    } else {
        (None, None)
    };
    let viewer_email = sdk_scopes
        .is_some()
        .then(|| opt_authed.map(|a| a.email.clone()))
        .flatten();
    Ok(EmbedTokenResponse {
        token,
        expiration,
        raw_app,
        sandbox: policy.sandbox,
        app_path: Some(app_path.to_string()),
        workspace_id: Some(w_id.to_string()),
        sdk_scopes,
        viewer_email,
    })
}

#[derive(Deserialize)]
pub struct PreviewSdkTokenRequest {
    /// The app being edited. May not exist yet (a draft), so this is only used for
    /// the write-scope check and the token label.
    pub path: String,
    /// Scopes from the policy currently being edited, so the preview behaves like
    /// the deployed app instead of the last-deployed policy.
    pub scopes: Vec<String>,
}

/// Mint an SDK token for the raw-app editor's own preview. The author grants it
/// to themselves — minted from their session and capped the same way, so it
/// conveys nothing `users/tokens/create` wouldn't. Scopes come from the request
/// so the preview matches an app whose scopes aren't deployed yet, as
/// `execute_component` already does for editor previews.
async fn mint_preview_sdk_token(
    authed: ApiAuthed,
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<PreviewSdkTokenRequest>,
) -> Result<String> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot preview raw apps".to_string(),
        ));
    }
    check_scopes(&authed, || format!("apps:write:{}", req.path))?;
    let (token, _expiration) =
        mint_raw_app_sdk_token(&db, &w_id, &req.path, &authed, &req.scopes, job_id).await?;
    Ok(token)
}

/// Query for the embed-token endpoints.
#[derive(Deserialize)]
pub struct EmbedTokenQuery {
    /// Raw apps: the viewer confirmed the SDK-permissions consent banner, so the
    /// viewer-scoped SDK token may actually be minted. Defaults to false — the
    /// first fetch only advertises the declared scopes.
    #[serde(default)]
    pub sdk_consent: bool,
}

#[derive(Serialize)]
pub struct EmbedTokenResponse {
    /// Narrowly-scoped token for the iframe. `None` for fully anonymous access
    /// (the iframe then calls the public endpoints anonymously).
    pub token: Option<String>,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
    /// WIN-2006: raw apps render single-iframe (the bundle is already isolated in
    /// its own opaque iframe), so the viewer skips the opaque-viewer indirection
    /// and the embed token entirely — it loads the app with the page credential.
    #[serde(default)]
    pub raw_app: bool,
    /// WIN-2006: publisher opted this app into sandbox isolation. When false the
    /// viewer runs the app same-origin with its full session (the default,
    /// pre-isolation behavior).
    #[serde(default)]
    pub sandbox: bool,
    /// WIN-2006: the resolved app path. The embedder uses it (together with
    /// `workspace_id`) to scope the app's backing `localStorage` per app (so
    /// sandboxed apps don't share one store). Not a new disclosure — the viewer
    /// already receives `path` when it loads the app (e.g. `get_public_app_by_secret`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_path: Option<String>,
    /// WIN-2006: the resolved workspace. Pairs with `app_path` for the per-app
    /// `localStorage` key so two apps at the same path in different workspaces don't
    /// share a store. For custom-path apps the viewer can't derive this itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// Raw apps: scopes the app policy declares for the frontend SDK token. The
    /// viewer renders these in the permission prompt; `token` stays `None` until
    /// the endpoint is re-called with `sdk_consent=true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdk_scopes: Option<Vec<String>>,
    /// The caller's own email, so the viewer can key its stored "do not ask
    /// again" per person — that localStorage is shared across everyone using the
    /// browser. Set alongside `sdk_scopes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewer_email: Option<String>,
}

/// Mint a short-lived, narrowly-scoped embed token for `app_path` when a caller
/// is authenticated. When `opt_authed` is `None` (anonymous access to an
/// anonymous app) no token is minted and the iframe relies on the public
/// endpoints.
///
/// The CALLER MUST verify the viewer's access to `app_path` before calling: this
/// mints a token on behalf of `opt_authed` unconditionally (DB access remains
/// gated by the viewer's own RLS, but the token's existence is not access-checked
/// here). All current call sites (`get_app_embed_token`,
/// `get_app_embed_token_for_path`, and the EE custom-path variant) do this.
///
/// Scope confinement IS enforced here: the minted scopes must be within the
/// caller's own (`ensure_scopes_within_caller`), so a scope-restricted bearer
/// token cannot bootstrap a broader-scoped embed token. For the normal caller —
/// an unscoped browser session — this is a no-op and the mint is purely
/// narrowing.
pub async fn mint_app_embed_token(
    db: &DB,
    w_id: &str,
    app_path: &str,
    opt_authed: Option<&ApiAuthed>,
    job_id: Option<uuid::Uuid>,
) -> Result<EmbedTokenResponse> {
    let token_and_exp = if let Some(authed) = opt_authed {
        // This credential outlives the request and its narrow scopes are not the
        // boundary — `users/tokens/update_scopes` can widen any same-email token —
        // so an elevated job token must not mint one (GHSA-hfh4-cx4h-3fcr).
        forbid_elevated_job_token(db, &authed.email, job_id).await?;
        // An app embed token represents untrusted app JS in the sandboxed iframe; it
        // must never reach this mint path to renew itself. The 12h expiry is the
        // blast-radius cap on a leaked embed token, and `ensure_scopes_within_caller`
        // below would pass a same-scoped renewal (the requested scopes equal the
        // caller's own), making the credential indefinitely self-renewable. Refresh
        // minting is the trusted embedder session/JWT's job.
        if windmill_api_auth::scopes::has_app_embed_sentinel(authed.scopes.as_deref()) {
            return Err(Error::NotAuthorized(
                "App embed tokens cannot mint or renew embed tokens".to_string(),
            ));
        }
        let requested_exp =
            chrono::Utc::now() + chrono::Duration::hours(APP_EMBED_TOKEN_VALIDITY_HOURS);
        let mut scopes: Vec<String> = APP_EMBED_SCOPES
            .iter()
            .filter(|s| **s != windmill_api_auth::scopes::APP_EMBED_SENTINEL)
            .map(|s| s.to_string())
            .collect();
        // Path-scoped read so the app can fetch its OWN definition (apps/get/p,
        // which the in-workspace sandboxed viewer uses) — but no other app's. The
        // public viewer fetches via apps_u/public_app and doesn't rely on this.
        scopes.push(format!("apps:read:{app_path}"));
        // Path-scoped run (NOT unqualified `apps:run`) so the token can only execute
        // THIS app's components: `execute_component` re-checks `apps:run:<path>` for
        // the requested app, so the token can't drive another app's runnables.
        scopes.push(format!("apps:run:{app_path}"));
        // A scope-restricted caller token must not bootstrap a broader-scoped
        // embed token (`create_token_internal` deliberately does not check this
        // itself). Checked on the real scopes only: a sentinel is a one-part string
        // that `ScopeDefinition::from_scope_string` rejects, so leaving it in the
        // requested set makes this fail outright for any scoped caller — which a
        // guest session is. `mint_raw_app_sdk_token` has the same shape.
        ensure_scopes_within_caller(authed, Some(&scopes))?;
        scopes.push(windmill_api_auth::scopes::APP_EMBED_SENTINEL.to_string());
        let (label, expiration) =
            match guest_derived_token_constraints(db, authed, requested_exp).await? {
                Some((label, exp)) => {
                    scopes.push(windmill_api_auth::scopes::GUEST_SENTINEL.to_string());
                    (label, exp)
                }
                None => (format!("embed_app:{app_path}"), requested_exp),
            };
        let token_config = NewToken::new(
            Some(label),
            Some(expiration),
            None,
            Some(scopes),
            Some(w_id.to_string()),
            // Never let an embed token gain write capability the caller's own
            // session lacks.
            Some(authed.read_only),
        );
        let mut tx = db.begin().await?;
        let token = create_token_internal(&mut *tx, db, authed, token_config).await?;
        tx.commit().await?;
        Some((token, expiration))
    } else {
        None
    };

    Ok(EmbedTokenResponse {
        token: token_and_exp.as_ref().map(|(t, _)| t.clone()),
        expiration: token_and_exp.map(|(_, e)| e),
        raw_app: false,
        sandbox: false,
        app_path: Some(app_path.to_string()),
        workspace_id: Some(w_id.to_string()),
        sdk_scopes: None,
        viewer_email: None,
    })
}

#[derive(Serialize)]
pub struct GuestEntry {
    /// The workspace and app path to name when starting a guest sign-in. The
    /// workspace is redundant on the secret route and load-bearing on the custom-path
    /// one, which may not carry it in its URL.
    pub workspace_id: String,
    pub app_path: String,
}

/// Whether the app behind this share secret admits guests, and under what path.
///
/// Unauthenticated on purpose: it is what a signed-out visitor reads to learn that
/// signing in would get them in. It discloses only the app's path, to a caller who
/// already holds the share secret — the secret is the capability here. A 404 when the
/// app is not open to guests, so it says nothing about apps that are not.
async fn get_guest_entry(
    Extension(db): Extension<DB>,
    Path((w_id, secret)): Path<(String, String)>,
) -> JsonResult<GuestEntry> {
    let id = get_id_from_secret(&db, &w_id, secret, None).await?;
    let app = sqlx::query!(
        "SELECT path FROM app WHERE id = $1 AND workspace_id = $2",
        id,
        &w_id
    )
    .fetch_optional(&db)
    .await?;
    let app = not_found_if_none(app, "App", id.to_string())?;
    if !windmill_common::workspaces::guest_app_admits(&db, &w_id, &app.path).await? {
        return Err(Error::NotFound("App is not open to guests".to_string()));
    }
    Ok(Json(GuestEntry { workspace_id: w_id, app_path: app.path }))
}

/// Issue an embed token for a public app addressed by its (secret) share id.
/// Mirrors the access check in [`get_public_app_by_secret`]: anonymous apps are
/// reachable without auth, otherwise the caller must be logged in and have read
/// access to the app.
async fn get_app_embed_token(
    OptAuthed(opt_authed): OptAuthed,
    // Resolved from the token when there is one: this endpoint serves anonymous
    // viewers too, so it cannot take an extractor that requires auth.
    OptTokened { token }: OptTokened,
    Extension(cache): Extension<Arc<AuthCache>>,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, secret)): Path<(String, String)>,
    Query(sdk_query): Query<EmbedTokenQuery>,
) -> JsonResult<EmbedTokenResponse> {
    let job_id = match token.as_deref() {
        Some(t) => cache
            .get_opt_job_authed(Some(w_id.clone()), t)
            .await
            .and_then(|x| x.job_id),
        None => None,
    };
    let id = get_id_from_secret(&db, &w_id, secret, None).await?;

    let app = sqlx::query!(
        "SELECT a.path, a.policy::text as policy, a.versions[array_upper(a.versions, 1)] as version, av.raw_app as raw_app
         FROM app a JOIN app_version av ON av.id = a.versions[array_upper(a.versions, 1)]
         WHERE a.id = $1 AND a.workspace_id = $2",
        id,
        &w_id
    )
    .fetch_optional(&db)
    .await?;
    let app = not_found_if_none(app, "App", id.to_string())?;
    let raw_app = app.raw_app;
    let policy_str = app
        .policy
        .ok_or_else(|| Error::internal_err("App policy missing".to_string()))?;
    // Lenient field-level read instead of a strict `Policy` parse: a legacy app
    // whose stored policy predates newer required fields must still resolve to
    // its (unsandboxed) render here rather than erroring out of the viewer.
    let policy = parse_embed_policy(&policy_str)?;

    let authed_for_token = if policy.anonymous_execution {
        // Anonymous app: still mint a scoped token if the viewer happens to be
        // logged in (so the app sees their identity), otherwise stay anonymous. A
        // guest's session names another app and cannot contain this one's scopes,
        // so it renders anonymously here rather than being refused.
        opt_authed.filter(|a| !windmill_api_auth::scopes::has_guest_sentinel(a.scopes.as_deref()))
    } else {
        let mode = if policy.guest_execution {
            ExecutionMode::Guest
        } else {
            ExecutionMode::Publisher
        };
        authorize_app_viewer(mode, &app.path, id, &w_id, &user_db, &opt_authed).await?;
        opt_authed
    };

    let resp = build_embed_token_response(
        &db,
        &w_id,
        &app.path,
        raw_app,
        &policy,
        authed_for_token.as_ref(),
        sdk_query.sdk_consent,
        job_id,
    )
    .await?;
    Ok(Json(resp))
}

/// Minimal, lenient view of an app policy for the embed-token endpoints
/// (WIN-2006). Reads only the fields the sandbox decision needs, via
/// `serde_json::Value`, so a legacy policy that no longer satisfies the strict
/// [`Policy`] struct (e.g. `triggerables_v2` entries predating now-required
/// fields) still renders instead of failing the viewer with "Not found".
/// A missing/unknown `execution_mode` is treated as NOT anonymous — the
/// strictest access interpretation.
pub struct EmbedPolicyView {
    pub anonymous_execution: bool,
    /// Open to anyone the identity provider authenticates. Like
    /// `anonymous_execution`, an unknown mode reads as `false` — the strict side.
    pub guest_execution: bool,
    pub sandbox: bool,
    /// Raw apps: author-declared scopes for the frontend SDK token; empty when
    /// the app doesn't use the frontend SDK (non-string entries are ignored).
    pub frontend_sdk_scopes: Vec<String>,
}

pub fn parse_embed_policy(policy_str: &str) -> Result<EmbedPolicyView> {
    let v: serde_json::Value = serde_json::from_str(policy_str).map_err(to_anyhow)?;
    Ok(EmbedPolicyView {
        anonymous_execution: v.get("execution_mode").and_then(|m| m.as_str()) == Some("anonymous"),
        guest_execution: v.get("execution_mode").and_then(|m| m.as_str()) == Some("guest"),
        sandbox: v.get("sandbox").and_then(|b| b.as_bool()).unwrap_or(false),
        frontend_sdk_scopes: v
            .get("frontend_sdk_scopes")
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// Authenticated, path-based embed token for the in-workspace app viewer
/// (WIN-2006). Mirrors [`get_app_embed_token`] but keyed by app path and gated by
/// the caller's read access (RLS), so the logged-in `/apps/get` viewer can render
/// the app sandboxed — isolated from the member's full session — using the same
/// scoped token. Raw apps get no token (single-iframe with the page credential).
async fn get_app_embed_token_for_path(
    authed: ApiAuthed,
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(sdk_query): Query<EmbedTokenQuery>,
) -> JsonResult<EmbedTokenResponse> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    // RLS: the caller must have read access to this app, otherwise it's not found.
    let mut tx = user_db.begin(&authed).await?;
    let app = sqlx::query!(
        "SELECT a.policy::text as policy, a.versions[array_upper(a.versions, 1)] as version, av.raw_app as raw_app
         FROM app a JOIN app_version av ON av.id = a.versions[array_upper(a.versions, 1)]
         WHERE a.path = $1 AND a.workspace_id = $2",
        path,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let app = not_found_if_none(app, "App", path)?;
    let raw_app = app.raw_app;
    let policy_str = app
        .policy
        .ok_or_else(|| Error::internal_err("App policy missing".to_string()))?;
    // Lenient parse — see [`get_app_embed_token`] for the rationale (identical here).
    let policy = parse_embed_policy(&policy_str)?;

    let resp = build_embed_token_response(
        &db,
        &w_id,
        path,
        raw_app,
        &policy,
        Some(&authed),
        sdk_query.sdk_consent,
        job_id,
    )
    .await?;
    Ok(Json(resp))
}

async fn get_id_from_secret(
    db: &DB,
    w_id: &str,
    secret: String,
    prefix: Option<&str>,
) -> Result<i64> {
    let mc = build_crypt(db, w_id).await?;
    let decrypted = mc
        .decrypt_bytes_to_bytes(&(hex::decode(secret)?))
        .map_err(|e| Error::internal_err(e.to_string()))?;
    let mut bytes = str::from_utf8(&decrypted).map_err(to_anyhow)?;
    if let Some(prefix) = prefix {
        if !bytes.starts_with(prefix) {
            return Err(Error::BadRequest("Invalid secret".to_string()));
        }
        bytes = bytes.strip_prefix(prefix).unwrap_or("");
    }
    let id: i64 = bytes.parse().map_err(to_anyhow)?;
    Ok(id)
}

async fn get_public_resource(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Option<serde_json::Value>> {
    let path = path.to_path();

    // This endpoint is unauthenticated (anonymous public apps must fetch their
    // theme and form schemas). Both branches MUST stay tightly constrained to
    // the non-sensitive resource types they serve, otherwise an unauthenticated
    // caller could read the raw value of any resource at the given path.
    let res = if path.starts_with("f/app_themes/") {
        sqlx::query_scalar!(
            "SELECT value from resource WHERE path = $1 AND workspace_id = $2 AND resource_type = 'app_theme'",
            path.to_owned(),
            &w_id
        )
        .fetch_optional(&db)
        .await?
    } else {
        sqlx::query_scalar!(
            "SELECT value from resource WHERE path = $1 AND workspace_id = $2 AND resource_type = 'json_schema'",
            path.to_owned(),
            &w_id
        )
        .fetch_optional(&db)
        .await?
    };

    Ok(Json(res.flatten()))
}

async fn get_secret_id(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let id_o = sqlx::query_scalar!(
        "SELECT app.id FROM app
        WHERE app.path = $1 AND app.workspace_id = $2",
        path,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let id = not_found_if_none(id_o, "App", path.to_string())?;

    let mc = build_crypt(&db, &w_id).await?;

    let hx = hex::encode(mc.encrypt_str_to_bytes(id.to_string()));

    Ok(hx)
}

const BUNDLE_SECRET_PREFIX: &str = "bundle_";

pub async fn compute_bundle_secret(db: &DB, w_id: &str, versions: &[i64]) -> Result<String> {
    let version_id = versions
        .last()
        .ok_or_else(|| Error::internal_err("App has no versions".to_string()))?;
    let mc = build_crypt(db, w_id).await?;
    let hx =
        hex::encode(mc.encrypt_str_to_bytes(format!("{}{}", BUNDLE_SECRET_PREFIX, version_id)));
    Ok(hx)
}

async fn get_latest_version_secret_id(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let id_o = sqlx::query_scalar!(
        "SELECT app.versions[array_upper(app.versions, 1)] FROM app
        WHERE app.path = $1 AND app.workspace_id = $2",
        path,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    tx.commit().await?;

    let id = not_found_if_none(id_o, "App", path.to_string())?;

    let mc = build_crypt(&db, &w_id).await?;

    let hx = hex::encode(mc.encrypt_str_to_bytes(format!("{}{}", BUNDLE_SECRET_PREFIX, id)));

    Ok(hx)
}

async fn store_raw_app_file<'a>(
    w_id: &str,
    id: &i64,
    file_type: &str,
    data: bytes::Bytes,
    tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
) -> Result<()> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    {
        let object_store = windmill_object_store::get_object_store().await;

        let path: String = format!("/app_bundles/{}/{}.{}", w_id, id, file_type);

        if let Some(os) = object_store {
            if let Err(e) = os
                .put(
                    &windmill_object_store::object_store_reexports::Path::from(path.clone()),
                    data.into(),
                )
                .await
            {
                tracing::error!("Failed to put snapshot to s3 at {path}: {:?}", e);
                return Err(windmill_common::error::Error::ExecutionErr(format!(
                    "Failed to put {path} to s3"
                )));
            }
            tracing::info!("Successfully put snapshot to s3 at {path}");
            return Ok(());
        }
    }

    sqlx::query!(
        "INSERT INTO app_bundles (app_version_id, w_id, file_type, data) VALUES ($1, $2, $3, $4)",
        id,
        w_id,
        file_type,
        data.to_vec()
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}
macro_rules! process_app_multipart {
    ($authed:expr, $user_db:expr, $db:expr, $w_id:expr, $path:expr, $multipart:expr, $internal_fn:expr) => {
        async {
            let mut saved_app = None;
            let mut uploaded_js = false;

            let request_size_limit_mb = *crate::REQUEST_SIZE_LIMIT.read().await / (1024 * 1024);
            let raw_app_limit_mb = request_size_limit_mb * 5;
            let mut multipart = $multipart;
            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|e| Error::BadRequest(format!("failed to read multipart field: {e}. Could be due to the request size limit for raw app bundles which is {raw_app_limit_mb}MB (adjustable in instance settings)")))?
            {
                let name = field
                    .name()
                    .ok_or_else(|| Error::BadRequest("multipart field missing name".to_string()))?
                    .to_string();
                let data = field.bytes().await.map_err(|e| {
                    Error::BadRequest(format!("failed to read multipart stream: {e}. Could be due to the request size limit for raw app bundles which is {raw_app_limit_mb}MB (adjustable in instance settings)"))
                })?;
                if name == "app" {
                    let app = serde_json::from_slice(&data).map_err(to_anyhow)?;
                    let (ntx, npath, nid) = $internal_fn(
                        $authed.clone(),
                        $db.clone(),
                        $user_db.clone(),
                        $w_id,
                        $path,
                        true,
                        app,
                    )
                    .await?;
                    saved_app = Some((npath, nid, ntx));
                } else if name == "js" {
                    if let Some((_npath, id, tx)) = saved_app.as_mut() {
                        store_raw_app_file($w_id, &id, "js", data, tx).await?;
                        uploaded_js = true;
                    } else {
                        return Err(Error::BadRequest(
                            "App payload need to be created first".to_string(),
                        ));
                    }
                } else if name == "css" {
                    if let Some((_npath, id, tx)) = saved_app.as_mut() {
                        store_raw_app_file($w_id, &id, "css", data, tx).await?;
                    } else {
                        return Err(Error::BadRequest(
                            "App payload need to be created first".to_string(),
                        ));
                    }
                } else {
                    return Err(Error::BadRequest(format!("Unsupported field: {}", name)));
                }
            }
            if !uploaded_js {
                return Err(Error::BadRequest("js or css file not uploaded".to_string()));
            }
            if let Some((npath, id, tx)) = saved_app {
                tx.commit().await?;
                Ok((npath, id))
            } else {
                Err(Error::BadRequest("App not created".to_string()))
            }
        }
    };
}

async fn create_app_raw<'a>(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    multipart: Multipart,
) -> Result<(StatusCode, String)> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot create apps for security reasons".to_string(),
        ));
    }

    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let (path, _id) = process_app_multipart!(
        authed,
        user_db,
        db,
        &w_id,
        "",
        multipart,
        |authed, db, user_db, w_id, _path, raw_app, app| create_app_internal(
            authed, db, user_db, w_id, raw_app, app
        )
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateApp { workspace: w_id, path: path.clone() },
    );
    Ok((StatusCode::CREATED, path))
}

async fn list_paths_from_workspace_runnable(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, runnable_kind, path)): Path<(String, RunnableKind, StripPath)>,
) -> JsonResult<Vec<String>> {
    let mut tx = user_db.begin(&authed).await?;
    let runnables = sqlx::query_scalar!(
        r#"SELECT a.path
            FROM workspace_runnable_dependencies wru 
            JOIN app a
                ON wru.app_path = a.path AND wru.workspace_id = a.workspace_id
            WHERE wru.runnable_path = $1 AND wru.runnable_is_flow = $2 AND wru.workspace_id = $3"#,
        path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(runnables))
}

async fn create_app(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(app): Json<CreateApp>,
) -> Result<(StatusCode, String)> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot create apps for security reasons".to_string(),
        ));
    }
    let path = app.path.clone();

    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    // scope is enforced inside create_app_internal, before any persistence.
    let (new_tx, _path, _id) = create_app_internal(authed, db, user_db, &w_id, false, app).await?;

    new_tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateApp { workspace: w_id, path: path.clone() },
    );

    Ok((StatusCode::CREATED, path))
}

/// Actionable error when a custom path is already taken. In global mode (not
/// CLOUD_HOSTED and `app_workspaced_route` off) custom paths are unique across
/// the whole instance, so the conflicting copy may live in another workspace
/// (e.g. the same app deployed/git-synced to staging and prod) — name it so
/// the operator knows exactly what to remove.
fn custom_path_conflict_error(
    custom_path: &str,
    conflict_path: &str,
    conflict_workspace: &str,
    scoped: bool,
) -> Error {
    if scoped {
        Error::BadRequest(format!(
            "Custom path '{}' is already used by app '{}' in this workspace",
            custom_path, conflict_path
        ))
    } else {
        Error::BadRequest(format!(
            "Custom path '{}' is already used by app '{}' in workspace '{}'. \
             Custom paths must be unique across the whole instance unless the \
             'app_workspaced_route' instance setting is enabled.",
            custom_path, conflict_path, conflict_workspace
        ))
    }
}

/// A raw app's `value.files` paths and `value.runnables` ids both become on-disk
/// paths a puller writes under the app folder (`wmill sync pull`): file keys are
/// app-root-relative paths, runnable ids name `<backend>/<id>.yaml`. A `..`
/// segment in either resolves outside that folder, so reject it at deploy time:
/// a pulled file must stay within its own app's folder.
///
/// Validate the value as it will be *stored and later read*, not as it arrived:
/// the INSERT strips NUL escapes (a `/..\u0000/` key becomes `/../` on disk) and
/// a puller parses with last-duplicate-wins JSON semantics. Mirror both — strip
/// first, then parse to `serde_json::Value` — so a key that looks safe on the
/// wire but is unsafe once stored cannot slip past.
fn validate_raw_app_path_keys(value: &RawValue) -> Result<()> {
    let stored = strip_json_nul(value.get());
    // Fail closed on a value we can't parse: `Box<RawValue>` accepts JSON nested
    // past serde's recursion limit while `Value` does not, so a value that fails
    // here could hide a traversal key in a part we never inspected.
    let parsed: serde_json::Value = serde_json::from_str(&stored)
        .map_err(|e| Error::BadRequest(format!("raw app value could not be validated: {e}")))?;
    for map in ["files", "runnables"] {
        let Some(entries) = parsed.get(map).and_then(|f| f.as_object()) else {
            continue;
        };
        for key in entries.keys() {
            // Mirror the CLI's write: it strips one leading `/` and joins the rest
            // under the app folder. Any `..` segment (either separator, since a
            // checkout can be pulled on Windows) escapes it.
            let rel = key.strip_prefix('/').unwrap_or(key);
            if rel.split(['/', '\\']).any(|seg| seg == "..") {
                return Err(Error::BadRequest(format!(
                    "raw app {map} path {key:?} is not allowed: keys must stay within \
                     the app folder and cannot contain '..' segments"
                )));
            }
        }
    }
    Ok(())
}

async fn create_app_internal<'a>(
    authed: ApiAuthed,
    db: sqlx::Pool<sqlx::Postgres>,
    user_db: UserDB,
    w_id: &String,
    raw_app: bool,
    mut app: CreateApp,
) -> Result<(sqlx::Transaction<'a, sqlx::Postgres>, String, i64)> {
    // Enforce scope before any persistence: the raw-app create path commits
    // inside process_app_multipart!, so checking after this call would leave a
    // denied app committed in the DB.
    check_scopes(&authed, || format!("apps:write:{}", &app.path))?;
    validate_frontend_sdk_scopes(&app.policy)?;
    if raw_app {
        validate_raw_app_path_keys(&app.value.0)?;
    }
    if *CLOUD_HOSTED {
        let nb_apps =
            sqlx::query_scalar!("SELECT COUNT(*) FROM app WHERE workspace_id = $1", &w_id)
                .fetch_one(&db)
                .await?;
        if nb_apps.unwrap_or(0) >= 1000 {
            return Err(Error::BadRequest(
                    "You have reached the maximum number of apps (1000) on cloud. Check your usage in Workspace Settings > General > Cloud Quotas. Contact support@windmill.dev to increase the limit"
                        .to_string(),
                ));
        }
        if app.summary.len() > 300 {
            return Err(Error::BadRequest(
                "Summary must be less than 300 characters on cloud".to_string(),
            ));
        }
    }
    // Resolve the on-behalf-of defaults on the (non-RLS) pool *before* opening
    // the RLS transaction below: doing these lookups mid-transaction would hold
    // a second simultaneous connection while `tx` is still checked out.
    let should_preserve = app.preserve_on_behalf_of.unwrap_or(false)
        && windmill_common::can_preserve_on_behalf_of(&authed)
        && app.policy.on_behalf_of.is_some();

    if !should_preserve {
        let folder_default = if windmill_common::can_preserve_on_behalf_of(&authed) {
            windmill_common::folders::resolve_folder_default_permissioned_as(&db, w_id, &app.path)
                .await?
        } else {
            None
        };
        if let Some(default_permissioned_as) = folder_default {
            let default_email = windmill_common::users::get_email_from_permissioned_as(
                &default_permissioned_as,
                w_id,
                &db,
            )
            .await?;
            app.policy.on_behalf_of = Some(default_permissioned_as);
            app.policy.on_behalf_of_email = Some(default_email);
        } else {
            app.policy.on_behalf_of = Some(username_to_permissioned_as(&authed.username));
            app.policy.on_behalf_of_email = Some(authed.email.clone());
        }
    }

    // Reject a forged superadmin run identity in the (possibly preserved) policy.
    // Done on the non-RLS pool before the transaction below, like the resolution
    // above, to avoid holding a second connection while `tx` is checked out.
    windmill_common::auth::validate_on_behalf_of(
        app.policy.on_behalf_of.as_deref(),
        app.policy.on_behalf_of_email.as_deref(),
    )?;

    let mut tx = user_db.clone().begin(&authed).await?;
    let path = app.path.clone();
    if &app.path == "" {
        return Err(Error::BadRequest("App path cannot be empty".to_string()));
    }
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2)",
        &app.path,
        w_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);
    if exists {
        return Err(Error::BadRequest(format!(
            "App with path {} already exists",
            &app.path
        )));
    }
    if let Some(custom_path) = &app.custom_path {
        require_admin(authed.is_admin, &authed.username)?;
        let scoped =
            *CLOUD_HOSTED || APP_WORKSPACED_ROUTE.load(std::sync::atomic::Ordering::Relaxed);

        let conflict = sqlx::query!(
            "SELECT workspace_id, path FROM app WHERE custom_path = $1 AND ($2::TEXT IS NULL OR workspace_id = $2) LIMIT 1",
            custom_path,
            if scoped { Some(w_id) } else { None }
        )
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(conflict) = conflict {
            return Err(custom_path_conflict_error(
                custom_path,
                &conflict.path,
                &conflict.workspace_id,
                scoped,
            ));
        }
    }
    // Pin the mode the app is created under, so the stored policy states one
    // even when the caller did not.
    app.policy.set_execution_mode(app.policy.execution_mode());
    refuse_unscopable_guest_app(&app.path, app.policy.execution_mode())?;
    if let Some(rule) = deployment_rule_for_mode(app.policy.execution_mode()) {
        if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
            w_id,
            &rule,
            &authed.username,
            &authed.groups,
            authed.is_admin,
            &db,
        )
        .await?
        {
            return Err(Error::PermissionDenied(msg));
        }
    }
    // CLI / git-sync deploys ask us to preserve any existing user draft at this
    // path instead of wiping it as part of the deploy. Only wipe the deployer's
    // own draft (plus the legacy NULL-email row); see scripts.rs.
    if !app.skip_draft_deletion.unwrap_or(false) {
        sqlx::query!(
            "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ IN ('app', 'raw_app') \
             AND (email = $3 OR email IS NULL)",
            &app.path,
            &w_id,
            &authed.email,
        )
        .execute(&mut *tx)
        .await?;
    }
    let id = sqlx::query_scalar!(
        "INSERT INTO app
            (workspace_id, path, summary, policy, versions, custom_path, labels)
            VALUES ($1, $2, $3, $4, '{}', $5, $6) RETURNING id",
        w_id,
        app.path,
        app.summary,
        json!(app.policy),
        app.custom_path
            .as_ref()
            .map(|s| if s.is_empty() { None } else { Some(s) })
            .flatten(),
        app.labels.as_deref() as Option<&[String]>
    )
    .fetch_one(&mut *tx)
    .await?;
    // `.get()` keeps the raw text (and thus key order); strip any NUL so the
    // `json`→`jsonb` conversion downstream (fork, indexing) can't choke on it.
    let value = strip_json_nul(app.value.0.get());
    if matches!(value, Cow::Owned(_)) {
        tracing::warn!(path = %app.path, "stripped NUL character(s) from app value on create");
    }
    let v_id = sqlx::query_scalar!(
        "INSERT INTO app_version
            (app_id, value, created_by, raw_app)
            VALUES ($1, $2::text::json, $3, $4) RETURNING id",
        id,
        value.as_ref(),
        authed.username,
        raw_app
    )
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE app SET versions = array_append(versions, $1::bigint) WHERE id = $2",
        v_id,
        id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "apps.create",
        ActionKind::Create,
        w_id,
        Some(&app.path),
        None,
    )
    .await?;
    if should_preserve {
        if let Some(ref obo_email) = app.policy.on_behalf_of_email {
            if obo_email != &authed.email {
                audit_log(
                    &mut *tx,
                    &authed,
                    "apps.on_behalf_of",
                    ActionKind::Create,
                    w_id,
                    Some(&app.path),
                    Some([("on_behalf_of", obo_email.as_str()), ("action", "create")].into()),
                )
                .await?;
            }
        }
    }
    let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
    if let Some(dm) = &app.deployment_message {
        args.insert("deployment_message".to_string(), to_raw_value(&dm));
    }
    let tx = PushIsolationLevel::Transaction(tx);
    let (dependency_job_uuid, new_tx) = push(
        &db,
        tx,
        w_id,
        JobPayload::AppDependencies {
            path: app.path.clone(),
            version: v_id,
            debouncing_settings: Default::default(),
        },
        PushArgs { args: &args, extra: None },
        &authed.username,
        &authed.email,
        windmill_common::users::username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        authed.username_override.as_deref(),
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        None,
        None,
        None,
        None,
        Some(&authed.clone().into()),
        false,
        None,
        None,
        None,
    )
    .await?;
    tracing::info!("Pushed app dependency job {}", dependency_job_uuid);

    Ok((new_tx, path, v_id))
}

async fn list_hub_apps(Extension(db): Extension<DB>) -> impl IntoResponse {
    let (status_code, headers, body) = query_elems_from_hub(
        &HTTP_CLIENT,
        &format!("{}/searchUiData?approved=true", **HUB_BASE_URL.load()),
        None,
        &db,
    )
    .await?;
    Ok::<_, Error>((status_code, headers, body))
}

pub async fn get_hub_app_by_id(
    Path(id): Path<i32>,
    Extension(db): Extension<DB>,
) -> JsonResult<Box<serde_json::value::RawValue>> {
    let value = http_get_from_hub(
        &HTTP_CLIENT,
        &format!("{}/apps/{}/json", **HUB_BASE_URL.load(), id),
        false,
        None,
        Some(&db),
    )
    .await?
    .json()
    .await
    .map_err(to_anyhow)?;
    Ok(Json(value))
}

pub async fn get_hub_raw_app_by_id(
    Path(id): Path<i32>,
    Extension(db): Extension<DB>,
) -> JsonResult<Box<serde_json::value::RawValue>> {
    let value = http_get_from_hub(
        &HTTP_CLIENT,
        &format!("{}/raw_apps/{}/json", **HUB_BASE_URL.load(), id),
        false,
        None,
        Some(&db),
    )
    .await?
    .json()
    .await
    .map_err(to_anyhow)?;
    Ok(Json(value))
}

async fn delete_app(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot delete apps for security reasons".to_string(),
        ));
    }
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:write:{}", path))?;

    if path == "g/all/setup_app" && w_id == "admins" {
        return Err(Error::BadRequest(
            "Cannot delete the global setup app".to_string(),
        ));
    }

    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    // Check if it's a raw app before deletion
    let is_raw_app = sqlx::query_scalar!(
        "SELECT app_version.raw_app FROM app
         JOIN app_version ON app_version.id = app.versions[array_upper(app.versions, 1)]
         WHERE app.path = $1 AND app.workspace_id = $2",
        path,
        &w_id
    )
    .fetch_optional(&db)
    .await?
    .unwrap_or(false);

    let mut tx = user_db.begin(&authed).await?;

    // Capture all related data for trashbin before deleting (CASCADE will remove app_version, etc.)
    let trash_app: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT to_jsonb(t) FROM app t WHERE path = $1 AND workspace_id = $2")
            .bind(path)
            .bind(&w_id)
            .fetch_optional(&mut *tx)
            .await?;

    let trash_app_versions: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM app_version t WHERE app_id = (SELECT id FROM app WHERE path = $1 AND workspace_id = $2)",
    )
    .bind(path)
    .bind(&w_id)
    .fetch_all(&mut *tx)
    .await?;

    let trash_drafts: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM draft t WHERE path = $1 AND workspace_id = $2 AND typ IN ('app', 'raw_app')",
    )
    .bind(path)
    .bind(&w_id)
    .fetch_all(&mut *tx)
    .await?;

    // Both `app` and `raw_app` draft kinds back the same `app` table.
    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ IN ('app', 'raw_app')",
        path,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM app WHERE path = $1 AND workspace_id = $2",
        path,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    if let Some(app_data) = trash_app {
        let mut trash_data = serde_json::json!({"row": app_data});
        if !trash_app_versions.is_empty() {
            trash_data["app_versions"] = serde_json::Value::Array(trash_app_versions);
        }
        if !trash_drafts.is_empty() {
            trash_data["drafts"] = serde_json::Value::Array(trash_drafts);
        }
        windmill_common::trashbin::move_to_trash(
            &mut *tx,
            &w_id,
            "app",
            path,
            trash_data,
            &authed.username,
        )
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "apps.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    let deployed_object = if is_raw_app {
        DeployedObject::RawApp {
            path: path.to_string(),
            parent_path: Some(path.to_string()),
            version: 0, // dummy version as it will not get inserted in db
        }
    } else {
        DeployedObject::App {
            path: path.to_string(),
            parent_path: Some(path.to_string()),
            version: 0, // dummy version as it will not get inserted in db
        }
    };

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        deployed_object,
        Some(format!("App '{}' deleted", path)),
        true,
        None,
    )
    .await?;

    sqlx::query!(
        "DELETE FROM deployment_metadata WHERE path = $1 AND workspace_id = $2 AND app_version IS NOT NULL",
        path,
        w_id
    )
    .execute(&db)
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "error deleting deployment metadata for script with path {path} in workspace {w_id}: {e:#}"
        ))
    })?;

    webhook.send_message(
        w_id.clone().clone(),
        WebhookMessage::DeleteApp { workspace: w_id, path: path.to_owned() },
    );

    Ok(format!("app {} deleted", path))
}

/// Report the path an app rename left behind (see `tally_rename_vacated_path`).
/// `raw_app` decides the diff row's kind, so read it back rather than guess — a
/// wrong kind writes a row nothing reads.
async fn tally_app_rename(db: &DB, w_id: &str, opath: &str, npath: &str, v_id: i64) {
    if opath == npath {
        return;
    }
    let raw_app = match sqlx::query_scalar!("SELECT raw_app FROM app_version WHERE id = $1", v_id)
        .fetch_optional(db)
        .await
    {
        Ok(raw_app) => raw_app.unwrap_or(false),
        Err(e) => {
            tracing::error!(%e, "could not read app_version {v_id} to tally the rename");
            return;
        }
    };
    let vacated = if raw_app {
        DeployedObject::RawApp { path: opath.to_string(), version: v_id, parent_path: None }
    } else {
        DeployedObject::App { path: opath.to_string(), version: v_id, parent_path: None }
    };
    if let Err(e) = windmill_git_sync::tally_rename_vacated_path(db, w_id, vacated).await {
        tracing::error!(%e, "error tallying the path renamed away from");
    }
}

async fn update_app(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ns): Json<EditApp>,
) -> Result<String> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot update apps for security reasons".to_string(),
        ));
    }
    // create_app_internal(authed, user_db, db, &w_id, &mut app).await?;
    let path = path.to_path();
    check_scopes(&authed, || format!("apps:write:{}", path))?;

    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let opath = path.to_string();
    let db2 = db.clone();
    let (new_tx, npath, v_id) =
        update_app_internal(authed, db, user_db, &w_id, path, false, ns, None).await?;
    new_tx.commit().await?;

    tally_app_rename(&db2, &w_id, &opath, &npath, v_id).await;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateApp {
            workspace: w_id.clone(),
            old_path: opath.clone(),
            new_path: npath.clone(),
        },
    );

    Ok(format!("app {} updated (npath: {:?})", opath, npath))
}

/// Deploy a raw app from its sources, compiling them on a worker. `update_raw`
/// takes the bundle the caller already built (the editor and the CLI both
/// bundle before calling it); an API client has nothing to build with, so this
/// is the raw-app deploy an MCP agent or a script can actually reach.
async fn update_app_raw_source(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ns): Json<EditApp>,
) -> Result<String> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot update apps for security reasons".to_string(),
        ));
    }

    let path = path.to_path();
    check_scopes(&authed, || format!("apps:write:{}", path))?;
    // The sources are compiled by a job on a worker: dependency resolution and
    // the build run there, on caller-supplied input. A token that can't run jobs
    // must not gain that through an app write, so require both scopes.
    check_scopes(&authed, || "jobs:run".to_string())?;

    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let Some(value) = ns.value.as_ref() else {
        return Err(Error::BadRequest(
            "value with the app's `files` is required to deploy a raw app".to_string(),
        ));
    };
    let value: RawAppSourceValue = serde_json::from_str(value.0.get())
        .map_err(|e| Error::BadRequest(format!("app value is not a raw app source: {e}")))?;

    // All before the compile, which costs a job on a worker: it must not run for
    // a path with no app, for a caller who can only read one (RLS refuses the
    // write, but not until their sources have been built), or for an app of the
    // other kind, which update_app_internal refuses anyway.
    let deployed_raw_app = deployed_app_kind(&user_db, &authed, &w_id, path)
        .await?
        .ok_or_else(|| Error::NotFound(format!("App {path} not found")))?;
    if !can_write_app(&user_db, &authed, &w_id, path).await? {
        return Err(Error::PermissionDenied(format!(
            "You do not have permission to update app {path}"
        )));
    }
    if !ns.allow_kind_change.unwrap_or(false) {
        reject_kind_change(path, true, Some(deployed_raw_app))?;
    }

    let bundled = apps_raw_bundle::bundle_raw_app_sources(
        &db,
        &user_db,
        &authed,
        &w_id,
        &value.files,
        &value.runnables,
    )
    .await?;

    let opath = path.to_string();
    let db2 = db.clone();
    // The new sources bring new runnables, so the deployed triggerables no longer
    // describe them. Merged inside, under the app-row lock.
    let (mut tx, npath, v_id) = update_app_internal(
        authed,
        db,
        user_db,
        &w_id,
        path,
        true,
        ns,
        Some(bundled.triggerables_v2),
    )
    .await?;
    store_raw_app_file(&w_id, &v_id, "js", bytes::Bytes::from(bundled.js), &mut tx).await?;
    if !bundled.css.is_empty() {
        store_raw_app_file(
            &w_id,
            &v_id,
            "css",
            bytes::Bytes::from(bundled.css),
            &mut tx,
        )
        .await?;
    }
    tx.commit().await?;
    tally_app_rename(&db2, &w_id, &opath, &npath, v_id).await;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateApp {
            workspace: w_id.clone(),
            old_path: opath.clone(),
            new_path: npath.clone(),
        },
    );

    Ok(format!("app {} updated (npath: {:?})", opath, npath))
}

/// Whether the caller may create an app at `path` — asked of the database rather
/// than restated here, like `can_write_app`: the app's RLS grants group members
/// write on a `g/<group>/…` path, which a hand-written check misses. The probe is
/// the insert itself, rolled back; `app` has no insert trigger, so the only trace
/// is a consumed sequence value.
async fn can_create_app(
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    path: &str,
) -> Result<bool> {
    let mut tx = user_db.clone().begin(authed).await?;
    let allowed = sqlx::query_scalar!(
        "INSERT INTO app (workspace_id, path, summary, policy, versions)
         VALUES ($1, $2, '', '{}'::jsonb, '{}') RETURNING 1",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await;
    tx.rollback().await?;
    match allowed {
        Ok(row) => Ok(row.is_some()),
        Err(sqlx::Error::Database(e)) => match e.code().as_deref() {
            // RLS refuses an insert outright rather than filtering it out.
            Some("42501") => Ok(false),
            // The path was taken between the existence check and this probe.
            // Permission is not what is wrong, so let the real insert say so.
            Some("23505") => Ok(true),
            _ => Err(sqlx::Error::Database(e).into()),
        },
        Err(e) => Err(e.into()),
    }
}

/// Whether a deployed app already occupies `path`. Through the privileged pool
/// on purpose: a path taken by an app the caller can't see is still taken, and
/// `create_app_internal` would fail on the unique index either way.
async fn app_exists(db: &DB, w_id: &str, path: &str) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false))
}

/// Create a raw app from its sources, compiling them on a worker. The counterpart
/// of `update_app_raw_source`: `create_raw` takes a bundle the caller already
/// built, which an API client has no way to produce.
async fn create_app_raw_source(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(mut app): Json<CreateApp>,
) -> Result<(StatusCode, String)> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot create apps for security reasons".to_string(),
        ));
    }

    let path = app.path.clone();
    check_scopes(&authed, || format!("apps:write:{}", path))?;
    // See update_app_raw_source: the compile runs caller-supplied sources on a
    // worker, so writing an app must not be a way to gain that.
    check_scopes(&authed, || "jobs:run".to_string())?;

    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let value: RawAppSourceValue = serde_json::from_str(app.value.0.get())
        .map_err(|e| Error::BadRequest(format!("app value is not a raw app source: {e}")))?;

    // Before the compile, which costs a job on a worker: it must not run for a
    // path already taken, nor for one the caller can't write. `create_app_internal`
    // rejects both, but only after the sources have been built.
    if app_exists(&db, &w_id, &path).await? {
        return Err(Error::BadRequest(format!("App {path} already exists")));
    }
    if !can_create_app(&user_db, &authed, &w_id, &path).await? {
        return Err(Error::PermissionDenied(format!(
            "You do not have permission to create app {path}"
        )));
    }

    let bundled = apps_raw_bundle::bundle_raw_app_sources(
        &db,
        &user_db,
        &authed,
        &w_id,
        &value.files,
        &value.runnables,
    )
    .await?;
    // The bundle carries the grants for the runnables it was built from, so the
    // app cannot be deployed with a policy that does not describe them.
    app.policy.triggerables = None;
    app.policy.triggerables_v2 = Some(bundled.triggerables_v2);

    let (mut tx, npath, v_id) = create_app_internal(authed, db, user_db, &w_id, true, app).await?;
    store_raw_app_file(&w_id, &v_id, "js", bytes::Bytes::from(bundled.js), &mut tx).await?;
    if !bundled.css.is_empty() {
        store_raw_app_file(
            &w_id,
            &v_id,
            "css",
            bytes::Bytes::from(bundled.css),
            &mut tx,
        )
        .await?;
    }
    tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateApp { workspace: w_id, path: npath.clone() },
    );

    Ok((StatusCode::CREATED, npath))
}

/// The part of a raw app's value the source endpoints read: `files` to bundle,
/// and `runnables` to derive the policy from, passed through untouched because
/// the CLI is what reads them.
#[derive(Deserialize)]
struct RawAppSourceValue {
    #[serde(default)]
    files: HashMap<String, String>,
    #[serde(default = "empty_runnables")]
    runnables: Box<RawValue>,
}

fn empty_runnables() -> Box<RawValue> {
    RawValue::from_string("{}".to_string()).expect("valid json")
}

fn reject_kind_change(path: &str, raw_app: bool, deployed_raw_app: Option<bool>) -> Result<()> {
    if !deployed_raw_app.is_some_and(|deployed| deployed != raw_app) {
        return Ok(());
    }
    // Name the folder suffix too: a sync push picks the endpoint from the repo
    // layout, so its operator has no endpoint to swap, only a folder.
    let (kind, endpoint, folder) = if raw_app {
        ("a low-code app", "/apps/update", ".app")
    } else {
        ("a raw app", "/apps/update_raw", ".raw_app")
    };
    Err(Error::BadRequest(format!(
        "App {path} is {kind}: deploying a value to it through the other kind's endpoint \
         would convert it and strand its bundle. Deploy it through {endpoint} instead \
         (from a synced repo, from a `{folder}` folder), or set allow_kind_change to \
         convert it on purpose."
    )))
}

/// Whether the caller may write the app — decided by the database rather than by
/// restating its policies here, which is how a hand-written check came to miss
/// that the app's RLS grants group members write on a `g/<group>/…` path. The
/// probe is the write itself, rolled back; `path = path` touches no column any
/// trigger watches.
async fn can_write_app(
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    path: &str,
) -> Result<bool> {
    let mut tx = user_db.clone().begin(authed).await?;
    let allowed = sqlx::query_scalar!(
        "UPDATE app SET path = path WHERE path = $1 AND workspace_id = $2 RETURNING 1",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .is_some();
    tx.rollback().await?;
    Ok(allowed)
}

/// Whether the app deployed at `path` is raw, or None when there is none the
/// caller can see. Through `user_db`, so it can't tell a caller anything about
/// an app they aren't allowed to read.
async fn deployed_app_kind(
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    path: &str,
) -> Result<Option<bool>> {
    let mut tx = user_db.clone().begin(authed).await?;
    let deployed_raw_app = sqlx::query_scalar!(
        "SELECT app_version.raw_app FROM app
         JOIN app_version ON app_version.id = app.versions[array_upper(app.versions, 1)]
         WHERE app.path = $1 AND app.workspace_id = $2",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(deployed_raw_app)
}

async fn update_app_raw<'a>(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
    multipart: Multipart,
) -> Result<String> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot update apps for security reasons".to_string(),
        ));
    }

    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let path = path.to_path();
    check_scopes(&authed, || format!("apps:write:{}", path))?;
    let opath = path.to_string();
    let db2 = db.clone();
    let (npath, v_id) = process_app_multipart!(
        authed,
        user_db,
        db,
        &w_id,
        path,
        multipart,
        // `/apps/update_raw` carries a whole prebuilt app: its policy comes from
        // the caller (the editor, the CLI), which derived the triggerables itself.
        |authed, db, user_db, w_id, path, raw_app, app| update_app_internal(
            authed, db, user_db, w_id, path, raw_app, app, None
        )
    )
    .await?;
    tally_app_rename(&db2, &w_id, &opath, &npath, v_id).await;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateApp {
            workspace: w_id.clone(),
            old_path: opath.to_owned(),
            new_path: npath.clone(),
        },
    );

    Ok(format!("app {} updated (npath: {:?})", opath, npath))
}
// async fn create_app_internal<'a>(
//     authed: ApiAuthed,
//     db: sqlx::Pool<sqlx::Postgres>,
//     user_db: UserDB,
//     w_id: &String,
//     app: &mut CreateApp,
// )

async fn update_app_internal<'a>(
    authed: ApiAuthed,
    db: sqlx::Pool<sqlx::Postgres>,
    user_db: UserDB,
    w_id: &str,
    path: &str,
    raw_app: bool,
    mut ns: EditApp,
    // Raw-app source deploys derive these from the value on a worker. Merged
    // into the policy here rather than by the caller so it happens under the
    // app-row lock taken below.
    derived_triggerables: Option<HashMap<String, PolicyTriggerableInputs>>,
) -> Result<(sqlx::Transaction<'a, sqlx::Postgres>, String, i64)> {
    use sql_builder::prelude::*;

    // A rename moves the app to ns.path, so the destination must also be within
    // the token's write scope, not just the source path.
    if let Some(npath) = ns.path.as_deref() {
        check_scopes(&authed, || format!("apps:write:{}", npath))?;
    }

    if raw_app {
        if let Some(value) = ns.value.as_ref() {
            validate_raw_app_path_keys(&value.0)?;
        }
    }

    // Reject a forged superadmin run identity in a preserved policy. Mirror the
    // `should_preserve` gate below (only a preserved value is caller-controlled;
    // otherwise the policy is rewritten to the deployer's own identity) and run
    // it on the non-RLS pool before the transaction to avoid a second connection.
    if let Some(npolicy) = ns.policy.as_ref() {
        let should_preserve = ns.preserve_on_behalf_of.unwrap_or(false)
            && windmill_common::can_preserve_on_behalf_of(&authed)
            && npolicy.on_behalf_of.is_some();
        if should_preserve {
            windmill_common::auth::validate_on_behalf_of(
                npolicy.on_behalf_of.as_deref(),
                npolicy.on_behalf_of_email.as_deref(),
            )?;
        }
    }

    let mut tx = user_db.clone().begin(&authed).await?;

    // `app_version.raw_app` is set by whichever endpoint writes the version, so a
    // value deployed through the wrong one converts the app and strands its bundle.
    // `FOR UPDATE` holds the app row until this transaction appends its own version,
    // so a concurrent deploy of the other kind can't land between check and append.
    if ns.value.is_some() && !ns.allow_kind_change.unwrap_or(false) {
        let deployed_version = sqlx::query_scalar!(
            "SELECT versions[array_upper(versions, 1)] FROM app
             WHERE path = $1 AND workspace_id = $2 FOR UPDATE",
            path,
            w_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        let deployed_raw_app = match deployed_version {
            // A separate statement: it needs the head the lock above pinned, not
            // the snapshot the locking statement started from.
            Some(version) => {
                sqlx::query_scalar!("SELECT raw_app FROM app_version WHERE id = $1", version)
                    .fetch_optional(&mut *tx)
                    .await?
            }
            None => None,
        };
        reject_kind_change(path, raw_app, deployed_raw_app)?;
    }

    let mut preserved_on_behalf_of: Option<String> = None;
    let npath = if ns.policy.is_some()
        || ns.path.is_some()
        || ns.summary.is_some()
        || ns.custom_path.is_some()
        || ns.labels.is_some()
        // A source deploy may send nothing but the value, and its runnables are
        // what the derived grants describe: skipping the UPDATE here would keep
        // the grants keyed to the sources this deploy just replaced.
        || derived_triggerables.is_some()
    {
        let mut sqlb = SqlBuilder::update_table("app");
        sqlb.and_where_eq("path", "?".bind(&path));
        sqlb.and_where_eq("workspace_id", "?".bind(&w_id));

        if let Some(npath) = &ns.path {
            if npath != path {
                require_owner_of_path(&authed, path)?;

                // The destination is what a guest session would be scoped to. A rename
                // that carries no policy keeps the deployed mode, read under the row
                // lock so a policy update landing alongside cannot slip a guest app
                // onto a path it cannot be scoped to.
                let mode = match ns.policy.as_ref().and_then(|p| p.stated_execution_mode()) {
                    Some(mode) => mode,
                    None => sqlx::query_scalar::<_, Option<String>>(
                        "SELECT policy->>'execution_mode' FROM app
                         WHERE path = $1 AND workspace_id = $2 FOR UPDATE",
                    )
                    .bind(path)
                    .bind(w_id)
                    .fetch_optional(&mut *tx)
                    .await?
                    .flatten()
                    .and_then(|m| {
                        serde_json::from_value::<ExecutionMode>(serde_json::Value::String(m)).ok()
                    })
                    .unwrap_or_default(),
                };
                refuse_unscopable_guest_app(npath, mode)?;

                let exists = sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2)",
                    npath,
                    w_id
                )
                .fetch_one(&mut *tx)
                .await?
                .unwrap_or(false);

                if exists {
                    return Err(Error::BadRequest(format!(
                        "App with path {} already exists",
                        npath
                    )));
                }
            }
            sqlb.set_str("path", npath);
        }

        if let Some(nsummary) = &ns.summary {
            sqlb.set_str("summary", nsummary);
        }

        if let Some(ncustom_path) = &ns.custom_path {
            require_admin(authed.is_admin, &authed.username)?;
            let scoped =
                *CLOUD_HOSTED || APP_WORKSPACED_ROUTE.load(std::sync::atomic::Ordering::Relaxed);

            if ncustom_path.is_empty() {
                sqlb.set("custom_path", "NULL");
            } else {
                // Same predicate as before (the check is correct): the app's
                // own row in this workspace is excluded, so a single-workspace
                // edit still works. In global mode a copy of this app in
                // another workspace is a genuine conflict (one global route) —
                // surface which workspace so it can be resolved.
                let conflict = sqlx::query!(
                    "SELECT workspace_id, path FROM app WHERE custom_path = $1 AND ($2::TEXT IS NULL OR workspace_id = $2) AND NOT (path = $3 AND workspace_id = $4) LIMIT 1",
                    ncustom_path,
                    if scoped { Some(w_id) } else { None },
                    path,
                    w_id
                )
                .fetch_optional(&mut *tx)
                .await?;

                if let Some(conflict) = conflict {
                    return Err(custom_path_conflict_error(
                        ncustom_path,
                        &conflict.path,
                        &conflict.workspace_id,
                        scoped,
                    ));
                }
                sqlb.set_str("custom_path", ncustom_path);
            }
        }

        // A source deploy brings triggerables derived from its value but may send
        // no policy at all, in which case the rest of the deployed one carries
        // over. Either way a policy is about to be written wholesale.
        let caller_sent_policy = ns.policy.is_some();
        if caller_sent_policy || derived_triggerables.is_some() {
            // One locked read serving everything below: the mode an omitted one
            // inherits, the already-anonymous check, and the policy a source
            // deploy carries over. FOR UPDATE holds the row until this
            // transaction's UPDATE commits, so a policy edit landing between the
            // read and the write cannot be clobbered by this stale snapshot.
            let deployed = sqlx::query_scalar!(
                "SELECT policy FROM app WHERE path = $1 AND workspace_id = $2 FOR UPDATE",
                path,
                w_id
            )
            .fetch_optional(&mut *tx)
            .await?;
            let deployed_policy = deployed
                .as_ref()
                .and_then(|p| serde_json::from_value::<Policy>(p.clone()).ok());

            let mut npolicy = match ns.policy.take() {
                Some(npolicy) => npolicy,
                // Carrying the deployed policy forward is only safe if all of it
                // survived the round trip; a partial one would silently drop
                // `on_behalf_of`, sandboxing or S3 rules.
                None => deployed_policy.clone().ok_or_else(|| {
                    Error::internal_err(format!(
                        "app {path} has no readable policy to deploy its new sources under"
                    ))
                })?,
            };
            if let Some(triggerables) = derived_triggerables {
                npolicy.triggerables = None;
                npolicy.triggerables_v2 = Some(triggerables);
            }
            validate_frontend_sdk_scopes(&npolicy)?;
            // The policy is written wholesale, so one that states no mode would
            // otherwise re-permission the app to the create-time default: a
            // `viewer` app would start running on the publisher's identity. Keep
            // the deployed mode instead, and make the stored policy state it.
            if npolicy.stated_execution_mode().is_none() {
                npolicy.set_execution_mode(
                    deployed_policy
                        .as_ref()
                        .map(|d| d.execution_mode())
                        .unwrap_or_default(),
                );
            }
            refuse_unscopable_guest_app(
                ns.path.as_deref().unwrap_or(path),
                npolicy.execution_mode(),
            )?;
            if let Some(rule) =
                deployment_rule_for_mode(npolicy.execution_mode()).filter(|_| !authed.is_admin)
            {
                // Restricted users may keep deploying an app that is already open
                // to this audience, but widening one is gated by the matching
                // protection rule. An unreadable deployed policy reads as not
                // already-widened, the strict direction.
                let already_in_mode = deployed
                    .as_ref()
                    .and_then(|p| p.get("execution_mode"))
                    .and_then(|m| m.as_str())
                    == Some(npolicy.execution_mode().as_str());
                if !already_in_mode {
                    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
                        w_id,
                        &rule,
                        &authed.username,
                        &authed.groups,
                        authed.is_admin,
                        &db,
                    )
                    .await?
                    {
                        return Err(Error::PermissionDenied(msg));
                    }
                }
            }
            let should_preserve = ns.preserve_on_behalf_of.unwrap_or(false)
                && windmill_common::can_preserve_on_behalf_of(&authed)
                && npolicy.on_behalf_of.is_some();

            if should_preserve {
                if let Some(ref obo_email) = npolicy.on_behalf_of_email {
                    if obo_email != &authed.email {
                        preserved_on_behalf_of = Some(obo_email.clone());
                    }
                }
            } else if caller_sent_policy {
                // Submitting a policy is how a deployer claims the app's
                // execution identity. A source deploy that sent none is not
                // claiming anything, so whoever the app already runs as stays.
                npolicy.on_behalf_of = Some(username_to_permissioned_as(&authed.username));
                npolicy.on_behalf_of_email = Some(authed.email.clone());
            }
            sqlb.set(
                "policy",
                quote(serde_json::to_string(&json!(npolicy)).map_err(|e| {
                    Error::BadRequest(format!("failed to serialize policy: {}", e))
                })?),
            );
        }

        sqlb.returning("path");

        let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
        let npath_o: Option<String> = sqlx::query_scalar(&sql).fetch_optional(&mut *tx).await?;
        let npath_val = not_found_if_none(npath_o, "App", path)?;

        if let Some(nlabels) = &ns.labels {
            sqlx::query!(
                "UPDATE app SET labels = $1 WHERE path = $2 AND workspace_id = $3",
                nlabels as &[String],
                &npath_val,
                w_id
            )
            .execute(&mut *tx)
            .await?;
        }

        npath_val
    } else {
        path.to_owned()
    };
    let v_id = if let Some(nvalue) = &ns.value {
        let app_id = sqlx::query_scalar!(
            "SELECT id FROM app WHERE path = $1 AND workspace_id = $2",
            npath,
            w_id
        )
        .fetch_one(&mut *tx)
        .await?;

        // `.get()` keeps the raw text (and thus key order); strip any NUL so the
        // `json`→`jsonb` conversion downstream (fork, indexing) can't choke on it.
        let value = strip_json_nul(nvalue.0.get());
        if matches!(value, Cow::Owned(_)) {
            tracing::warn!(path = %npath, "stripped NUL character(s) from app value on update");
        }
        let v_id = sqlx::query_scalar!(
            "INSERT INTO app_version
                (app_id, value, created_by, raw_app)
                VALUES ($1, $2::text::json, $3, $4) RETURNING id",
            app_id,
            value.as_ref(),
            authed.username,
            raw_app
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE app SET versions = array_append(versions, $1::bigint) WHERE path = $2 AND workspace_id = $3",
            v_id,
            npath,
            w_id
        )
        .execute(&mut *tx)
        .await?;
        v_id
    } else {
        let v_id = sqlx::query_scalar!(
            "SELECT  app.versions[array_upper(app.versions, 1)] FROM app WHERE path = $1 AND workspace_id = $2",
            npath,
            w_id
        )
        .fetch_one(&mut *tx)
        .await?;
        if let Some(v_id) = v_id {
            v_id
        } else {
            return Err(Error::BadRequest(format!(
                "App with path {} not found",
                npath
            )));
        }
    };
    // CLI / git-sync deploys ask us to preserve any existing user draft at this
    // path instead of wiping it as part of the deploy. Only wipe the deployer's
    // own draft (plus the legacy NULL-email row) — see create_app_internal.
    if !ns.skip_draft_deletion.unwrap_or(false) {
        sqlx::query!(
            "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ IN ('app', 'raw_app') \
             AND (email = $3 OR email IS NULL)",
            path,
            &w_id,
            &authed.email,
        )
        .execute(&mut *tx)
        .await?;
    }
    audit_log(
        &mut *tx,
        &authed,
        "apps.update",
        ActionKind::Update,
        w_id,
        Some(&npath),
        None,
    )
    .await?;
    if let Some(on_behalf_of) = preserved_on_behalf_of {
        audit_log(
            &mut *tx,
            &authed,
            "apps.on_behalf_of",
            ActionKind::Update,
            w_id,
            Some(&npath),
            Some(
                [
                    ("on_behalf_of", on_behalf_of.as_str()),
                    ("action", "update"),
                ]
                .into(),
            ),
        )
        .await?;
    }
    let tx = PushIsolationLevel::Transaction(tx);
    let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
    if let Some(dm) = ns.deployment_message {
        args.insert("deployment_message".to_string(), to_raw_value(&dm));
    }
    args.insert("parent_path".to_string(), to_raw_value(&path));
    let (dependency_job_uuid, new_tx) = push(
        &db,
        tx,
        w_id,
        JobPayload::AppDependencies {
            path: npath.clone(),
            version: v_id,
            debouncing_settings: Default::default(),
        },
        PushArgs { args: &args, extra: None },
        &authed.username,
        &authed.email,
        windmill_common::users::username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        authed.username_override.as_deref(),
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        None,
        None,
        None,
        None,
        Some(&authed.clone().into()),
        false,
        None,
        None,
        None,
    )
    .await?;
    tracing::info!("Pushed app dependency job {}", dependency_job_uuid);
    Ok((new_tx, npath, v_id))
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExecuteApp {
    /// The app version the caller last loaded. Used only as a policy-cache
    /// freshness hint: when it differs from the cached entry the policy is
    /// refetched, so a redeploy takes effect immediately. It does not select which
    /// version runs — the policy and runnables are always the app's current ones.
    pub version: Option<i64>,
    /// The app script id (from the `app_script` table) to execute.
    pub id: Option<i64>,
    pub args: HashMap<String, Box<RawValue>>,
    // - script: script/<path>
    // - flow: flow/<path>
    pub path: Option<String>,
    pub component: String,
    pub raw_code: Option<RawCode>,
    // if set, the app is executed as viewer with the given static fields
    pub force_viewer_static_fields: Option<StaticFields>,
    pub force_viewer_one_of_fields: Option<OneOfFields>,
    pub force_viewer_allow_user_resources: Option<AllowUserResources>,
    pub force_viewer_sensitive_inputs: Option<Vec<String>>,
    pub force_viewer_delete_after_secs: Option<i32>,
    /// Runnable query parameters (e.g., memory_id for chat-enabled flows)
    pub run_query_params: Option<RunJobQuery>,
    /// Map of relative-import script path -> temp storage hash. Only honored for
    /// inline-script (raw_code, preview) execution so `wmill app dev` resolves
    /// those imports from not-yet-deployed local content instead of deployed.
    pub temp_script_refs: Option<HashMap<String, String>>,
}

fn digest(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code);
    let result = hasher.finalize();
    format!("rawscript/{:x}", result)
}

async fn get_on_behalf_details_from_policy_and_authed(
    policy: &Policy,
    opt_authed: &Option<ApiAuthed>,
) -> Result<(String, String, String)> {
    // A guest acts only through an app open to guests — or to everyone. A members-only
    // mode means the policy changed after the session was issued. Decided here, in the
    // one resolver every on-behalf path (runs, S3 reads, uploads) goes through.
    if opt_authed
        .as_ref()
        .is_some_and(|a| windmill_api_auth::scopes::has_guest_sentinel(a.scopes.as_deref()))
        && !matches!(
            policy.execution_mode(),
            ExecutionMode::Guest | ExecutionMode::Anonymous
        )
    {
        return Err(Error::PermissionDenied(
            "this app is not open to guests".to_string(),
        ));
    }
    let (username, permissioned_as, email) = match policy.execution_mode() {
        ExecutionMode::Anonymous => {
            let username = opt_authed
                .as_ref()
                .map(|a| a.username.clone())
                .unwrap_or_else(|| "anonymous".to_string());
            let (permissioned_as, email) = get_on_behalf_of(&policy)?;
            (username, permissioned_as, email)
        }
        // Guest runs as the publisher exactly as Publisher does; the two differ only
        // in who is let through the door, which is settled before we get here.
        ExecutionMode::Publisher | ExecutionMode::Guest => {
            let username = opt_authed
                .as_ref()
                .map(|a| a.username.clone())
                .ok_or_else(|| {
                    Error::BadRequest(
                        "publisher execution mode requires authentication".to_string(),
                    )
                })?;
            let (permissioned_as, email) = get_on_behalf_of(&policy)?;
            (username, permissioned_as, email)
        }
        ExecutionMode::Viewer => {
            let (username, email) = opt_authed
                .as_ref()
                .map(|a| (a.username.clone(), a.email.clone()))
                .ok_or_else(|| {
                    Error::BadRequest("Required to be authed in viewer mode".to_string())
                })?;
            (
                username.clone(),
                username_to_permissioned_as(&username),
                email,
            )
        }
    };

    Ok((username, permissioned_as, email))
}

/// Convert the triggerables from the old format to the new format.
fn empty_triggerables(mut policy: Policy) -> Policy {
    use std::mem::take;
    if let Some(triggerables) = take(&mut policy.triggerables) {
        let mut triggerables_v2 = take(&mut policy.triggerables_v2).unwrap_or_default();
        for (k, static_inputs) in triggerables.into_iter() {
            triggerables_v2.insert(
                k,
                PolicyTriggerableInputs { static_inputs, ..Default::default() },
            );
        }
        policy.triggerables_v2 = Some(triggerables_v2);
    }
    policy
}

lazy_static! {
    /// Deployed-app policies keyed by `(workspace_id, path)`, value
    /// `(cached_version, policy, cached_at)`. The policy carries the
    /// authorization-critical `execution_mode` (anonymous/public), so a stale entry
    /// keeps a revoked app publicly executable (GHSA-r5v4-cxh9-7qhq). An entry is
    /// reused only when all three freshness signals agree, each covering a case the
    /// others can't:
    /// - the caller's `version` still matches `cached_version` (a redeploy bumps the
    ///   version -> instant refetch, for free since it's in the request);
    /// - the `notify_app_policy_change` event has not evicted the key (policy-only
    ///   change or deletion, which don't bump the version — see
    ///   `invalidate_app_policy_cache` and `process_notify_event`);
    /// - the entry is within its TTL (backstop for a missed event or a restart).
    static ref APP_POLICY_CACHE: cache::Cache<(String, String), (Option<i64>, Arc<Policy>, std::time::Instant)> =
        cache::Cache::new(1000);
}

/// Backstop time-to-live for an [`APP_POLICY_CACHE`] entry — the maximum a policy
/// change can go unreflected if its invalidation event is never consumed.
const APP_POLICY_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(300);

/// Drop the cached policy for one app so the next execute re-reads it live. Invoked
/// by the `notify_app_policy_change` poller arm on every policy change or deletion.
pub fn invalidate_app_policy_cache(workspace_id: &str, path: &str) {
    APP_POLICY_CACHE.remove(&(workspace_id.to_string(), path.to_string()));
}

async fn execute_component(
    OptAuthed(opt_authed): OptAuthed,
    tokened: OptTokened,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(mut payload): Json<ExecuteApp>,
) -> Result<String> {
    let path = path.to_path();
    // Authorize before touching the payload: the route layer is resource-blind, so a
    // path-scoped caller (app embed token, or a picker-minted `apps:run|write:<path>`)
    // is confined to its own app only here. No-op for unscoped callers; anonymous ones
    // are policy-gated below, and a guest's confinement waits for the app's mode
    // (`guest_caller_for_mode`).
    if let Some(authed) = opt_authed
        .as_ref()
        .filter(|a| !windmill_api_auth::scopes::has_guest_sentinel(a.scopes.as_deref()))
    {
        check_scopes(authed, || format!("apps:run:{}", path))?;
    }
    // Only honor temp_script_refs for the inline-script preview path:
    // preview/editor mode (force_viewer_static_fields set, == `is_preview`),
    // raw_code present, and no deployed app_script id — i.e. `wmill app dev`.
    let temp_script_refs = payload.temp_script_refs.take();
    let inject_temp_refs = temp_script_refs.is_some()
        && payload.force_viewer_static_fields.is_some()
        && payload.raw_code.is_some()
        && payload.id.is_none();
    match (payload.path.is_some(), payload.raw_code.is_some()) {
        (false, false) => {
            return Err(Error::BadRequest(
                "path or raw_code is required".to_string(),
            ))
        }
        (true, true) => {
            return Err(Error::BadRequest(
                "path and raw_code cannot be set at the same time".to_string(),
            ))
        }
        _ => {}
    };

    let arc_policy: Arc<Policy>;
    let policy_triggerables_default = Default::default();
    // Preview mode means the request was issued from the editor; the editing
    // user is already trusted by the policy check, so client-supplied `tag`
    // on the inline script is honored. In any other case we must read the
    // tag from the deployed policy and ignore the request body.
    let is_preview = payload.force_viewer_static_fields.is_some();

    // Preview mode runs request-supplied code as a `Viewer`-mode job (the
    // app-editor equivalent of `/jobs/run/preview`), so it enforces the same
    // guards. Operators must never run preview jobs. `jobs:run` is required
    // because this route is reachable with an `apps:run`-scoped token (the
    // route maps to the `apps` scope domain), which must not be able to escalate
    // to arbitrary code execution. The client-supplied inline `raw_code.tag`
    // must stay within the caller's allowed worker tags. A preview can also
    // *reference* an existing runnable the caller may not be allowed to read — a
    // deployed script/flow via `payload.path` or a persisted `app_script` via
    // `payload.id`, both resolved with the root DB handle — so those (and only
    // those) are confined to paths the caller can read. Inline `raw_code` is not
    // path-gated: a non-operator member can already run arbitrary inline code
    // via `/jobs/run/preview`.
    if is_preview {
        let authed = opt_authed.as_ref().ok_or_else(|| {
            Error::NotAuthorized("App component preview requires authentication".to_string())
        })?;
        if authed.is_operator {
            return Err(Error::NotAuthorized(
                "Operators cannot run preview jobs for security reasons".to_string(),
            ));
        }
        check_scopes(authed, || format!("jobs:run"))?;
        if let Some(p) = payload.path.as_deref() {
            let runnable_path = p
                .strip_prefix("script/")
                .or_else(|| p.strip_prefix("flow/"))
                .unwrap_or(p);
            require_path_read_access_for_preview(authed, &Some(runnable_path.to_string()))?;
        }
        if let Some(id) = payload.id {
            let owner_path = sqlx::query_scalar!(
                "SELECT a.path FROM app_script s JOIN app a ON a.id = s.app
                 WHERE s.id = $1 AND a.workspace_id = $2",
                id,
                &w_id,
            )
            .fetch_optional(&db)
            .await?
            .ok_or_else(|| {
                Error::NotAuthorized(format!(
                    "App script {id} does not belong to an app in this workspace"
                ))
            })?;
            require_path_read_access_for_preview(authed, &Some(owner_path))?;
        }
    }

    // Two cases here:
    // 1. The component is executed from the editor (i.e. in "preview" mode), then:
    //    - The policy is set to default (in `Viewer` execution mode).
    //    - The policy triggerables are built by the frontend and retrieved from the request
    //      payload.
    //    - In case of inline script, the `RawCode` from the request is pushed as is to the
    //      job queue.
    // 2. Otherwise (i.e. "run" mode):
    //    - The policy and triggerables are fetched from the database.
    //    - In case of inline script, if an entry exists in the `app_script` table, push
    //      an `AppScript` job payload, as in (.1) otherwise.
    let (policy, policy_triggerables) = match payload {
        // 1. "preview" mode.
        ExecuteApp {
            force_viewer_static_fields: Some(static_inputs),
            force_viewer_one_of_fields,
            force_viewer_allow_user_resources,
            force_viewer_sensitive_inputs,
            force_viewer_delete_after_secs,
            ..
        } => (
            &Policy { stated_execution_mode: Some(ExecutionMode::Viewer), ..Default::default() },
            &PolicyTriggerableInputs {
                static_inputs,
                one_of_inputs: force_viewer_one_of_fields.unwrap_or_default(),
                allow_user_resources: force_viewer_allow_user_resources.unwrap_or_default(),
                delete_after_secs: force_viewer_delete_after_secs,
                sensitive_inputs: force_viewer_sensitive_inputs.unwrap_or_default(),
                tag: None,
            },
        ),
        // 2. "run" mode.
        _ => {
            // Policy is fetched from the database on app `path` and `workspace_id`.
            let policy_fut = sqlx::query_scalar!(
                "SELECT policy as \"policy: sqlx::types::Json<Box<RawValue>>\"
                FROM app WHERE app.path = $1 AND app.workspace_id = $2 LIMIT 1",
                path,
                &w_id,
            )
            .fetch_optional(&db)
            .map_err(Into::<Error>::into)
            .map(|policy_o| Result::Ok(not_found_if_none(policy_o?, "App", path)?))
            .map(|policy| Result::Ok(serde_json::from_str(policy?.get())?))
            .map_ok(empty_triggerables);

            // Serve the policy from the `(workspace, path)`-keyed cache, refetching
            // unless all three freshness signals agree (GHSA-r5v4-cxh9-7qhq): the
            // caller's `version` matches, the key wasn't evicted by
            // `notify_app_policy_change`, and the entry is within its TTL.
            let cache_key = (w_id.to_string(), path.to_string());
            let fresh = APP_POLICY_CACHE
                .get(&cache_key)
                .filter(|(v, _, cached_at)| {
                    *v == payload.version && cached_at.elapsed() < APP_POLICY_CACHE_TTL
                })
                .map(|(_, p, _)| p);
            let policy = if let Some(p) = fresh {
                arc_policy = p;
                &*arc_policy
            } else {
                arc_policy = Arc::new(policy_fut.await?);
                APP_POLICY_CACHE.insert(
                    cache_key,
                    (
                        payload.version,
                        arc_policy.clone(),
                        std::time::Instant::now(),
                    ),
                );
                &*arc_policy
            };

            // Caller-supplied inline code (`raw_code`), with or without an
            // `app_script` id. Its resolved `rawscript/<sha>` key must be present
            // in the policy triggerables below — it must never resolve via the
            // Viewer default fallback. Without `id` the caller supplies the code
            // verbatim; with `id` it selects any `app_script` row by number (no
            // app/workspace scoping), so both let a caller run code the publisher
            // never pinned for this app.
            let is_inline_raw_code = payload.raw_code.is_some();

            // Compute the path for the triggerables map:
            // - flow: `flow/<payload.path>`
            // - script: `script/<payload.path>`
            // - inline script: `rawscript/<sha256(raw_code.content)>`
            let path = match &payload {
                // flow or script: just use the `payload.path`.
                ExecuteApp { path: Some(path), .. } => path,
                // inline script: without entry in the `app_script` table.
                ExecuteApp { raw_code: Some(raw_code), id: None, .. } => &digest(&raw_code.content),
                // inline script: with an entry in the `app_script` table.
                ExecuteApp { raw_code: Some(_), id: Some(id), .. } => {
                    let cache = cache::anon!({ u64 => Arc<String> } in "appscriptpath" <= 10000);
                    // `id` is unique, cache the result.
                    &*sqlx::query_scalar!(
                        "SELECT format('rawscript/%s', code_sha256) as \"path!: String\"
                        FROM app_script WHERE id = $1 LIMIT 1",
                        id
                    )
                    .fetch_one(&db)
                    .map_err(Into::<Error>::into)
                    .map_ok(Arc::new)
                    .cached(cache, *id as u64)
                    .await?
                }
                _ => unreachable!(),
            };

            // Retrieve the triggerables from the policy on `path` or `<component>:<path>`.
            let triggerables_v2 = policy
                .triggerables_v2
                .as_ref()
                .ok_or_else(|| Error::BadRequest(format!("Policy is missing triggerables")))?;

            let policy_triggerables = triggerables_v2
                .get(path) // start with `path` in case we can avoid the next` format!`.
                .or_else(|| triggerables_v2.get(&format!("{}:{}", payload.component, &path)))
                .or(match policy.execution_mode() {
                    // A Viewer app may invoke any deployed `script`/`flow` it
                    // references (resolved as the caller), but caller-supplied
                    // inline `raw_code` must match a publisher-pinned
                    // `rawscript/<sha>` entry — otherwise an unauthorized caller
                    // (e.g. an operator, barred from `/jobs/run/preview`) could
                    // run code the publisher never pinned for this app.
                    ExecutionMode::Viewer if !is_inline_raw_code => {
                        Some(&policy_triggerables_default)
                    }
                    _ => None,
                })
                .ok_or_else(|| Error::BadRequest(format!("Path {path} forbidden by policy")))?;

            (policy, policy_triggerables)
        }
    };

    // Rate limit for executions by callers the workspace does not know: anonymous
    // viewers, and guests — on an instance whose provider accepts any consumer
    // account, "anyone the IdP authenticates" is close to the anonymous population,
    // and each run costs a job as the publisher.
    let is_guest_caller = opt_authed
        .as_ref()
        .is_some_and(|a| windmill_api_auth::scopes::has_guest_sentinel(a.scopes.as_deref()));
    if (matches!(policy.execution_mode(), ExecutionMode::Anonymous) && opt_authed.is_none())
        || is_guest_caller
    {
        if let Some(limit) = crate::workspaces::get_public_app_rate_limit(&db, &w_id).await? {
            if limit > 0 {
                crate::public_app_rate_limit::check_and_increment(&w_id, limit)?;
            }
        }
    }

    let opt_authed = guest_caller_for_mode(opt_authed, policy.execution_mode(), path)?;

    // Execution is publisher and an user is authenticated: check if the user is authorized to
    // execute the app.
    if let (ExecutionMode::Publisher, Some(authed)) = (policy.execution_mode(), opt_authed.as_ref())
    {
        lazy_static! {
            /// Cache for the permit to execute an app component.
            static ref PERMIT_CACHE: cache::Cache<[u8; 32], bool> = cache::Cache::new(1000);
        }

        // Avoid allocation for the permit key using a sha256 hash of:
        // - the user email,
        // - the application path,
        // - the workspace id.
        let permit_key: [u8; 32] = [authed.email.as_bytes(), path.as_bytes(), &w_id.as_bytes()]
            .iter()
            .fold(Sha256::new(), |hasher, bytes| hasher.chain_update(bytes))
            .finalize()
            .into();
        let permit_fut = PERMIT_CACHE.get_or_insert_async(&permit_key, async {
            let mut tx = user_db.clone().begin(authed).await?;
            // Permissions are checked by the database; just fetch a row from app using `user_db`:
            let row = sqlx::query_scalar!(
                "SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2 LIMIT 1",
                path,
                &w_id,
            )
            .fetch_optional(&mut *tx)
            .await?;
            tx.commit().await?;
            Result::Ok(row.is_some_and(|x| x.is_some()))
        });

        if !permit_fut.await? {
            return Err(Error::NotAuthorized(format!(
                "Missing read permissions on the `{}` app to execute `{}` runnable",
                path, payload.component
            )));
        }
    }

    let (username, permissioned_as, email) =
        get_on_behalf_details_from_policy_and_authed(&policy, &opt_authed).await?;

    let resolved_delete_secs =
        resolve_delete_after_secs(None, policy_triggerables.delete_after_secs);

    let (mut args, job_id) = build_args(
        policy,
        policy_triggerables,
        payload.args,
        opt_authed.as_ref(),
        &user_db,
        &db,
        &w_id,
    )
    .await?;

    if inject_temp_refs {
        if let Some(refs) = temp_script_refs {
            args.extra
                .get_or_insert_with(HashMap::new)
                .insert("_TEMP_SCRIPT_REFS".to_string(), to_raw_value(&refs));
        }
    }

    let is_flow = payload
        .path
        .as_ref()
        .map(|p| p.starts_with("flow/"))
        .unwrap_or(false);

    // Tag for inline-script jobs is read from the deployed policy in run mode;
    // only preview mode (editor) honors the client-supplied tag. This applies to
    // both the `id`-bearing app_script path and the legacy `rawscript/<sha>`
    // path that has no app_script entry yet.
    let resolved_inline_tag = |client_tag: Option<String>| -> Option<String> {
        if is_preview {
            client_tag
        } else {
            policy_triggerables.tag.clone()
        }
        .filter(|t| !t.is_empty())
    };
    let (job_payload, tag, _runnable_on_behalf_of) =
        match (payload.path, payload.raw_code, payload.id) {
            // flow or script:
            (Some(path), None, None) => {
                get_payload_tag_from_prefixed_path(&path, &db, &w_id).await?
            }
            // inline script: "preview" mode, or run mode without an entry in the
            // `app_script` table (legacy `rawscript/<sha>`-keyed triggerables).
            (None, Some(raw_code), None) => {
                let tag = resolved_inline_tag(raw_code.tag.clone());
                (JobPayload::Code(raw_code), tag, None)
            }
            // inline script: run mode (deployed app) with an entry in `app_script`.
            (None, Some(RawCode { language, path, cache_ttl, tag, .. }), Some(id)) => (
                JobPayload::AppScript { id: AppScriptId(id), cache_ttl, language, path },
                resolved_inline_tag(tag),
                None,
            ),
            _ => unreachable!(),
        };
    // Preview honors the client-supplied inline tag (`resolved_inline_tag`), so
    // — like `/jobs/run/preview` — confine it to worker tags the caller may use
    // (a `if_jobs:filter_tags`-restricted token must not escape its filter).
    // `is_preview` implies an authed caller (the guard above returns otherwise).
    if is_preview {
        if let Some(authed) = opt_authed.as_ref() {
            crate::jobs::check_tag_available_for_workspace(&db, &w_id, &tag, authed).await?;
        }
    }
    // Identity is already resolved to the requesting user in preview mode (the
    // policy is forced to `ExecutionMode::Viewer`, so the job runs as the
    // caller). The enqueue stays root-isolated as before — switching the insert
    // to user-RLS is not what contains the bypass (the auth guards above are)
    // and would add unnecessary breakage risk to the legitimate editor flow.
    let tx = PushIsolationLevel::IsolatedRoot(db.clone());

    // An app component runs on-behalf of the APP identity (resolved above), never
    // the referenced runnable's own `on_behalf_of` — else a Viewer-mode app could
    // execute as that identity and a preview would run as it, not the caller.
    // (Direct `/jobs/run` still honors a runnable's `on_behalf_of`.)
    let (email, permissioned_as) = (email.as_str(), permissioned_as);

    let end_user_email =
        get_end_user_email(&db, opt_authed.as_ref(), tokened.token.as_deref()).await;

    // Stamp app-origination (trigger_kind='app' + trigger=<app path>), the signal
    // the deployed-app S3 provenance gate trusts (unforgeable via `/jobs/run`).
    // Deployed runs only: a preview runs as the caller and is read back as the caller
    // (viewer-scoped), so it must never be app-provenanced (else it could forge one).
    let app_trigger =
        (!is_preview).then(|| TriggerMetadata::new(Some(path.to_string()), JobTriggerKind::App));

    let (uuid, mut tx) = push(
        &db,
        tx,
        &w_id,
        job_payload,
        PushArgs { args: &args.args, extra: args.extra },
        &username,
        email,
        permissioned_as,
        opt_authed
            .and_then(|a| a.token_prefix)
            .or_else(|| tokened.token.map(|t| t[0..TOKEN_PREFIX_LEN].to_string()))
            .as_deref(),
        // The caller of an app run is whoever used the app, which `push` already derives from
        // `username` here — the app identity took `permissioned_as`.
        None,
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
        None,
        None,
        None,
        None,
        false,
        end_user_email,
        app_trigger,
        None,
    )
    .await?;

    // Apply runnable query parameters if provided
    if let Some(ref run_query) = payload.run_query_params {
        if is_flow {
            crate::jobs::process_flow_run_query_params(&mut tx, uuid, run_query).await?;
        }
    }

    tx.commit().await?;

    if let Some(secs) = resolved_delete_secs {
        if let Err(e) = schedule_job_deletion(&db, uuid, &w_id, secs).await {
            tracing::error!("Failed to schedule deletion for app job {uuid} after {secs}s: {e:#}");
        }
    }

    Ok(uuid.to_string())
}

#[cfg(not(feature = "parquet"))]
async fn upload_s3_file_from_app() -> Result<()> {
    return Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ));
}

#[cfg(not(feature = "parquet"))]
async fn delete_s3_file_from_app() -> Result<()> {
    return Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ));
}

#[cfg(feature = "parquet")]
#[derive(Debug, Deserialize, Clone)]
struct UploadFileToS3Query {
    file_key: Option<String>,
    file_extension: Option<String>,
    s3_resource_path: Option<String>,
    content_type: Option<String>,
    content_disposition: Option<String>,
    force_viewer_file_key_regex: Option<String>,
    force_viewer_allow_user_resources: Option<bool>,
    force_viewer_allow_workspace_resource: Option<bool>,
    force_viewer_allowed_resources: Option<String>,
}

#[cfg(feature = "parquet")]
#[derive(Serialize, Deserialize)]
struct S3DeleteTokenClaims {
    file_key: String,
    on_behalf_of_email: String,
    permissioned_as: String,
    username: String,
    s3_resource_path: Option<String>,
    workspace: String,
    pub exp: usize,
}

#[cfg(feature = "parquet")]
const SIGN_S3_DEFAULT_EXPIRY_SECS: i64 = 12 * 60 * 60;
#[cfg(feature = "parquet")]
const SIGN_S3_MIN_EXPIRY_SECS: i64 = 60;
#[cfg(feature = "parquet")]
const SIGN_S3_MAX_EXPIRY_SECS: i64 = 7 * 24 * 60 * 60;

#[cfg(feature = "parquet")]
#[derive(Deserialize)]
struct S3TokenRequestBody {
    s3_objects: Vec<S3Object>,
    expiry_secs: Option<i64>,
}
#[cfg(feature = "parquet")]
async fn sign_s3_objects(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(body): Json<S3TokenRequestBody>,
) -> Result<Json<Vec<S3Object>>> {
    let workspace_key = get_workspace_key(&w_id, &db).await?;

    let expiry_secs = body
        .expiry_secs
        .unwrap_or(SIGN_S3_DEFAULT_EXPIRY_SECS)
        .clamp(SIGN_S3_MIN_EXPIRY_SECS, SIGN_S3_MAX_EXPIRY_SECS);
    let exp = (chrono::Utc::now() + chrono::Duration::seconds(expiry_secs)).timestamp();

    let futures = body.s3_objects.into_iter().map(|s3_object| async {
        // The signature this mints is a transferable bearer capability: `validate_s3_signature`
        // only checks the HMAC and expiry, so anyone who obtains the string can read this key.
        // Authorize the CALLER's own read permission before signing — otherwise any workspace
        // member (operators included) could mint a signature for any key and bypass the advanced
        // S3 permission rules. This is the fix; do NOT move the check to validation time.
        // ...and refuse outright when the storage does not resolve, because
        // `get_workspace_s3_resource_and_check_paths` returns early in that case, *before* the
        // permission loop below runs at all. Checked here rather than on its `None` return: that
        // is also `None` when the resource resolved but was denied (an RLS miss, or an instance
        // bucket restriction), and those ran the loop and must keep their existing behaviour.
        if get_large_file_storage(&db, &w_id, s3_object.storage.clone())
            .await?
            .is_none()
        {
            return Err(windmill_object_store::workspace_storage_not_found(
                s3_object.storage.as_deref(),
            ));
        }
        let db_with_opt_authed = DbWithOptAuthed::from_authed(&authed, db.clone(), None);
        get_workspace_s3_resource_and_check_paths(
            &db_with_opt_authed,
            Some(&authed),
            &w_id,
            s3_object.storage.clone(),
            &[(&s3_object.s3, S3Permission::READ)],
            None,
        )
        .await?;

        let message = format!(
            "file_key={}&exp={}{}",
            s3_object.s3.clone(),
            exp,
            windmill_object_store::s3_signature_storage_fragment(s3_object.storage.as_deref())
        );

        let mut max = HmacSha256::new_from_slice(workspace_key.as_bytes())
            .map_err(|err| Error::internal_err(format!("Failed to create hmac: {}", err)))?;
        max.update(message.as_bytes());
        let result = max.finalize();
        let signature = hex::encode(result.into_bytes());

        let presigned = format!("exp={}&sig={}", exp, signature);

        Ok::<_, Error>(S3Object { presigned: Some(presigned), ..s3_object })
    });

    let signed_s3_objects = futures::future::try_join_all(futures).await?;

    Ok(Json(signed_s3_objects))
}

#[cfg(not(feature = "parquet"))]
async fn sign_s3_objects() -> Result<()> {
    return Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ));
}

#[cfg(feature = "parquet")]
#[derive(Serialize)]
struct AppUploadFileResponse {
    file_key: String,
    delete_token: String,
}

#[cfg(feature = "parquet")]
async fn upload_s3_file_from_app(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<UploadFileToS3Query>,
    request: axum::extract::Request,
) -> JsonResult<AppUploadFileResponse> {
    // Same path confinement as `execute_component`: without it a token scoped to app A
    // could drive app B's upload policy. A guest's waits for the app's mode, below.
    if let Some(authed) = opt_authed
        .as_ref()
        .filter(|a| !windmill_api_auth::scopes::has_guest_sentinel(a.scopes.as_deref()))
    {
        check_scopes(authed, || format!("apps:run:{}", path.to_path()))?;
    }
    let policy = if let Some(file_key_regex) = query.force_viewer_file_key_regex {
        // `force_viewer_*` lets the caller supply a synthetic upload policy that
        // bypasses the deployed app's file_key_regex / resource restrictions.
        // It is intended for the app editor's preview path, so it must enforce
        // the same guards as `execute_component`'s preview mode (PR #9235):
        // authed caller, not an operator, and `apps:write` scope to make sure
        // an `apps:run`-scoped token cannot pick its own policy.
        let authed = opt_authed.as_ref().ok_or_else(|| {
            Error::NotAuthorized("App S3 preview upload requires authentication".to_string())
        })?;
        if authed.is_operator {
            return Err(Error::NotAuthorized(
                "Operators cannot run app S3 previews for security reasons".to_string(),
            ));
        }
        check_scopes(authed, || format!("apps:write:{}", path.to_path()))?;
        Some(Policy {
            stated_execution_mode: Some(ExecutionMode::Viewer),
            triggerables: None,
            triggerables_v2: None,
            on_behalf_of: None,
            on_behalf_of_email: None,
            s3_inputs: Some(vec![S3Input {
                file_key_regex: file_key_regex,
                allow_user_resources: query.force_viewer_allow_user_resources.unwrap_or(false),
                allow_workspace_resource: query
                    .force_viewer_allow_workspace_resource
                    .unwrap_or(false),
                allowed_resources: query
                    .force_viewer_allowed_resources
                    .map(|s| s.split(',').map(|s| s.to_string()).collect())
                    .unwrap_or_default(),
            }]),
            allowed_s3_keys: None,
            sandbox: None,
            frontend_sdk_scopes: None,
        })
    } else {
        let policy_o = sqlx::query_scalar!(
            "SELECT policy from app WHERE path = $1 AND workspace_id = $2",
            &path.0,
            &w_id
        )
        .fetch_optional(&db)
        .await?;

        policy_o
            .map(|p| serde_json::from_value::<Policy>(p).map_err(to_anyhow))
            .transpose()?
    };
    let opt_authed = guest_caller_for_mode(
        opt_authed,
        policy
            .as_ref()
            .map(Policy::execution_mode)
            .unwrap_or_default(),
        path.to_path(),
    )?;

    let user_db = UserDB::new(db.clone());

    let (s3_resource_opt, file_key, on_behalf_of_email, permissioned_as, username) = if policy
        .as_ref()
        .is_some_and(|p| p.s3_inputs.is_some())
    {
        let policy = policy.unwrap();
        let s3_inputs = policy.s3_inputs.as_ref().unwrap();

        let (username, permissioned_as, email) =
            get_on_behalf_details_from_policy_and_authed(&policy, &opt_authed).await?;

        let on_behalf_authed = fetch_api_authed_from_permissioned_as(
            permissioned_as.clone(),
            email.clone(),
            &w_id,
            &db,
            Some(username.clone()),
        )
        .await?;

        if let Some(file_key) = query.file_key {
            // file key is provided => requires workspace, user or list policy and must match the regex
            let matching_s3_inputs = if let Some(ref s3_resource_path) = query.s3_resource_path {
                s3_inputs
                    .iter()
                    .filter(|s3_input| {
                        s3_input.allowed_resources.contains(s3_resource_path)
                            || s3_input.allow_user_resources
                    })
                    .sorted_by_key(|i| i.allow_user_resources) // consider user resources last
                    .collect::<Vec<_>>()
            } else {
                s3_inputs
                    .iter()
                    .filter(|s3_input| s3_input.allow_workspace_resource)
                    .collect::<Vec<_>>()
            };

            let matched_input = matching_s3_inputs.iter().find(|s3_input| {
                match Regex::new(&s3_input.file_key_regex) {
                    Ok(re) => re.is_match(&file_key),
                    Err(e) => {
                        tracing::error!("Error compiling regex: {}", e);
                        false
                    }
                }
            });

            if let Some(matched_input) = matched_input {
                if let Some(ref s3_resource_path) = query.s3_resource_path {
                    if matched_input.allow_user_resources {
                        if let Some(authed) = opt_authed {
                            let db_with_opt_authed = DbWithOptAuthed::from_authed(
                                &authed,
                                db.clone(),
                                Some(user_db.clone()),
                            );
                            (
                                Some(
                                    get_s3_resource(
                                        &db_with_opt_authed,
                                        &w_id,
                                        s3_resource_path,
                                        None,
                                        None,
                                    )
                                    .await?,
                                ),
                                file_key,
                                email,
                                permissioned_as,
                                username,
                            )
                        } else {
                            return Err(Error::BadRequest(
                                "User resources are not allowed without being logged in"
                                    .to_string(),
                            ));
                        }
                    } else {
                        let db_with_opt_authed = DbWithOptAuthed::from_authed(
                            &on_behalf_authed,
                            db.clone(),
                            Some(user_db.clone()),
                        );
                        (
                            Some(
                                get_s3_resource(
                                    &db_with_opt_authed,
                                    &w_id,
                                    s3_resource_path,
                                    None,
                                    None,
                                )
                                .await?,
                            ),
                            file_key,
                            email,
                            permissioned_as,
                            username,
                        )
                    }
                } else {
                    let db_with_opt_authed = DbWithOptAuthed::from_authed(
                        &on_behalf_authed,
                        db.clone(),
                        Some(user_db.clone()),
                    );
                    let (_, s3_resource_opt) = get_workspace_s3_resource_and_check_paths(
                        &db_with_opt_authed,
                        Some(&on_behalf_authed),
                        &w_id,
                        None,
                        &[(&file_key, S3Permission::WRITE)],
                        None,
                    )
                    .await?;
                    (s3_resource_opt, file_key, email, permissioned_as, username)
                }
            } else {
                return Err(Error::BadRequest(
                    "No matching s3 resource found for the given file key".to_string(),
                ));
            }
        } else {
            // no file key => requires unnamed upload policy => allow workspace resource and file_key_regex is empty
            let has_unnamed_policy = s3_inputs.iter().any(|s3_input| {
                s3_input.allow_workspace_resource && s3_input.file_key_regex.is_empty()
            });

            if !has_unnamed_policy {
                return Err(Error::BadRequest(
                    "no policy found for unnamed s3 file upload".to_string(),
                ));
            }

            // for now, we place all files into `windmill_uploads` folder with a random name
            // TODO: make the folder configurable via the workspace settings
            let file_key = get_random_file_name(query.file_extension);
            let db_with_opt_authed =
                DbWithOptAuthed::from_authed(&on_behalf_authed, db.clone(), None);
            let (_, s3_resource_opt) = get_workspace_s3_resource_and_check_paths(
                &db_with_opt_authed,
                Some(&on_behalf_authed),
                &w_id,
                None,
                &[(&file_key, S3Permission::WRITE)],
                None,
            )
            .await?;

            (s3_resource_opt, file_key, email, permissioned_as, username)
        }
    } else {
        // backward compatibility (no policy)
        // if no policy but logged in, use the user's auth to get the s3 resource. A guest
        // has no standing of its own to upload with, so without a policy it is refused
        // exactly as an anonymous caller is.
        if let Some(authed) = opt_authed
            .filter(|a| !windmill_api_auth::scopes::has_guest_sentinel(a.scopes.as_deref()))
        {
            let file_key = query
                .file_key
                .unwrap_or_else(|| get_random_file_name(query.file_extension));

            let (on_behalf_of_email, permissioned_as, username) = (
                authed.email.clone(),
                username_to_permissioned_as(&authed.username),
                authed.display_username().to_string(),
            );

            if let Some(ref s3_resource_path) = query.s3_resource_path {
                let db_with_opt_authed =
                    DbWithOptAuthed::from_authed(&authed, db.clone(), Some(user_db.clone()));
                (
                    Some(
                        get_s3_resource(&db_with_opt_authed, &w_id, s3_resource_path, None, None)
                            .await?,
                    ),
                    file_key,
                    on_behalf_of_email,
                    permissioned_as,
                    username,
                )
            } else {
                let db_with_opt_authed = DbWithOptAuthed::from_authed(&authed, db.clone(), None);
                let (_, s3_resource) = get_workspace_s3_resource_and_check_paths(
                    &db_with_opt_authed,
                    Some(&authed),
                    &w_id,
                    None,
                    &[(&file_key, S3Permission::WRITE)],
                    None,
                )
                .await?;

                (
                    s3_resource,
                    file_key,
                    on_behalf_of_email,
                    permissioned_as,
                    username,
                )
            }
        } else {
            return Err(Error::BadRequest("Missing s3 policy".to_string()));
        }
    };

    let s3_resource = s3_resource_opt.ok_or(Error::internal_err(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = windmill_object_store::build_object_store_client(&s3_resource).await?;

    let options = Attributes::from_iter(vec![
        (
            Attribute::ContentType,
            query.content_type.unwrap_or_else(|| {
                mime_guess::from_path(&file_key)
                    .first_or_octet_stream()
                    .to_string()
            }),
        ),
        (
            Attribute::ContentDisposition,
            query.content_disposition.unwrap_or("inline".to_string()),
        ),
    ])
    .into();

    // Only workspace storage is quota-metered; a custom-resource upload lands in
    // the user's own bucket and is neither capped nor counted. An overwrite of an
    // existing key only spends the difference over its current size.
    let _is_workspace_storage = query.s3_resource_path.is_none();
    #[cfg(all(feature = "parquet", not(feature = "enterprise")))]
    if _is_workspace_storage {
        reject_reserved_volume_key(&file_key)?;
    }
    #[cfg(all(feature = "parquet", not(feature = "enterprise")))]
    let (max_size, _existing_size) = if _is_workspace_storage {
        let content_length = request
            .headers()
            .get(http::header::CONTENT_LENGTH)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok());
        let budget = ce_upload_budget(&db, &w_id, &s3_client, &file_key, content_length).await?;
        (Some(budget.max_size), budget.existing_size)
    } else {
        (None, 0)
    };
    #[cfg(any(not(feature = "parquet"), feature = "enterprise"))]
    let max_size: Option<usize> = None;

    match upload_file_from_req(s3_client, &file_key, request, options, max_size).await {
        Ok((_, _size)) =>
        {
            #[cfg(all(feature = "parquet", not(feature = "enterprise")))]
            if _is_workspace_storage {
                bump_storage_usage(
                    &db,
                    &w_id,
                    windmill_object_store::DEFAULT_STORAGE,
                    _size as i64 - _existing_size,
                )
                .await;
            }
        }
        Err(e) => {
            #[cfg(all(feature = "parquet", not(feature = "enterprise")))]
            spawn_storage_usage_recount_floored(&db, &w_id);
            return Err(e);
        }
    }

    let delete_token = jwt::encode_with_internal_secret(S3DeleteTokenClaims {
        file_key: file_key.clone(),
        on_behalf_of_email,
        permissioned_as,
        username,
        s3_resource_path: query.s3_resource_path,
        workspace: w_id.clone(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(12)).timestamp() as usize,
    })
    .await?;

    return Ok(Json(AppUploadFileResponse { file_key, delete_token }));
}

#[cfg(feature = "parquet")]
#[derive(Deserialize)]
struct DeleteS3FileQuery {
    delete_token: String,
}

#[cfg(feature = "parquet")]
async fn delete_s3_file_from_app(
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<DeleteS3FileQuery>,
) -> Result<()> {
    let S3DeleteTokenClaims {
        file_key,
        on_behalf_of_email,
        permissioned_as,
        username,
        s3_resource_path,
        workspace,
        ..
    } = jwt::decode_with_internal_secret::<S3DeleteTokenClaims>(&query.delete_token).await?;

    let path = windmill_object_store::object_store_reexports::Path::parse(file_key.as_str())
        .map_err(|e| Error::internal_err(format!("Error parsing file key: {}", e)))?;

    if workspace != w_id {
        return Err(Error::BadRequest("Invalid workspace".to_string()));
    }

    let on_behalf_authed = fetch_api_authed_from_permissioned_as(
        permissioned_as,
        on_behalf_of_email,
        &w_id,
        &db,
        Some(username),
    )
    .await?;

    let s3_resource = if let Some(s3_resource_path) = s3_resource_path {
        let db_with_opt_authed =
            DbWithOptAuthed::from_authed(&on_behalf_authed, db.clone(), Some(user_db.clone()));
        get_s3_resource(
            &db_with_opt_authed,
            &w_id,
            s3_resource_path.as_str(),
            None,
            None,
        )
        .await?
    } else {
        let db_with_opt_authed = DbWithOptAuthed::from_authed(&on_behalf_authed, db.clone(), None);
        let (_, s3_resource) = get_workspace_s3_resource_and_check_paths(
            &db_with_opt_authed,
            Some(&on_behalf_authed),
            &w_id,
            None,
            &[(&path.to_string(), S3Permission::DELETE)],
            None,
        )
        .await?;

        s3_resource.ok_or(Error::internal_err(
            "No files storage resource defined at the workspace level".to_string(),
        ))?
    };

    let s3_client = windmill_object_store::build_object_store_client(&s3_resource).await?;

    s3_client.delete(&path).await.map_err(|err| {
        tracing::error!("Error deleting file: {:?}", err);
        Error::internal_err(format!("Error deleting file: {}", err.to_string()))
    })?;

    Ok(())
}

#[cfg(not(feature = "parquet"))]
async fn download_s3_file_from_app() -> Result<()> {
    return Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ));
}

#[cfg(feature = "parquet")]
async fn get_on_behalf_authed_from_app(
    db: &DB,
    path: &str,
    w_id: &str,
    opt_authed: &Option<ApiAuthed>,
    force_allowed_s3_keys: Option<Vec<S3Key>>,
) -> Result<(ApiAuthed, Policy)> {
    let policy = if let Some(force_allowed_s3_keys) = force_allowed_s3_keys {
        Policy {
            stated_execution_mode: Some(ExecutionMode::Viewer),
            triggerables: None,
            triggerables_v2: None,
            on_behalf_of: None,
            on_behalf_of_email: None,
            s3_inputs: None,
            allowed_s3_keys: Some(force_allowed_s3_keys),
            sandbox: None,
            frontend_sdk_scopes: None,
        }
    } else {
        // TODO: improve db query to not return uneeded fields
        let policy_o = sqlx::query_scalar!(
            "SELECT policy from app WHERE path = $1 AND workspace_id = $2",
            path,
            w_id
        )
        .fetch_optional(db)
        .await?;

        policy_o
            .map(|p| serde_json::from_value::<Policy>(p).map_err(to_anyhow))
            .transpose()?
            .unwrap_or_else(|| Policy {
                stated_execution_mode: Some(ExecutionMode::Viewer),
                triggerables: None,
                triggerables_v2: None,
                on_behalf_of: None,
                on_behalf_of_email: None,
                s3_inputs: None,
                allowed_s3_keys: None,
                sandbox: None,
                frontend_sdk_scopes: None,
            })
    };

    let opt_authed = guest_caller_for_mode(opt_authed.clone(), policy.execution_mode(), path)?;
    let (username, permissioned_as, email) =
        get_on_behalf_details_from_policy_and_authed(&policy, &opt_authed).await?;

    let on_behalf_authed =
        fetch_api_authed_from_permissioned_as(permissioned_as, email, &w_id, &db, Some(username))
            .await?;

    Ok((on_behalf_authed, policy))
}

/// Which identity a deployed `apps_u/*` S3 read runs as.
#[cfg(feature = "parquet")]
enum AppS3ReadIdentity {
    /// The gate passed: read with the policy's on-behalf identity (the app author in
    /// author-mode, the viewer in viewer-mode).
    OnBehalf,
    /// The gate did not pass but a logged-in, non-embed viewer is present: read with
    /// the viewer's OWN identity so the downstream S3 permission check self-enforces
    /// their entitlement (never the author's).
    AsViewer(ApiAuthed),
}

#[cfg(feature = "parquet")]
async fn check_if_allowed_to_access_s3_file_from_app(
    db: &DB,
    opt_authed: &Option<ApiAuthed>,
    file_query: &AppS3FileQuery,
    w_id: &str,
    path: &str,
    policy: &Policy,
) -> Result<AppS3ReadIdentity> {
    let is_app_embed = opt_authed.as_ref().is_some_and(|authed| {
        windmill_api_auth::scopes::has_app_embed_sentinel(authed.scopes.as_deref())
    });

    // A valid presigned bearer is a self-authorizing capability, so it short-circuits
    // the provenance gate. OSS builds cannot validate signatures (no workspace-key
    // HMAC), so there the bearer is ignored and the request falls through to the
    // checks below — the same path these routes took before presigning.
    if file_query.sig.is_some() {
        #[cfg(feature = "private")]
        {
            crate::s3_proxy_ee::validate_s3_signature(
                &file_query.s3,
                &file_query.sig,
                &file_query.exp,
                &file_query.storage,
                w_id,
                &db,
            )
            .await?;
            return Ok(AppS3ReadIdentity::OnBehalf);
        }
    }

    if matches!(policy.execution_mode(), ExecutionMode::Viewer) && !is_app_embed {
        // Viewer mode: the on-behalf identity IS the viewer, so the downstream
        // get_workspace_s3_resource_and_check_paths already bounds the read by
        // their own perms — no provenance gate (it would over-restrict). Embed
        // tokens are excluded (untrusted app JS stays confined below).
        return Ok(AppS3ReadIdentity::OnBehalf);
    }

    // Author-mode/embed: confine reads to the app's declared keys or files THIS
    // app produced, else a viewer could launder the author's S3 perms via an
    // arbitrary file_key (confused deputy). Provenance is the un-forgeable
    // app-origination marker (`trigger_kind='app'` + `trigger=<this app>`);
    // `created_by=<caller>` is ANDed only as a per-viewer isolation filter (it
    // can narrow — one viewer can't read another's result — never forge).
    let creator = opt_authed
        .as_ref()
        .map(|authed| authed.username.clone())
        .unwrap_or_else(|| "anonymous".to_string());
    let allowed = policy.allowed_s3_keys.as_ref().is_some_and(|keys| {
        keys.iter()
            .any(|key| key.s3_path == file_query.s3 && key.storage == file_query.storage)
    }) || {
        sqlx::query_scalar!(
            r#"SELECT EXISTS (
                SELECT 1 FROM v2_job_completed c JOIN v2_job j USING (id)
                WHERE j.workspace_id = $2
                    AND c.started_at > now() - interval '3 hours'
                    AND c.result @> ('{"s3":"' || $1 ||  '"}')::jsonb
                    AND j.trigger_kind = 'app'
                    AND j.trigger = $3
                    AND j.created_by = $4
            )"#,
            file_query.s3,
            w_id,
            path,
            creator,
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false)
    };

    if allowed {
        return Ok(AppS3ReadIdentity::OnBehalf);
    }

    // Gate denied. A viewer whose token is effectively unscoped falls back to reading
    // as THEMSELVES: the file is still bounded by their own S3 perms downstream, and
    // such a token can already fetch it via `job_helpers/download_s3_file`, so the
    // fallback adds zero capability. `is_effectively_unscoped` (the same predicate the
    // route-scope middleware uses) is what makes that true: a genuinely scope-restricted
    // token (e.g. `apps:read:<path>`) is allowed on `apps_u/*` but REJECTED on
    // `job_helpers/*`, so serving it the file here WOULD be a new capability — it stays
    // gated. `!is_app_embed` keeps that confinement explicit (embed tokens carry the
    // `app_embed` scope, so they are already scope-restricted). Anonymous callers (no
    // identity) also have no viewer to fall back to. Only the confused-deputy denial
    // reaches the message below.
    match opt_authed.as_ref() {
        Some(viewer)
            if !is_app_embed
                && windmill_api_auth::is_effectively_unscoped(viewer.scopes.as_deref()) =>
        {
            Ok(AppS3ReadIdentity::AsViewer(viewer.clone()))
        }
        _ => Err(Error::BadRequest(format!(
            "S3 file \"{}\" is not accessible from this app. A deployed app running on \
             behalf of its author only serves files it generated, files in its declared \
             allowlist, or presigned files. To expose a pre-existing file, sign it \
             (signS3Object / sign_s3_object) or set the app's execution mode to \"viewer\".",
            file_query.s3
        ))),
    }
}

#[cfg(feature = "parquet")]
#[derive(Deserialize, Debug)]
struct AppS3FileQuery {
    s3: String,
    storage: Option<String>,
    sig: Option<String>,
    #[cfg(feature = "private")]
    exp: Option<String>,
}

#[cfg(feature = "parquet")]
#[derive(Deserialize, Debug)]
struct AppS3FileQueryWithForceViewerAllowedS3Keys {
    #[serde(flatten)]
    pub file_query: AppS3FileQuery,
    pub force_viewer_allowed_s3_keys: Option<String>,
}

/// Confine a scoped caller to THIS app's S3 files, or it could read another app's
/// through that app's on-behalf policy. Either grant reads them: `apps:read:<path>`
/// (app embed tokens) or `apps:run:<path>` (a run-scoped token fetching back what its
/// own runs produced). Unscoped/anonymous callers fall through to the provenance gate.
#[cfg(feature = "parquet")]
fn check_app_s3_read_scope(opt_authed: &Option<ApiAuthed>, path: &str) -> Result<()> {
    let Some(authed) = opt_authed.as_ref() else {
        return Ok(());
    };
    // A guest's confinement waits for the app's mode (`get_on_behalf_authed_from_app`).
    if windmill_api_auth::scopes::has_guest_sentinel(authed.scopes.as_deref()) {
        return Ok(());
    }
    check_scopes(authed, || format!("apps:run:{}", path))
        .or_else(|_| check_scopes(authed, || format!("apps:read:{}", path)))
}

#[cfg(feature = "parquet")]
async fn download_s3_file_from_app(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<AppS3FileQueryWithForceViewerAllowedS3Keys>,
) -> Result<Response> {
    use crate::db::OptJobAuthed;

    let path = path.to_path();

    check_app_s3_read_scope(&opt_authed, path)?;

    let force_viewer_allowed_s3_keys = if let Some(force_viewer_allowed_s3_keys) =
        query.force_viewer_allowed_s3_keys.clone()
    {
        // `force_viewer_allowed_s3_keys` lets the caller supply a synthetic
        // allowlist that bypasses the deployed app policy. Apply the same
        // preview-mode guard as `execute_component` (PR #9235): authed, not an
        // operator, `apps:write` scope so an `apps:run`-scoped token cannot
        // pick its own allowlist.
        let authed = opt_authed.as_ref().ok_or_else(|| {
            Error::NotAuthorized("App S3 preview download requires authentication".to_string())
        })?;
        if authed.is_operator {
            return Err(Error::NotAuthorized(
                "Operators cannot run app S3 previews for security reasons".to_string(),
            ));
        }
        check_scopes(authed, || format!("apps:write:{}", path))?;
        Some(serde_json::from_str::<Vec<S3Key>>(&force_viewer_allowed_s3_keys).unwrap_or_default())
    } else {
        None
    };

    let (on_behalf_authed, policy) =
        get_on_behalf_authed_from_app(&db, &path, &w_id, &opt_authed, force_viewer_allowed_s3_keys)
            .await?;

    let read_authed = match check_if_allowed_to_access_s3_file_from_app(
        &db,
        &opt_authed,
        &query.file_query,
        &w_id,
        &path,
        &policy,
    )
    .await?
    {
        AppS3ReadIdentity::OnBehalf => on_behalf_authed,
        AppS3ReadIdentity::AsViewer(viewer) => viewer,
    };

    download_s3_file_internal(
        OptJobAuthed { authed: read_authed, job_id: None },
        &db,
        None,
        &w_id,
        DownloadFileQuery {
            file_key: query.file_query.s3,
            s3_resource_path: None,
            storage: query.file_query.storage,
        },
    )
    .await
}

// Presigned bearer params (`exp=..&sig=..`) extracted as a second `Query` so the
// app-scoped preview/count/metadata routes honor a presigned key the same way the
// raw `download_s3_file` route does.
#[cfg(feature = "parquet")]
#[derive(Deserialize)]
struct AppS3Sig {
    sig: Option<String>,
    #[cfg(feature = "private")]
    exp: Option<String>,
}

#[cfg(feature = "parquet")]
fn app_s3_file_query(s3: String, storage: Option<String>, sig: AppS3Sig) -> AppS3FileQuery {
    AppS3FileQuery {
        s3,
        storage,
        sig: sig.sig,
        #[cfg(feature = "private")]
        exp: sig.exp,
    }
}

/// Shared entry for every app-scoped (`apps_u/*`) S3 display op: confine a scoped
/// caller to this app, resolve the on-behalf identity per `execution_mode`, then run
/// the provenance gate (`check_if_allowed_to_access_s3_file_from_app`) once before
/// dispatching to the S3 helpers.
#[cfg(feature = "parquet")]
async fn app_s3_on_behalf_and_provenance(
    db: &DB,
    path: &str,
    w_id: &str,
    opt_authed: &Option<ApiAuthed>,
    file_query: &AppS3FileQuery,
) -> Result<crate::db::OptJobAuthed> {
    check_app_s3_read_scope(opt_authed, path)?;
    let (on_behalf_authed, policy) =
        get_on_behalf_authed_from_app(db, path, w_id, opt_authed, None).await?;
    let read_authed = match check_if_allowed_to_access_s3_file_from_app(
        db, opt_authed, file_query, w_id, path, &policy,
    )
    .await?
    {
        AppS3ReadIdentity::OnBehalf => on_behalf_authed,
        AppS3ReadIdentity::AsViewer(viewer) => viewer,
    };
    Ok(crate::db::OptJobAuthed { authed: read_authed, job_id: None })
}

// The app-scoped display ops carry the app path in the URL and everything else
// (file_key + op args) in the query, so they avoid a second `{*path}` wildcard.
// `LoadCountQuery` / `LoadPreviewQuery` don't include the file key (it's a path
// param on the raw `job_helpers/*` route), so restate their fields here with the
// file key added. Do NOT `#[serde(flatten)]` the inner struct: axum's `Query`
// uses `serde_urlencoded`, which cannot deserialize a flattened field's typed
// (numeric/bool) values and 400s on `limit`/`offset` — the fields must be
// declared directly on the outer struct.
#[cfg(feature = "parquet")]
#[derive(Deserialize)]
struct AppLoadCountQuery {
    file_key: String,
    search_col: Option<String>,
    search_term: Option<String>,
    storage: Option<String>,
}

#[cfg(feature = "parquet")]
impl AppLoadCountQuery {
    fn into_inner(self) -> (String, LoadCountQuery) {
        (
            self.file_key,
            LoadCountQuery {
                search_col: self.search_col,
                search_term: self.search_term,
                storage: self.storage,
            },
        )
    }
}

#[cfg(feature = "parquet")]
#[derive(Deserialize)]
struct AppLoadPreviewQuery {
    file_key: String,
    limit: Option<u32>,
    offset: Option<i64>,
    sort_col: Option<String>,
    sort_desc: Option<bool>,
    search_col: Option<String>,
    search_term: Option<String>,
    storage: Option<String>,
    csv_separator: Option<String>,
}

#[cfg(feature = "parquet")]
impl AppLoadPreviewQuery {
    fn into_inner(self) -> (String, LoadPreviewQuery) {
        (
            self.file_key,
            LoadPreviewQuery {
                limit: self.limit,
                offset: self.offset,
                sort_col: self.sort_col,
                sort_desc: self.sort_desc,
                search_col: self.search_col,
                search_term: self.search_term,
                storage: self.storage,
                csv_separator: self.csv_separator,
            },
        )
    }
}

#[cfg(feature = "parquet")]
async fn app_download_s3_parquet_file_as_csv(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<DownloadFileQuery>,
    Query(sig): Query<AppS3Sig>,
) -> Result<Response> {
    let path = path.to_path();
    let file_query = app_s3_file_query(query.file_key.clone(), query.storage.clone(), sig);
    let job_authed =
        app_s3_on_behalf_and_provenance(&db, &path, &w_id, &opt_authed, &file_query).await?;
    crate::job_helpers_oss::download_s3_parquet_file_as_csv_internal(
        job_authed,
        &db,
        None,
        &w_id,
        DownloadFileQuery {
            file_key: query.file_key,
            s3_resource_path: None,
            storage: query.storage,
        },
    )
    .await
}

#[cfg(feature = "parquet")]
async fn app_load_file_metadata(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(mut query): Query<LoadFileMetadataQuery>,
    Query(sig): Query<AppS3Sig>,
) -> Result<Response> {
    let path = path.to_path();
    let file_query = app_s3_file_query(query.file_key.clone(), query.storage.clone(), sig);
    let job_authed =
        app_s3_on_behalf_and_provenance(&db, &path, &w_id, &opt_authed, &file_query).await?;
    // On-behalf app reads are confined to the workspace storage; a
    // viewer-supplied custom resource must not be honored.
    query.s3_resource_path = None;
    let resp =
        crate::job_helpers_oss::load_file_metadata_internal(job_authed, &db, None, &w_id, query)
            .await?;
    Ok(Json(resp).into_response())
}

#[cfg(feature = "parquet")]
async fn app_load_file_preview(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(mut query): Query<LoadFilePreviewQuery>,
    Query(sig): Query<AppS3Sig>,
) -> Result<Response> {
    let path = path.to_path();
    let file_query = app_s3_file_query(query.file_key.clone(), query.storage.clone(), sig);
    let job_authed =
        app_s3_on_behalf_and_provenance(&db, &path, &w_id, &opt_authed, &file_query).await?;
    // On-behalf app reads are confined to the workspace storage; a
    // viewer-supplied custom resource must not be honored.
    query.s3_resource_path = None;
    let resp =
        crate::job_helpers_oss::load_file_preview_internal(job_authed, &db, None, &w_id, query)
            .await?;
    Ok(Json(resp).into_response())
}

#[cfg(feature = "parquet")]
async fn app_load_table_count(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<AppLoadCountQuery>,
    Query(sig): Query<AppS3Sig>,
) -> Result<Response> {
    let path = path.to_path();
    let (file_key, inner) = query.into_inner();
    let file_query = app_s3_file_query(file_key.clone(), inner.storage.clone(), sig);
    let job_authed =
        app_s3_on_behalf_and_provenance(&db, &path, &w_id, &opt_authed, &file_query).await?;
    let resp =
        crate::job_helpers_oss::load_table_count_internal(job_authed, &db, &w_id, file_key, inner)
            .await?;
    Ok(Json(resp).into_response())
}

#[cfg(feature = "parquet")]
async fn app_load_parquet_preview(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<AppLoadPreviewQuery>,
    Query(sig): Query<AppS3Sig>,
) -> Result<Response> {
    let path = path.to_path();
    let (file_key, inner) = query.into_inner();
    let file_query = app_s3_file_query(file_key.clone(), inner.storage.clone(), sig);
    let job_authed =
        app_s3_on_behalf_and_provenance(&db, &path, &w_id, &opt_authed, &file_query).await?;
    let resp = crate::job_helpers_oss::load_preview_internal(
        job_authed, &db, &w_id, file_key, inner, true,
    )
    .await?;
    Ok(Json(resp).into_response())
}

#[cfg(feature = "parquet")]
async fn app_load_csv_preview(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<AppLoadPreviewQuery>,
    Query(sig): Query<AppS3Sig>,
) -> Result<Response> {
    let path = path.to_path();
    let (file_key, inner) = query.into_inner();
    let file_query = app_s3_file_query(file_key.clone(), inner.storage.clone(), sig);
    let job_authed =
        app_s3_on_behalf_and_provenance(&db, &path, &w_id, &opt_authed, &file_query).await?;
    let resp = crate::job_helpers_oss::load_preview_internal(
        job_authed, &db, &w_id, file_key, inner, false,
    )
    .await?;
    Ok(Json(resp).into_response())
}

#[cfg(not(feature = "parquet"))]
async fn app_download_s3_parquet_file_as_csv() -> Result<()> {
    Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ))
}

#[cfg(not(feature = "parquet"))]
async fn app_load_file_metadata() -> Result<()> {
    Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ))
}

#[cfg(not(feature = "parquet"))]
async fn app_load_file_preview() -> Result<()> {
    Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ))
}

#[cfg(not(feature = "parquet"))]
async fn app_load_table_count() -> Result<()> {
    Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ))
}

#[cfg(not(feature = "parquet"))]
async fn app_load_parquet_preview() -> Result<()> {
    Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ))
}

#[cfg(not(feature = "parquet"))]
async fn app_load_csv_preview() -> Result<()> {
    Err(Error::BadRequest(
        "This endpoint requires the parquet feature to be enabled".to_string(),
    ))
}

fn get_on_behalf_of(policy: &Policy) -> Result<(String, String)> {
    let permissioned_as = policy
        .on_behalf_of
        .as_ref()
        .ok_or_else(|| {
            Error::BadRequest(
                "on_behalf_of is missing in the app policy and is required for anonymous execution"
                    .to_string(),
            )
        })?
        .to_string();
    let email = policy
        .on_behalf_of_email
        .as_ref()
        .ok_or_else(|| {
            Error::BadRequest(
                "on_behalf_of_email is missing in the app policy and is required for anonymous execution"
                    .to_string(),
            )
        })?
        .to_string();
    // Defence in depth against a policy that already carries a forged superadmin
    // sentinel (deployed before validation existed, or copied verbatim by a
    // workspace fork): the sentinels are internal-only and never a legitimate app
    // run identity, so refuse to execute rather than mint a superadmin token.
    if windmill_common::auth::is_reserved_on_behalf_of_identity(
        Some(&permissioned_as),
        Some(&email),
    ) {
        return Err(Error::BadRequest(
            "app on_behalf_of is a reserved internal identity and cannot be executed".to_string(),
        ));
    }
    Ok((permissioned_as, email))
}

async fn exists_app(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2)",
        path.to_path(),
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn build_args(
    policy: &Policy,
    PolicyTriggerableInputs {
        static_inputs,
        one_of_inputs,
        allow_user_resources,
        sensitive_inputs,
        ..
    }: &PolicyTriggerableInputs,
    mut args: HashMap<String, Box<RawValue>>,
    authed: Option<&ApiAuthed>,
    user_db: &UserDB,
    db: &DB,
    w_id: &str,
) -> Result<(PushArgsOwned, Option<Uuid>)> {
    let mut job_id: Option<Uuid> = None;
    let mut safe_args = HashMap::<String, Box<RawValue>>::new();

    // tracing::error!("{:?}", allow_user_resources);
    for k in allow_user_resources.iter() {
        if let Some(arg_val) = args.get(k) {
            let key = serde_json::from_str::<String>(arg_val.get()).ok();
            if let Some(path) =
                key.and_then(|x| x.clone().strip_prefix("$res:").map(|x| x.to_string()))
            {
                if let Some(authed) = authed {
                    let db_with_opt_authed =
                        DbWithOptAuthed::from_authed(authed, db.clone(), Some(user_db.clone()));
                    let res = get_resource_value_interpolated_internal(
                        &db_with_opt_authed,
                        w_id,
                        &path,
                        None,
                        None,
                        false,
                    )
                    .await?;
                    if res.is_none() {
                        return Err(Error::BadRequest(format!(
                            "Resource {} not found or not allowed for viewer",
                            path
                        )));
                    }
                    let job_id = if let Some(job_id) = job_id {
                        job_id
                    } else {
                        job_id = Some(ulid::Ulid::new().into());
                        job_id.unwrap()
                    };
                    let mc = build_crypt_with_key_suffix(&db, &w_id, &job_id.to_string()).await?;
                    let encrypted = encrypt(&mc, to_raw_value(&res.unwrap()).get());
                    safe_args.insert(
                        k.to_string(),
                        to_raw_value(&format!("$encrypted:{encrypted}")),
                    );
                } else {
                    return Err(Error::BadRequest(
                        "User resources are not allowed without being logged in".to_string(),
                    ));
                }
            }
        }
    }

    for (k, v) in one_of_inputs {
        if safe_args.contains_key(k) {
            continue;
        }
        if let Some(arg_val) = args.get(k) {
            let arg_str = arg_val.get();

            let options_str_vec = v.iter().map(|x| x.get()).collect::<Vec<&str>>();
            if options_str_vec.contains(&arg_str) {
                safe_args.insert(k.to_string(), arg_val.clone());
                args.remove(k);
                continue;
            }

            // check if multiselect
            if let Ok(args_str_vec) = serde_json::from_str::<Vec<Box<RawValue>>>(arg_val.get()) {
                if args_str_vec
                    .iter()
                    .all(|x| options_str_vec.contains(&x.get()))
                {
                    safe_args.insert(k.to_string(), arg_val.clone());
                    args.remove(k);
                    continue;
                }
            }

            return Err(Error::BadRequest(format!(
                "argument {} with value {} must be one of [{}]",
                k,
                arg_str,
                options_str_vec.join(",")
            )));
        }
    }

    for (k, v) in args {
        if safe_args.contains_key(&k) {
            continue;
        }

        let arg_str = v.get();

        if arg_str.starts_with("\"$ctx:") {
            let prop = arg_str.trim_start_matches("\"$ctx:").trim_end_matches("\"");
            let value = match prop {
                "username" => authed
                    .as_ref()
                    .map(|a| serde_json::to_value(a.display_username())),
                "email" => authed.as_ref().map(|a| serde_json::to_value(&a.email)),
                "workspace" => Some(serde_json::to_value(&w_id)),
                "groups" => authed.as_ref().map(|a| serde_json::to_value(&a.groups)),
                "author" => Some(serde_json::to_value(&policy.on_behalf_of_email)),
                _ => {
                    return Err(Error::BadRequest(format!(
                        "context variable {} not allowed",
                        prop
                    )))
                }
            };
            safe_args.insert(
                k.to_string(),
                to_raw_value(&value.unwrap_or(Ok(serde_json::Value::Null)).map_err(|e| {
                    Error::internal_err(format!(
                        "failed to serialize ctx variable for {}: {}",
                        k, e
                    ))
                })?),
            );
        } else if !arg_str.contains("\"$var:") && !arg_str.contains("\"$res:") {
            safe_args.insert(k.to_string(), v);
        } else {
            safe_args.insert(
                k.to_string(),
                RawValue::from_string(
                    arg_str
                        .replace(
                            "$var:",
                            "The following variable has been omitted for security reasons: ",
                        )
                        .replace(
                            "$res:",
                            "The following resource has been omitted for security reasons, to allow it, toggle: 'Allow resources from users' on that field input: ",
                        ),
                )
                .map_err(|e| {
                    Error::internal_err(format!(
                        "failed to remove sensitive variable(s)/resource(s) with error: {}",
                        e
                    ))
                })?,
            );
        }
    }
    for k in sensitive_inputs {
        let Some(v) = safe_args.get(k) else { continue };
        let raw = v.get();
        if raw.starts_with("\"$encrypted:") {
            continue;
        }
        let job_id = if let Some(job_id) = job_id {
            job_id
        } else {
            job_id = Some(ulid::Ulid::new().into());
            job_id.unwrap()
        };
        let mc = build_crypt_with_key_suffix(&db, &w_id, &job_id.to_string()).await?;
        let encrypted = encrypt(&mc, raw);
        safe_args.insert(
            k.to_string(),
            to_raw_value(&format!("$encrypted:{encrypted}")),
        );
    }

    let mut extra = HashMap::new();
    for (k, v) in static_inputs {
        extra.insert(k.to_string(), v.to_owned());
    }
    Ok((
        PushArgsOwned { extra: Some(extra), args: safe_args },
        job_id,
    ))
}

#[cfg(test)]
mod embed_token_tests {
    use super::APP_EMBED_SCOPES;
    use windmill_api_auth::scopes::check_scopes_for_route;

    /// The embed token must reach exactly the endpoints an app needs and nothing
    /// else. This locks the allow/deny matrix that confines a malicious or
    /// compromised app to app-only routes (WIN-2006).
    #[test]
    fn embed_scopes_allow_app_routes_and_deny_the_rest() {
        let mut scopes: Vec<String> = APP_EMBED_SCOPES.iter().map(|s| s.to_string()).collect();
        // Mirror mint_app_embed_token: the per-app path-scoped read + run.
        scopes.push("apps:read:u/admin/app".to_string());
        scopes.push("apps:run:u/admin/app".to_string());
        let scopes = Some(scopes.as_slice());

        // Allowed: the routes a running app legitimately calls.
        let allowed = [
            // Own definition + the public app-serving / execution endpoints.
            ("/api/w/test/apps/get/p/u/admin/app", "GET"),
            ("/api/w/test/apps_u/public_app/secret", "GET"),
            ("/api/w/test/apps_u/get_data/v/secret.js", "GET"),
            ("/api/w/test/apps_u/public_resource/f/app_themes/t", "GET"),
            ("/api/w/test/apps_u/execute_component/u/admin/app", "POST"),
            // S3 file upload from the app's S3 File Input component: a `run` action
            // (RUN_PATH_ACTIONS) so the embed token reaches it; the handler re-checks
            // `apps:run:<path>` to confine it to this app, like execute_component.
            ("/api/w/test/apps_u/upload_s3_file/u/admin/app", "POST"),
            // By-id job poll routes (the JobLoader surface) stay allowed.
            ("/api/w/test/jobs_u/get/some-uuid", "GET"),
            ("/api/w/test/jobs_u/getupdate/some-uuid", "GET"),
            ("/api/w/test/jobs_u/getupdate_sse/some-uuid", "GET"),
            ("/api/w/test/jobs_u/completed/get_result/some-uuid", "GET"),
            ("/api/w/test/jobs_u/completed/get_timing/some-uuid", "GET"),
            // By-id cancel (POST): permitted at the route layer; the handler confines
            // it to the app's own jobs (created_by == viewer).
            ("/api/w/test/jobs_u/queue/cancel/some-uuid", "POST"),
            ("/api/w/test/users/whoami", "GET"),
            // Resource METADATA only (picker list + type schemas) — never values.
            ("/api/w/test/resources/list", "GET"),
            ("/api/w/test/resources/exists/u/admin/r", "GET"),
            ("/api/w/test/resources/type/list", "GET"),
            ("/api/w/test/folders/listnames", "GET"),
        ];
        for (path, method) in allowed {
            assert!(
                check_scopes_for_route(scopes, path, method).is_ok(),
                "embed token should allow {method} {path}"
            );
        }

        // Denied: anything outside what an app needs, including app management
        // (apps:write is intentionally withheld), resource VALUE reads (which can
        // hold credentials), and other workspace domains.
        let denied = [
            ("/api/w/test/apps/update/u/admin/app", "POST"),
            ("/api/w/test/apps/delete/u/admin/app", "DELETE"),
            // Workspace app inventory must NOT be reachable (Apps domain is
            // default-denied for the embed sentinel; only own-def + apps_u/* allowed).
            ("/api/w/test/apps/exists/u/admin/app", "GET"),
            ("/api/w/test/apps/custom_path_exists/foo", "GET"),
            (
                "/api/w/test/apps/list_paths_from_workspace_runnable/script/u/admin/x",
                "GET",
            ),
            ("/api/w/test/apps/list", "GET"),
            // The embed-token MINT endpoints are public app routes (`apps_u/`) but
            // create credentials — denied so a captured embed token can't renew
            // itself indefinitely past the 12h expiry (refresh is the embedder's job).
            ("/api/w/test/apps_u/embed_token/secret", "GET"),
            ("/api/w/test/apps_u/embed_token_by_custom_path/foo", "GET"),
            ("/api/w/test/scripts/list", "GET"),
            ("/api/w/test/variables/list", "GET"),
            ("/api/w/test/resources/update/u/admin/r", "POST"),
            // Resource value reads must NOT be reachable with the embed token.
            ("/api/w/test/resources/get/u/admin/r", "GET"),
            ("/api/w/test/resources/get_value/u/admin/r", "GET"),
            (
                "/api/w/test/resources/get_value_interpolated/u/admin/r",
                "GET",
            ),
            ("/api/w/test/resources/list_search", "GET"),
            // Workspace-wide job enumeration/export must NOT be reachable — an app
            // reads only jobs it launched, by id (blocked via the app_embed sentinel).
            ("/api/w/test/jobs/list", "GET"),
            ("/api/w/test/jobs/list_filtered_uuids", "GET"),
            ("/api/w/test/jobs/completed/list", "GET"),
            ("/api/w/test/jobs/completed/export", "GET"),
            ("/api/w/test/jobs/queue/list", "GET"),
            ("/api/w/test/jobs/queue/list_filtered_uuids", "GET"),
            ("/api/w/test/jobs/queue/export", "GET"),
            // Job counts (workspace-wide aggregates) and the capability-minting
            // routes (signed resume/approval URLs) are NOT by-id polling — denied.
            ("/api/w/test/jobs/completed/count", "GET"),
            ("/api/w/test/jobs/completed/count_jobs", "GET"),
            ("/api/w/test/jobs/queue/count", "GET"),
            ("/api/w/test/jobs/job_signature/some-uuid/some-rid", "GET"),
            ("/api/w/test/jobs/resume_urls/some-uuid/some-rid", "GET"),
            // get_root_job_id has no access check in its handler and the app never
            // calls it — denied so the token can't probe foreign jobs' flow lineage.
            ("/api/w/test/jobs_u/get_root_job_id/some-uuid", "GET"),
            // `users:read`/`folders:read` exist only for whoami/listnames — every
            // other route in those domains is denied via the app_embed sentinel
            // (the whole /users and /folders routers are CORS-enabled for the iframe).
            ("/api/w/test/users/list", "GET"),
            ("/api/w/test/users/list_usage", "GET"),
            ("/api/w/test/users/username_to_email/admin", "GET"),
            ("/api/w/test/folders/list", "GET"),
            ("/api/w/test/folders/get/myfolder", "GET"),
            ("/api/w/test/folders/getusage/myfolder", "GET"),
        ];
        for (path, method) in denied {
            assert!(
                check_scopes_for_route(scopes, path, method).is_err(),
                "embed token should deny {method} {path}"
            );
        }
    }

    /// `apps:run` satisfies read at the route layer, so `apps/list` / `apps/list_search`
    /// pass the route check — that's why those handlers ALSO call
    /// `check_scopes(apps:read)`, which uses `ScopeDefinition::includes` (where run
    /// does NOT include read). Lock that: no embed scope, including the
    /// dynamically-minted path-scoped read, satisfies a domain-level `apps:read`, so
    /// the token cannot list all apps' definitions (their full `value`/code).
    #[test]
    fn embed_scopes_cannot_satisfy_domain_app_read() {
        use windmill_api_auth::scopes::ScopeDefinition;
        let mut scopes: Vec<String> = APP_EMBED_SCOPES.iter().map(|s| s.to_string()).collect();
        // mint_app_embed_token also grants read scoped to the single app path:
        scopes.push("apps:read:u/admin/app".to_string());
        let required = ScopeDefinition::from_scope_string("apps:read").unwrap();
        for s in &scopes {
            // The `app_embed` sentinel intentionally doesn't parse as a domain:action
            // scope (it grants nothing; it only drives the job-enumeration deny).
            let Ok(def) = ScopeDefinition::from_scope_string(s) else {
                continue;
            };
            assert!(
                !def.includes(&required),
                "embed scope {s} must not satisfy domain-level apps:read (would leak apps/list[_search])"
            );
        }
        // Sanity: a genuine domain-level apps:read token does satisfy it.
        assert!(ScopeDefinition::from_scope_string("apps:read")
            .unwrap()
            .includes(&required));
    }

    /// Locks what the SDK token can reach: no `apps:*` (so it can't renew itself),
    /// and the `raw_app_sdk` sentinel keeping `jobs:run` off the code-running
    /// endpoints and `users:read` off the member directory.
    #[test]
    fn frontend_sdk_scopes_reach_declared_domains_but_never_mint_routes() {
        // Mirror mint_raw_app_sdk_token: declared scopes + the narrowing sentinel.
        let mut scopes: Vec<String> = super::FRONTEND_SDK_ALLOWED_SCOPES
            .iter()
            .map(|s| s.to_string())
            .collect();
        scopes.push(windmill_api_auth::scopes::RAW_APP_SDK_SENTINEL.to_string());
        let scopes = Some(scopes.as_slice());

        let allowed = [
            // Deployed runnables by path/hash — what the SDK's run helpers call.
            ("/api/w/test/jobs/run/p/u/admin/script", "POST"),
            ("/api/w/test/jobs/run_wait_result/p/u/admin/script", "POST"),
            // A deployed runnable whose own path contains "preview" must not be
            // caught by the request-supplied-code deny.
            ("/api/w/test/jobs/run/p/f/team/preview_report", "POST"),
            ("/api/w/test/jobs_u/completed/get_result/some-uuid", "GET"),
            ("/api/w/test/users/whoami", "GET"),
            // Unlike the embed token, resource VALUE reads are intended here —
            // the author declared it and the viewer consented.
            ("/api/w/test/resources/get_value/u/admin/r", "GET"),
            ("/api/w/test/variables/get_value/u/admin/v", "GET"),
        ];
        for (path, method) in allowed {
            assert!(
                check_scopes_for_route(scopes, path, method).is_ok(),
                "SDK token should allow {method} {path}"
            );
        }

        let denied = [
            // No apps scope → every mint endpoint (and the Apps domain at large)
            // is unreachable: no self-renewal, no embed-token bootstrap.
            ("/api/w/test/apps_u/embed_token/secret", "GET"),
            ("/api/w/test/apps/embed_token/p/u/admin/app", "GET"),
            ("/api/w/test/apps_u/embed_token_by_custom_path/foo", "GET"),
            ("/api/w/test/apps/get/p/u/admin/app", "GET"),
            // Read-level scopes must not grant writes.
            ("/api/w/test/resources/update/u/admin/r", "POST"),
            ("/api/w/test/variables/create", "POST"),
            // `jobs:run` must not reach the request-supplied-code endpoints: a
            // preview job runs attacker-chosen code and its own credential is
            // unscoped and permissioned as the viewer, so reaching these would
            // make a captured SDK token equivalent to the viewer's whole account.
            ("/api/w/test/jobs/run/preview", "POST"),
            ("/api/w/test/jobs/run_inline/preview", "POST"),
            ("/api/w/test/jobs/run_wait_result/preview", "POST"),
            ("/api/w/test/jobs/run/preview_bundle", "POST"),
            ("/api/w/test/jobs/run/preview_flow", "POST"),
            ("/api/w/test/jobs/run_wait_result/preview_flow", "POST"),
            // Dependency jobs are the same class: they resolve and install
            // request-supplied imports on a worker.
            ("/api/w/test/jobs/run/dependencies", "POST"),
            ("/api/w/test/jobs/run/dependencies_async", "POST"),
            ("/api/w/test/jobs/run/flow_dependencies", "POST"),
            ("/api/w/test/jobs/run/flow_dependencies_async", "POST"),
            // Replay a named job's already-stored code as the caller: the queued
            // preview's raw_code, or a completed preview flow's raw_flow.
            (
                "/api/w/test/jobs/run/workflow_as_code/some-uuid/main",
                "POST",
            ),
            ("/api/w/test/jobs/restart/f/some-uuid", "POST"),
            // `users:read` is presented to the viewer as "read your identity", so
            // the workspace member directory must stay out of reach.
            ("/api/w/test/users/list", "GET"),
            ("/api/w/test/users/list_usage", "GET"),
            ("/api/w/test/users/username_to_email/admin", "GET"),
        ];
        for (path, method) in denied {
            assert!(
                check_scopes_for_route(scopes, path, method).is_err(),
                "SDK token should deny {method} {path}"
            );
        }
    }

    /// The token carries path-scoped `apps:run:<own path>` and `apps:read:<own path>`
    /// (NOT unqualified `apps:run`). Every handler that resolves an app and acts on
    /// its behalf re-checks the requested path via `ScopeDefinition::includes`, so the
    /// token is confined to its OWN app:
    /// - `apps:run:<path>` — `execute_component`.
    /// - `apps:read:<path>` — `get_app` (apps/get/p), `get_public_app_by_secret`,
    ///   the EE custom-path `get_public_app_by_custom_path`, and
    ///   `download_s3_file_from_app`.
    /// This blocks cross-app execution, definition reads (by secret / custom path),
    /// and S3 file reads through another app's on-behalf policy.
    #[test]
    fn embed_run_scope_is_path_scoped_to_its_app() {
        use windmill_api_auth::scopes::ScopeDefinition;
        // The mint must not grant unqualified run (which would include any path).
        assert!(
            !APP_EMBED_SCOPES.contains(&"apps:run"),
            "embed scopes must not include unqualified apps:run"
        );
        for action in ["run", "read"] {
            let own =
                ScopeDefinition::from_scope_string(&format!("apps:{action}:u/admin/app")).unwrap();
            assert!(
                own.includes(
                    &ScopeDefinition::from_scope_string(&format!("apps:{action}:u/admin/app"))
                        .unwrap()
                ),
                "apps:{action} must grant its own app"
            );
            assert!(
                !own.includes(
                    &ScopeDefinition::from_scope_string(&format!("apps:{action}:u/admin/other"))
                        .unwrap()
                ),
                "apps:{action} must NOT grant another app (cross-app)"
            );
        }
    }

    /// `mint_app_embed_token` guards its `create_token_internal` call with
    /// `ensure_scopes_within_caller`, so a scope-restricted bearer token cannot
    /// bootstrap the broader embed-scope set. Lock that boundary on the exact
    /// scope vec the mint builds: rejected for a path-scoped caller, no-op for
    /// the unscoped browser session that is the normal embed flow.
    #[test]
    fn embed_token_mint_is_scope_bounded() {
        use windmill_api_auth::{ensure_scopes_within_caller, ApiAuthed};

        // Same scope set mint_app_embed_token assembles for an app.
        let mut minted: Vec<String> = APP_EMBED_SCOPES.iter().map(|s| s.to_string()).collect();
        minted.push("apps:read:u/admin/app".to_string());

        // A caller restricted to a single app read must not widen to the full
        // embed set (apps:run, jobs:read, resources:read, ...).
        let restricted = ApiAuthed {
            scopes: Some(vec!["apps:read:u/admin/app".to_string()]),
            ..Default::default()
        };
        assert!(
            ensure_scopes_within_caller(&restricted, Some(&minted)).is_err(),
            "a path-scoped caller must not mint the broader embed-scope set"
        );

        // An unscoped session (the normal embed flow) passes — the mint only
        // narrows.
        let unscoped = ApiAuthed { scopes: None, ..Default::default() };
        assert!(ensure_scopes_within_caller(&unscoped, Some(&minted)).is_ok());
    }

    /// The embed-token endpoints must keep working for legacy apps whose stored
    /// policy no longer satisfies the strict `Policy` struct (pre-dating
    /// now-required fields): `parse_embed_policy` reads only the sandbox-decision
    /// fields, leniently, and treats a missing/unknown `execution_mode` as NOT
    /// anonymous (the strictest access interpretation).
    #[test]
    fn embed_policy_parse_is_lenient() {
        use super::parse_embed_policy;

        // Quirky legacy policy: triggerables_v2 entry missing required fields,
        // no execution_mode at all — must still parse, and absent `sandbox`
        // resolves to the unsandboxed default.
        let p = parse_embed_policy(r#"{"triggerables_v2": {"x": {}}}"#).unwrap();
        assert!(!p.sandbox);
        assert!(
            !p.anonymous_execution,
            "missing execution_mode must not grant anonymous access"
        );

        // Normal policies map field-for-field.
        let p = parse_embed_policy(r#"{"execution_mode": "anonymous", "sandbox": true}"#).unwrap();
        assert!(p.anonymous_execution);
        assert!(p.sandbox);

        // Unknown execution_mode value: lenient parse, but not anonymous.
        let p = parse_embed_policy(r#"{"execution_mode": "weird"}"#).unwrap();
        assert!(!p.anonymous_execution);

        // Invalid JSON still errors.
        assert!(parse_embed_policy("not json").is_err());
    }
}

#[cfg(test)]
mod policy_tests {
    /// `execution_mode` is optional in the OpenAPI schema the API clients and the
    /// MCP tools are generated from, so an omitted one must deserialize. It must
    /// resolve to `publisher`, never `anonymous`, and must stay distinguishable
    /// from a stated `publisher` so the update path can keep the deployed mode
    /// instead of re-permissioning the app.
    #[test]
    fn policy_execution_mode_defaults_to_publisher() {
        use super::{ExecutionMode, Policy};

        let p: Policy = serde_json::from_str("{}").expect("empty policy must deserialize");
        assert_eq!(p.execution_mode(), ExecutionMode::Publisher);
        assert_eq!(p.stated_execution_mode(), None);

        // An explicit mode still wins, and reads back as stated.
        let p: Policy = serde_json::from_str(r#"{"execution_mode": "anonymous"}"#).unwrap();
        assert_eq!(p.execution_mode(), ExecutionMode::Anonymous);
        assert_eq!(p.stated_execution_mode(), Some(ExecutionMode::Anonymous));
    }
}

#[cfg(test)]
mod raw_app_file_key_tests {
    use super::validate_raw_app_path_keys;
    use serde_json::value::RawValue;

    fn value_with_files(files: &str) -> Box<RawValue> {
        RawValue::from_string(format!(r#"{{"runnables":{{}},"files":{files}}}"#)).unwrap()
    }

    #[test]
    fn accepts_app_root_relative_keys() {
        let v = value_with_files(r#"{"/index.tsx":"x","/src/util.ts":"y","/package.json":"{}"}"#);
        validate_raw_app_path_keys(&v).expect("legit keys must be accepted");
    }

    #[test]
    fn rejects_traversal_keys() {
        // Each resolves outside the app folder when a puller joins it under the
        // app folder on `sync pull`.
        for files in [
            r#"{"/index.tsx":"x","/../sibling.ts":"y"}"#,
            r#"{"/../../../f/other/outside.ts":"y"}"#,
            r#"{"/../../../../../../elsewhere.txt":"y"}"#,
            r#"{"/src/../../escape.ts":"y"}"#,
            r#"{"/win\\..\\..\\escape.ts":"y"}"#,
        ] {
            let v = value_with_files(files);
            assert!(
                validate_raw_app_path_keys(&v).is_err(),
                "traversal key must be rejected: {files}"
            );
        }
    }

    #[test]
    fn rejects_traversal_in_runnable_ids() {
        // A runnable id names its yaml file, so it is a write path too.
        let v = RawValue::from_string(
            r#"{"files":{"/index.tsx":"x"},"runnables":{"../../../../etc/evil":{}}}"#.to_string(),
        )
        .unwrap();
        assert!(
            validate_raw_app_path_keys(&v).is_err(),
            "a runnable id with a `..` segment must be rejected"
        );
    }

    #[test]
    fn ignores_values_without_a_files_object() {
        // A low-code app value has no `files` map to police; validation is a no-op.
        let v = RawValue::from_string(r#"{"grid":[]}"#.to_string()).unwrap();
        validate_raw_app_path_keys(&v).expect("no `files` means nothing to reject");
    }

    #[test]
    fn rejects_value_too_nested_to_inspect() {
        // `Box<RawValue>` accepts JSON nested past serde's recursion limit, but the
        // validator's `Value` parse does not: a value it can't fully inspect must
        // be refused, not accepted, or a traversal key could ride in unseen.
        let deep = format!("{}{}", "[".repeat(300), "]".repeat(300));
        let v =
            RawValue::from_string(format!(r#"{{"files":{{"/ok.ts":"x"}},"d":{deep}}}"#)).unwrap();
        assert!(
            validate_raw_app_path_keys(&v).is_err(),
            "a value too deeply nested to parse must be rejected"
        );
    }

    #[test]
    fn rejects_nul_that_becomes_traversal_once_stored() {
        // `/..\u0000/escape.ts` has no literal `..` segment on the wire, but the
        // NUL is stripped before storage, leaving `/../escape.ts` for a puller.
        let v = value_with_files(r#"{"/..\u0000/escape.ts":"y"}"#);
        assert!(
            validate_raw_app_path_keys(&v).is_err(),
            "a key that becomes a traversal after NUL-stripping must be rejected"
        );
    }

    #[test]
    fn rejects_last_of_duplicate_files_fields() {
        // A puller's JSON parse keeps the last duplicate field; validation must
        // look at that one, not fail open because the value has two `files`.
        let v = RawValue::from_string(
            r#"{"runnables":{},"files":{"/ok.ts":"x"},"files":{"/../evil.ts":"y"}}"#.to_string(),
        )
        .unwrap();
        assert!(
            validate_raw_app_path_keys(&v).is_err(),
            "the last of duplicate `files` fields must be validated"
        );
    }
}
