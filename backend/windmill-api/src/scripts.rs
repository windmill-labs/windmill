/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use reqwest::Client;
use sql_builder::prelude::*;
use windmill_audit::{audit_log, ActionKind};

use crate::{
    db::{UserDB, DB},
    schedule::clear_schedule,
    users::{require_owner_of_path, Authed},
};
use axum::{
    extract::{Extension, Path, Query},
    routing::{get, post},
    Json, Router,
};
use hyper::StatusCode;
use serde::Serialize;
use serde_json::json;
use sql_builder::SqlBuilder;
use sqlx::{FromRow, Postgres, Transaction};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
use windmill_common::{
    error::{Error, JsonResult, Result},
    schedule::Schedule,
    scripts::{
        to_i64, HubScript, ListScriptQuery, ListableScript, NewScript, Script, ScriptHash,
        ScriptKind, ScriptLang,
    },
    users::username_to_permissioned_as,
    utils::{
        list_elems_from_hub, not_found_if_none, paginate, require_admin, Pagination, StripPath,
    },
};
use windmill_queue::{self, schedule::push_scheduled_job};

const MAX_HASH_HISTORY_LENGTH_STORED: usize = 20;

pub fn global_service() -> Router {
    Router::new()
        .route(
            "/python/tojsonschema",
            post(parse_python_code_to_jsonschema),
        )
        .route("/deno/tojsonschema", post(parse_deno_code_to_jsonschema))
        .route("/go/tojsonschema", post(parse_go_code_to_jsonschema))
        .route("/bash/tojsonschema", post(parse_bash_code_to_jsonschema))
        .route("/hub/list", get(list_hub_scripts))
        .route("/hub/get/*path", get(get_hub_script_by_path))
        .route("/hub/get_full/*path", get(get_full_hub_script_by_path))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_scripts))
        .route("/create", post(create_script))
        .route("/archive/p/*path", post(archive_script_by_path))
        .route("/get/p/*path", get(get_script_by_path))
        .route("/raw/p/*path", get(raw_script_by_path))
        .route("/exists/p/*path", get(exists_script_by_path))
        .route("/archive/h/:hash", post(archive_script_by_hash))
        .route("/delete/h/:hash", post(delete_script_by_hash))
        .route("/get/h/:hash", get(get_script_by_hash))
        .route("/raw/h/:hash", get(raw_script_by_hash))
        .route("/deployment_status/h/:hash", get(get_deployment_status))
        .route("/list_paths", get(list_paths))
}
async fn list_scripts(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListScriptQuery>,
) -> JsonResult<Vec<ListableScript>> {
    let (per_page, offset) = paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("script as o")
        .fields(&[
            "o.workspace_id",
            "hash",
            "o.path",
            "array_remove(array[parent_hashes[1]], NULL) as parent_hashes",
            "summary",
            "description",
            "created_by",
            "created_at",
            "archived",
            "deleted",
            "is_template",
            "extra_perms",
            "CASE WHEN lock_error_logs IS NOT NULL THEN 'error' ELSE null END as lock_error_logs",
            "language",
            "kind",
            "favorite.path IS NOT NULL as starred",
        ])
        .left()
        .join("favorite")
        .on(
            "favorite.favorite_kind = 'script' AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?"
                .bind(&authed.username),
        )
        .order_desc("favorite.path IS NOT NULL")
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .and_where("o.workspace_id = ? OR o.workspace_id = 'starter'".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    if lq.show_archived.unwrap_or(false) {
        sqlb.and_where_eq(
            "created_at",
            "(select max(created_at) from script where o.path = path 
            AND (workspace_id = ? OR workspace_id = 'starter'))"
                .bind(&w_id),
        );
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
    if let Some(k) = &lq.kind {
        sqlb.and_where_eq("kind", "?".bind(&k.to_lowercase()));
    }
    if lq.starred_only.unwrap_or(false) {
        sqlb.and_where_is_not_null("favorite.path");
    }

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, ListableScript>(&sql)
        .fetch_all(&mut tx)
        .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_hub_scripts(
    Authed { email, .. }: Authed,
    Extension(http_client): Extension<Client>,
) -> JsonResult<serde_json::Value> {
    let asks = list_elems_from_hub(
        http_client,
        "https://hub.windmill.dev/searchData?approved=true",
        &email,
    )
    .await?;
    Ok(Json(asks))
}

fn hash_script(ns: &NewScript) -> i64 {
    let mut dh = DefaultHasher::new();
    ns.hash(&mut dh);
    dh.finish() as i64
}

async fn create_script(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(ns): Json<NewScript>,
) -> Result<(StatusCode, String)> {
    let hash = ScriptHash(hash_script(&ns));
    let mut tx = user_db.begin(&authed).await?;

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
        (None, Some(s)) => Err(Error::BadRequest(format!(
            "Path conflict for {} with non-archived hash {}",
            &ns.path, &s.hash
        ))),
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

            let ps = get_script_by_hash_internal(&mut tx, &w_id, p_hash).await?;

            if ps.path != ns.path {
                if !authed.is_admin {
                    require_owner_of_path(&w_id, &authed.username, &authed.groups, &ps.path, &db)
                        .await?;
                }
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

    let lock = if ns.language == ScriptLang::Bash || ns.language == ScriptLang::Deno {
        Some(String::new())
    } else {
        ns.lock
            .as_ref()
            .map(|x| x.join("\n"))
            .and_then(|e| if e.is_empty() { None } else { Some(e) })
    };

    let needs_lock_gen = lock.is_none();
    //::text::json is to ensure we use serde_json with preserve order
    sqlx::query!(
        "INSERT INTO script (workspace_id, hash, path, parent_hashes, summary, description, \
         content, created_by, schema, is_template, extra_perms, lock, language, kind) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::text::json, $10, $11, $12, $13, $14)",
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
        ns.language: ScriptLang,
        ns.kind.unwrap_or(ScriptKind::Script): ScriptKind,
    )
    .execute(&mut tx)
    .await?;

    if let Some(p_path) = parent_hashes_and_perms.as_ref().map(|x| x.p_path.clone()) {
        let schedulables = sqlx::query_as!(
        Schedule,
            "UPDATE schedule SET script_path = $1 WHERE script_path = $2 AND workspace_id = $3 AND is_flow IS false RETURNING *",
            ns.path,
            p_path,
            w_id,
        )
        .fetch_all(&mut tx)
        .await?;

        for schedule in schedulables {
            clear_schedule(&mut tx, &schedule.path, false).await?;

            if schedule.enabled {
                tx = push_scheduled_job(tx, schedule).await?;
            }
        }
    }

    let mut tx = if needs_lock_gen {
        let dependencies = match ns.language {
            ScriptLang::Python3 => {
                windmill_parser_py::parse_python_imports(&ns.content)?.join("\n")
            }
            _ => ns.content,
        };
        let (_, tx) = windmill_queue::push(
            tx,
            &w_id,
            windmill_queue::JobPayload::Dependencies { hash, dependencies, language: ns.language },
            serde_json::Map::new(),
            &authed.username,
            &authed.email,
            username_to_permissioned_as(&authed.username),
            None,
            None,
            None,
            false,
            false,
            None,
            true,
        )
        .await?;
        tx
    } else {
        tx
    };

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
    }

    tx.commit().await?;

    Ok((StatusCode::CREATED, format!("{}", hash)))
}

pub async fn get_hub_script_by_path(
    authed: Authed,
    Path(path): Path<StripPath>,
    Extension(http_client): Extension<Client>,
) -> Result<String> {
    windmill_common::scripts::get_hub_script_by_path(&authed.email, path, http_client).await
}

pub async fn get_full_hub_script_by_path(
    Authed { email, .. }: Authed,
    Path(path): Path<StripPath>,
    Extension(http_client): Extension<Client>,
) -> JsonResult<HubScript> {
    Ok(Json(
        windmill_common::scripts::get_full_hub_script_by_path(&email, path, http_client).await?,
    ))
}

async fn get_script_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Script> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let script_o = sqlx::query_as::<_, Script>(
        "SELECT * FROM script WHERE path = $1 AND (workspace_id = $2 OR workspace_id = 'starter') \
         AND created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND \
         (workspace_id = $2 OR workspace_id = 'starter'))",
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(&mut tx)
    .await?;
    tx.commit().await?;

    let script = not_found_if_none(script_o, "Script", path)?;
    Ok(Json(script))
}

async fn list_paths(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    let mut tx = user_db.begin(&authed).await?;

    let scripts = sqlx::query_scalar!(
        "SELECT distinct(path) FROM script WHERE  workspace_id = $1",
        w_id
    )
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;

    Ok(Json(scripts))
}

async fn raw_script_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path().split(".").next().unwrap_or_default();
    let mut tx = user_db.begin(&authed).await?;

    let content_o = sqlx::query_scalar!(
        "SELECT content FROM script WHERE path = $1 AND (workspace_id = $2 OR workspace_id = 'starter') \
         AND
         created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND archived = false AND \
         (workspace_id = $2 OR workspace_id = 'starter'))",
         path, w_id
    )
    .fetch_optional(&mut tx)
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
        "SELECT EXISTS(SELECT 1 FROM script WHERE path = $1 AND (workspace_id = $2 OR \
         workspace_id = 'starter') AND
         created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND (workspace_id = $2 \
         OR workspace_id = 'starter')))",
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
    let script_o = sqlx::query_as::<_, Script>(
        "SELECT * FROM script WHERE hash = $1 AND (workspace_id = $2 OR workspace_id = 'starter')",
    )
    .bind(hash)
    .bind(workspace_id)
    .fetch_optional(db)
    .await?;

    let script = not_found_if_none(script_o, "Script", hash.to_string())?;
    Ok(script)
}

async fn get_script_by_hash(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, hash)): Path<(String, ScriptHash)>,
) -> JsonResult<Script> {
    let mut tx = user_db.begin(&authed).await?;
    let r = get_script_by_hash_internal(&mut tx, &w_id, &hash).await?;
    tx.commit().await?;

    Ok(Json(r))
}

async fn raw_script_by_hash(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, hash_str)): Path<(String, String)>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;
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
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, hash)): Path<(String, ScriptHash)>,
) -> JsonResult<DeploymentStatus> {
    let mut tx = user_db.begin(&authed).await?;
    let status_o: Option<DeploymentStatus> = sqlx::query_as!(
        DeploymentStatus,
        "SELECT lock, lock_error_logs FROM script WHERE hash = $1 AND (workspace_id = $2 OR \
         workspace_id = 'starter')",
        hash.0,
        w_id,
    )
    .fetch_optional(&mut tx)
    .await?;

    let status = not_found_if_none(status_o, "DeploymentStatus", hash.to_string())?;

    tx.commit().await?;
    Ok(Json(status))
}

async fn archive_script_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<()> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let hash: i64 = sqlx::query_scalar!(
        "UPDATE script SET archived = true WHERE path = $1 AND workspace_id = $2 RETURNING hash",
        path,
        &w_id
    )
    .fetch_one(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("archiving script in {w_id}: {e}")))?;
    audit_log(
        &mut tx,
        &authed.username,
        "scripts.archive",
        ActionKind::Delete,
        &w_id,
        Some(&ScriptHash(hash).to_string()),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(())
}

async fn archive_script_by_hash(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, hash)): Path<(String, ScriptHash)>,
) -> JsonResult<Script> {
    let mut tx = user_db.begin(&authed).await?;

    let script = sqlx::query_as::<_, Script>(
        "UPDATE script SET archived = true WHERE hash = $1 RETURNING *",
    )
    .bind(&hash.0)
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("archiving script in {w_id}: {e}")))?;

    audit_log(
        &mut tx,
        &authed.username,
        "scripts.archive",
        ActionKind::Delete,
        &w_id,
        Some(&hash.to_string()),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(script))
}

async fn delete_script_by_hash(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
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
        &mut tx,
        &authed.username,
        "scripts.delete",
        ActionKind::Delete,
        &w_id,
        Some(&hash.to_string()),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(script))
}

async fn parse_python_code_to_jsonschema(
    Json(code): Json<String>,
) -> JsonResult<windmill_parser::MainArgSignature> {
    windmill_parser_py::parse_python_signature(&code).map(Json)
}

async fn parse_deno_code_to_jsonschema(
    Json(code): Json<String>,
) -> JsonResult<windmill_parser::MainArgSignature> {
    windmill_parser_ts::parse_deno_signature(&code).map(Json)
}
async fn parse_go_code_to_jsonschema(
    Json(code): Json<String>,
) -> JsonResult<windmill_parser::MainArgSignature> {
    windmill_parser_go::parse_go_sig(&code).map(Json)
}

async fn parse_bash_code_to_jsonschema(
    Json(code): Json<String>,
) -> JsonResult<windmill_parser::MainArgSignature> {
    windmill_parser_bash::parse_bash_sig(&code).map(Json)
}
