/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::extract::Multipart;
use windmill_api_auth::{
    auth::{list_tokens_internal, AuthCache, TruncatedTokenWithEmail},
    check_scopes, maybe_refresh_folders, require_owner_of_path, ApiAuthed,
};
use windmill_common::{
    utils::{BulkDeleteRequest, WithStarredInfoQuery, HTTP_CLIENT},
    webhook::{WebhookMessage, WebhookShared},
    workspaces::{check_user_against_rule, ProtectionRuleKind, RuleCheckResult},
    DB,
};
use windmill_queue::schedule::clear_schedule;

use axum::{
    extract::{Extension, Path, Query},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use futures::future::try_join_all;
use http::header;
use hyper::StatusCode;
use itertools::Itertools;
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::value::RawValue;
use sql_builder::prelude::*;
use sqlx::{FromRow, Postgres, Transaction};
use std::{collections::HashMap, sync::Arc};
use windmill_audit::audit_oss::{audit_log, AuditAuthorable};
use windmill_audit::ActionKind;
use windmill_dep_map::process_relative_imports;
use windmill_dep_map::scoped_dependency_map::ScopedDependencyMap;

use windmill_common::{
    assets::{
        clear_static_asset_usage, clear_static_asset_usage_by_script_hash,
        insert_static_asset_usage, AssetUsageKind, AssetWithAltAccessType,
    },
    error::{self, to_anyhow},
    min_version::{MIN_VERSION_SUPPORTS_DEBOUNCING, MIN_VERSION_SUPPORTS_DEBOUNCING_V2},
    runnable_settings::{
        min_version_supports_runnable_settings_v0, RunnableSettings, RunnableSettingsTrait,
    },
    scripts::{hash_script, ScriptRunnableSettingsHandle, ScriptRunnableSettingsInline},
    utils::{paginate_without_limits, WarnAfterExt},
    worker::CLOUD_HOSTED,
};
use windmill_object_store::upload_artifact_to_store;

use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    jobs::JobPayload,
    schedule::Schedule,
    schema::should_validate_schema,
    scripts::{
        to_i64, HubScript, ListScriptQuery, ListableScript, NewScript, Schema, Script, ScriptHash,
        ScriptHistory, ScriptHistoryUpdate, ScriptKind, ScriptLang, ScriptWithStarred,
    },
    users::username_to_permissioned_as,
    utils::{not_found_if_none, query_elems_from_hub, require_admin, Pagination, StripPath},
    worker::to_raw_value,
    HUB_BASE_URL,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_parser_ts::remove_pinned_imports;
use windmill_queue::{
    schedule::push_scheduled_job, PushIsolationLevel, WMDEBUG_FORCE_NO_LEGACY_DEBOUNCING_COMPAT,
};

const MAX_HASH_HISTORY_LENGTH_STORED: usize = 20;

#[derive(Serialize, sqlx::FromRow)]
pub struct ScriptWDraft<SR> {
    pub hash: ScriptHash,
    pub path: String,
    pub summary: String,
    pub description: String,
    pub content: String,
    pub language: ScriptLang,
    pub kind: ScriptKind,
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<sqlx::types::Json<Box<RawValue>>>,
    pub schema: Option<Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ignore_s3_path: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_worker: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_unless_cancelled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_after_use: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_to_runner_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_main_func: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_preprocessor: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_behalf_of_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(json(nullable))]
    pub assets: Option<Vec<AssetWithAltAccessType>>,
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub runnable_settings: SR,
}

impl ScriptWDraft<ScriptRunnableSettingsHandle> {
    pub async fn prefetch_cached<'a>(
        self,
        db: &DB,
    ) -> error::Result<ScriptWDraft<ScriptRunnableSettingsInline>> {
        let (debouncing_settings, concurrency_settings) =
            windmill_common::runnable_settings::prefetch_cached_from_handle(
                self.runnable_settings.runnable_settings_handle,
                db,
            )
            .await?;

        Ok(ScriptWDraft {
            runnable_settings: ScriptRunnableSettingsInline {
                concurrency_settings: concurrency_settings.maybe_fallback(
                    self.runnable_settings.concurrency_key,
                    self.runnable_settings.concurrent_limit,
                    self.runnable_settings.concurrency_time_window_s,
                ),
                debouncing_settings: debouncing_settings.maybe_fallback(
                    self.runnable_settings.debounce_key,
                    self.runnable_settings.debounce_delay_s,
                ),
            },
            hash: self.hash,
            path: self.path,
            summary: self.summary,
            description: self.description,
            content: self.content,
            language: self.language,
            kind: self.kind,
            tag: self.tag,
            draft: self.draft,
            schema: self.schema,
            draft_only: self.draft_only,
            envs: self.envs,
            cache_ttl: self.cache_ttl,
            cache_ignore_s3_path: self.cache_ignore_s3_path,
            dedicated_worker: self.dedicated_worker,
            ws_error_handler_muted: self.ws_error_handler_muted,
            priority: self.priority,
            restart_unless_cancelled: self.restart_unless_cancelled,
            delete_after_use: self.delete_after_use,
            timeout: self.timeout,
            visible_to_runner_only: self.visible_to_runner_only,
            no_main_func: self.no_main_func,
            has_preprocessor: self.has_preprocessor,
            on_behalf_of_email: self.on_behalf_of_email,
            assets: self.assets,
        })
    }
}

pub fn global_service() -> Router {
    Router::new()
        .route("/hub/top", get(get_top_hub_scripts))
        .route("/hub/get/*path", get(get_hub_script_by_path))
        .route("/hub/get_full/*path", get(get_full_hub_script_by_path))
        .route("/hub/pick/*path", get(pick_hub_script_by_path))
}

pub fn global_unauthed_service() -> Router {
    Router::new()
        .route(
            "/tokened_raw/:workspace/:token/*path",
            get(get_tokened_raw_script_by_path),
        )
        .route("/empty_ts/*path", get(get_empty_ts_script_by_path))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_scripts))
        .route("/list_search", get(list_search_scripts))
        .route("/create", post(create_script))
        .route("/create_snapshot", post(create_snapshot_script))
        .route("/archive/p/*path", post(archive_script_by_path))
        .route("/get/draft/*path", get(get_script_by_path_w_draft))
        .route("/get/p/*path", get(get_script_by_path))
        .route("/list_tokens/*path", get(list_tokens))
        .route("/raw/p/*path", get(raw_script_by_path))
        .route("/raw_unpinned/p/*path", get(raw_script_by_path_unpinned))
        .route("/exists/p/*path", get(exists_script_by_path))
        .route("/archive/h/:hash", post(archive_script_by_hash))
        .route("/delete/h/:hash", post(delete_script_by_hash))
        .route("/delete/p/*path", post(delete_script_by_path))
        .route("/delete_bulk", delete(delete_scripts_bulk))
        .route("/get/h/:hash", get(get_script_by_hash))
        .route("/raw/h/:hash", get(raw_script_by_hash))
        .route("/deployment_status/h/:hash", get(get_deployment_status))
        .route("/list_paths", get(list_paths))
        .route(
            "/toggle_workspace_error_handler/p/*path",
            post(toggle_workspace_error_handler),
        )
        .route("/history/p/*path", get(get_script_history))
        .route("/get_latest_version/*path", get(get_latest_version))
        .route(
            "/list_paths_from_workspace_runnable/*path",
            get(list_paths_from_workspace_runnable),
        )
        .route(
            "/history_update/h/:hash/p/*path",
            post(update_script_history),
        )
}

#[derive(Serialize, FromRow)]
pub struct SearchScript {
    path: String,
    content: String,
}
async fn list_search_scripts(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<SearchScript>> {
    let mut tx = user_db.begin(&authed).await?;
    #[cfg(feature = "enterprise")]
    let n = 10000;

    #[cfg(not(feature = "enterprise"))]
    let n = 10;

    let rows = sqlx::query_as!(
        SearchScript,
        "SELECT path, content from script WHERE workspace_id = $1 AND archived = false LIMIT $2",
        &w_id,
        n
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect::<Vec<_>>();
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_scripts(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListScriptQuery>,
) -> JsonResult<Vec<ListableScript>> {
    let (per_page, offset) = paginate_without_limits(pagination);
    let mut sqlb = SqlBuilder::select_from("script as o")
        .fields(&[
            "hash",
            "o.path",
            "summary",
            "COALESCE(draft.created_at, o.created_at) as created_at",
            "archived",
            "extra_perms",
            if !lq.without_description.unwrap_or(false) {
                "description"
            } else {
                "NULL as description"
            },
            "CASE WHEN lock_error_logs IS NOT NULL THEN true ELSE false END as has_deploy_errors",
            "language",
            "favorite.path IS NOT NULL as starred",
            "tag",
            "draft.path IS NOT NULL as has_draft",
            "draft_only",
            "ws_error_handler_muted",
            "no_main_func",
            "codebase IS NOT NULL as use_codebase",
            "kind"
        ])
        .left()
        .join("favorite")
        .on(
            "favorite.favorite_kind = 'script' AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?"
                .bind(&authed.username),
        )
        .left()
        .join("draft")
        .on(
            "draft.path = o.path AND draft.workspace_id = o.workspace_id AND draft.typ = 'script'"
        )
        .order_desc("favorite.path IS NOT NULL")
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .and_where("o.workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    let lowercased_kinds: Option<Vec<String>> = lq
        .kinds
        .map(|x| x.split(",").map(&str::to_lowercase).collect());

    if (!lq.include_without_main.unwrap_or(false)
        && lowercased_kinds
            .as_ref()
            .map(|x| !x.contains(&"preprocessor".to_string()))
            .unwrap_or(true))
        || authed.is_operator
    {
        // only include scripts that have a main function
        // do not hide scripts without main if preprocessor is in the kinds
        sqlb.and_where("o.no_main_func IS NOT TRUE");
    }

    if !lq.include_draft_only.unwrap_or(false) || authed.is_operator {
        sqlb.and_where("draft_only IS NOT TRUE");
    }

    if lq.show_archived.unwrap_or(false) {
        sqlb.and_where_eq(
            "o.ctid",
            "(SELECT ctid FROM script 
              WHERE path = o.path 
                AND workspace_id = ? 
              ORDER BY created_at DESC 
              LIMIT 1)"
                .bind(&w_id),
        );
        sqlb.and_where_eq("archived", true);
    } else {
        sqlb.and_where_eq("archived", false);
    }
    if let Some(ps) = &lq.path_start {
        sqlb.and_where_like_left("o.path", ps);
    }
    if let Some(p) = &lq.path_exact {
        sqlb.and_where_eq("o.path", "?".bind(p));
    }
    if let Some(cb) = &lq.created_by {
        sqlb.and_where_eq("created_by", "?".bind(cb));
    }
    if let Some(ph) = &lq.first_parent_hash {
        sqlb.and_where_eq("parent_hashes[1]", &ph.0);
    }
    if let Some(ph) = &lq.last_parent_hash {
        sqlb.and_where_eq("parent_hashes[array_upper(parent_hashes, 1)]", &ph.0);
    }
    if let Some(ph) = &lq.parent_hash {
        sqlb.and_where_eq("any(parent_hashes)", &ph.0);
    }
    if let Some(it) = &lq.is_template {
        sqlb.and_where_eq("is_template", it);
    }
    if let Some(dw) = &lq.dedicated_worker {
        sqlb.and_where_eq("dedicated_worker", dw);
    }
    if authed.is_operator {
        sqlb.and_where_eq("kind", quote("script"));
    } else if let Some(lowercased_kinds) = lowercased_kinds {
        let safe_kinds = lowercased_kinds
            .into_iter()
            .map(sql_builder::quote)
            .collect_vec();
        if safe_kinds.len() > 0 {
            sqlb.and_where_in("kind", safe_kinds.as_slice());
        }
    }
    if lq.starred_only.unwrap_or(false) {
        sqlb.and_where_is_not_null("favorite.path");
    }

    if lq.with_deployment_msg.unwrap_or(false) {
        sqlb.join("deployment_metadata dm")
            .left()
            .on("dm.script_hash = o.hash")
            .fields(&["dm.deployment_msg"]);
    }

    if let Some(languages) = lq.languages {
        sqlb.and_where_in(
            "language",
            &languages
                .iter()
                .map(|language| quote(language.as_str()))
                .collect_vec(),
        );
    }

    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, ListableScript>(&sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
struct TopHubScriptsQuery {
    limit: Option<i64>,
    app: Option<String>,
    kind: Option<String>,
}

async fn get_top_hub_scripts(
    Query(query): Query<TopHubScriptsQuery>,
    Extension(db): Extension<DB>,
) -> impl IntoResponse {
    let mut query_params = vec![];
    if let Some(query_limit) = query.limit {
        query_params.push(("limit", query_limit.to_string().clone()));
    }
    if let Some(query_app) = query.app {
        query_params.push(("app", query_app.to_string().clone()));
    }
    if let Some(query_kind) = query.kind {
        query_params.push(("kind", query_kind.to_string().clone()));
    }

    let (status_code, headers, response) = query_elems_from_hub(
        &HTTP_CLIENT,
        &format!("{}/scripts/top", *HUB_BASE_URL.read().await),
        Some(query_params),
        &db,
    )
    .await?;
    Ok::<_, Error>((status_code, headers, response))
}

async fn create_snapshot_script(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    mut multipart: Multipart,
) -> Result<(StatusCode, String)> {
    // TODO: Check for debouncing here as well.
    let mut script_hash = None;
    let mut tx = None;
    let mut uploaded = false;
    let mut handle_deployment_metadata = None;
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();
        if name == "script" {
            let ns: NewScript = Some(serde_json::from_slice(&data).map_err(to_anyhow)?).unwrap();
            let is_tar = ns.codebase.as_ref().is_some_and(|x| x.ends_with(".tar"));
            let use_esm = ns.codebase.as_ref().is_some_and(|x| x.contains(".esm"));
            let (new_hash, ntx, hdm) = create_script_internal(
                ns,
                w_id.clone(),
                authed.clone(),
                db.clone(),
                user_db.clone(),
                webhook.clone(),
            )
            .await?;
            let mut nh = new_hash.to_string();
            if use_esm {
                nh = format!("{nh}.esm");
            }
            if is_tar {
                nh = format!("{nh}.tar");
            }
            script_hash = Some(nh);
            tx = Some(ntx);
            handle_deployment_metadata = hdm;
        }
        if name == "file" {
            let hash = script_hash.as_ref().ok_or_else(|| {
                Error::BadRequest(
                    "script need to be passed first in the multipart upload".to_string(),
                )
            })?;

            uploaded = true;

            let path = windmill_object_store::bundle(&w_id, &hash);
            upload_artifact_to_store(
                &path,
                data,
                &windmill_common::worker::ROOT_STANDALONE_BUNDLE_DIR,
            )
            .await?;
        }
        // println!("Length of `{}` is {} bytes", name, data.len());
    }
    if !uploaded {
        return Err(Error::BadRequest("No file uploaded".to_string()));
    }
    if script_hash.is_none() {
        return Err(Error::BadRequest(
            "No script found in the uploaded file".to_string(),
        ));
    }

    tx.unwrap().commit().await?;
    if let Some(hdm) = handle_deployment_metadata {
        hdm.handle(&db).await?;
    }
    return Ok((StatusCode::CREATED, format!("{}", script_hash.unwrap())));
}

async fn list_paths_from_workspace_runnable(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Vec<String>> {
    let mut tx = user_db.begin(&authed).await?;
    let runnables = sqlx::query_scalar!(
        r#"SELECT importer_path FROM dependency_map 
            WHERE workspace_id = $1 AND imported_path = $2"#,
        w_id,
        path.to_path(),
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(runnables))
}

async fn create_script(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(ns): Json<NewScript>,
) -> Result<(StatusCode, String)> {
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }
    let (hash, tx, hdm) =
        create_script_internal(ns, w_id, authed, db.clone(), user_db, webhook).await?;
    tx.commit().await?;
    if let Some(hdm) = hdm {
        hdm.handle(&db).await?;
    }
    Ok((StatusCode::CREATED, format!("{}", hash)))
}

struct HandleDeploymentMetadata {
    email: String,
    created_by: String,
    w_id: String,
    obj: DeployedObject,
    deployment_message: Option<String>,
    renamed_from: Option<String>,
}

impl HandleDeploymentMetadata {
    async fn handle(self, db: &DB) -> Result<()> {
        handle_deployment_metadata(
            &self.email,
            &self.created_by,
            &db,
            &self.w_id,
            self.obj,
            self.deployment_message,
            false,
            self.renamed_from.as_deref(),
        )
        .await
    }
}

async fn create_script_internal<'c>(
    ns: NewScript,
    w_id: String,
    authed: ApiAuthed,
    db: sqlx::Pool<Postgres>,
    user_db: UserDB,
    webhook: WebhookShared,
) -> Result<(
    ScriptHash,
    Transaction<'c, Postgres>,
    Option<HandleDeploymentMetadata>,
)> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot create scripts for security reasons".to_string(),
        ));
    }
    check_scopes(&authed, || format!("scripts:write:{}", ns.path))?;

    guard_script_from_debounce_data(&ns).await?;

    let codebase = ns.codebase.as_ref();
    #[cfg(not(feature = "enterprise"))]
    if ns.ws_error_handler_muted.is_some_and(|val| val) {
        return Err(Error::BadRequest(
            "Muting the error handler for certain script is only available in enterprise version"
                .to_string(),
        ));
    }
    if *CLOUD_HOSTED {
        let nb_scripts =
            sqlx::query_scalar!("SELECT COUNT(*) FROM script WHERE workspace_id = $1", &w_id)
                .fetch_one(&db)
                .await?;
        if nb_scripts.unwrap_or(0) >= 5000 {
            return Err(Error::BadRequest(
                    "You have reached the maximum number of scripts (5000) on cloud. Contact support@windmill.dev to increase the limit"
                        .to_string(),
                ));
        }

        if ns.summary.len() > 300 {
            return Err(Error::BadRequest(
                "Summary must be less than 300 characters on cloud".to_string(),
            ));
        }
        if ns.description.len() > 3000 {
            return Err(Error::BadRequest(
                "Description must be less than 3000 characters on cloud".to_string(),
            ));
        }
    }
    let script_path = ns.path.clone();
    let hash = ScriptHash(hash_script(&ns));
    let authed = maybe_refresh_folders(&ns.path, &w_id, authed, &db).await;
    let mut tx: Transaction<'_, Postgres> = user_db.begin(&authed).await?;
    if sqlx::query_scalar!(
        "SELECT 1 FROM script WHERE hash = $1 AND workspace_id = $2",
        hash.0,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .is_some()
    {
        return Err(Error::BadRequest(
            "A script with same hash (hence same path, description, summary, content) already \
             exists!"
                .to_owned(),
        ));
    };
    let clashing_script = sqlx::query_as::<_, Script<ScriptRunnableSettingsHandle>>(
        "SELECT * FROM script WHERE path = $1 AND archived = false AND workspace_id = $2",
    )
    .bind(&ns.path)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;
    struct ParentInfo {
        p_hashes: Vec<i64>,
        perms: serde_json::Value,
        p_path: String,
    }
    let parent_hashes_and_perms: Option<ParentInfo> = match (&ns.parent_hash, clashing_script) {
        (None, None) => Ok(None),
        (None, Some(s)) if !s.draft_only.unwrap_or(false) => Err(Error::BadRequest(format!(
            "Path conflict for {} with non-archived hash {}",
            &ns.path, &s.hash
        ))),
        (None, Some(s)) => {
            sqlx::query!(
                "DELETE FROM script WHERE hash = $1 AND workspace_id = $2",
                s.hash.0,
                &w_id
            )
            .execute(&mut *tx)
            .await?;
            Ok(None)
        }
        (Some(p_hash), o) => {
            // Lock the parent row to prevent concurrent updates with the same parent_hash
            // This ensures linear lineage - only one script can have a given parent at a time
            if sqlx::query_scalar!(
                "SELECT 1 FROM script WHERE hash = $1 AND workspace_id = $2 FOR UPDATE",
                p_hash.0,
                &w_id
            )
            .fetch_optional(&mut *tx)
            .await?
            .is_none()
            {
                return Err(Error::BadRequest(
                    "The parent hash does not seem to exist".to_owned(),
                ));
            };

            let clashing_hash_o = sqlx::query_scalar!(
                "SELECT hash FROM script WHERE parent_hashes[1] = $1 AND workspace_id = $2",
                p_hash.0,
                &w_id
            )
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(clashing_hash) = clashing_hash_o {
                return Err(Error::BadRequest(format!(
                    "A script with hash {} with same parent_hash has been found. However, the \
                         lineage must be linear: no 2 scripts can have the same parent",
                    ScriptHash(clashing_hash)
                )));
            };

            let ScriptWithStarred { script: ps, .. } =
                get_script_by_hash_internal(&mut tx, &w_id, p_hash, None).await?;

            if ps.path != ns.path {
                require_owner_of_path(&authed, &ps.path)?;
            }

            let ph = {
                let v = ps.parent_hashes.map(|x| x.0).unwrap_or_default();
                let mut v: Vec<i64> = v
                    .into_iter()
                    .take(MAX_HASH_HISTORY_LENGTH_STORED - 1)
                    .collect();
                v.insert(0, p_hash.0);
                v
            };
            let r: Result<Option<ParentInfo>> = match o {
                Some(clashing_script)
                    if clashing_script.path == ns.path && clashing_script.hash.0 != p_hash.0 =>
                {
                    Err(Error::BadRequest(format!(
                        "Path conflict for {} with non-archived hash {}",
                        &ns.path, &clashing_script.hash
                    )))
                }
                Some(_) | None => Ok(Some(ParentInfo {
                    p_hashes: ph,
                    perms: ps.extra_perms,
                    p_path: ps.path,
                })),
            };
            sqlx::query!(
                "UPDATE script SET archived = true WHERE hash = $1 AND workspace_id = $2",
                p_hash.0,
                &w_id
            )
            .execute(&mut *tx)
            .await?;

            clear_static_asset_usage_by_script_hash(&mut *tx, &w_id, hash).await?;

            r
        }
    }?;
    let p_hashes = parent_hashes_and_perms.as_ref().map(|v| &v.p_hashes[..]);
    let extra_perms = parent_hashes_and_perms
        .as_ref()
        .map(|v| v.perms.clone())
        .unwrap_or(json!({}));
    let lock = if ns.codebase.is_some() {
        Some(String::new())
    } else if !(
        ns.language == ScriptLang::Python3
            || ns.language == ScriptLang::Go
            || ns.language == ScriptLang::Bun
            || ns.language == ScriptLang::Bunnative
            || ns.language == ScriptLang::Deno
            || ns.language == ScriptLang::Rust
            || ns.language == ScriptLang::Ansible
            || ns.language == ScriptLang::CSharp
            || ns.language == ScriptLang::Nu
            || ns.language == ScriptLang::Php
            || ns.language == ScriptLang::Java
            || ns.language == ScriptLang::Ruby
        // for related places search: ADD_NEW_LANG
    ) {
        Some(String::new())
    } else {
        ns.lock.as_ref().and_then(|e| {
            if e.is_empty() {
                None
            } else {
                Some(e.to_string())
            }
        })
    };

    let needs_lock_gen = lock.is_none() && codebase.is_none();
    let envs = ns.envs.as_ref().map(|x| x.as_slice());
    let envs = if ns.envs.is_none() || ns.envs.as_ref().unwrap().is_empty() {
        None
    } else {
        envs
    };

    let lang = if &ns.language == &ScriptLang::Bun || &ns.language == &ScriptLang::Bunnative {
        let anns = windmill_common::worker::TypeScriptAnnotations::parse(&ns.content);
        if anns.native {
            ScriptLang::Bunnative
        } else {
            ScriptLang::Bun
        }
    } else {
        ns.language.clone()
    };

    let validate_schema = should_validate_schema(&ns.content, &ns.language);

    let (no_main_func, has_preprocessor) = if matches!(ns.kind, Some(ScriptKind::Preprocessor)) {
        (ns.no_main_func, ns.has_preprocessor)
    } else {
        match lang {
            ScriptLang::Bun | ScriptLang::Bunnative | ScriptLang::Deno | ScriptLang::Nativets => {
                let args = windmill_parser_ts::parse_deno_signature(&ns.content, true, true, None);
                match args {
                    Ok(args) => (args.no_main_func, args.has_preprocessor),
                    Err(e) => {
                        tracing::warn!(
                            "Error parsing deno signature when deploying script {}: {:?}",
                            ns.path,
                            e
                        );
                        (None, None)
                    }
                }
            }
            #[cfg(feature = "python")]
            ScriptLang::Python3 => {
                let args = windmill_parser_py::parse_python_signature(&ns.content, None, true);
                match args {
                    Ok(args) => (args.no_main_func, args.has_preprocessor),
                    Err(e) => {
                        tracing::warn!(
                            "Error parsing python signature when deploying script {}: {:?}",
                            ns.path,
                            e
                        );
                        (None, None)
                    }
                }
            }
            _ => (ns.no_main_func, ns.has_preprocessor),
        }
    };

    let runnable_settings_handle = windmill_common::runnable_settings::insert_rs(
        RunnableSettings {
            debouncing_settings: ns.debouncing_settings.insert_cached(&db).await?,
            concurrency_settings: ns.concurrency_settings.insert_cached(&db).await?,
        },
        &db,
    )
    .await?;

    let (
        guarded_concurrent_limit,
        guarded_concurrency_time_window_s,
        guarded_concurrency_key,
        guarded_debounce_key,
        guarded_debounce_delay_s,
    ) = if min_version_supports_runnable_settings_v0().await {
        Default::default()
    } else {
        (
            ns.concurrency_settings.concurrent_limit.clone(),
            ns.concurrency_settings.concurrency_time_window_s.clone(),
            ns.concurrency_settings.concurrency_key.clone(),
            ns.debouncing_settings.debounce_key.clone(),
            ns.debouncing_settings.debounce_delay_s.clone(),
        )
    };

    sqlx::query!(
        "INSERT INTO script (workspace_id, hash, path, parent_hashes, summary, description, \
         content, created_by, schema, is_template, extra_perms, lock, language, kind, tag, \
         draft_only, envs, concurrent_limit, concurrency_time_window_s, cache_ttl, \
         dedicated_worker, ws_error_handler_muted, priority, restart_unless_cancelled, \
         delete_after_use, timeout, concurrency_key, visible_to_runner_only, no_main_func, codebase, has_preprocessor, on_behalf_of_email, schema_validation, assets, debounce_key, debounce_delay_s, cache_ignore_s3_path, runnable_settings_handle) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::text::json, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38)",
        &w_id,
        &hash.0,
        ns.path,
        p_hashes,
        ns.summary,
        ns.description,
        &ns.content,
        &authed.username,
        ns.schema.and_then(|x| serde_json::to_string(&x.0).ok()),
        ns.is_template.unwrap_or(false),
        extra_perms,
        lock,
        lang as ScriptLang,
        ns.kind.unwrap_or(ScriptKind::Script) as ScriptKind,
        ns.tag,
        ns.draft_only,
        envs,
        guarded_concurrent_limit,
        guarded_concurrency_time_window_s,
        ns.cache_ttl,
        ns.dedicated_worker,
        ns.ws_error_handler_muted.unwrap_or(false),
        ns.priority,
        ns.restart_unless_cancelled,
        ns.delete_after_use,
        ns.timeout,
        guarded_concurrency_key,
        ns.visible_to_runner_only,
        no_main_func.filter(|x: &bool| *x), // should be Some(true) or None
        codebase,
        has_preprocessor.filter(|x: &bool| *x), // should be Some(true) or None
        windmill_common::resolve_on_behalf_of_email(
            ns.on_behalf_of_email.as_deref(),
            ns.preserve_on_behalf_of.unwrap_or(false),
            &authed,
        ),
        validate_schema,
        ns.assets.as_ref().and_then(|a| serde_json::to_value(a).ok()),
        guarded_debounce_key,
        guarded_debounce_delay_s,
        ns.cache_ignore_s3_path,
        runnable_settings_handle
    )
    .execute(&mut *tx)
    .await?;

    let p_path_opt = parent_hashes_and_perms.as_ref().map(|x| x.p_path.clone());
    if let Some(ref p_path) = p_path_opt {
        sqlx::query!(
            "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'script'",
            p_path,
            &w_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE capture_config SET path = $1 WHERE path = $2 AND workspace_id = $3 AND is_flow IS FALSE",
            ns.path,
            p_path,
            w_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE capture SET path = $1 WHERE path = $2 AND workspace_id = $3 AND is_flow IS FALSE",
            ns.path,
            p_path,
            w_id
        )
        .execute(&mut *tx)
        .await?;

        let mut schedulables = sqlx::query_as::<_, Schedule>(
            "UPDATE schedule SET script_path = $1 WHERE script_path = $2 AND path != $2 AND workspace_id = $3 AND is_flow IS false RETURNING *")
            .bind(&ns.path)
            .bind(&p_path)
            .bind(&w_id)
        .fetch_all(&mut *tx)
        .await?;

        let schedule = sqlx::query_as::<_, Schedule>(
            "UPDATE schedule SET path = $1, script_path = $1 WHERE path = $2 AND workspace_id = $3 AND is_flow IS false RETURNING *")
            .bind(&ns.path)
            .bind(&p_path)
            .bind(&w_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(schedule) = schedule {
            schedulables.push(schedule);
        }

        // Update dynamic_skip references when script is renamed
        sqlx::query!(
            "UPDATE schedule SET dynamic_skip = $1 WHERE dynamic_skip = $2 AND workspace_id = $3",
            &ns.path,
            &p_path,
            &w_id
        )
        .execute(&mut *tx)
        .await?;

        for schedule in schedulables {
            clear_schedule(&mut tx, &schedule.path, &w_id).await?;

            if schedule.enabled {
                tx = push_scheduled_job(&db, tx, &schedule, None, None).await?;
            }
        }
    } else {
        sqlx::query!(
            "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'script'",
            ns.path,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    }
    if p_hashes.is_some() && !p_hashes.unwrap().is_empty() {
        audit_log(
            &mut *tx,
            &authed,
            "scripts.update",
            ActionKind::Update,
            &w_id,
            Some(&ns.path),
            Some([("hash", hash.to_string().as_str())].into()),
        )
        .await?;
        if let Some(on_behalf_of) = windmill_common::check_on_behalf_of_preservation(
            ns.on_behalf_of_email.as_deref(),
            ns.preserve_on_behalf_of.unwrap_or(false),
            &authed,
            &authed.email,
        ) {
            audit_log(
                &mut *tx,
                &authed,
                "scripts.on_behalf_of",
                ActionKind::Update,
                &w_id,
                Some(&ns.path),
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
        webhook.send_message(
            w_id.clone(),
            WebhookMessage::UpdateScript {
                workspace: w_id.clone(),
                path: ns.path.clone(),
                hash: hash.to_string(),
            },
        );
    } else {
        audit_log(
            &mut *tx,
            &authed,
            "scripts.create",
            ActionKind::Create,
            &w_id,
            Some(&ns.path),
            Some(
                [
                    ("workspace", w_id.as_str()),
                    ("hash", hash.to_string().as_str()),
                ]
                .into(),
            ),
        )
        .await?;
        if let Some(on_behalf_of) = windmill_common::check_on_behalf_of_preservation(
            ns.on_behalf_of_email.as_deref(),
            ns.preserve_on_behalf_of.unwrap_or(false),
            &authed,
            &authed.email,
        ) {
            audit_log(
                &mut *tx,
                &authed,
                "scripts.on_behalf_of",
                ActionKind::Create,
                &w_id,
                Some(&ns.path),
                Some(
                    [
                        ("on_behalf_of", on_behalf_of.as_str()),
                        ("action", "create"),
                    ]
                    .into(),
                ),
            )
            .await?;
        }
        webhook.send_message(
            w_id.clone(),
            WebhookMessage::CreateScript {
                workspace: w_id.clone(),
                path: ns.path.clone(),
                hash: hash.to_string(),
            },
        );
    }

    clear_static_asset_usage(&mut *tx, &w_id, &script_path, AssetUsageKind::Script).await?;
    for asset in ns.assets.as_ref().into_iter().flatten() {
        insert_static_asset_usage(&mut *tx, &w_id, &asset, &ns.path, AssetUsageKind::Script)
            .await?;
    }

    let permissioned_as = username_to_permissioned_as(&authed.username);
    if let Some(parent_hash) = ns.parent_hash {
        tracing::info!(
            "creating script {hash:?} at path {script_path} with parent {parent_hash} on workspace {w_id}",
        );
    } else {
        tracing::info!("creating script {hash:?} at path {script_path} on workspace {w_id}",);
    }
    if needs_lock_gen {
        let tag = if ns.dedicated_worker.is_some_and(|x| x) {
            Some(windmill_common::worker::dedicated_worker_tag(
                &w_id, &ns.path,
            ))
        } else if ns.tag.as_ref().is_some_and(|x| x.contains("$args[")) {
            None
        } else if lang == ScriptLang::Bunnative {
            // if a custom tag is set for a bunnative script, this prevents the custom tag to be used for the dependency job
            // forcing the bundling to run on a worker with the bun tag
            None
        } else {
            ns.tag
        };

        let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
        if let Some(dm) = ns.deployment_message {
            args.insert("deployment_message".to_string(), to_raw_value(&dm));
        }
        if let Some(ref p_path) = p_path_opt {
            args.insert("parent_path".to_string(), to_raw_value(&p_path));
        }

        let tx = PushIsolationLevel::Transaction(tx);
        let (job_id, mut new_tx) = windmill_queue::push(
            &db,
            tx,
            &w_id,
            JobPayload::Dependencies {
                hash,
                language: ns.language,
                path: ns.path.clone(),
                dedicated_worker: ns.dedicated_worker,
                debouncing_settings: Default::default(),
            },
            windmill_queue::PushArgs::from(&args),
            &authed.username,
            &authed.email,
            permissioned_as,
            authed.token_prefix.as_deref(),
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
            tag,
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

        // Store the job_id in deployment_metadata for this script deployment
        sqlx::query!(
            "INSERT INTO deployment_metadata (workspace_id, path, script_hash, job_id)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (workspace_id, script_hash) WHERE script_hash IS NOT NULL
             DO UPDATE SET job_id = EXCLUDED.job_id",
            w_id,
            ns.path,
            hash.0,
            job_id
        )
        .execute(&mut *new_tx)
        .await?;

        Ok((hash, new_tx, None))
    } else {
        if codebase.is_none() {
            let db2 = db.clone();
            let w_id2 = w_id.clone();
            let authed2 = authed.clone();
            let permissioned_as2 = permissioned_as.clone();
            let script_path2 = script_path.clone();
            let parent_path = p_path_opt.clone();
            let deployment_message = ns.deployment_message.clone();
            let content = ns.content.clone();
            let language = ns.language.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                if let Err(e) = process_relative_imports(
                    &db2,
                    None,
                    None,
                    &w_id2,
                    &script_path2,
                    parent_path,
                    deployment_message,
                    &content,
                    &Some(language),
                    &authed2.email,
                    &authed2.username,
                    &permissioned_as2,
                )
                .await
                {
                    tracing::error!(%e, "error processing relative imports");
                }
            });
        }

        // handle_deployment_metadata(
        //     &authed.email,
        //     &authed.username,
        //     &db,
        //     &w_id,
        //     DeployedObject::Script {
        //         hash: hash.clone(),
        //         path: script_path.clone(),
        //         parent_path: p_path_opt,
        //     },
        //     ns.deployment_message,
        //     false,
        // )
        // .await?;

        Ok((
            hash,
            tx,
            Some(HandleDeploymentMetadata {
                email: authed.email,
                created_by: authed.username,
                w_id,
                obj: DeployedObject::Script {
                    hash: hash.clone(),
                    path: script_path.clone(),
                    parent_path: p_path_opt.clone(),
                },
                deployment_message: ns.deployment_message,
                renamed_from: p_path_opt,
            }),
        ))
    }
}

pub async fn get_hub_script_by_path(
    Path(path): Path<StripPath>,
    Extension(db): Extension<DB>,
) -> Result<String> {
    windmill_common::scripts::get_hub_script_by_path(path, &HTTP_CLIENT, &db).await
}

pub async fn get_full_hub_script_by_path(
    Path(path): Path<StripPath>,
    Extension(db): Extension<DB>,
) -> JsonResult<HubScript> {
    Ok(Json(
        windmill_common::scripts::get_full_hub_script_by_path(path, &HTTP_CLIENT, Some(&db))
            .await?,
    ))
}

pub async fn pick_hub_script_by_path(
    Path(path): Path<StripPath>,
    Extension(db): Extension<DB>,
) -> impl IntoResponse {
    let path_str = path.to_path();

    // Extract version_id from path (format: {hub}/{version_id}/{summary})
    let version_id = path_str.split('/').nth(1).unwrap_or("");

    let hub_base_url = HUB_BASE_URL.read().await.clone();

    // Determine which hub to use based on version_id
    // If version_id < PRIVATE_HUB_MIN_VERSION, use default hub
    let target_hub_url = if version_id
        .parse::<i32>()
        .is_ok_and(|v| v < windmill_common::PRIVATE_HUB_MIN_VERSION)
    {
        windmill_common::DEFAULT_HUB_BASE_URL
    } else {
        &hub_base_url
    };

    // Call the hub's pick endpoint: /scripts/{version_id}/pick
    let (status_code, headers, response) = query_elems_from_hub(
        &HTTP_CLIENT,
        &format!("{}/scripts/{}/pick", target_hub_url, version_id),
        None,
        &db,
    )
    .await?;

    Ok::<_, Error>((status_code, headers, response))
}

#[axum::debug_handler]
async fn get_script_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<WithStarredInfoQuery>,
) -> JsonResult<ScriptWithStarred<ScriptRunnableSettingsInline>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("scripts:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let script_o = if query.with_starred_info.unwrap_or(false) {
        sqlx::query_as::<_, ScriptWithStarred<ScriptRunnableSettingsHandle>>(
            "SELECT s.*, favorite.path IS NOT NULL as starred
            FROM script s
            LEFT JOIN favorite
            ON favorite.favorite_kind = 'script' 
                AND favorite.workspace_id = s.workspace_id 
                AND favorite.path = s.path 
                AND favorite.usr = $3
            WHERE s.path = $1
                AND s.workspace_id = $2
            ORDER BY s.created_at DESC LIMIT 1",
        )
        .bind(path)
        .bind(w_id)
        .bind(&authed.username)
        .fetch_optional(&mut *tx)
        .await?
    } else {
        sqlx::query_as::<_, ScriptWithStarred<ScriptRunnableSettingsHandle>>(
            "SELECT *, NULL as starred FROM script WHERE path = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(path)
        .bind(w_id)
        .fetch_optional(&mut *tx)
        .await?
    };
    tx.commit().await?;

    let script = windmill_common::scripts::prefetch_cached_script_with_starred(
        not_found_if_none(script_o, "Script", path)?,
        &db,
    )
    .await?;

    Ok(Json(script))
}

async fn list_tokens(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Vec<TruncatedTokenWithEmail>> {
    let path = path.to_path();
    list_tokens_internal(&db, &w_id, &path, false).await
}

async fn get_script_by_path_w_draft(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<ScriptWDraft<ScriptRunnableSettingsInline>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("scripts:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let script_o = sqlx::query_as::<_, ScriptWDraft<ScriptRunnableSettingsHandle>>(
        "SELECT hash, script.path, summary, description, content, language, kind, tag, schema, draft_only, envs, runnable_settings_handle, concurrent_limit, concurrency_time_window_s, cache_ttl, cache_ignore_s3_path, ws_error_handler_muted, draft.value as draft, dedicated_worker, priority, restart_unless_cancelled, delete_after_use, timeout, concurrency_key, visible_to_runner_only, no_main_func, has_preprocessor, on_behalf_of_email, assets, debounce_key, debounce_delay_s FROM script LEFT JOIN draft ON 
         script.path = draft.path AND script.workspace_id = draft.workspace_id AND draft.typ = 'script'
         WHERE script.path = $1 AND script.workspace_id = $2
         ORDER BY script.created_at DESC LIMIT 1",
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let script = not_found_if_none(script_o, "Script", path)?;
    Ok(Json(script.prefetch_cached(&db).await?))
}

async fn get_script_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Vec<ScriptHistory>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("scripts:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;
    let query_result = sqlx::query!(
        "SELECT s.hash as hash, dm.deployment_msg as deployment_msg 
        FROM script s LEFT JOIN deployment_metadata dm ON s.hash = dm.script_hash
        WHERE s.workspace_id = $1 AND s.path = $2
        ORDER by s.created_at DESC",
        w_id,
        path,
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    let result: Vec<ScriptHistory> = query_result
        .into_iter()
        .map(|row| ScriptHistory {
            script_hash: ScriptHash(row.hash),
            deployment_msg: row.deployment_msg,
        })
        .collect();
    return Ok(Json(result));
}

async fn get_latest_version(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Option<ScriptHistory>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("scripts:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;
    let row_o = sqlx::query!(
        "SELECT s.hash as hash, dm.deployment_msg as deployment_msg 
        FROM script s LEFT JOIN deployment_metadata dm ON s.hash = dm.script_hash
        WHERE s.workspace_id = $1 AND s.path = $2
        ORDER by s.created_at DESC LIMIT 1",
        w_id,
        path,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    if let Some(row) = row_o {
        let result = ScriptHistory {
            script_hash: ScriptHash(row.hash),
            deployment_msg: row.deployment_msg, //
        };
        return Ok(Json(Some(result)));
    } else {
        return Ok(Json(None));
    }
}

async fn update_script_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_hash, script_path)): Path<(String, ScriptHash, StripPath)>,
    Json(script_history_update): Json<ScriptHistoryUpdate>,
) -> Result<()> {
    let script_path = script_path.to_path();
    check_scopes(&authed, || format!("scripts:write:{}", script_path))?;

    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "INSERT INTO deployment_metadata (workspace_id, path, script_hash, deployment_msg) VALUES ($1, $2, $3, $4) ON CONFLICT (workspace_id, script_hash) WHERE script_hash IS NOT NULL
         DO UPDATE SET deployment_msg = EXCLUDED.deployment_msg",
        w_id,
        script_path,
        script_hash.0,
        script_history_update.deployment_msg,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(());
}

async fn list_paths(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    let mut tx = user_db.begin(&authed).await?;

    let scripts = sqlx::query_scalar!(
        "SELECT distinct(path) FROM script WHERE  workspace_id = $1",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(scripts))
}

#[derive(Deserialize)]
pub struct ToggleWorkspaceErrorHandler {
    #[cfg(feature = "enterprise")]
    pub muted: Option<bool>,
}

#[cfg(not(feature = "enterprise"))]
async fn toggle_workspace_error_handler(
    _authed: ApiAuthed,
    Extension(_user_db): Extension<UserDB>,
    Path((_w_id, _path)): Path<(String, StripPath)>,
    Json(_req): Json<ToggleWorkspaceErrorHandler>,
) -> Result<String> {
    return Err(Error::BadRequest(
        "Muting the error handler for certain script is only available in enterprise version"
            .to_string(),
    ));
}

#[cfg(feature = "enterprise")]
async fn toggle_workspace_error_handler(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(req): Json<ToggleWorkspaceErrorHandler>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    let error_handler_maybe: Option<String> = sqlx::query_scalar!(
        "SELECT error_handler->>'path' FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(None);

    match error_handler_maybe {
        Some(_) => {
            sqlx::query_scalar!(
                "UPDATE script 
                SET ws_error_handler_muted = $3 
                WHERE ctid = (
                    SELECT ctid FROM script
                    WHERE path = $1 AND workspace_id = $2
                    ORDER BY created_at DESC
                    LIMIT 1
                )
",
                path.to_path(),
                w_id,
                req.muted,
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok("".to_string())
        }
        None => {
            tx.commit().await?;
            Err(Error::BadRequest(
                "Workspace error handler needs to be defined".to_string(),
            ))
        }
    }
}

async fn get_tokened_raw_script_by_path(
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(cache): Extension<Arc<AuthCache>>,
    Path((w_id, token, path)): Path<(String, String, StripPath)>,
    Query(query): Query<RawScriptByPathQuery>,
) -> Result<StringWithLength> {
    let authed = cache
        .get_authed(Some(w_id.clone()), &token)
        .await
        .ok_or_else(|| Error::NotAuthorized("Invalid token".to_string()))?;
    return raw_script_by_path(
        authed,
        Extension(user_db),
        Extension(db),
        Path((w_id, path)),
        Query(query),
    )
    .await;
}

async fn get_empty_ts_script_by_path() -> String {
    return String::new();
}

#[derive(Deserialize)]
struct RawScriptByPathQuery {
    // used to make cache immutable with respect to importer
    cache_key: Option<String>,
    // used specifically for python to cache folders on import success to avoid extra db calls on package fetch
    cache_folders: Option<bool>,
}

struct StringWithLength(String);

impl IntoResponse for StringWithLength {
    fn into_response(self) -> axum::response::Response {
        let len = self.0.len();
        ([(header::CONTENT_LENGTH, len.to_string())], self.0).into_response()
    }
}

async fn raw_script_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<RawScriptByPathQuery>,
) -> Result<StringWithLength> {
    if *DEBUG_RAW_SCRIPT_ENDPOINTS {
        tracing::warn!("Raw script by path request: {}", path.to_path());
    }
    let r = raw_script_by_path_internal(path, user_db, db, authed, w_id, false, query).await?;
    Ok(StringWithLength(r))
}

async fn raw_script_by_path_unpinned(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<RawScriptByPathQuery>,
) -> Result<StringWithLength> {
    let r = raw_script_by_path_internal(path, user_db, db, authed, w_id, true, query).await?;
    Ok(StringWithLength(r))
}

lazy_static::lazy_static! {
    static ref DEBUG_RAW_SCRIPT_ENDPOINTS: bool =
        std::env::var("DEBUG_RAW_SCRIPT_ENDPOINTS").is_ok();
}

lazy_static::lazy_static! {
    pub static ref RAW_SCRIPT_CACHE: Cache<String, String> = Cache::new(1000);
    pub static ref CACHE_FOLDERS_PATH: Cache<String, i64> = Cache::new(1000);

}

async fn raw_script_by_path_internal(
    path: StripPath,
    user_db: UserDB,
    db: DB,
    authed: ApiAuthed,
    w_id: String,
    unpin: bool,
    query: RawScriptByPathQuery,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("scripts:read:{}", path))?;
    let cache_path = query
        .cache_key
        .map(|x| format!("{w_id}:{path}:{x}{}", if unpin { ":unpinned" } else { "" }));
    if let Some(cache_path) = cache_path.clone() {
        let cached_content = RAW_SCRIPT_CACHE.get(&cache_path);
        if let Some(cached_content) = cached_content {
            if *DEBUG_RAW_SCRIPT_ENDPOINTS {
                tracing::warn!("Raw script by path request: {} (cached)", path);
            }
            return Ok(cached_content);
        }
    }

    if *DEBUG_RAW_SCRIPT_ENDPOINTS {
        tracing::warn!("Raw script by path request: {} (not cached)", path);
    }

    if !path.ends_with(".py")
        && !path.ends_with(".ts")
        && !path.ends_with(".go")
        && !path.ends_with(".sh")
    {
        return Err(Error::BadRequest(format!(
            "Path must ends with a .py, .ts, .go. or .sh extension: {}",
            path
        )));
    }
    let path = path
        .trim_end_matches(".py")
        .trim_end_matches(".bun.ts")
        .trim_end_matches(".deno.ts")
        .trim_end_matches(".ts")
        .trim_end_matches(".go")
        .trim_end_matches(".sh");

    // folder cache is only useful for python given it needs to recuse over all intermediate folders to find the package.
    // When a script exists in a folder, we can cache the fact that the folder exists to avoid extra db calls.
    let mut split_path = path.split("/").collect::<Vec<&str>>();
    let folder_path = if query.cache_folders.is_some() && split_path.len() > 2 {
        Some(format!("{w_id}:{path}/"))
    } else {
        None
    };

    let has_folder_cache = folder_path.is_some();
    if let Some(cache_folders) = folder_path {
        let cached_content = CACHE_FOLDERS_PATH.get(&cache_folders);
        if let Some(cached_ts) = cached_content {
            if cached_ts >= chrono::Utc::now().timestamp() - 300 {
                // 5 minutes
                if *DEBUG_RAW_SCRIPT_ENDPOINTS {
                    tracing::warn!("Raw script by path request: {} (cached folders)", path);
                }
                return Ok("WINDMILL_IS_FOLDER".to_string());
            } else {
                if *DEBUG_RAW_SCRIPT_ENDPOINTS {
                    tracing::warn!(
                        "Raw script by path request: {} (cached folders expired)",
                        path
                    );
                }
            }
        }
    }

    let mut tx = user_db.begin(&authed).await?;

    let content_o = sqlx::query_scalar!(
        "SELECT content FROM script WHERE path = $1 AND workspace_id = $2 AND archived = false ORDER BY created_at DESC LIMIT 1",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .warn_after_seconds(5)
    .await?;
    tx.commit().await?;
    if *DEBUG_RAW_SCRIPT_ENDPOINTS {
        tracing::warn!(
            "Raw script by path request: {} (content: {:?})",
            path,
            content_o
        );
    }

    if content_o.is_none() {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM script WHERE path = $1 AND workspace_id = $2 AND archived = false ORDER BY created_at DESC LIMIT 1)",
            path,
            w_id
        )
        .fetch_one(&db)
        .warn_after_seconds(5)
        .await?
        .unwrap_or(false);

        if exists {
            return Err(Error::NotFound(format!(
                "Script {path} exists but {} does not have permissions to access it",
                authed.username
            )));
        } else {
            if *DEBUG_RAW_SCRIPT_ENDPOINTS {
                let other_script_o = sqlx::query_scalar!(
                    "SELECT path FROM script WHERE workspace_id = $1 AND archived = false",
                    w_id
                )
                .fetch_all(&db)
                .await?;
                let other_script_archived = sqlx::query_scalar!(
                    "SELECT distinct(path) FROM script WHERE workspace_id = $1 AND archived = true",
                    w_id
                )
                .fetch_all(&db)
                .await?;
                tracing::warn!(
                    "Script {path} does not exist in workspace {w_id} but these paths do, non-archived: {:?} | archived: {:?}",
                    other_script_o.join(", "),
                    other_script_archived.join(", ")
                )
            }
        }
    }

    let content = not_found_if_none(content_o, "Script", path)?;

    let content = if unpin {
        remove_pinned_imports(&content)?
    } else {
        content
    };

    if has_folder_cache {
        while split_path.len() >= 2 {
            split_path.pop();
            let npath = split_path.join("/");
            CACHE_FOLDERS_PATH.insert(format!("{w_id}:{npath}/"), chrono::Utc::now().timestamp());
        }
    }

    if let Some(cache_path) = cache_path {
        RAW_SCRIPT_CACHE.insert(cache_path, content.clone());
    }
    if *DEBUG_RAW_SCRIPT_ENDPOINTS {
        tracing::warn!("Raw script by path request: {} (content response)", path);
    }
    Ok(content)
}

async fn exists_script_by_path(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM script WHERE path = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1)",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn get_script_by_hash_internal<'c>(
    db: &mut Transaction<'c, Postgres>,
    workspace_id: &str,
    hash: &ScriptHash,
    with_starred_info_for_username: Option<&str>,
) -> Result<ScriptWithStarred<ScriptRunnableSettingsHandle>> {
    let script_o = if let Some(username) = with_starred_info_for_username {
        sqlx::query_as::<_, ScriptWithStarred<ScriptRunnableSettingsHandle>>(
            "SELECT s.*, favorite.path IS NOT NULL as starred
            FROM script s
            LEFT JOIN favorite 
            ON favorite.favorite_kind = 'script' 
                AND favorite.workspace_id = s.workspace_id 
                AND favorite.path = s.path 
                AND favorite.usr = $1 
            WHERE s.hash = $2 AND s.workspace_id = $3",
        )
        .bind(&username)
        .bind(hash)
        .bind(workspace_id)
        .fetch_optional(&mut **db)
        .await?
    } else {
        sqlx::query_as::<_, ScriptWithStarred<ScriptRunnableSettingsHandle>>(
            "SELECT *, NULL as starred FROM script WHERE hash = $1 AND workspace_id = $2",
        )
        .bind(hash)
        .bind(workspace_id)
        .fetch_optional(&mut **db)
        .await?
    };

    let script = not_found_if_none(script_o, "Script", hash.to_string())?;
    Ok(script)
}

#[derive(Deserialize)]
struct GetScriptByHashQuery {
    authed: Option<bool>,
}
async fn get_script_by_hash(
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, hash)): Path<(String, ScriptHash)>,
    Query(query): Query<WithStarredInfoQuery>,
    Query(query_auth): Query<GetScriptByHashQuery>,
    Extension(authed): Extension<ApiAuthed>,
) -> JsonResult<ScriptWithStarred<ScriptRunnableSettingsInline>> {
    let mut tx = if query_auth.authed.is_some_and(|x| x) {
        user_db.begin(&authed).await?
    } else {
        db.begin().await?
    };
    let r = get_script_by_hash_internal(
        &mut tx,
        &w_id,
        &hash,
        query.with_starred_info.and_then(|x| {
            if x {
                Some(authed.username.as_str())
            } else {
                None
            }
        }),
    )
    .await?;

    check_scopes(&authed, || format!("scripts:read:{}", &r.script.path))?;

    tx.commit().await?;

    Ok(Json(
        windmill_common::scripts::prefetch_cached_script_with_starred(r, &db).await?,
    ))
}

async fn raw_script_by_hash(
    Extension(db): Extension<DB>,
    Path((w_id, hash_str)): Path<(String, String)>,
) -> Result<String> {
    let mut tx = db.begin().await?;
    let hash = ScriptHash(to_i64(hash_str.strip_suffix(".ts").ok_or_else(|| {
        Error::BadRequest("Raw script path must end with .ts".to_string())
    })?)?);
    let r = get_script_by_hash_internal(&mut tx, &w_id, &hash, None).await?;
    tx.commit().await?;

    Ok(r.script.content)
}

#[derive(Serialize)]
struct DeploymentStatus {
    lock: Option<String>,
    lock_error_logs: Option<String>,
    job_id: Option<sqlx::types::Uuid>,
}
async fn get_deployment_status(
    Extension(db): Extension<DB>,
    Path((w_id, hash)): Path<(String, ScriptHash)>,
) -> JsonResult<DeploymentStatus> {
    let mut tx = db.begin().await?;
    let status_o = sqlx::query!(
        "SELECT s.lock, s.lock_error_logs, dm.job_id
         FROM script s
         LEFT JOIN deployment_metadata dm ON s.hash = dm.script_hash AND s.workspace_id = dm.workspace_id
         WHERE s.hash = $1 AND s.workspace_id = $2",
        hash.0,
        w_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let status = not_found_if_none(status_o, "DeploymentStatus", hash.to_string())?;

    let deployment_status = DeploymentStatus {
        lock: status.lock,
        lock_error_logs: status.lock_error_logs,
        job_id: status.job_id,
    };

    tx.commit().await?;
    Ok(Json(deployment_status))
}

pub async fn require_is_writer(authed: &ApiAuthed, path: &str, w_id: &str, db: DB) -> Result<()> {
    return windmill_api_auth::require_is_writer(
        authed,
        path,
        w_id,
        db,
        "SELECT extra_perms FROM script WHERE path = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
        "script",
    )
    .await;
}

async fn archive_script_by_path(
    authed: ApiAuthed,
    Extension(webhook): Extension<WebhookShared>,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<()> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot archive scripts for security reasons".to_string(),
        ));
    }
    let path = path.to_path();
    check_scopes(&authed, || format!("scripts:write:{}", path))?;
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }
    let mut tx = user_db.begin(&authed).await?;

    require_owner_of_path(&authed, path)?;

    let hash: i64 = sqlx::query_scalar!(
        "UPDATE script SET archived = true WHERE path = $1 AND workspace_id = $2 RETURNING hash",
        path,
        &w_id
    )
    .fetch_one(&db)
    .await
    .map_err(|e| Error::internal_err(format!("archiving script in {w_id}: {e:#}")))?;

    clear_static_asset_usage(&mut *tx, &w_id, path, AssetUsageKind::Script).await?;

    audit_log(
        &mut *tx,
        &authed,
        "scripts.archive",
        ActionKind::Delete,
        &w_id,
        Some(&ScriptHash(hash).to_string()),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;

    ScopedDependencyMap::clear_map_for_item(path, &w_id, "script", tx, &None)
        .await
        .commit()
        .await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Script {
            hash: ScriptHash(0), // dummy hash as it will not get inserted in db
            path: path.to_string(),
            parent_path: Some(path.to_string()),
        },
        Some(format!("Script '{}' archived", path)),
        true,
        None,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::DeleteScript { workspace: w_id, hash: hash.to_string() },
    );

    Ok(())
}

async fn archive_script_by_hash(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, hash)): Path<(String, ScriptHash)>,
) -> JsonResult<Script<ScriptRunnableSettingsInline>> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot archive scripts for security reasons".to_string(),
        ));
    }
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }
    let mut tx = user_db.begin(&authed).await?;

    let script = sqlx::query_as::<_, Script<ScriptRunnableSettingsHandle>>(
        "UPDATE script SET archived = true WHERE hash = $1 AND workspace_id = $2 RETURNING *",
    )
    .bind(&hash.0)
    .bind(&w_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::internal_err(format!("archiving script in {w_id}: {e:#}")))?;

    check_scopes(&authed, || format!("scripts:write:{}", &script.path))?;
    clear_static_asset_usage_by_script_hash(&mut *tx, &w_id, hash).await?;

    audit_log(
        &mut *tx,
        &authed,
        "scripts.archive",
        ActionKind::Delete,
        &w_id,
        Some(&hash.to_string()),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;

    ScopedDependencyMap::clear_map_for_item(&script.path, &w_id, "script", tx, &None)
        .await
        .commit()
        .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::DeleteScript { workspace: w_id, hash: hash.to_string() },
    );

    Ok(Json(
        windmill_common::scripts::prefetch_cached_script(script, &db).await?,
    ))
}

async fn delete_script_by_hash(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(db): Extension<DB>,
    Path((w_id, hash)): Path<(String, ScriptHash)>,
) -> JsonResult<Script<ScriptRunnableSettingsInline>> {
    require_admin(authed.is_admin, &authed.username)?;
    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }
    let mut tx = user_db.begin(&authed).await?;
    let script = sqlx::query_as::<_, Script<ScriptRunnableSettingsHandle>>(
        "UPDATE script SET content = '', archived = true, deleted = true, lock = '', schema = null WHERE hash = $1 AND \
         workspace_id = $2 RETURNING *",
    )
    .bind(&hash.0)
    .bind(&w_id)
    .fetch_one(&db)
    .await
    .map_err(|e| Error::internal_err(format!("deleting script by hash {w_id}: {e:#}")))?;

    check_scopes(&authed, || format!("scripts:write:{}", &script.path))?;

    clear_static_asset_usage_by_script_hash(&mut *tx, &w_id, hash).await?;

    audit_log(
        &mut *tx,
        &authed,
        "scripts.delete",
        ActionKind::Delete,
        &w_id,
        Some(&hash.to_string()),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::DeleteScript { workspace: w_id, hash: hash.to_string() },
    );

    Ok(Json(
        windmill_common::scripts::prefetch_cached_script(script, &db).await?,
    ))
}

#[derive(Deserialize)]
struct DeleteScriptQuery {
    keep_captures: Option<bool>,
}

async fn delete_script_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<DeleteScriptQuery>,
) -> JsonResult<String> {
    let path = path.to_path();

    check_scopes(&authed, || format!("scripts:write:{}", path))?;

    if path == "u/admin/hub_sync" && w_id == "admins" {
        return Err(Error::BadRequest(
            "Cannot delete the global setup app".to_string(),
        ));
    }

    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let mut tx = user_db.begin(&authed).await?;

    let draft_only = sqlx::query_scalar!(
        "SELECT draft_only FROM script WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    let script = if !draft_only {
        require_admin(authed.is_admin, &authed.username)?;
        sqlx::query_scalar!(
            "DELETE FROM script WHERE path = $1 AND workspace_id = $2 RETURNING path",
            path,
            w_id
        )
        .fetch_one(&db)
        .await
        .map_err(|e| Error::internal_err(format!("deleting script by path {w_id}: {e:#}")))?
    } else {
        sqlx::query_scalar!(
            "DELETE FROM script WHERE path = $1 AND workspace_id = $2 RETURNING path",
            path,
            w_id
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| Error::internal_err(format!("deleting script by path {w_id}: {e:#}")))?
    };

    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'script'",
        path,
        w_id
    )
    .execute(&db)
    .await?;

    if !query.keep_captures.unwrap_or(false) {
        sqlx::query!(
            "DELETE FROM capture_config WHERE path = $1 AND workspace_id = $2 AND is_flow IS FALSE",
            path,
            w_id
        )
        .execute(&db)
        .await?;

        sqlx::query!(
            "DELETE FROM capture WHERE path = $1 AND workspace_id = $2 AND is_flow IS FALSE",
            path,
            w_id
        )
        .execute(&db)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "scripts.delete",
        ActionKind::Delete,
        &w_id,
        Some(&path),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Script {
            hash: ScriptHash(0), // Temporary value as it will get removed right after
            path: path.to_string(),
            parent_path: Some(path.to_string()),
        },
        Some(format!("Script '{}' deleted", path)),
        true,
        None,
    )
    .await?;

    sqlx::query!(
        "DELETE FROM deployment_metadata WHERE path = $1 AND workspace_id = $2 AND script_hash IS NOT NULL",
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
        w_id.clone(),
        WebhookMessage::DeleteScriptPath { workspace: w_id, path: path.to_owned() },
    );

    Ok(Json(script))
}

async fn delete_scripts_bulk(
    authed: ApiAuthed,
    Extension(webhook): Extension<WebhookShared>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(request): Json<BulkDeleteRequest>,
) -> JsonResult<Vec<String>> {
    for path in &request.paths {
        check_scopes(&authed, || format!("scripts:write:{}", path))?;
    }

    require_admin(authed.is_admin, &authed.username)?;

    if request.paths.contains(&"u/admin/hub_sync".to_string()) && w_id == "admins" {
        return Err(Error::BadRequest(
            "Cannot delete the global setup app".to_string(),
        ));
    }

    if let RuleCheckResult::Blocked(msg) = check_user_against_rule(
        &w_id,
        &ProtectionRuleKind::DisableDirectDeployment,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let mut tx = db.begin().await?;

    let mut deleted_paths = sqlx::query_scalar!(
        "DELETE FROM script WHERE workspace_id = $1 AND path = ANY($2) RETURNING path",
        w_id,
        &request.paths
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| Error::internal_err(format!("deleting scripts in bulk {w_id}: {e:#}")))?;

    // remove duplicates from deleted_paths
    deleted_paths.sort();
    deleted_paths.dedup();

    sqlx::query!(
        "DELETE FROM draft WHERE workspace_id = $1 AND path = ANY($2) AND typ = 'script'",
        w_id,
        &deleted_paths
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM capture_config WHERE workspace_id = $1 AND path = ANY($2) AND is_flow IS FALSE",
        w_id,
        &deleted_paths
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM capture WHERE workspace_id = $1 AND path = ANY($2) AND is_flow IS FALSE",
        w_id,
        &deleted_paths
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "scripts.delete_bulk",
        ActionKind::Delete,
        &w_id,
        Some(&deleted_paths.join(", ")),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;

    tx.commit().await?;

    try_join_all(deleted_paths.iter().map(|path| {
        handle_deployment_metadata(
            &authed.email,
            &authed.username,
            &db,
            &w_id,
            DeployedObject::Script { hash: ScriptHash(0), path: path.clone(), parent_path: None },
            Some(format!("Script '{}' deleted", path)),
            true,
            None,
        )
    }))
    .await?;

    sqlx::query!(
        "DELETE FROM deployment_metadata WHERE workspace_id = $1 AND path = ANY($2) AND script_hash IS NOT NULL",
        w_id,
        &deleted_paths
    )
    .execute(&db)
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "error deleting deployment metadata for scripts with paths {} in workspace {w_id}: {e:#}", deleted_paths.join(", ")
        ))
    })?;

    for path in &deleted_paths {
        webhook.send_message(
            w_id.clone(),
            WebhookMessage::DeleteScriptPath { workspace: w_id.clone(), path: path.to_owned() },
        );
    }

    Ok(Json(deleted_paths))
}

/// Validates that script debouncing configuration is supported by all workers
/// Returns an error if debouncing is configured but workers are behind required version
async fn guard_script_from_debounce_data(ns: &NewScript) -> Result<()> {
    if !MIN_VERSION_SUPPORTS_DEBOUNCING.met().await && !ns.debouncing_settings.is_default() {
        tracing::warn!(
            "Script debouncing configuration rejected: workers are behind minimum required version for debouncing feature"
        );
        Err(Error::WorkersAreBehind { feature: "Debouncing".into(), min_version: "1.566.0".into() })
    } else if !MIN_VERSION_SUPPORTS_DEBOUNCING_V2.met().await
        && !ns.debouncing_settings.is_legacy_compatible()
        && !*WMDEBUG_FORCE_NO_LEGACY_DEBOUNCING_COMPAT
    {
        tracing::warn!(
            "Script debouncing configuration rejected: workers are behind minimum required version for debouncing feature"
        );
        Err(Error::WorkersAreBehind {
            feature: "V2 Debouncing".into(),
            min_version: "1.597.0".into(),
        })
    } else {
        Ok(())
    }
}
