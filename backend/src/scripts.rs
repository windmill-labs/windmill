/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use reqwest::Client;
use serde::Deserializer;
use sql_builder::prelude::*;

use crate::{
    audit::{audit_log, ActionKind},
    db::{UserDB, DB},
    error::{to_anyhow, Error, JsonResult, Result},
    jobs, parser,
    users::{owner_to_token_owner, truncate_token, Authed, Tokened},
    utils::{http_get_from_hub, list_elems_from_hub, require_admin, Pagination, StripPath},
};
use axum::{
    extract::{Extension, Host, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use hyper::StatusCode;
use serde::{de::Error as _, ser::SerializeSeq, Deserialize, Serialize};
use serde_json::{json, to_string_pretty};
use sql_builder::SqlBuilder;
use sqlx::{FromRow, Postgres, Transaction};
use std::{
    collections::hash_map::DefaultHasher,
    fmt::Display,
    hash::{Hash, Hasher},
};

const MAX_HASH_HISTORY_LENGTH_STORED: usize = 20;

pub fn global_service() -> Router {
    Router::new()
        .route(
            "/python/tojsonschema",
            post(parse_python_code_to_jsonschema),
        )
        .route("/deno/tojsonschema", post(parse_deno_code_to_jsonschema))
        .route("/hub/list", get(list_hub_scripts))
        .route("/hub/get/*path", get(get_hub_script_by_path))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/", get(list_scripts))
        .route("/", post(create_script))
        .route("/p/archive/*path", post(archive_script_by_path))
        
        // FIXME: Could be simplified by using GET /p/*path and 
        // returning a 404 if the script doesn't exist. 
        .route("/p/get/*path", get(get_script_by_path))
        .route("/p/exists/*path", get(exists_script_by_path))
        
        .route("/h/:hash/archive", post(archive_script_by_hash))
        .route("/h/:hash", get(get_script_by_hash))
        .route("/h/:hash", delete(delete_script_by_hash))
        .route("/h/:hash/deployment-status", get(get_deployment_status))
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
#[sqlx(type_name = "SCRIPT_LANG", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ScriptLang {
    Deno,
    Python3,
}

impl ScriptLang {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptLang::Deno => "deno",
            ScriptLang::Python3 => "python3",
        }
    }
}

#[derive(sqlx::Type, PartialEq, Debug, Hash, Clone, Copy)]
#[sqlx(transparent)]
pub struct ScriptHash(pub i64);

#[derive(sqlx::Type, PartialEq)]
#[sqlx(transparent)]
pub struct ScriptHashes(Vec<i64>);

impl Display for ScriptHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", to_hex_string(&self.0))
    }
}
impl Serialize for ScriptHash {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(to_hex_string(&self.0).as_str())
    }
}
impl<'de> Deserialize<'de> for ScriptHash {
    fn deserialize<D>(deserializer: D) -> std::result::Result<ScriptHash, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let i = to_i64(&s).map_err(|e| D::Error::custom(format!("{}", e)))?;
        Ok(ScriptHash(i))
    }
}

impl Serialize for ScriptHashes {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for element in &self.0 {
            seq.serialize_element(&ScriptHash(*element))?;
        }
        seq.end()
    }
}

#[derive(FromRow, Serialize)]
pub struct Script {
    pub workspace_id: String,
    pub hash: ScriptHash,
    pub path: String,
    pub parent_hashes: Option<ScriptHashes>,
    pub summary: String,
    pub description: String,
    pub content: String,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub archived: bool,
    pub schema: Option<Schema>,
    pub deleted: bool,
    pub is_template: bool,
    pub extra_perms: serde_json::Value,
    pub lock: Option<String>,
    pub lock_error_logs: Option<String>,
    pub language: ScriptLang,
    pub is_trigger: bool,
}

#[derive(Serialize, Deserialize, sqlx::Type, Debug)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct Schema(pub serde_json::Value);

impl Hash for Schema {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Ok(s) = to_string_pretty(&self.0) {
            s.hash(state);
        }
    }
}

#[derive(Serialize, Deserialize, Hash)]
pub struct NewScript {
    pub path: String,
    pub parent_hash: Option<ScriptHash>,
    pub summary: String,
    pub description: String,
    pub content: String,
    pub schema: Option<Schema>,
    pub is_template: Option<bool>,
    pub lock: Option<Vec<String>>,
    pub language: ScriptLang,
    pub is_trigger: Option<bool>,
}

#[derive(Deserialize)]
pub struct ListScriptQuery {
    pub path_start: Option<String>,
    pub path_exact: Option<String>,
    pub created_by: Option<String>,
    pub first_parent_hash: Option<ScriptHash>,
    pub last_parent_hash: Option<ScriptHash>,
    pub parent_hash: Option<ScriptHash>,
    pub show_archived: Option<bool>,
    pub order_by: Option<String>,
    pub order_desc: Option<bool>,
    pub is_template: Option<bool>,
    pub is_trigger: Option<bool>,
}

async fn list_scripts(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListScriptQuery>,
) -> JsonResult<Vec<Script>> {
    let (per_page, offset) = crate::utils::paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("script as o")
        .fields(&[
            "workspace_id",
            "hash",
            "path",
            "array_remove(array[parent_hashes[1]], NULL) as parent_hashes",
            "summary",
            "description",
            "'' as content",
            "created_by",
            "created_at",
            "archived",
            "null as schema",
            "deleted",
            "is_template",
            "extra_perms",
            "null as lock",
            "CASE WHEN lock_error_logs IS NOT NULL THEN 'error' ELSE null END as lock_error_logs",
            "language",
            "is_trigger",
        ])
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .and_where("workspace_id = ? OR workspace_id = 'starter'".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    if lq.show_archived.unwrap_or(false) {
        sqlb.and_where_eq(
            "created_at",
            "(select max(created_at) from script where o.path = path 
            AND (workspace_id = $1 OR workspace_id = 'starter'))",
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
    if let Some(it) = &lq.is_trigger {
        sqlb.and_where_eq("is_trigger", it);
    }

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, Script>(&sql).fetch_all(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_hub_scripts(
    Authed { email, username, .. }: Authed,
    Extension(http_client): Extension<Client>,
    Host(host): Host,
) -> JsonResult<serde_json::Value> {
    let asks = list_elems_from_hub(
        http_client,
        "https://hub.windmill.dev/searchData?approved=true",
        email,
        username,
        host,
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
    Tokened { token }: Tokened,
    Extension(user_db): Extension<UserDB>,
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

    let parent_hashes_and_perms: Option<(Vec<i64>, serde_json::Value)> =
        match (&ns.parent_hash, clashing_script) {
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

                let ph = {
                    let v = ps.parent_hashes.map(|x| x.0).unwrap_or_default();
                    let mut v: Vec<i64> = v
                        .into_iter()
                        .take(MAX_HASH_HISTORY_LENGTH_STORED - 1)
                        .collect();
                    v.insert(0, p_hash.0);
                    v
                };
                let r: Result<Option<(Vec<i64>, serde_json::Value)>> = match o {
                    Some(clashing_script)
                        if clashing_script.path == ns.path
                            && clashing_script.hash.0 != p_hash.0 =>
                    {
                        Err(Error::BadRequest(format!(
                            "Path conflict for {} with non-archived hash {}",
                            &ns.path, &clashing_script.hash
                        )))
                    }
                    Some(_) => Ok(Some((ph, ps.extra_perms))),
                    None => Ok(Some((ph, ps.extra_perms))),
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

    let p_hashes = parent_hashes_and_perms.as_ref().map(|v| &v.0[..]);
    let extra_perms = parent_hashes_and_perms
        .as_ref()
        .map(|v| v.1.clone())
        .unwrap_or(json!({}));

    let lock = if ns.language == ScriptLang::Deno {
        Some("".to_string())
    } else {
        ns.lock.as_ref().map(|x| x.join("\n"))
    };
    //::text::json is to ensure we use serde_json with preserve order
    sqlx::query!(
        "INSERT INTO script (workspace_id, hash, path, parent_hashes, summary, description, \
         content, created_by, schema, is_template, extra_perms, lock, language, is_trigger) \
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
        ns.is_trigger.unwrap_or(false),
    )
    .execute(&mut tx)
    .await?;

    let mut tx = if ns.lock.is_none() && ns.language == ScriptLang::Python3 {
        let dependencies = parser::parse_python_imports(&ns.content)?;
        let (_, tx) = jobs::push(
            tx,
            &w_id,
            jobs::JobPayload::Dependencies { hash, dependencies },
            None,
            &authed.username,
            owner_to_token_owner(&authed.username, false),
            None,
            None,
            None,
            false,
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
            Some(
                [
                    ("hash", hash.to_string().as_str()),
                    ("token", &truncate_token(&token)),
                ]
                .into(),
            ),
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
                    ("token", &truncate_token(&token)),
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
    Authed { email, username, .. }: Authed,
    Path(path): Path<StripPath>,
    Extension(http_client): Extension<Client>,
    Host(host): Host,
) -> Result<String> {
    let path = path
        .to_path()
        .strip_prefix("hub/")
        .ok_or_else(|| Error::BadRequest("Impossible to remove prefix hex".to_string()))?;

    let content = http_get_from_hub(
        http_client,
        &format!("https://hub.windmill.dev/raw/{path}.ts"),
        email,
        username,
        host,
        true,
    )
    .await?
    .text()
    .await
    .map_err(to_anyhow)?;
    Ok(content)
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
         AND
         created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND archived = false AND \
         (workspace_id = $2 OR workspace_id = 'starter'))",
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(&mut tx)
    .await?;
    tx.commit().await?;

    let script = crate::utils::not_found_if_none(script_o, "Script", path)?;
    Ok(Json(script))
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

    let script = crate::utils::not_found_if_none(script_o, "Script", hash.to_string())?;
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

    let status = crate::utils::not_found_if_none(status_o, "DeploymentStatus", hash.to_string())?;

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
        "UPDATE script SET content = '', archived = true, deleted = true WHERE hash = $1 AND \
         workspace_id = $2RETURNING *",
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
) -> JsonResult<parser::MainArgSignature> {
    parser::parse_python_signature(&code).map(Json)
}

async fn parse_deno_code_to_jsonschema(
    Json(code): Json<String>,
) -> JsonResult<parser::MainArgSignature> {
    parser::parse_deno_signature(&code).map(Json)
}

pub fn to_i64(s: &str) -> Result<i64> {
    let v = hex::decode(s)?;
    let nb: u64 = u64::from_be_bytes(
        v[0..8]
            .try_into()
            .map_err(|_| hex::FromHexError::InvalidStringLength)?,
    );
    Ok(nb as i64)
}

pub fn to_hex_string(i: &i64) -> String {
    hex::encode(i.to_be_bytes())
}
