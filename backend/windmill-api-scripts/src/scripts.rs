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
    build_scope_path_predicate, check_scopes, maybe_refresh_folders, require_owner_of_path,
    ApiAuthed,
};
use windmill_common::{
    user_drafts::{overlay_or_draft_only, DraftUserRef, UserDraftItemKind, WithDraftOverlay},
    utils::{BulkDeleteRequest, WithStarredInfoQuery, HTTP_CLIENT},
    webhook::{WebhookMessage, WebhookShared},
    workspaces::{check_deploy_rules, RuleCheckResult},
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
use sql_builder::prelude::*;
use sqlx::{FromRow, Postgres, Transaction};
use std::{collections::HashMap, sync::Arc};
use windmill_audit::audit_oss::{audit_log, AuditAuthorable};
use windmill_audit::ActionKind;
use windmill_dep_map::process_relative_imports;
use windmill_dep_map::scoped_dependency_map::ScopedDependencyMap;

use windmill_common::{
    assets::{
        clear_script_triggers, clear_static_asset_usage, clear_static_asset_usage_by_script_hash,
        insert_script_trigger, parse_duration_secs, parse_pipeline_annotations,
        replace_static_asset_usage, trigger_spec_to_row, AssetUsageKind, TriggerSpec,
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
        ScriptHistory, ScriptHistoryUpdate, ScriptKind, ScriptLang, ScriptModule,
        ScriptWithStarred,
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

pub fn global_service() -> Router {
    Router::new()
        .route("/hub/top", get(get_top_hub_scripts))
        .route("/hub/get/{*path}", get(get_hub_script_by_path))
        .route("/hub/get_full/{*path}", get(get_full_hub_script_by_path))
        .route("/hub/pick/{*path}", get(pick_hub_script_by_path))
}

pub fn global_unauthed_service() -> Router {
    Router::new()
        .route(
            "/tokened_raw/{workspace}/{token}/{*path}",
            get(get_tokened_raw_script_by_path),
        )
        .route("/empty_ts/{*path}", get(get_empty_ts_script_by_path))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_scripts))
        .route("/list_search", get(list_search_scripts))
        .route("/create", post(create_script))
        .route("/create_snapshot", post(create_snapshot_script))
        .route("/archive/p/{*path}", post(archive_script_by_path))
        .route("/get/p/{*path}", get(get_script_by_path))
        .route("/list_tokens/{*path}", get(list_tokens))
        .route("/raw/p/{*path}", get(raw_script_by_path))
        .route("/raw_unpinned/p/{*path}", get(raw_script_by_path_unpinned))
        .route("/exists/p/{*path}", get(exists_script_by_path))
        .route("/archive/h/{hash}", post(archive_script_by_hash))
        .route("/delete/h/{hash}", post(delete_script_by_hash))
        .route("/delete/p/{*path}", post(delete_script_by_path))
        .route("/delete_bulk", delete(delete_scripts_bulk))
        .route("/get/h/{hash}", get(get_script_by_hash))
        .route("/raw/h/{hash}", get(raw_script_by_hash))
        .route("/deployment_status/h/{hash}", get(get_deployment_status))
        .route("/list_paths", get(list_paths))
        .route(
            "/toggle_workspace_error_handler/p/{*path}",
            post(toggle_workspace_error_handler),
        )
        .route("/history/p/{*path}", get(get_script_history))
        .route("/get_latest_version/{*path}", get(get_latest_version))
        .route(
            "/list_paths_from_workspace_runnable/{*path}",
            get(list_paths_from_workspace_runnable),
        )
        .route(
            "/history_update/h/{hash}/p/{*path}",
            post(update_script_history),
        )
        .route("/list_dedicated_with_deps", get(list_dedicated_with_deps))
        // Temporary raw script storage for CLI lock generation
        .route("/raw_temp/store", post(store_raw_script_temp))
        .route("/raw_temp/diff", post(diff_raw_scripts_with_deployed))
        // CI test results
        .route("/ci_test_results/{kind}/{*path}", get(get_ci_test_results))
        .route("/ci_test_results_batch", post(get_ci_test_results_batch))
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

    let allowed = build_scope_path_predicate(&authed, "scripts", "read");
    let rows = sqlx::query_as!(
        SearchScript,
        "SELECT path, content from script WHERE workspace_id = $1 AND archived = false LIMIT $2",
        &w_id,
        n
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .filter(|r| allowed(&r.path))
    .collect::<Vec<_>>();
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_scripts(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
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
            "o.created_at as created_at",
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
            "ws_error_handler_muted",
            "auto_kind",
            "codebase IS NOT NULL as use_codebase",
            "kind",
            "o.labels",
            "draft.email IS NOT NULL as is_draft",
            // Canonical reference for the draft-feature comments; flows/apps point here.
            // Per-path draft owners as a JSON array (`Json<Vec<DraftUserRef>>`); NULL -> None,
            // never an empty array. LEFT JOIN `usr` keeps orphaned drafts (user left workspace)
            // visible with `username = None`. A superadmin authoring in a workspace they are not
            // a member of has no `usr` row, so fall back to their instance-derived username
            // (`password.username`), or their email when derivation is disabled — this keeps the
            // raw email out of the payload whenever a derived username exists. The genuine
            // NULL-email legacy row stays None (no `usr`/`password` match, `d.email` is NULL).
            "(SELECT json_agg(json_build_object('username', COALESCE(u.username, p.username, CASE WHEN p.email IS NOT NULL THEN d.email END)) ORDER BY COALESCE(u.username, p.username, CASE WHEN p.email IS NOT NULL THEN d.email END) NULLS LAST) \
              FROM draft d \
              LEFT JOIN usr u ON u.workspace_id = d.workspace_id AND u.email = d.email \
              LEFT JOIN password p ON p.email = d.email AND p.super_admin = true \
              WHERE d.workspace_id = o.workspace_id AND d.path = o.path AND d.typ = 'script') as draft_users",
            "folder_labels(o.workspace_id, o.path) as inherited_labels"
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
            "draft.path = o.path AND draft.workspace_id = o.workspace_id AND draft.typ = 'script' AND draft.email = ?"
                .bind(&authed.email),
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
        // only include scripts that have a runnable entrypoint. Use a
        // deny-list: anything that isn't a 'lib' (library script without
        // main) is callable, including future `auto_kind` values.
        sqlb.and_where("(o.auto_kind IS NULL OR o.auto_kind <> 'lib')");
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
    if let Some(label) = &lq.label {
        for l in label.split(',') {
            sqlb.and_where(
                "(o.labels @> ARRAY[?] OR folder_labels(o.workspace_id, o.path) @> ARRAY[?])"
                    .bind(&l.trim())
                    .bind(&l.trim()),
            );
        }
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

    if let Some(languages) = &lq.languages {
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
    let allowed = build_scope_path_predicate(&authed, "scripts", "read");
    let mut rows = sqlx::query_as::<_, ListableScript>(&sql)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .filter(|r| allowed(&r.path))
        .collect::<Vec<_>>();
    tx.commit().await?;

    // Canonical reference for draft-only synthesis; the other kinds point here.
    // Append the authed user's drafts at paths with no deployed script. Gated on
    // `include_draft_only` so picker callers stay deployed-only (home page opts in);
    // skipped past page 0 or under any narrowing filter to keep pagination clean.
    if lq.include_draft_only.unwrap_or(false)
        && !authed.is_operator
        && offset == 0
        && lq.path_start.is_none()
        && lq.path_exact.is_none()
        && lq.created_by.is_none()
        && lq.first_parent_hash.is_none()
        && lq.last_parent_hash.is_none()
        && lq.parent_hash.is_none()
        && lq.is_template.is_none()
        && lq.dedicated_worker.is_none()
        && lq.label.is_none()
        && lq.languages.is_none()
        && !lq.starred_only.unwrap_or(false)
        && !lq.show_archived.unwrap_or(false)
    {
        // `(email = $2 OR email IS NULL)` surfaces the user's own draft-only rows plus
        // legacy NULL-email workspace rows; `DISTINCT ON (path)` ordered `email IS NULL`
        // last collapses a path holding both to the owned row.
        let draft_only_rows = sqlx::query!(
            r#"SELECT DISTINCT ON (path)
                      path,
                      value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>",
                      created_at
               FROM draft
               WHERE workspace_id = $1
                 AND typ = 'script'
                 AND (email = $2 OR email IS NULL)
                 AND NOT EXISTS (
                     SELECT 1 FROM script s
                     WHERE s.workspace_id = draft.workspace_id
                       AND s.path = draft.path
                 )
               ORDER BY path, (email IS NULL)"#,
            &w_id,
            &authed.email,
        )
        .fetch_all(&db)
        .await?;

        for row in draft_only_rows {
            let v: serde_json::Value =
                serde_json::from_str(row.value.0.get()).unwrap_or(serde_json::Value::Null);
            let language: ScriptLang = v
                .get("language")
                .and_then(|x| serde_json::from_value(x.clone()).ok())
                .unwrap_or_default();
            let kind: ScriptKind = v
                .get("kind")
                .and_then(|x| serde_json::from_value(x.clone()).ok())
                .unwrap_or(ScriptKind::Script);
            // Scripts bind the Path widget to `script.path`, so the typed path
            // round-trips through the draft JSON's own `path` (no `draft_path` field).
            let draft_path = v
                .get("path")
                .and_then(|s| s.as_str())
                .filter(|s| !s.is_empty() && *s != row.path.as_str())
                .map(|s| s.to_string());
            // A draft-only pipeline node (`// pipeline`) has no deployed row to carry
            // auto_kind, so compute it from the draft content — mirroring the create
            // path — so the home page folds it into its pipeline like a deployed member.
            let auto_kind = v
                .get("content")
                .and_then(|s| s.as_str())
                .filter(|c| parse_pipeline_annotations(c).in_pipeline)
                .map(|_| "pipeline".to_string());
            rows.push(ListableScript {
                hash: ScriptHash(0),
                path: row.path,
                summary: v
                    .get("summary")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                created_at: row.created_at,
                archived: false,
                extra_perms: serde_json::Value::Object(serde_json::Map::new()),
                language,
                starred: false,
                tag: v.get("tag").and_then(|s| s.as_str()).map(|s| s.to_string()),
                description: v
                    .get("description")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string()),
                draft_only: Some(true),
                has_deploy_errors: false,
                ws_error_handler_muted: None,
                auto_kind,
                use_codebase: false,
                deployment_msg: None,
                kind,
                labels: None,
                // Synthesized rows have no deployed row to inherit folder labels from.
                inherited_labels: None,
                is_draft: true,
                draft_path,
                // Synthesized rows are the authed user's own draft (single-user case).
                draft_users: Some(sqlx::types::Json(vec![DraftUserRef {
                    username: Some(authed.username.clone()),
                }])),
            });
        }
    }

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
        &format!("{}/scripts/top", **HUB_BASE_URL.load()),
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
    Query(query): Query<CreateScriptQuery>,
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
                query.skip_if_noop,
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

#[derive(Deserialize, Default, Clone, Copy)]
struct CreateScriptQuery {
    /// When set by the caller (currently only the CLI), the backend
    /// short-circuits deploys whose content, lockfile, and metadata are
    /// identical to the parent: no new version is inserted and the git
    /// sync / promotion callbacks are suppressed.
    #[serde(default)]
    skip_if_noop: bool,
}

async fn create_script(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(query): Query<CreateScriptQuery>,
    Json(ns): Json<NewScript>,
) -> Result<(StatusCode, String)> {
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
    let script_path = ns.path.clone();
    let email = authed.email.clone();
    let username = authed.username.clone();
    let (hash, tx, hdm) = create_script_internal(
        ns,
        w_id.clone(),
        authed,
        db.clone(),
        user_db,
        webhook,
        query.skip_if_noop,
    )
    .await?;
    tx.commit().await?;
    if let Some(hdm) = hdm {
        // hdm is Some when no lock generation is needed (script is ready immediately).
        // Trigger CI tests for any items that reference this script.
        hdm.handle(&db).await?;
        let db2 = db.clone();
        tokio::spawn(async move {
            if let Err(e) = windmill_dep_map::ci_tests::trigger_ci_tests_for_item(
                &db2,
                &w_id,
                &script_path,
                "script",
                &email,
                &username,
            )
            .await
            {
                tracing::error!(%e, "error triggering CI tests after script deploy");
            }
        });
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

/// Returns true when `ns` is effectively identical to its `parent`: same
/// path, content, lockfile, and all persisted metadata. Used by the deploy
/// path to short-circuit no-op re-deploys and suppress the follow-on git
/// sync / promotion callbacks.
///
/// ## How drift is prevented
///
/// `NewScript` grows over time. If a new field is added but not wired in
/// here, a real change would be silently classified as a no-op and
/// dropped. The exhaustive destructure below turns that into a *compile
/// error*: every field of `NewScript` must either feed into a comparison
/// or be explicitly opted out via `field: _` with a one-line rationale.
///
/// When adding a field to `NewScript`:
/// 1. Add it to the destructure here. The compiler will refuse to build
///    until you do.
/// 2. Either add a `!=` comparison against `parent` below, or bind it to
///    `_` and write why it shouldn't affect the no-op decision.
/// 3. Don't forget `impl Hash for NewScript` in windmill-types.
async fn is_noop_deploy_against_parent(
    ns: &NewScript,
    parent: &Script<ScriptRunnableSettingsHandle>,
    db: &DB,
) -> Result<bool> {
    if parent.archived || parent.deleted {
        return Ok(false);
    }

    // Compile-time drift guard — see the docstring above. The `ns.foo` bindings
    // below shadow each field as `&T`; every binding must be referenced in a
    // comparison (the compiler will warn on any that isn't), and every ignored
    // field (`_`) carries its rationale inline.
    let NewScript {
        path,
        // version-identity field — always differs between child and parent by design
        parent_hash: _,
        summary,
        description,
        content,
        schema,
        is_template,
        lock,
        language,
        kind,
        tag,
        envs,
        concurrency_settings,
        debouncing_settings,
        cache_ttl,
        cache_ignore_s3_path,
        dedicated_worker,
        ws_error_handler_muted,
        priority,
        timeout,
        delete_after_use,
        delete_after_secs,
        restart_unless_cancelled,
        // per-deploy audit field — does not change what the script *is*
        deployment_message: _,
        visible_to_runner_only,
        // derived from `content` at deploy time; content equality implies equality here
        auto_kind: _,
        codebase,
        has_preprocessor,
        on_behalf_of_email,
        // caller-intent flag (permission preservation), not script state
        preserve_on_behalf_of: _,
        assets,
        modules,
        // caller-intent flag (auto-resolve parent), not script state
        auto_parent: _,
        labels,
        // caller-intent flag (preserve user drafts on CLI/git-sync deploys);
        // transient, never persisted, does not change what the script *is*
        skip_draft_deletion: _,
    } = ns;

    if path != &parent.path {
        return Ok(false);
    }
    if content != &parent.content {
        return Ok(false);
    }
    if normalize_optional_text(lock.as_deref()) != normalize_optional_text(parent.lock.as_deref()) {
        return Ok(false);
    }
    if summary != &parent.summary {
        return Ok(false);
    }
    if description != &parent.description {
        return Ok(false);
    }
    if language != &parent.language {
        return Ok(false);
    }
    if std::mem::discriminant(kind.as_ref().unwrap_or(&ScriptKind::Script))
        != std::mem::discriminant(&parent.kind)
    {
        return Ok(false);
    }
    if tag != &parent.tag {
        return Ok(false);
    }
    if envs.as_deref().unwrap_or_default() != parent.envs.as_deref().unwrap_or_default() {
        return Ok(false);
    }
    if labels != &parent.labels {
        return Ok(false);
    }
    if cache_ttl != &parent.cache_ttl
        || cache_ignore_s3_path != &parent.cache_ignore_s3_path
        || dedicated_worker != &parent.dedicated_worker
        || ws_error_handler_muted != &parent.ws_error_handler_muted
        || priority != &parent.priority
        || timeout != &parent.timeout
        || delete_after_use != &parent.delete_after_use
        || delete_after_secs != &parent.delete_after_secs
        || restart_unless_cancelled != &parent.restart_unless_cancelled
        || visible_to_runner_only != &parent.visible_to_runner_only
        || has_preprocessor != &parent.has_preprocessor
        || is_template.unwrap_or(false) != parent.is_template.unwrap_or(false)
    {
        return Ok(false);
    }
    if normalize_optional_text(codebase.as_deref())
        != normalize_optional_text(parent.codebase.as_deref())
    {
        return Ok(false);
    }
    if on_behalf_of_email != &parent.on_behalf_of_email {
        return Ok(false);
    }
    if !schema_opt_eq(schema.as_ref(), parent.schema.as_ref()) {
        return Ok(false);
    }
    if !json_serialize_eq(assets, &parent.assets) {
        return Ok(false);
    }
    if !modules_eq(modules.as_ref(), parent.modules.as_ref()) {
        return Ok(false);
    }

    let (parent_debouncing, parent_concurrency) =
        windmill_common::runnable_settings::prefetch_cached_from_handle(
            parent.runnable_settings.runnable_settings_handle,
            db,
        )
        .await?;
    if concurrency_settings != &parent_concurrency {
        return Ok(false);
    }
    if debouncing_settings != &parent_debouncing {
        return Ok(false);
    }

    Ok(true)
}

/// Treats `None` and `Some("")` as equivalent — matches how the insert path
/// normalizes optional text fields like `lock` and `codebase`.
fn normalize_optional_text(s: Option<&str>) -> &str {
    s.unwrap_or("")
}

fn schema_opt_eq(a: Option<&Schema>, b: Option<&Schema>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            match (
                serde_json::from_str::<serde_json::Value>(x.0.get()),
                serde_json::from_str::<serde_json::Value>(y.0.get()),
            ) {
                (Ok(xv), Ok(yv)) => xv == yv,
                _ => false,
            }
        }
        _ => false,
    }
}

fn json_serialize_eq<T: Serialize>(a: &T, b: &T) -> bool {
    match (serde_json::to_value(a), serde_json::to_value(b)) {
        (Ok(av), Ok(bv)) => av == bv,
        _ => false,
    }
}

fn modules_eq(
    a: Option<&HashMap<String, ScriptModule>>,
    b: Option<&HashMap<String, ScriptModule>>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            if x.len() != y.len() {
                return false;
            }
            x.iter().all(|(k, v)| match y.get(k) {
                Some(ov) => {
                    v.content == ov.content && v.language == ov.language && v.lock == ov.lock
                }
                None => false,
            })
        }
        _ => false,
    }
}

async fn create_script_internal<'c>(
    mut ns: NewScript,
    w_id: String,
    authed: ApiAuthed,
    db: sqlx::Pool<Postgres>,
    user_db: UserDB,
    webhook: WebhookShared,
    skip_if_noop: bool,
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
                    "You have reached the maximum number of scripts (5000) on cloud. Check your usage in Workspace Settings > General > Cloud Quotas. Contact support@windmill.dev to increase the limit"
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
    // Caller-intent: CLI / git-sync deploys ask us to preserve any existing
    // user draft at this path instead of wiping it as part of the deploy.
    let skip_draft_deletion = ns.skip_draft_deletion.unwrap_or(false);
    let hash = ScriptHash(hash_script(&ns));
    let authed = maybe_refresh_folders(&ns.path, &w_id, authed, &db).await;

    let mut tx: Transaction<'_, Postgres> = user_db.begin(&authed).await?;

    // Apply folder default_permissioned_as the first time a script is deployed
    // at this path. Check inside the transaction to avoid TOCTOU with concurrent deploys.
    let explicit_preserve = ns.on_behalf_of_email.is_some()
        && ns.preserve_on_behalf_of.unwrap_or(false)
        && windmill_common::can_preserve_on_behalf_of(&authed);
    if !explicit_preserve && windmill_common::can_preserve_on_behalf_of(&authed) {
        let path_already_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM script WHERE path = $1 AND workspace_id = $2)",
            &ns.path,
            &w_id
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(false);
        if !path_already_exists {
            if let Some(default_email) =
                windmill_common::folders::resolve_folder_default_on_behalf_of_email(
                    &db, &w_id, &ns.path,
                )
                .await?
            {
                ns.on_behalf_of_email = Some(default_email);
                ns.preserve_on_behalf_of = Some(true);
            }
        }
    }
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
    // When auto_parent is set, serialize concurrent creates for the same (workspace, path)
    // so the clashing_script query always sees the latest committed head.
    if ns.auto_parent.unwrap_or(false) {
        sqlx::query_scalar!(
            "SELECT pg_advisory_xact_lock(hashtext($1 || '/' || $2))",
            &w_id,
            &ns.path
        )
        .fetch_one(&mut *tx)
        .await?;
    }
    let clashing_script = sqlx::query_as::<_, Script<ScriptRunnableSettingsHandle>>(&format!(
        "SELECT {} FROM script WHERE path = $1 AND archived = false AND workspace_id = $2",
        windmill_common::scripts::SCRIPT_COLUMNS,
    ))
    .bind(&ns.path)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;
    struct ParentInfo {
        p_hashes: Vec<i64>,
        perms: serde_json::Value,
        p_path: String,
    }
    // When auto_parent is set, resolve parent_hash to the current head for this path
    // within the transaction. The advisory lock above ensures the second concurrent
    // request waits until the first commits, so this query sees the updated head.
    if ns.auto_parent.unwrap_or(false) {
        if let Some(ref cs) = clashing_script {
            ns.parent_hash = Some(cs.hash.clone());
        } else {
            ns.parent_hash = None;
        }
    }

    let parent_hashes_and_perms: Option<ParentInfo> = match (&ns.parent_hash, clashing_script) {
        (None, None) => Ok(None),
        (None, Some(s)) => Err(Error::BadRequest(format!(
            "Path conflict for {} with non-archived hash {}",
            &ns.path, &s.hash
        ))),
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

            // No-op detection (opt-in via `?skip_if_noop=true`, currently only
            // passed by the CLI): when the new script is effectively identical
            // to its parent (same path, content, lockfile, and metadata), skip
            // creating a new version entirely. Returning `None` for the
            // deployment-metadata handle also suppresses the follow-on git
            // sync / promotion callbacks — the whole point is that idempotent
            // CLI pushes must not produce phantom commits on the downstream
            // git repository.
            if skip_if_noop && is_noop_deploy_against_parent(&ns, &ps, &db).await? {
                tracing::info!(
                    workspace_id = %w_id,
                    path = %ns.path,
                    parent_hash = %p_hash.0,
                    "Skipping no-op script deploy (identical to parent)"
                );
                return Ok((p_hash.clone(), tx, None));
            }

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
            || ns.language == ScriptLang::Rlang
            || ns.language == ScriptLang::Powershell
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

    // Always generate a lock for dedicated worker scripts, even if a lock was provided.
    // The dependency job runs on the dedicated worker and triggers a restart so it picks
    // up the new script version (see result_processor.rs: is_dependency_job && is_dedicated_worker).
    let needs_lock_gen =
        (lock.is_none() && codebase.is_none()) || ns.dedicated_worker.is_some_and(|x| x);
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

    let (auto_kind, has_preprocessor) = if matches!(ns.kind, Some(ScriptKind::Preprocessor)) {
        (ns.auto_kind.clone(), ns.has_preprocessor)
    } else {
        match lang {
            ScriptLang::Bun | ScriptLang::Bunnative | ScriptLang::Deno | ScriptLang::Nativets => {
                let args = windmill_parser_ts::parse_deno_signature(&ns.content, true, true, None);
                match args {
                    Ok(args) => (
                        ns.auto_kind.clone().or(args.auto_kind),
                        args.has_preprocessor,
                    ),
                    Err(e) => {
                        tracing::warn!(
                            "Error parsing deno signature when deploying script {}: {:?}",
                            ns.path,
                            e
                        );
                        (ns.auto_kind.clone(), None)
                    }
                }
            }
            #[cfg(feature = "python")]
            ScriptLang::Python3 => {
                let args = windmill_parser_py::parse_python_signature(&ns.content, None, true);
                match args {
                    Ok(args) => (
                        ns.auto_kind.clone().or(args.auto_kind),
                        args.has_preprocessor,
                    ),
                    Err(e) => {
                        tracing::warn!(
                            "Error parsing python signature when deploying script {}: {:?}",
                            ns.path,
                            e
                        );
                        (ns.auto_kind.clone(), None)
                    }
                }
            }
            _ => (ns.auto_kind.clone(), ns.has_preprocessor),
        }
    };

    // Failure, Trigger, and Approval scripts are runnable entrypoints by
    // definition. They must never be marked `auto_kind = 'lib'`, or they
    // disappear from the flow error-handler / trigger / approval pickers
    // (which filter out lib scripts). Strip a stray `lib` here so a parser
    // misclassification — e.g. failing to detect `main` after a deno_ast
    // bump — cannot orphan these scripts in the UI.
    let auto_kind = if matches!(
        ns.kind,
        Some(ScriptKind::Failure) | Some(ScriptKind::Trigger) | Some(ScriptKind::Approval)
    ) && auto_kind.as_deref() == Some("lib")
    {
        None
    } else {
        auto_kind
    };

    let ci_test_refs =
        windmill_common::schema::parse_ci_test_annotation(&ns.content, &lang.as_comment_lit());
    // `pipeline` wins over `test` and any client-supplied auto_kind. The
    // bare `// pipeline` marker is the opt-in signal for pipeline
    // membership; parsed writes tell us what is produced (we don't record
    // them in auto_kind itself).
    let pipeline_annotations = parse_pipeline_annotations(&ns.content);
    // `// freshness` is parsed but enforcement is a not-yet-implemented
    // enterprise feature (skeleton in windmill_common::pipeline_advanced).
    // Surface a clear TODO at deploy rather than silently accepting an
    // annotation that does nothing.
    if pipeline_annotations.freshness.is_some() {
        tracing::warn!(
            "{}",
            windmill_common::pipeline_advanced::freshness_enforcement_todo()
        );
    }
    // `// materialize` materializes a `ducklake://<name>/<table>` target from a
    // DuckDB script. These two constraints hold for *both* modes: a non-DuckLake
    // target would otherwise deploy, register a producer in the asset graph, then
    // silently no-op at run time (`build_materialized_query` returns `Ok(None)`),
    // and a non-DuckDB script never reaches the executor that records state. The
    // managed-only checks (single trailing SELECT, no SQL args) come after — a
    // `manual` script owns its DDL and skips them.
    if let Some(m) = pipeline_annotations.materialize.as_ref() {
        if ns.language != ScriptLang::DuckDb {
            return Err(Error::BadRequest(format!(
                "`// materialize` is only supported for DuckDB scripts, not {}. Use the \
                 wmll.ducklake helpers to materialize from other languages.",
                ns.language.as_str()
            )));
        }
        if m.target_kind != windmill_parser::asset_parser::AssetKind::Ducklake {
            return Err(Error::BadRequest(
                "`// materialize` only supports a DuckLake target \
                 (`ducklake://<name>/<table>`); other asset kinds aren't materializable."
                    .to_string(),
            ));
        }
        if !m.target_path.contains('/') {
            return Err(Error::BadRequest(format!(
                "`// materialize` needs a table in the target: \
                 `ducklake://{0}/<table>` (got `ducklake://{0}`).",
                m.target_path
            )));
        }
        if !m.manual {
            if let Err(e) = windmill_parser::sql_materialize::classify_wrap(&ns.content) {
                return Err(Error::BadRequest(e.message()));
            }
            // SQL args are supported: managed materialize strips line comments
            // (including `-- $name (type)` declarations) when it wraps the SELECT,
            // but the executor parses the signature from the un-wrapped script, so
            // `$name` references in the SELECT stay bound at run time.
        }
        // Reconciliation strategies are mutually exclusive; surface a conflict
        // rather than silently dropping behavior the author may have intended.
        // Precedence must mirror the runtime (`duckdb_executor` strategy
        // derivation): scd2 (`history`) > append > merge (`key=`) > replace.
        if m.scd2 && m.append {
            tracing::warn!(
                "script {}: both `history`/`scd2` and `append` set on // materialize; history wins (SCD2), append ignored",
                ns.path
            );
        } else if m.unique_key.is_some() && m.append {
            tracing::warn!(
                "script {}: both `key=` and `append` set on // materialize; append wins (INSERT-only, no dedup)",
                ns.path
            );
        }
    }
    // `// macros` — this script is a workspace macro library: its body is
    // CREATE [OR REPLACE] MACRO statements plus plain setup, registered into
    // `macro_definition` and injected as TEMP macros into consumer jobs.
    // Pure shape checks happen here; the registry writes and the cross-lib
    // name-collision check run inside the deploy transaction below.
    let macro_lib_defs: Option<Vec<windmill_parser::duckdb_macros::ParsedMacro>> =
        if pipeline_annotations.macros {
            if ns.language != ScriptLang::DuckDb {
                return Err(Error::BadRequest(format!(
                    "`// macros` is only supported for DuckDB scripts, not {}",
                    ns.language.as_str()
                )));
            }
            if pipeline_annotations.materialize.is_some() {
                return Err(Error::BadRequest(
                    "`// macros` and `// materialize` are mutually exclusive — a macro \
                     library defines reusable macros, it does not produce an asset"
                        .to_string(),
                ));
            }
            let lib = windmill_parser::duckdb_macros::parse_macro_library(&ns.content)
                .map_err(Error::BadRequest)?;
            let macros: Vec<_> = lib
                .into_iter()
                .filter_map(|s| match s {
                    windmill_parser::duckdb_macros::LibStatement::Macro(m) => Some(m),
                    windmill_parser::duckdb_macros::LibStatement::Setup(_) => None,
                })
                .collect();
            if macros.is_empty() {
                return Err(Error::BadRequest(
                    "`// macros` library defines no macros".to_string(),
                ));
            }
            let all_names: std::collections::HashSet<String> =
                macros.iter().map(|m| m.name.clone()).collect();
            // DuckDB bind-checks a macro body at CREATE time, so a body may
            // only call same-file macros defined *earlier* — this also makes
            // file order a valid creation order for whole-lib (`// use`)
            // injection and rules out within-lib cycles.
            let mut defined: std::collections::HashSet<String> = Default::default();
            for m in &macros {
                if windmill_parser::duckdb_builtins::is_duckdb_builtin(&m.name) {
                    return Err(Error::BadRequest(format!(
                        "macro `{}` shadows a DuckDB built-in function; pick another name",
                        m.name
                    )));
                }
                if defined.contains(&m.name) {
                    return Err(Error::BadRequest(format!(
                        "macro `{}` is defined twice in this library",
                        m.name
                    )));
                }
                let called =
                    windmill_parser::duckdb_macros::detect_macro_calls(&m.body, &all_names);
                if let Some(fwd) = called
                    .iter()
                    .find(|c| !defined.contains(*c) && *c != &m.name)
                {
                    return Err(Error::BadRequest(format!(
                        "macro `{}` calls `{}`, which is defined later in the file; define \
                         `{}` first (DuckDB bind-checks macro bodies at creation)",
                        m.name, fwd, fwd
                    )));
                }
                defined.insert(m.name.clone());
            }
            Some(macros)
        } else {
            None
        };
    let in_pipeline = pipeline_annotations.in_pipeline;
    // `// trigger all` → AND join barrier (else OR, the default).
    let pipeline_join_all = !pipeline_annotations.join_mode.is_any();
    // Script-level `// debounce <dur>` default; a per-`// on debounce=`
    // overrides it (precedence resolved per edge below).
    let pipeline_debounce_default = pipeline_annotations.debounce_default;
    let pipeline_triggers = pipeline_annotations.triggers;
    // `// tag <name>` overrides the caller-supplied tag at deploy. Source
    // wins, matching the wipe-and-reinsert convention of other pipeline
    // annotations. Applied before the dep-job tag selection below (which
    // special-cases dedicated_worker / bunnative / `$args[`) so that path
    // sees the annotation-overridden value.
    if let Some(t) = pipeline_annotations.tag.clone() {
        ns.tag = Some(t);
    }
    // `// retry <count> [<delay>]` is persisted to `script_trigger` below (asset
    // edges only) and drives native subscriber retry in the cascade: a failed
    // subscriber re-runs as a `Script` job (not a flow step), so it stays
    // eligible for asset dispatch and can trigger its own downstream on success.
    // Asset presence is server-authoritative: re-parse the deployed content
    // (same parsers the frontend wasm wraps) and union with the client list.
    // The `asset` rows written below drive the asset-trigger cascade, so a
    // client deploying `assets: null` (e.g. broken wasm inference) must not
    // silently kill the producer side while `// on` subscribers stay wired.
    let effective_assets = crate::asset_inference::effective_script_assets(
        &ns.language,
        &ns.content,
        ns.assets.take(),
    );
    // Register the `// materialize` target as a write asset so the deployed
    // asset graph shows this script as the producer of the managed table — the
    // body's `SELECT` doesn't express the write (the runtime generates it), so
    // server-side inference wouldn't otherwise link it.
    let effective_assets = if let Some(m) = pipeline_annotations.materialize.as_ref() {
        let kind = windmill_common::assets::asset_kind_from_parser(m.target_kind);
        let mut a = effective_assets.unwrap_or_default();
        // Produced assets: the managed table, plus — for managed scd2 — the
        // `<dim>_current` companion view the runtime (re)creates each run.
        // Registering the view as a write asset lets `// on
        // ducklake://…/<dim>_current` subscribers be dispatched by the cascade
        // (which fans out from these deploy-time asset rows); without it a
        // subscriber on the view would silently never fire. Gated on `!manual`:
        // manual mode owns its own DDL and short-circuits before the scd2 codegen
        // (no view is created), so registering it there would be a false edge.
        let mut targets = vec![m.target_path.clone()];
        if m.scd2 && !m.manual {
            targets.push(format!("{}_current", m.target_path));
        }
        for path in targets {
            if !a.iter().any(|x| x.kind == kind && x.path == path) {
                a.push(windmill_common::assets::AssetWithAltAccessType {
                    path,
                    kind,
                    access_type: Some(windmill_common::assets::AssetUsageAccessType::W),
                    alt_access_type: None,
                    columns: None,
                });
            }
        }
        Some(a)
    } else {
        effective_assets
    };
    let auto_kind = if in_pipeline || macro_lib_defs.is_some() {
        // A macro library is a pipeline member (graph node) even without the
        // bare `// pipeline` marker.
        Some("pipeline".to_string())
    } else if ci_test_refs.is_some() {
        Some("test".to_string())
    } else {
        auto_kind
    };

    let runnable_settings_handle = windmill_common::runnable_settings::insert_rs(
        RunnableSettings {
            debouncing_settings: ns.debouncing_settings.insert_cached(&db).await?,
            concurrency_settings: ns.concurrency_settings.insert_cached(&db).await?,
            retry_settings: None,
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
         envs, concurrent_limit, concurrency_time_window_s, cache_ttl, \
         dedicated_worker, ws_error_handler_muted, priority, restart_unless_cancelled, \
         delete_after_use, delete_after_secs, timeout, concurrency_key, visible_to_runner_only, auto_kind, codebase, has_preprocessor, on_behalf_of_email, schema_validation, assets, debounce_key, debounce_delay_s, cache_ignore_s3_path, runnable_settings_handle, modules, labels) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::text::json, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38, $39, $40)",
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
        envs,
        guarded_concurrent_limit,
        guarded_concurrency_time_window_s,
        ns.cache_ttl,
        ns.dedicated_worker,
        ns.ws_error_handler_muted.unwrap_or(false),
        ns.priority,
        ns.restart_unless_cancelled,
        ns.delete_after_use,
        ns.delete_after_secs,
        ns.timeout,
        guarded_concurrency_key,
        ns.visible_to_runner_only,
        auto_kind.as_deref(),
        codebase,
        has_preprocessor.filter(|x: &bool| *x),
        windmill_common::resolve_on_behalf_of_email(
            ns.on_behalf_of_email.as_deref(),
            ns.preserve_on_behalf_of.unwrap_or(false),
            &authed,
        ),
        validate_schema,
        effective_assets
            .as_ref()
            .and_then(|a| serde_json::to_value(a).ok()),
        guarded_debounce_key,
        guarded_debounce_delay_s,
        ns.cache_ignore_s3_path,
        runnable_settings_handle,
        ns.modules.as_ref().and_then(|m| serde_json::to_value(m).ok()),
        ns.labels.as_deref() as Option<&[String]>
    )
    .execute(&mut *tx)
    .await?;

    // Update ci_test_reference table for test scripts
    // Delete by both new and old path to handle renames
    let old_path = parent_hashes_and_perms.as_ref().map(|x| x.p_path.as_str());
    sqlx::query!(
        "DELETE FROM ci_test_reference WHERE workspace_id = $1 AND (test_script_path = $2 OR test_script_path = $3)",
        &w_id,
        &ns.path,
        old_path.unwrap_or(&ns.path)
    )
    .execute(&mut *tx)
    .await?;

    if let Some(ref refs) = ci_test_refs {
        for item in refs {
            sqlx::query!(
                "INSERT INTO ci_test_reference (workspace_id, test_script_path, test_script_hash, tested_item_path, tested_item_kind) \
                 VALUES ($1, $2, $3, $4, $5)",
                &w_id,
                &ns.path,
                &hash.0,
                &item.path,
                &item.kind
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    // DuckDB workspace macros: wipe-and-reinsert this script's registry rows
    // (as provider) and call-site edges (as consumer), by new+old path so
    // renames are handled — mirrors the ci_test_reference block above.
    sqlx::query!(
        "DELETE FROM macro_definition WHERE workspace_id = $1 AND (provider_path = $2 OR provider_path = $3)",
        &w_id,
        &ns.path,
        old_path.unwrap_or(&ns.path)
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM macro_usage WHERE workspace_id = $1 AND (consumer_path = $2 OR consumer_path = $3)",
        &w_id,
        &ns.path,
        old_path.unwrap_or(&ns.path)
    )
    .execute(&mut *tx)
    .await?;

    if let Some(ref macros) = macro_lib_defs {
        // Macro names are workspace-unique (they're injected unqualified into
        // consumers). This lib's own rows — old and new path — were deleted
        // just above, so any remaining match is a genuine cross-lib clash.
        let names: Vec<String> = macros.iter().map(|m| m.name.clone()).collect();
        if let Some(clash) = sqlx::query!(
            "SELECT name, provider_path FROM macro_definition WHERE workspace_id = $1 AND name = ANY($2) LIMIT 1",
            &w_id,
            &names[..]
        )
        .fetch_optional(&mut *tx)
        .await?
        {
            return Err(Error::BadRequest(format!(
                "macro `{}` is already defined by `{}`; macro names are workspace-unique",
                clash.name, clash.provider_path
            )));
        }
        for m in macros {
            sqlx::query!(
                "INSERT INTO macro_definition (workspace_id, name, provider_path, params, body, is_table_macro) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &w_id,
                &m.name,
                &ns.path,
                &m.params,
                &m.body,
                m.is_table
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    if ns.language == ScriptLang::DuckDb {
        // Record this script's macro-call edges for the asset graph (the
        // worker re-detects calls live at job time, so these are display
        // metadata only). A library's edges point at *other* libs' macros.
        let registry_names: Vec<String> = sqlx::query_scalar!(
            "SELECT name FROM macro_definition WHERE workspace_id = $1 AND provider_path != $2",
            &w_id,
            &ns.path
        )
        .fetch_all(&mut *tx)
        .await?;
        let names_set: std::collections::HashSet<String> = registry_names.into_iter().collect();
        let mut called: Vec<String> =
            windmill_parser::duckdb_macros::detect_macro_calls(&ns.content, &names_set)
                .into_iter()
                .collect();
        called.sort();
        for name in called {
            sqlx::query!(
                "INSERT INTO macro_usage (workspace_id, consumer_path, macro_name) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                &w_id,
                &ns.path,
                &name
            )
            .execute(&mut *tx)
            .await?;
        }
        // `// use` targets should be deployed macro libraries — warn only:
        // git-sync deploys scripts in arbitrary order, so the consumer landing
        // first must not hard-fail. The runtime is the enforcement point.
        for lib in &pipeline_annotations.use_libs {
            let exists = sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM macro_definition WHERE workspace_id = $1 AND provider_path = $2) AS "e!""#,
                &w_id,
                lib
            )
            .fetch_one(&mut *tx)
            .await?;
            if !exists {
                tracing::warn!(
                    "script {}: `// use {}` does not (yet) match a deployed macro library",
                    ns.path,
                    lib
                );
            }
        }
    } else if !pipeline_annotations.use_libs.is_empty() {
        tracing::warn!(
            "script {}: `// use` is only honored on DuckDB scripts; annotation ignored",
            ns.path
        );
    }

    if let Some(ref macros) = macro_lib_defs {
        // Refresh every other DuckDB script's edges for this lib's names (a
        // redeploy may add/remove macros; late-bound runtime picks changes up
        // regardless — this only keeps the deployed graph current). The rescan
        // covers ALL live DuckDB scripts, not just pipeline members: their
        // edges were recorded at their own deploy and must survive the wipe.
        let names: Vec<String> = macros.iter().map(|m| m.name.clone()).collect();
        sqlx::query!(
            "DELETE FROM macro_usage WHERE workspace_id = $1 AND macro_name = ANY($2) AND consumer_path != $3",
            &w_id,
            &names[..],
            &ns.path
        )
        .execute(&mut *tx)
        .await?;
        let names_set: std::collections::HashSet<String> = names.into_iter().collect();
        let members = sqlx::query!(
            r#"SELECT DISTINCT ON (path) path AS "path!", content AS "content!"
               FROM script
               WHERE workspace_id = $1
                 AND language = 'duckdb'::script_lang
                 AND archived = false
                 AND deleted = false
                 AND path != $2
               ORDER BY path, created_at DESC"#,
            &w_id,
            &ns.path
        )
        .fetch_all(&mut *tx)
        .await?;
        for member in members {
            let mut called: Vec<String> =
                windmill_parser::duckdb_macros::detect_macro_calls(&member.content, &names_set)
                    .into_iter()
                    .collect();
            called.sort();
            for name in called {
                sqlx::query!(
                    "INSERT INTO macro_usage (workspace_id, consumer_path, macro_name) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                    &w_id,
                    &member.path,
                    &name
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    let p_path_opt = parent_hashes_and_perms.as_ref().map(|x| x.p_path.clone());
    if let Some(ref p_path) = p_path_opt {
        if !skip_draft_deletion {
            // Canonical: on deploy only wipe the deployer's own draft (plus the legacy
            // NULL-email row). Teammates' drafts are independent — they stay and fire the
            // StaleDraftModal on the teammate's next reload rather than vanishing silently.
            sqlx::query!(
                "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'script' \
                 AND (email = $3 OR email IS NULL)",
                p_path,
                &w_id,
                &authed.email,
            )
            .execute(&mut *tx)
            .await?;
        }

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

        // Update ci_test_reference when a tested item is renamed
        sqlx::query!(
            "UPDATE ci_test_reference SET tested_item_path = $1 WHERE tested_item_path = $2 AND workspace_id = $3 AND tested_item_kind = 'script'",
            &ns.path,
            &p_path,
            &w_id
        )
        .execute(&mut *tx)
        .await?;

        if p_path != &ns.path {
            windmill_common::triggers::update_triggers_script_path(
                &mut tx, &ns.path, p_path, &w_id, false,
            )
            .await
            .map_err(|e| {
                error::Error::internal_err(format!(
                    "Error updating triggers due to runnable path change: {e:#}"
                ))
            })?;
        }

        for schedule in schedulables {
            clear_schedule(&mut tx, &schedule.path, &w_id).await?;

            if schedule.enabled {
                tx = push_scheduled_job(&db, tx, &schedule, None, None).await?;
            }
        }
    } else if !skip_draft_deletion {
        // See the matching branch above — only wipe the deployer's own
        // draft (plus the legacy NULL-email row).
        sqlx::query!(
            "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'script' \
             AND (email = $3 OR email IS NULL)",
            ns.path,
            &w_id,
            &authed.email,
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

    // Clear + reinsert this script's producer rows at script_path (== ns.path),
    // invalidating the producer-writes cache once iff the write-producer set
    // changed (see replace_static_asset_usage).
    replace_static_asset_usage(
        &mut tx,
        &w_id,
        &script_path,
        effective_assets.as_deref().unwrap_or(&[]),
    )
    .await?;

    // Pipeline trigger edges: wipe-and-reinsert per deploy so removing an
    // `// on ...` annotation drops the edge. Only Asset / Schedule produce
    // a row — native trigger marker annotations (`// on kafka`, etc.) are
    // discovered by the graph endpoint directly from the per-kind trigger
    // tables, so `trigger_spec_to_row` returns None for those.
    clear_script_triggers(&mut *tx, &w_id, &ns.path, AssetUsageKind::Script).await?;
    // On rename, also drop the OLD path's trigger rows. clear is keyed by
    // path (no by-hash variant), and only `ns.path` is wiped above — without
    // this, stale `// on` edges for the old path keep matching producers and
    // would trigger a script later recreated at that path even if it has no
    // annotation (P1). (Producer/asset rows for the old path are already
    // cleared via clear_static_asset_usage_by_script_hash on the parent.)
    if let Some(ref old) = p_path_opt {
        if old != &ns.path {
            clear_script_triggers(&mut *tx, &w_id, old, AssetUsageKind::Script).await?;
        }
    }
    for spec in &pipeline_triggers {
        let Some((trigger_kind, trigger_ref)) = trigger_spec_to_row(spec) else {
            continue;
        };
        // Effective debounce for this edge: per-`// on debounce=` wins,
        // else the script-level `// debounce` default. Debounce only
        // applies to asset-cascade edges; other trigger kinds get none.
        let debounce_s = match spec {
            TriggerSpec::Asset { debounce: Some(d), .. } => parse_duration_secs(d),
            TriggerSpec::Asset { .. } => pipeline_debounce_default
                .as_deref()
                .and_then(parse_duration_secs),
            _ => None,
        };
        // `// retry` applies only to the asset cascade: a failed subscriber is
        // re-run natively (a `Script` job, not a flow step), so it can still
        // trigger its own downstream on success — see asset_dispatch.
        let (retry_count, retry_delay_s) = match spec {
            TriggerSpec::Asset { .. } => (
                pipeline_annotations
                    .retry
                    .as_ref()
                    // `// retry <count>` count is u32; saturate the narrowing to i16.
                    .map(|r| r.count.min(i16::MAX as u32) as i16),
                pipeline_annotations
                    .retry
                    .as_ref()
                    .and_then(|r| r.delay.as_deref())
                    .and_then(parse_duration_secs),
            ),
            _ => (None, None),
        };
        insert_script_trigger(
            &mut *tx,
            &w_id,
            AssetUsageKind::Script,
            &ns.path,
            trigger_kind,
            &trigger_ref,
            pipeline_join_all,
            debounce_s,
            retry_count,
            retry_delay_s,
        )
        .await?;
    }

    // Schedule annotations (`// on schedule`) are marker-only — the binding
    // lives on the schedule row's own `script_path` field, which the user
    // creates separately via the schedule editor. No script-create-time
    // reconciliation is needed (and there are no "managed" schedules to
    // upsert/delete anymore).

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

    let hub_base_url = (**HUB_BASE_URL.load()).clone();

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

// Canonical: fields inlined rather than `#[serde(flatten)]` from
// `WithStarredInfoQuery` / `WithDraftQuery`. axum's `serde_urlencoded` extractor
// drops type info through flatten, so `?get_draft=true` arrives as a String and
// fails the inner bool deserializer. Inlining lets the bool adapter see it directly.
#[derive(Deserialize)]
struct GetScriptByPathQuery {
    with_starred_info: Option<bool>,
    #[serde(default)]
    get_draft: bool,
}

#[axum::debug_handler]
async fn get_script_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(query): Query<GetScriptByPathQuery>,
) -> JsonResult<WithDraftOverlay> {
    let path = path.to_path();
    check_scopes(&authed, || format!("scripts:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let script_o = if query.with_starred_info.unwrap_or(false) {
        sqlx::query_as::<_, ScriptWithStarred<ScriptRunnableSettingsHandle>>(
            "SELECT s.*, favorite.path IS NOT NULL as starred,
                folder_labels(s.workspace_id, s.path) as inherited_labels
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
        .bind(&w_id)
        .bind(&authed.username)
        .fetch_optional(&mut *tx)
        .await?
    } else {
        sqlx::query_as::<_, ScriptWithStarred<ScriptRunnableSettingsHandle>>(
            &format!(
                "SELECT {}, NULL as starred, folder_labels(workspace_id, path) as inherited_labels FROM script WHERE path = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
                windmill_common::scripts::SCRIPT_COLUMNS,
            ),
        )
        .bind(path)
        .bind(&w_id)
        .fetch_optional(&mut *tx)
        .await?
    };
    tx.commit().await?;

    // Canonical: with no deployed row and `get_draft` set, fall back to the draft
    // table so editing a never-deployed draft works like a deployed reload.
    let deployed = match script_o {
        Some(script_o) => Some(
            windmill_common::scripts::prefetch_cached_script_with_starred(script_o, &db).await?,
        ),
        None => None,
    };
    let overlay = overlay_or_draft_only(
        &db,
        &w_id,
        &authed.email,
        UserDraftItemKind::Script,
        path,
        query.get_draft,
        deployed,
        || windmill_common::error::Error::NotFound(format!("Script not found at path {path}")),
    )
    .await?;

    Ok(Json(overlay))
}

async fn list_tokens(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Vec<TruncatedTokenWithEmail>> {
    let path = path.to_path();
    check_scopes(&authed, || format!("scripts:read:{}", path))?;
    list_tokens_internal(&db, &w_id, &path, false).await
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
        "SELECT s.hash as hash, dm.deployment_msg as deployment_msg, s.created_at as created_at
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
            created_at: Some(row.created_at),
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
        "SELECT s.hash as hash, dm.deployment_msg as deployment_msg, s.created_at as created_at
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
            deployment_msg: row.deployment_msg,
            created_at: Some(row.created_at),
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
    // If provided, load content from raw_script_temp table using this hash instead of deployed script.
    // Used by CLI lock generation to resolve imports from not-yet-deployed scripts.
    temp_script_hash: Option<String>,
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

    /// Fallback freshness window (seconds) for [`RAW_SCRIPT_LATEST_HASH_CACHE`].
    /// Primary invalidation is event-driven: deploying a script writes a
    /// `notify_runnable_version_change` row, and the server's polling-events handler
    /// evicts the entry across all replicas (see `main.rs`). This TTL only bounds
    /// staleness if that event is missed. Defaults to 60s (matches
    /// `DEPLOYED_SCRIPT_HASH_CACHE`). Override with `RAW_SCRIPT_CACHE_TTL_SECONDS`.
    static ref RAW_SCRIPT_CACHE_TTL_S: i64 = std::env::var("RAW_SCRIPT_CACHE_TTL_SECONDS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|s| *s >= 0)
        .unwrap_or(60);
}

lazy_static::lazy_static! {
    // Imported-script content, keyed by
    // `{ws}:{path}:{importer_cache_key}[:unpinned]:{latest_hash}`. Including the
    // imported script's own latest hash makes each entry immutable, so no
    // per-entry TTL is needed; staleness is bounded by RAW_SCRIPT_LATEST_HASH_CACHE.
    pub static ref RAW_SCRIPT_CACHE: Cache<String, String> = Cache::new(1000);
    // `{ws}:{path}` (bare path) -> (latest non-archived hash, unix_ts cached).
    // Resolving the imported script's own hash and keying content by it is what
    // fixes relative-import staleness for deployed scripts, whose importer hash
    // never moves (see #6769). Evicted on deploy by the `notify_runnable_version_change`
    // handler in main.rs (cross-replica, within a poll interval); RAW_SCRIPT_CACHE_TTL_S
    // is a fallback bound.
    pub static ref RAW_SCRIPT_LATEST_HASH_CACHE: Cache<String, (i64, i64)> = Cache::new(1000);
    pub static ref CACHE_FOLDERS_PATH: Cache<String, i64> = Cache::new(1000);

}

/// Records a [`RAW_SCRIPT_CACHE`] lookup outcome (`hit` / `expired` / `miss`) to
/// the `raw_script_cache_total` counter when the prometheus feature is enabled.
#[cfg(feature = "prometheus")]
fn record_raw_script_cache(result: &str) {
    if let Some(c) = RAW_SCRIPT_CACHE_METRIC.as_ref() {
        c.with_label_values(&[result]).inc();
    }
}

#[cfg(not(feature = "prometheus"))]
fn record_raw_script_cache(_result: &str) {}

#[cfg(feature = "prometheus")]
lazy_static::lazy_static! {
    /// Raw relative-import cache lookups, labeled by `result` (hit/expired/miss).
    static ref RAW_SCRIPT_CACHE_METRIC: Option<prometheus::IntCounterVec> =
        if windmill_common::METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(prometheus::register_int_counter_vec!(
                "raw_script_cache_total",
                "Raw script relative-import cache lookups by result (hit/expired/miss)",
                &["result"]
            ).unwrap())
        } else {
            None
        };
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

    // If temp_script_hash is provided, try loading from temp storage first.
    // This is used by CLI lock generation to resolve imports from not-yet-deployed scripts.
    // Falls back to the normal deployed script lookup if not found in temp storage.
    if let Some(hash) = query.temp_script_hash {
        if let Ok(content) = windmill_common::cache::raw_script_temp::load(hash, &db).await {
            return Ok(content);
        }
    }

    // Validate + strip the language extension up front so cache keys use the bare
    // script path. This matches the `notify_runnable_version_change` event payload
    // (which carries the bare path), so a deploy can evict RAW_SCRIPT_LATEST_HASH_CACHE
    // by key from the polling-events handler in the server binary.
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

    // Content cache is keyed by the IMPORTED script's own latest hash, not by the
    // importer's runnable hash (`query.cache_key`). The importer hash never moves
    // when only an imported script's content changes (relock is in-place — see
    // #6769), so keying solely on it served stale content indefinitely. The
    // importer + unpin dimensions are kept to preserve per-runnable authorization
    // scoping (a content-cache hit skips the authed RLS query, so an entry must
    // stay scoped to the runnable that fetched it); the imported latest hash is
    // appended for content correctness.
    let cache_path_base = query
        .cache_key
        .as_ref()
        .map(|x| format!("{w_id}:{path}:{x}{}", if unpin { ":unpinned" } else { "" }));

    // Resolve the imported script's latest hash from RAW_SCRIPT_LATEST_HASH_CACHE
    // (keyed by the bare path so the deploy event can evict it). A fresh entry
    // serves from the immutable content cache with no DB hit; a stale/absent entry
    // falls through to the query below, which refreshes both caches.
    let hash_cache_key = format!("{w_id}:{path}");
    let (fresh_hash, had_stale_hash) = match RAW_SCRIPT_LATEST_HASH_CACHE.get(&hash_cache_key) {
        Some((hash, cached_at))
            if chrono::Utc::now().timestamp() - cached_at <= *RAW_SCRIPT_CACHE_TTL_S =>
        {
            (Some(hash), false)
        }
        Some(_) => (None, true),
        None => (None, false),
    };

    if let (Some(base), Some(latest_hash)) = (cache_path_base.as_ref(), fresh_hash) {
        let content_key = format!("{base}:{latest_hash}");
        if let Some(cached_content) = RAW_SCRIPT_CACHE.get(&content_key) {
            if *DEBUG_RAW_SCRIPT_ENDPOINTS {
                tracing::warn!("Raw script by path request: {path} (cached, key={content_key})");
            }
            record_raw_script_cache("hit");
            return Ok(cached_content);
        }
    }
    if cache_path_base.is_some() {
        record_raw_script_cache(if had_stale_hash { "expired" } else { "miss" });
    }

    if *DEBUG_RAW_SCRIPT_ENDPOINTS {
        tracing::warn!("Raw script by path request: {} (not cached)", path);
    }

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

    // Fetch the latest non-archived row's hash AND content in one query: the hash
    // keys the (immutable) content cache and refreshes RAW_SCRIPT_LATEST_HASH_CACHE.
    let row_o = sqlx::query!(
        "SELECT hash, content FROM script WHERE path = $1 AND workspace_id = $2 AND archived = false ORDER BY created_at DESC LIMIT 1",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .warn_after_seconds(5)
    .await?;
    tx.commit().await?;
    let (db_hash, content_o) = match row_o {
        Some(r) => (Some(r.hash), Some(r.content)),
        None => (None, None),
    };
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

    // content_o was Some, so db_hash is Some too (same row). Refresh the latest-hash
    // cache and store the content under the hash-qualified key.
    if let Some(db_hash) = db_hash {
        RAW_SCRIPT_LATEST_HASH_CACHE
            .insert(hash_cache_key, (db_hash, chrono::Utc::now().timestamp()));
        if let Some(base) = cache_path_base {
            RAW_SCRIPT_CACHE.insert(format!("{base}:{db_hash}"), content.clone());
        }
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
        "SELECT EXISTS(SELECT 1 FROM script WHERE path = $1 AND workspace_id = $2 AND archived = false ORDER BY created_at DESC LIMIT 1)",
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
            "SELECT s.*, favorite.path IS NOT NULL as starred,
                folder_labels(s.workspace_id, s.path) as inherited_labels
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
        sqlx::query_as::<_, ScriptWithStarred<ScriptRunnableSettingsHandle>>(&format!(
            "SELECT {}, NULL as starred, folder_labels(workspace_id, path) as inherited_labels FROM script WHERE hash = $1 AND workspace_id = $2",
            windmill_common::scripts::SCRIPT_COLUMNS,
        ))
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
    // Pipeline event hygiene: an archived script must not be triggered by
    // anything. Wipe declared `// on ...` edges (asset-event subscribers
    // look these up).
    clear_script_triggers(&mut *tx, &w_id, path, AssetUsageKind::Script).await?;
    clear_macro_registry(&mut *tx, &w_id, path).await?;

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
    // Pipeline event hygiene: archived scripts must not be triggered by
    // anything. Wipe declared `// on ...` edges.
    clear_script_triggers(&mut *tx, &w_id, &script.path, AssetUsageKind::Script).await?;
    clear_macro_registry(&mut *tx, &w_id, &script.path).await?;

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
    // Pipeline event hygiene: a deleted script must not be triggered by
    // anything. Wipe declared `// on ...` edges. Idempotent — safe even if
    // the script was never a pipeline member.
    clear_script_triggers(&mut *tx, &w_id, &script.path, AssetUsageKind::Script).await?;
    clear_macro_registry(&mut *tx, &w_id, &script.path).await?;

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

    let mut tx = user_db.begin(&authed).await?;

    require_admin(authed.is_admin, &authed.username)?;

    // Capture all script versions and drafts for trashbin before deleting
    let trash_scripts: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM script t WHERE path = $1 AND workspace_id = $2",
    )
    .bind(path)
    .bind(&w_id)
    .fetch_all(&mut *tx)
    .await?;

    let trash_drafts: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM draft t WHERE path = $1 AND workspace_id = $2 AND typ = 'script'",
    )
    .bind(path)
    .bind(&w_id)
    .fetch_all(&mut *tx)
    .await?;

    let script = sqlx::query_scalar!(
        "DELETE FROM script WHERE path = $1 AND workspace_id = $2 RETURNING path",
        path,
        w_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::internal_err(format!("deleting script by path {w_id}: {e:#}")))?;

    if !trash_scripts.is_empty() {
        let mut trash_data = serde_json::json!({"scripts": trash_scripts});
        if !trash_drafts.is_empty() {
            trash_data["drafts"] = serde_json::Value::Array(trash_drafts);
        }
        windmill_common::trashbin::move_to_trash(
            &mut *tx,
            &w_id,
            "script",
            path,
            trash_data,
            &authed.username,
        )
        .await?;
    }

    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'script'",
        path,
        w_id
    )
    .execute(&mut *tx)
    .await?;

    // Pipeline event hygiene: a deleted script must not be triggered by
    // anything. Wipe declared `// on ...` edges. Idempotent — safe even if
    // the script was never a pipeline member.
    clear_script_triggers(&mut *tx, &w_id, path, AssetUsageKind::Script).await?;
    clear_macro_registry(&mut *tx, &w_id, path).await?;

    if !query.keep_captures.unwrap_or(false) {
        sqlx::query!(
            "DELETE FROM capture_config WHERE path = $1 AND workspace_id = $2 AND is_flow IS FALSE",
            path,
            w_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "DELETE FROM capture WHERE path = $1 AND workspace_id = $2 AND is_flow IS FALSE",
            path,
            w_id
        )
        .execute(&mut *tx)
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

    let mut tx = db.begin().await?;

    // Capture scripts for trashbin per path before bulk delete
    for path in &request.paths {
        let trash_scripts: Vec<serde_json::Value> = sqlx::query_scalar(
            "SELECT to_jsonb(t) FROM script t WHERE path = $1 AND workspace_id = $2",
        )
        .bind(path)
        .bind(&w_id)
        .fetch_all(&mut *tx)
        .await?;

        if !trash_scripts.is_empty() {
            let trash_data = serde_json::json!({"scripts": trash_scripts});
            windmill_common::trashbin::move_to_trash(
                &mut *tx,
                &w_id,
                "script",
                path,
                trash_data,
                &authed.username,
            )
            .await?;
        }
    }

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

    for path in &deleted_paths {
        clear_macro_registry(&mut *tx, &w_id, path).await?;
    }

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

/// Deployed-macro hygiene for an archived/deleted script: it must neither
/// provide macros to the workspace registry nor keep stale call-site edges.
/// Usage edges pointing at the script's own macros go too (names are
/// workspace-unique, so those edges can only reference this provider).
async fn clear_macro_registry(db: &mut sqlx::PgConnection, w_id: &str, path: &str) -> Result<()> {
    sqlx::query!(
        "DELETE FROM macro_usage WHERE workspace_id = $1 AND (consumer_path = $2 \
         OR macro_name IN (SELECT name FROM macro_definition WHERE workspace_id = $1 AND provider_path = $2))",
        w_id,
        path
    )
    .execute(&mut *db)
    .await?;
    sqlx::query!(
        "DELETE FROM macro_definition WHERE workspace_id = $1 AND provider_path = $2",
        w_id,
        path
    )
    .execute(&mut *db)
    .await?;
    Ok(())
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

#[derive(Serialize)]
struct DedicatedScriptDeps {
    path: String,
    language: ScriptLang,
    workspace_dep_names: Vec<String>,
}

async fn list_dedicated_with_deps(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<DedicatedScriptDeps>> {
    let mut tx = user_db.begin(&authed).await?;

    let rows = sqlx::query!(
        "SELECT DISTINCT ON (path) path, language AS \"language: ScriptLang\", content FROM script
         WHERE workspace_id = $1
           AND archived = false
           AND dedicated_worker = true
           AND language = ANY($2::SCRIPT_LANG[])
         ORDER BY path, created_at DESC",
        &w_id,
        &[
            ScriptLang::Python3,
            ScriptLang::Bun,
            ScriptLang::Bunnative,
            ScriptLang::Deno,
        ] as &[ScriptLang],
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let result = rows
        .into_iter()
        .map(|row| {
            let dep_names =
                windmill_common::scripts::extract_workspace_dependencies_annotated_refs(
                    &row.language,
                    &row.content,
                    &row.path,
                )
                .map(|refs| refs.external)
                .unwrap_or_default();
            DedicatedScriptDeps {
                path: row.path,
                language: row.language,
                workspace_dep_names: dep_names,
            }
        })
        .collect();

    Ok(Json(result))
}

// ============================================================================
// Temporary Raw Script Storage for CLI Lock Generation
// ============================================================================

/// Store raw script content temporarily for CLI lock generation.
async fn store_raw_script_temp(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(content): Json<String>,
) -> Result<Json<String>> {
    check_scopes(&authed, || "scripts:write".to_string())?;

    let hash = windmill_common::cache::raw_script_temp::compute_hash(&w_id, &content);

    // Store to DB
    sqlx::query!(
        "INSERT INTO raw_script_temp (workspace_id, hash, content, created_at)
         VALUES ($1, $2, $3, NOW())
         ON CONFLICT (workspace_id, hash) DO UPDATE SET created_at = NOW()",
        &w_id,
        &hash,
        &content
    )
    .execute(&db)
    .await?;

    // Clean up old entries (1 week TTL)
    sqlx::query!("DELETE FROM raw_script_temp WHERE created_at < NOW() - INTERVAL '1 week'")
        .execute(&db)
        .await?;

    Ok(Json(hash))
}

/// Compare local script content hashes with deployed versions.
/// Receives a map of path → SHA256(content), returns paths where the hash
/// differs from the deployed script (or the script doesn't exist on remote).
/// Hash comparison is done entirely in Postgres to avoid transferring content.
#[derive(Deserialize)]
struct WorkspaceDepDiff {
    path: String,
    language: ScriptLang,
    name: Option<String>,
    hash: String,
}

#[derive(Deserialize)]
struct DiffRequest {
    scripts: std::collections::HashMap<String, String>,
    #[serde(default)]
    workspace_deps: Vec<WorkspaceDepDiff>,
}

async fn diff_raw_scripts_with_deployed(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<DiffRequest>,
) -> Result<Json<Vec<String>>> {
    check_scopes(&authed, || "scripts:read".to_string())?;

    let mut matching_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut all_paths: Vec<String> = Vec::new();

    // --- Scripts ---
    if !req.scripts.is_empty() {
        let paths: Vec<String> = req.scripts.keys().cloned().collect();
        let hashes: Vec<String> = paths.iter().map(|p| req.scripts[p].clone()).collect();

        let matching: Vec<String> = sqlx::query_scalar(
            "SELECT local.path FROM \
             unnest($1::text[], $2::text[]) AS local(path, hash) \
             INNER JOIN LATERAL ( \
               SELECT encode(sha256(convert_to(s.content, 'UTF8')), 'hex') AS deployed_hash \
               FROM script s \
               WHERE s.path = local.path AND s.workspace_id = $3 AND s.archived = false \
               ORDER BY s.created_at DESC LIMIT 1 \
             ) deployed ON deployed.deployed_hash = local.hash",
        )
        .bind(&paths)
        .bind(&hashes)
        .bind(&w_id)
        .fetch_all(&db)
        .await?;

        matching_set.extend(matching);
        all_paths.extend(paths);
    }

    // --- Workspace dependencies ---
    for dep in &req.workspace_deps {
        let matching: Option<String> = sqlx::query_scalar(
            "SELECT $1::text \
             WHERE EXISTS ( \
               SELECT 1 FROM workspace_dependencies wd \
               WHERE wd.workspace_id = $2 AND wd.archived = false \
                 AND wd.language = $3::SCRIPT_LANG \
                 AND wd.name IS NOT DISTINCT FROM $4 \
                 AND encode(sha256(convert_to(wd.content, 'UTF8')), 'hex') = $5 \
             )",
        )
        .bind(&dep.path)
        .bind(&w_id)
        .bind(dep.language.as_str())
        .bind(&dep.name)
        .bind(&dep.hash)
        .fetch_optional(&db)
        .await?;

        if let Some(path) = matching {
            matching_set.insert(path);
        }
        all_paths.push(dep.path.clone());
    }

    let mismatched: Vec<String> = all_paths
        .into_iter()
        .filter(|p| !matching_set.contains(p))
        .collect();

    Ok(Json(mismatched))
}

#[derive(Serialize, FromRow)]
struct CiTestResult {
    test_script_path: String,
    job_id: Option<sqlx::types::Uuid>,
    status: Option<String>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(FromRow)]
struct CiTestResultWithPattern {
    test_script_path: String,
    tested_item_path: String,
    job_id: Option<sqlx::types::Uuid>,
    status: Option<String>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<CiTestResultWithPattern> for CiTestResult {
    fn from(r: CiTestResultWithPattern) -> Self {
        CiTestResult {
            test_script_path: r.test_script_path,
            job_id: r.job_id,
            status: r.status,
            started_at: r.started_at,
        }
    }
}

async fn get_ci_test_results(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, kind, path)): Path<(String, String, StripPath)>,
) -> JsonResult<Vec<CiTestResult>> {
    let path = path.to_path();
    let expected_trigger = format!("{}/{}", kind, path);

    // Exact matches — uses the primary ci_test_reference index.
    // Job lookup is scoped by trigger so multi-target tests report the right job per target.
    let exact = sqlx::query_as!(
        CiTestResult,
        r#"SELECT
            ctr.test_script_path,
            j.id as "job_id?",
            COALESCE(jc.status::text, CASE WHEN j.id IS NOT NULL THEN 'running' ELSE NULL END) as "status?",
            j.created_at as "started_at?"
        FROM ci_test_reference ctr
        LEFT JOIN LATERAL (
            SELECT id, created_at FROM v2_job
            WHERE workspace_id = $1
              AND runnable_path = ctr.test_script_path
              AND trigger_kind = 'ci_test'
              AND trigger = $4
            ORDER BY created_at DESC
            LIMIT 1
        ) j ON true
        LEFT JOIN v2_job_completed jc ON jc.id = j.id
        WHERE ctr.workspace_id = $1
          AND ctr.tested_item_path = $2
          AND ctr.tested_item_kind = $3
          AND NOT ctr.has_wildcard"#,
        &w_id,
        path,
        &kind,
        &expected_trigger
    )
    .fetch_all(&db)
    .await?;

    // Wildcard candidates — partial index; filter patterns in Rust.
    let wildcard = sqlx::query_as!(
        CiTestResultWithPattern,
        r#"SELECT
            ctr.test_script_path,
            ctr.tested_item_path,
            j.id as "job_id?",
            COALESCE(jc.status::text, CASE WHEN j.id IS NOT NULL THEN 'running' ELSE NULL END) as "status?",
            j.created_at as "started_at?"
        FROM ci_test_reference ctr
        LEFT JOIN LATERAL (
            SELECT id, created_at FROM v2_job
            WHERE workspace_id = $1
              AND runnable_path = ctr.test_script_path
              AND trigger_kind = 'ci_test'
              AND trigger = $3
            ORDER BY created_at DESC
            LIMIT 1
        ) j ON true
        LEFT JOIN v2_job_completed jc ON jc.id = j.id
        WHERE ctr.workspace_id = $1
          AND ctr.tested_item_kind = $2
          AND ctr.has_wildcard"#,
        &w_id,
        &kind,
        &expected_trigger
    )
    .fetch_all(&db)
    .await?;

    let mut results = exact;
    results.extend(
        wildcard
            .into_iter()
            .filter(|r| windmill_common::schema::ci_test_path_matches(path, &r.tested_item_path))
            .map(CiTestResult::from),
    );

    Ok(Json(results))
}

#[derive(Deserialize)]
struct CiTestBatchItem {
    path: String,
    kind: String,
}

#[derive(Deserialize)]
struct CiTestBatchRequest {
    items: Vec<CiTestBatchItem>,
}

async fn get_ci_test_results_batch(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<CiTestBatchRequest>,
) -> JsonResult<HashMap<String, Vec<CiTestResult>>> {
    if req.items.len() > 200 {
        return Err(Error::BadRequest(
            "too many items in batch request (max 200)".to_string(),
        ));
    }

    // Initialize every requested key with an empty vec so the response shape matches input.
    let mut result_map: HashMap<String, Vec<CiTestResult>> = req
        .items
        .iter()
        .map(|it| (format!("{}:{}", it.kind, it.path), Vec::new()))
        .collect();

    // Group paths by kind so we can do one exact query + one wildcard query per distinct kind.
    // Dedupe via HashSet so duplicate items in the input don't push matches multiple times.
    let mut paths_by_kind: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    for item in &req.items {
        paths_by_kind
            .entry(item.kind.clone())
            .or_default()
            .insert(item.path.clone());
    }

    for (kind, paths) in &paths_by_kind {
        let paths_vec: Vec<String> = paths.iter().cloned().collect();
        // Exact matches for all paths of this kind in one query.
        // Trigger scoping ensures the job returned matches the requested target.
        let exact = sqlx::query_as!(
            CiTestResultWithPattern,
            r#"SELECT
                ctr.test_script_path,
                ctr.tested_item_path,
                j.id as "job_id?",
                COALESCE(jc.status::text, CASE WHEN j.id IS NOT NULL THEN 'running' ELSE NULL END) as "status?",
                j.created_at as "started_at?"
            FROM ci_test_reference ctr
            LEFT JOIN LATERAL (
                SELECT id, created_at FROM v2_job
                WHERE workspace_id = $1
                  AND runnable_path = ctr.test_script_path
                  AND trigger_kind = 'ci_test'
                  AND trigger = $3 || '/' || ctr.tested_item_path
                ORDER BY created_at DESC
                LIMIT 1
            ) j ON true
            LEFT JOIN v2_job_completed jc ON jc.id = j.id
            WHERE ctr.workspace_id = $1
              AND ctr.tested_item_path = ANY($2::text[])
              AND ctr.tested_item_kind = $3
              AND NOT ctr.has_wildcard"#,
            &w_id,
            &paths_vec[..],
            kind
        )
        .fetch_all(&db)
        .await?;

        for row in exact {
            let key = format!("{}:{}", kind, row.tested_item_path);
            if let Some(bucket) = result_map.get_mut(&key) {
                bucket.push(row.into());
            }
        }

        // Wildcard candidates for this kind — one row per (pattern, requested_path) pair so the
        // LATERAL can scope the job lookup by the specific trigger.
        let wildcard = sqlx::query!(
            r#"SELECT
                ctr.test_script_path as "test_script_path!",
                ctr.tested_item_path as "tested_item_path!",
                req.path as "requested_path!",
                j.id as "job_id?",
                COALESCE(jc.status::text, CASE WHEN j.id IS NOT NULL THEN 'running' ELSE NULL END) as "status?",
                j.created_at as "started_at?"
            FROM ci_test_reference ctr
            CROSS JOIN unnest($3::text[]) AS req(path)
            LEFT JOIN LATERAL (
                SELECT id, created_at FROM v2_job
                WHERE workspace_id = $1
                  AND runnable_path = ctr.test_script_path
                  AND trigger_kind = 'ci_test'
                  AND trigger = $2 || '/' || req.path
                ORDER BY created_at DESC
                LIMIT 1
            ) j ON true
            LEFT JOIN v2_job_completed jc ON jc.id = j.id
            WHERE ctr.workspace_id = $1
              AND ctr.tested_item_kind = $2
              AND ctr.has_wildcard"#,
            &w_id,
            kind,
            &paths_vec[..]
        )
        .fetch_all(&db)
        .await?;

        for row in wildcard {
            if windmill_common::schema::ci_test_path_matches(
                &row.requested_path,
                &row.tested_item_path,
            ) {
                let key = format!("{}:{}", kind, row.requested_path);
                if let Some(bucket) = result_map.get_mut(&key) {
                    bucket.push(CiTestResult {
                        test_script_path: row.test_script_path,
                        job_id: row.job_id,
                        status: row.status,
                        started_at: row.started_at,
                    });
                }
            }
        }
    }

    Ok(Json(result_map))
}
