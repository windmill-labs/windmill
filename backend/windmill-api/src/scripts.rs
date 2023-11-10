/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{ApiAuthed, DB},
    schedule::clear_schedule,
    users::{maybe_refresh_folders, require_owner_of_path, AuthCache},
    webhook_util::{WebhookMessage, WebhookShared},
    HTTP_CLIENT,
};
use axum::{
    body::StreamBody,
    extract::{Extension, Path, Query},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sql_builder::prelude::*;
use sql_builder::SqlBuilder;
use sqlx::{FromRow, Postgres, Transaction};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    jobs::JobPayload,
    schedule::Schedule,
    scripts::{
        to_i64, HubScript, ListScriptQuery, ListableScript, NewScript, Schema, Script, ScriptHash,
        ScriptKind, ScriptLang,
    },
    users::username_to_permissioned_as,
    utils::{
        not_found_if_none, paginate, query_elems_from_hub, require_admin, Pagination, StripPath,
    },
};
use windmill_queue::{
    self, schedule::push_scheduled_job, PushArgs, PushIsolationLevel, QueueTransaction,
};

const MAX_HASH_HISTORY_LENGTH_STORED: usize = 20;

#[derive(Serialize, sqlx::FromRow)]
pub struct ScriptWDraft {
    pub hash: ScriptHash,
    pub path: String,
    pub summary: String,
    pub description: String,
    pub content: String,
    pub language: ScriptLang,
    pub kind: ScriptKind,
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<serde_json::Value>,
    pub schema: Option<Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrent_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_time_window_s: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_worker: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i16>,
}

pub fn global_service() -> Router {
    Router::new()
        .route("/hub/top", get(get_top_hub_scripts))
        .route("/hub/get/*path", get(get_hub_script_by_path))
        .route("/hub/get_full/*path", get(get_full_hub_script_by_path))
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
        .route("/archive/p/*path", post(archive_script_by_path))
        .route("/get/draft/*path", get(get_script_by_path_w_draft))
        .route("/get/p/*path", get(get_script_by_path))
        .route("/raw/p/*path", get(raw_script_by_path))
        .route("/exists/p/*path", get(exists_script_by_path))
        .route("/archive/h/:hash", post(archive_script_by_hash))
        .route("/delete/h/:hash", post(delete_script_by_hash))
        .route("/delete/p/*path", post(delete_script_by_path))
        .route("/get/h/:hash", get(get_script_by_hash))
        .route("/raw/h/:hash", get(raw_script_by_hash))
        .route("/deployment_status/h/:hash", get(get_deployment_status))
        .route("/list_paths", get(list_paths))
        .route(
            "/toggle_workspace_error_handler/p/*path",
            post(toggle_workspace_error_handler),
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
    let n = 1000;

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
    let (per_page, offset) = paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("script as o")
        .fields(&[
            "hash",
            "o.path",
            "summary",
            "COALESCE(draft.created_at, o.created_at) as created_at",
            "archived",
            "extra_perms",
            "CASE WHEN lock_error_logs IS NOT NULL THEN true ELSE false END as has_deploy_errors",
            "language",
            "favorite.path IS NOT NULL as starred",
            "tag",
            "draft.path IS NOT NULL as has_draft",
            "draft_only",
            "ws_error_handler_muted"
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

    if lq.show_archived.unwrap_or(false) {
        sqlb.and_where_eq(
            "o.created_at",
            "(select max(created_at) from script where o.path = path 
            AND workspace_id = ?)"
                .bind(&w_id),
        );
        sqlb.and_where_eq("archived", true);
    } else {
        sqlb.and_where_eq("archived", false);
    }
    if let Some(ps) = &lq.path_start {
        sqlb.and_where_like_left("path", "?".bind(ps));
    }
    if let Some(p) = &lq.path_exact {
        sqlb.and_where_eq("path", "?".bind(p));
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
    if let Some(kinds_val) = &lq.kinds {
        let lowercased_kinds: Vec<String> = kinds_val
            .split(",")
            .map(&str::to_lowercase)
            .map(sql_builder::quote)
            .collect();
        if lowercased_kinds.len() > 0 {
            sqlb.and_where_in("kind", lowercased_kinds.as_slice());
        }
    }
    if lq.starred_only.unwrap_or(false) {
        sqlb.and_where_is_not_null("favorite.path");
    }

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
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
        "https://hub.windmill.dev/scripts/top",
        Some(query_params),
        &db,
    )
    .await?;
    Ok::<_, Error>((
        status_code,
        headers,
        StreamBody::new(response.bytes_stream()),
    ))
}

fn hash_script(ns: &NewScript) -> i64 {
    let mut dh = DefaultHasher::new();
    ns.hash(&mut dh);
    dh.finish() as i64
}

async fn create_script(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(ns): Json<NewScript>,
) -> Result<(StatusCode, String)> {
    #[cfg(not(feature = "enterprise"))]
    if ns.ws_error_handler_muted.is_some_and(|val| val) {
        return Err(Error::BadRequest(
            "Muting the error handler for certain script is only available in enterprise version"
                .to_string(),
        ));
    }

    let hash = ScriptHash(hash_script(&ns));
    let authed = maybe_refresh_folders(&ns.path, &w_id, authed, &db).await;
    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.begin(&authed).await?).into();

    if sqlx::query_scalar!(
        "SELECT 1 FROM script WHERE hash = $1 AND workspace_id = $2",
        hash.0,
        &w_id
    )
    .fetch_optional(&mut tx)
    .await?
    .is_some()
    {
        return Err(Error::BadRequest(
            "A script with same hash (hence same path, description, summary, content) already \
             exists!"
                .to_owned(),
        ));
    };

    let clashing_script = sqlx::query_as::<_, Script>(
        "SELECT * FROM script WHERE path = $1 AND archived = false AND workspace_id = $2",
    )
    .bind(&ns.path)
    .bind(&w_id)
    .fetch_optional(&mut tx)
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
            .execute(&mut tx)
            .await?;
            Ok(None)
        }
        (Some(p_hash), o) => {
            if sqlx::query_scalar!(
                "SELECT 1 FROM script WHERE hash = $1 AND workspace_id = $2",
                p_hash.0,
                &w_id
            )
            .fetch_optional(&mut tx)
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
            .fetch_optional(&mut tx)
            .await?;

            if let Some(clashing_hash) = clashing_hash_o {
                return Err(Error::BadRequest(format!(
                    "A script with hash {} with same parent_hash has been found. However, the \
                         lineage must be linear: no 2 scripts can have the same parent",
                    ScriptHash(clashing_hash)
                )));
            };

            let ps = get_script_by_hash_internal(tx.transaction_mut(), &w_id, p_hash).await?;

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
            .execute(&mut tx)
            .await?;
            r
        }
    }?;

    let p_hashes = parent_hashes_and_perms.as_ref().map(|v| &v.p_hashes[..]);
    let extra_perms = parent_hashes_and_perms
        .as_ref()
        .map(|v| v.perms.clone())
        .unwrap_or(json!({}));

    let lock = if !(ns.language == ScriptLang::Python3
        || ns.language == ScriptLang::Go
        || ns.language == ScriptLang::Bun
        || ns.language == ScriptLang::Deno)
    {
        Some(String::new())
    } else {
        ns.lock
            .as_ref()
            .map(|x| x.join("\n"))
            .and_then(|e| if e.is_empty() { None } else { Some(e) })
    };

    let needs_lock_gen = lock.is_none();

    let envs = ns.envs.as_ref().map(|x| x.as_slice());
    let envs = if ns.envs.is_none() || ns.envs.as_ref().unwrap().is_empty() {
        None
    } else {
        envs
    };
    //::text::json is to ensure we use serde_json with preserve order
    sqlx::query!(
        "INSERT INTO script (workspace_id, hash, path, parent_hashes, summary, description, \
         content, created_by, schema, is_template, extra_perms, lock, language, kind, tag, \
         draft_only, envs, concurrent_limit, concurrency_time_window_s, cache_ttl, \
         dedicated_worker, ws_error_handler_muted, priority) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::text::json, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)",
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
        ns.language.clone() as ScriptLang,
        ns.kind.unwrap_or(ScriptKind::Script) as ScriptKind,
        ns.tag,
        ns.draft_only,
        envs,
        ns.concurrent_limit,
        ns.concurrency_time_window_s,
        ns.cache_ttl,
        ns.dedicated_worker,
        ns.ws_error_handler_muted.unwrap_or(false),
        ns.priority,
    )
    .execute(&mut tx)
    .await?;

    if let Some(p_path) = parent_hashes_and_perms.as_ref().map(|x| x.p_path.clone()) {
        sqlx::query!(
            "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'script'",
            p_path,
            &w_id
        )
        .execute(&mut tx)
        .await?;

        let mut schedulables = sqlx::query_as!(
        Schedule,
            "UPDATE schedule SET script_path = $1 WHERE script_path = $2 AND path != $2 AND workspace_id = $3 AND is_flow IS false RETURNING *",
            ns.path,
            p_path,
            w_id,
        )
        .fetch_all(&mut tx)
        .await?;

        let schedule = sqlx::query_as!(Schedule,
            "UPDATE schedule SET path = $1, script_path = $1 WHERE path = $2 AND workspace_id = $3 AND is_flow IS false RETURNING *",
            ns.path,
            p_path,
            w_id,
        )
        .fetch_optional(&mut tx)
        .await?;

        if let Some(schedule) = schedule {
            schedulables.push(schedule);
        }

        for schedule in schedulables {
            clear_schedule(tx.transaction_mut(), &schedule.path, false, &w_id).await?;

            if schedule.enabled {
                tx = push_scheduled_job(&db, tx, schedule).await?;
            }
        }
    } else {
        sqlx::query!(
            "DELETE FROM draft WHERE path = $1 AND workspace_id = $2 AND typ = 'script'",
            ns.path,
            &w_id
        )
        .execute(&mut tx)
        .await?;
    }

    if p_hashes.is_some() && !p_hashes.unwrap().is_empty() {
        audit_log(
            &mut tx,
            &authed.username,
            "scripts.update",
            ActionKind::Update,
            &w_id,
            Some(&ns.path),
            Some([("hash", hash.to_string().as_str())].into()),
        )
        .await?;
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
            &mut tx,
            &authed.username,
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
        webhook.send_message(
            w_id.clone(),
            WebhookMessage::CreateScript {
                workspace: w_id.clone(),
                path: ns.path.clone(),
                hash: hash.to_string(),
            },
        );
    }

    let mut tx = PushIsolationLevel::Transaction(tx);

    if needs_lock_gen {
        let dependencies = match ns.language {
            ScriptLang::Python3 => {
                windmill_parser_py_imports::parse_python_imports(&ns.content, &w_id, &ns.path, &db)
                    .await?
                    .join("\n")
            }
            _ => ns.content,
        };
        let tag = if ns.dedicated_worker.is_some_and(|x| x) {
            Some(format!("{}:{}", &w_id, &ns.path,))
        } else {
            ns.tag
        };
        let (_, new_tx) = windmill_queue::push(
            &db,
            tx,
            &w_id,
            JobPayload::Dependencies { hash, dependencies, language: ns.language, path: ns.path },
            PushArgs::empty(),
            &authed.username,
            &authed.email,
            username_to_permissioned_as(&authed.username),
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
        )
        .await?;
        tx = PushIsolationLevel::Transaction(new_tx);
    }

    match tx {
        PushIsolationLevel::Transaction(tx) => tx.commit().await?,
        _ => {
            return Err(Error::InternalErr(
                "Expected a transaction here".to_string(),
            ));
        }
    }

    Ok((StatusCode::CREATED, format!("{}", hash)))
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
        windmill_common::scripts::get_full_hub_script_by_path(path, &HTTP_CLIENT, &db).await?,
    ))
}

async fn get_script_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Script> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let script_o = sqlx::query_as::<_, Script>(
        "SELECT * FROM script WHERE path = $1 AND workspace_id = $2 \
         AND created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND \
         workspace_id = $2)",
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let script = not_found_if_none(script_o, "Script", path)?;
    Ok(Json(script))
}

async fn get_script_by_path_w_draft(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<ScriptWDraft> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let script_o = sqlx::query_as::<_, ScriptWDraft>(
        "SELECT hash, script.path, summary, description, content, language, kind, tag, schema, draft_only, envs, concurrent_limit, concurrency_time_window_s, cache_ttl, ws_error_handler_muted, draft.value as draft, dedicated_worker, priority FROM script LEFT JOIN draft ON 
         script.path = draft.path AND script.workspace_id = draft.workspace_id AND draft.typ = 'script'
         WHERE script.path = $1 AND script.workspace_id = $2 \
         AND script.created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND \
         workspace_id = $2)",
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let script = not_found_if_none(script_o, "Script", path)?;
    Ok(Json(script))
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
    pub muted: Option<bool>,
}
async fn toggle_workspace_error_handler(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(req): Json<ToggleWorkspaceErrorHandler>,
) -> Result<String> {
    #[cfg(not(feature = "enterprise"))]
    {
        return Err(Error::BadRequest(
            "Muting the error handler for certain script is only available in enterprise version"
                .to_string(),
        ));
    }

    let mut tx = user_db.begin(&authed).await?;

    let error_handler_maybe: Option<String> = sqlx::query_scalar!(
        "SELECT error_handler FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(None);

    match error_handler_maybe {
        Some(_) => {
            sqlx::query_scalar!(
                "UPDATE script SET ws_error_handler_muted = $3 WHERE workspace_id = $2 AND path = $1 AND created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND workspace_id = $2)",
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
            Err(Error::ExecutionErr(
                "Workspace error handler needs to be defined".to_string(),
            ))
        }
    }
}

async fn get_tokened_raw_script_by_path(
    Extension(user_db): Extension<UserDB>,
    Path((w_id, token, path)): Path<(String, String, StripPath)>,
    Extension(cache): Extension<Arc<AuthCache>>,
) -> Result<String> {
    let authed = cache
        .get_authed(Some(w_id.clone()), &token)
        .await
        .ok_or_else(|| Error::NotAuthorized("Invalid token".to_string()))?;
    return raw_script_by_path(authed, Extension(user_db), Path((w_id, path))).await;
}

async fn get_empty_ts_script_by_path() -> String {
    return String::new();
}

async fn raw_script_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
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
        .trim_end_matches(".ts")
        .trim_end_matches(".go")
        .trim_end_matches(".sh");
    let mut tx = user_db.begin(&authed).await?;

    let content_o = sqlx::query_scalar!(
        "SELECT content FROM script WHERE path = $1 AND workspace_id = $2 \
         AND
         created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND archived = false AND \
         workspace_id = $2)",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let content = not_found_if_none(content_o, "Script", path)?;
    Ok(content)
}

async fn exists_script_by_path(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM script WHERE path = $1 AND workspace_id = $2 AND
         created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND workspace_id = $2))",
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
) -> Result<Script> {
    let script_o =
        sqlx::query_as::<_, Script>("SELECT * FROM script WHERE hash = $1 AND workspace_id = $2")
            .bind(hash)
            .bind(workspace_id)
            .fetch_optional(&mut **db)
            .await?;

    let script = not_found_if_none(script_o, "Script", hash.to_string())?;
    Ok(script)
}

async fn get_script_by_hash(
    Extension(db): Extension<DB>,
    Path((w_id, hash)): Path<(String, ScriptHash)>,
) -> JsonResult<Script> {
    let mut tx = db.begin().await?;
    let r = get_script_by_hash_internal(&mut tx, &w_id, &hash).await?;
    tx.commit().await?;

    Ok(Json(r))
}

async fn raw_script_by_hash(
    Extension(db): Extension<DB>,
    Path((w_id, hash_str)): Path<(String, String)>,
) -> Result<String> {
    let mut tx = db.begin().await?;
    let hash = ScriptHash(to_i64(hash_str.strip_suffix(".ts").ok_or_else(|| {
        Error::BadRequest("Raw script path must end with .ts".to_string())
    })?)?);
    let r = get_script_by_hash_internal(&mut tx, &w_id, &hash).await?;
    tx.commit().await?;

    Ok(r.content)
}

#[derive(FromRow, Serialize)]
struct DeploymentStatus {
    lock: Option<String>,
    lock_error_logs: Option<String>,
}
async fn get_deployment_status(
    Extension(db): Extension<DB>,
    Path((w_id, hash)): Path<(String, ScriptHash)>,
) -> JsonResult<DeploymentStatus> {
    let mut tx = db.begin().await?;
    let status_o: Option<DeploymentStatus> = sqlx::query_as!(
        DeploymentStatus,
        "SELECT lock, lock_error_logs FROM script WHERE hash = $1 AND workspace_id = $2",
        hash.0,
        w_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let status = not_found_if_none(status_o, "DeploymentStatus", hash.to_string())?;

    tx.commit().await?;
    Ok(Json(status))
}

pub async fn require_is_writer(authed: &ApiAuthed, path: &str, w_id: &str, db: DB) -> Result<()> {
    return crate::users::require_is_writer(
        authed,
        path,
        w_id,
        db,
        "SELECT extra_perms FROM script WHERE path = $1 AND workspace_id = $2 \
             AND created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND \
             workspace_id = $2)",
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
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    require_owner_of_path(&authed, path)?;

    let hash: i64 = sqlx::query_scalar!(
        "UPDATE script SET archived = true WHERE path = $1 AND workspace_id = $2 RETURNING hash",
        path,
        &w_id
    )
    .fetch_one(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("archiving script in {w_id}: {e}")))?;
    audit_log(
        &mut *tx,
        &authed.username,
        "scripts.archive",
        ActionKind::Delete,
        &w_id,
        Some(&ScriptHash(hash).to_string()),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;
    tx.commit().await?;
    webhook.send_message(
        w_id.clone(),
        WebhookMessage::DeleteScript { workspace: w_id, hash: hash.to_string() },
    );

    Ok(())
}

async fn archive_script_by_hash(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, hash)): Path<(String, ScriptHash)>,
) -> JsonResult<Script> {
    let mut tx = user_db.begin(&authed).await?;

    let script = sqlx::query_as::<_, Script>(
        "UPDATE script SET archived = true WHERE hash = $1 RETURNING *",
    )
    .bind(&hash.0)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::InternalErr(format!("archiving script in {w_id}: {e}")))?;

    audit_log(
        &mut *tx,
        &authed.username,
        "scripts.archive",
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

    Ok(Json(script))
}

async fn delete_script_by_hash(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(db): Extension<DB>,
    Path((w_id, hash)): Path<(String, ScriptHash)>,
) -> JsonResult<Script> {
    let mut tx = user_db.begin(&authed).await?;

    require_admin(authed.is_admin, &authed.username)?;
    let script = sqlx::query_as::<_, Script>(
        "UPDATE script SET content = '', archived = true, deleted = true, lock = '', schema = null WHERE hash = $1 AND \
         workspace_id = $2 RETURNING *",
    )
    .bind(&hash.0)
    .bind(&w_id)
    .fetch_one(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("deleting script by hash {w_id}: {e}")))?;

    audit_log(
        &mut *tx,
        &authed.username,
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

    Ok(Json(script))
}

async fn delete_script_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<String> {
    let path = path.to_path();

    if path == "u/admin/hub_sync" && w_id == "admins" {
        return Err(Error::BadRequest(
            "Cannot delete the global setup app".to_string(),
        ));
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

    if !draft_only {
        require_admin(authed.is_admin, &authed.username)?;
    }

    let script = sqlx::query_scalar!(
        "DELETE FROM script WHERE path = $1 AND workspace_id = $2 RETURNING path",
        path,
        w_id
    )
    .fetch_one(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("deleting script by path {w_id}: {e}")))?;

    audit_log(
        &mut *tx,
        &authed.username,
        "scripts.delete",
        ActionKind::Delete,
        &w_id,
        Some(&path),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::DeleteScriptPath { workspace: w_id, path: path.to_string() },
    );

    Ok(Json(script))
}
