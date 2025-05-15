#[cfg(feature = "http_trigger")]
use crate::http_trigger_args::{HttpMethod, RawHttpTriggerArgs};
#[cfg(feature = "parquet")]
use crate::job_helpers_ee::get_workspace_s3_resource;
use crate::resources::try_get_resource_from_db_as;
use crate::trigger_helpers::{get_runnable_format, RunnableId};
use crate::utils::{non_empty_str, ExpiringCacheEntry};
use crate::{
    auth::{AuthCache, OptTokened},
    db::{ApiAuthed, DB},
    jobs::{
        run_flow_by_path_inner, run_script_by_path_inner, run_wait_result_flow_by_path_internal,
        run_wait_result_script_by_path_internal, RunJobQuery,
    },
    users::fetch_api_authed,
};
use axum::response::Response;
use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    routing::{delete, get, post},
    Extension, Json, Router,
};
#[cfg(feature = "parquet")]
use http::header::IF_NONE_MATCH;
use http::{HeaderMap, StatusCode};
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::prelude::FromRow;
use sqlx::PgTransaction;
use std::borrow::Cow;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, RwLockReadGuard};
use tower_http::cors::CorsLayer;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::error::Error;
#[cfg(feature = "parquet")]
use windmill_common::s3_helpers::build_object_store_client;
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult},
    s3_helpers::S3Object,
    utils::{not_found_if_none, paginate, require_admin, Pagination, StripPath},
    worker::CLOUD_HOSTED,
};
use windmill_queue::TriggerKind;

lazy_static::lazy_static! {
    static ref ROUTE_PATH_KEY_RE: regex::Regex = regex::Regex::new(r"/?:[-\w]+").unwrap();
    static ref VALID_ROUTE_PATH_RE: regex::Regex = regex::Regex::new(r"^:?[-\w]+(/:?[-\w]+)*$").unwrap();
}

pub fn routes_global_service() -> Router {
    let cors = CorsLayer::new()
        .allow_methods([
            http::Method::GET,
            http::Method::POST,
            http::Method::DELETE,
            http::Method::PUT,
            http::Method::PATCH,
        ])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
        .allow_origin(tower_http::cors::Any);
    Router::new()
        .route(
            "/*path",
            get(route_job)
                .post(route_job)
                .delete(route_job)
                .put(route_job)
                .patch(route_job)
                .head(|| async { "" }),
        )
        .layer(cors)
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_trigger))
        .route("/list", get(list_triggers))
        .route("/get/*path", get(get_trigger))
        .route("/update/*path", post(update_trigger))
        .route("/delete/*path", delete(delete_trigger))
        .route("/exists/*path", get(exists_trigger))
        .route("/route_exists", post(exists_route))
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
#[sqlx(type_name = "AUTHENTICATION_METHOD", rename_all = "snake_case")]
#[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub enum AuthenticationMethod {
    None,
    Windmill,
    ApiKey,
    BasicHttp,
    CustomScript,
    Signature,
}

#[derive(Debug, Deserialize)]
struct NewTrigger {
    path: String,
    route_path: String,
    script_path: String,
    is_flow: bool,
    is_async: bool,
    authentication_resource_path: Option<String>,
    authentication_method: AuthenticationMethod,
    static_asset_config: Option<sqlx::types::Json<S3Object>>,
    http_method: HttpMethod,
    workspaced_route: Option<bool>,
    is_static_website: bool,
    wrap_body: Option<bool>,
    raw_string: Option<bool>,
}

#[derive(FromRow, Serialize)]
pub struct HttpTrigger {
    pub workspace_id: String,
    pub path: String,
    pub route_path: String,
    pub route_path_key: String,
    pub script_path: String,
    pub is_flow: bool,
    pub edited_by: String,
    pub email: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub extra_perms: serde_json::Value,
    pub is_async: bool,
    pub authentication_method: AuthenticationMethod,
    pub http_method: HttpMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_asset_config: Option<sqlx::types::Json<S3Object>>,
    pub is_static_website: bool,
    pub authentication_resource_path: Option<String>,
    pub workspaced_route: bool,
    pub wrap_body: bool,
    pub raw_string: bool,
}

#[derive(Deserialize)]
struct EditTrigger {
    path: String,
    route_path: Option<String>,
    script_path: String,
    is_flow: bool,
    is_async: bool,
    authentication_method: AuthenticationMethod,
    #[serde(deserialize_with = "non_empty_str")]
    authentication_resource_path: Option<String>,
    http_method: HttpMethod,
    static_asset_config: Option<sqlx::types::Json<S3Object>>,
    workspaced_route: Option<bool>,
    is_static_website: bool,
    wrap_body: Option<bool>,
    raw_string: Option<bool>,
}

#[derive(Deserialize)]
pub struct ListTriggerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
    pub path_start: Option<String>,
}

async fn list_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(lst): Query<ListTriggerQuery>,
) -> error::JsonResult<Vec<HttpTrigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(Pagination { per_page: lst.per_page, page: lst.page });
    let mut sqlb = SqlBuilder::select_from("http_trigger")
        .fields(&[
            "workspace_id",
            "path",
            "route_path",
            "route_path_key",
            "workspaced_route",
            "wrap_body",
            "raw_string",
            "script_path",
            "is_flow",
            "http_method",
            "edited_by",
            "email",
            "edited_at",
            "extra_perms",
            "is_async",
            "authentication_method",
            "static_asset_config",
            "is_static_website",
            "authentication_resource_path",
        ])
        .order_by("edited_at", true)
        .and_where("workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();
    if let Some(path) = lst.path {
        sqlb.and_where_eq("script_path", "?".bind(&path));
    }
    if let Some(is_flow) = lst.is_flow {
        sqlb.and_where_eq("is_flow", "?".bind(&is_flow));
    }
    if let Some(path_start) = &lst.path_start {
        sqlb.and_where_like_left("path", path_start);
    }
    let sql = sqlb
        .sql()
        .map_err(|e| error::Error::internal_err(e.to_string()))?;
    let rows = sqlx::query_as::<_, HttpTrigger>(&sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::JsonResult<HttpTrigger> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();
    let trigger = sqlx::query_as!(
        HttpTrigger,
        r#"
        SELECT 
            workspace_id, 
            path, 
            route_path, 
            route_path_key,
            workspaced_route,
            script_path, 
            is_flow, 
            http_method as "http_method: _", 
            edited_by, 
            email, 
            edited_at, 
            extra_perms, 
            is_async, 
            authentication_method as "authentication_method: _", 
            static_asset_config as "static_asset_config: _", 
            is_static_website,
            authentication_resource_path,
            wrap_body,
            raw_string
        FROM 
            http_trigger
        WHERE 
            workspace_id = $1 AND 
            path = $2
        "#,
        w_id,
        path,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let trigger = not_found_if_none(trigger, "Trigger", path)?;

    Ok(Json(trigger))
}

fn validate_authentication_method(
    authentication_method: AuthenticationMethod,
    raw_string: Option<bool>,
) -> error::Result<()> {
    match (authentication_method, raw_string) {
        (AuthenticationMethod::CustomScript, raw) if !raw.unwrap_or(false) == true => {
            return Err(Error::BadRequest(
                "To use custom script authentication, please enable the raw body option."
                    .to_string(),
            ));
        }
        _ => {}
    }

    Ok(())
}

async fn increase_trigger_version_and_commit(mut tx: PgTransaction<'_>) -> error::Result<()> {
    sqlx::query!("SELECT nextval('http_trigger_version_seq')",)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

async fn create_trigger(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(ct): Json<NewTrigger>,
) -> error::Result<(StatusCode, String)> {
    require_admin(authed.is_admin, &authed.username)?;

    if !VALID_ROUTE_PATH_RE.is_match(&ct.route_path) {
        return Err(error::Error::BadRequest("Invalid route path".to_string()));
    }

    validate_authentication_method(ct.authentication_method, ct.raw_string)?;

    // route path key is extracted from the route path to check for uniqueness
    // it replaces /?:{key} with :key
    // it will also remove the leading / if present, not an issue as we only allow : after slashes
    let route_path_key = ROUTE_PATH_KEY_RE.replace_all(&ct.route_path, ":key");
    let exists = route_path_key_exists(
        &route_path_key,
        &ct.http_method,
        &w_id,
        None,
        ct.workspaced_route,
        &db,
    )
    .await?;
    if exists {
        return Err(error::Error::BadRequest(
            "A route already exists with this path".to_string(),
        ));
    }

    if *CLOUD_HOSTED && (ct.is_static_website || ct.static_asset_config.is_some()) {
        return Err(error::Error::BadRequest(
            "Static website and static asset are not supported on cloud".to_string(),
        ));
    }

    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        r#"
        INSERT INTO http_trigger (
            workspace_id, 
            path, 
            route_path, 
            route_path_key,
            workspaced_route,
            authentication_resource_path,
            wrap_body,
            raw_string,
            script_path, 
            is_flow, 
            is_async, 
            authentication_method, 
            http_method, 
            static_asset_config, 
            edited_by, 
            email, 
            edited_at, 
            is_static_website
        ) 
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, now(), $17
        )
        "#,
        w_id,
        ct.path,
        ct.route_path,
        &route_path_key,
        ct.workspaced_route,
        ct.authentication_resource_path,
        ct.wrap_body.unwrap_or(false),
        ct.raw_string.unwrap_or(false),
        ct.script_path,
        ct.is_flow,
        ct.is_async,
        ct.authentication_method as _,
        ct.http_method as _,
        ct.static_asset_config as _,
        &authed.username,
        &authed.email,
        ct.is_static_website
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "http_triggers.create",
        ActionKind::Create,
        &w_id,
        Some(ct.path.as_str()),
        None,
    )
    .await?;

    increase_trigger_version_and_commit(tx).await?;

    Ok((StatusCode::CREATED, format!("{}", ct.path)))
}

async fn update_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ct): Json<EditTrigger>,
) -> error::Result<String> {
    let path = path.to_path();
    if *CLOUD_HOSTED && (ct.is_static_website || ct.static_asset_config.is_some()) {
        return Err(error::Error::BadRequest(
            "Static website and static asset are not supported on cloud".to_string(),
        ));
    }

    validate_authentication_method(ct.authentication_method, ct.raw_string)?;

    let mut tx;
    if authed.is_admin {
        let Some(route_path) = ct.route_path else {
            return Err(error::Error::BadRequest(
                "route_path is required".to_string(),
            ));
        };

        if !VALID_ROUTE_PATH_RE.is_match(&route_path) {
            return Err(error::Error::BadRequest("Invalid route path".to_string()));
        }

        let route_path_key = ROUTE_PATH_KEY_RE.replace_all(&route_path, ":key");

        let exists = route_path_key_exists(
            &route_path_key,
            &ct.http_method,
            &w_id,
            Some(&path),
            ct.workspaced_route,
            &db,
        )
        .await?;
        if exists {
            return Err(error::Error::BadRequest(
                "A route already exists with this path".to_string(),
            ));
        }

        tx = user_db.begin(&authed).await?;
        sqlx::query!(
            r#"
            UPDATE 
                http_trigger 
            SET 
                route_path = $1, 
                route_path_key = $2, 
                workspaced_route = $3,
                wrap_body = $4,
                raw_string = $5,
                authentication_resource_path = $6,
                script_path = $7, 
                path = $8, 
                is_flow = $9, 
                http_method = $10, 
                static_asset_config = $11, 
                edited_by = $12, 
                email = $13, 
                is_async = $14, 
                authentication_method = $15, 
                edited_at = now(), 
                is_static_website = $16
            WHERE 
                workspace_id = $17 AND 
                path = $18
            "#,
            route_path,
            &route_path_key,
            ct.workspaced_route,
            ct.wrap_body,
            ct.raw_string,
            ct.authentication_resource_path,
            ct.script_path,
            ct.path,
            ct.is_flow,
            ct.http_method as _,
            ct.static_asset_config as _,
            &authed.username,
            &authed.email,
            ct.is_async,
            ct.authentication_method as _,
            ct.is_static_website,
            w_id,
            path,
        )
        .execute(&mut *tx)
        .await?;
    } else {
        tx = user_db.begin(&authed).await?;
        sqlx::query!(
            r#"
            UPDATE 
                http_trigger 
            SET 
                workspaced_route = $1,
                wrap_body = $2,
                raw_string = $3,
                authentication_resource_path = $4,
                script_path = $5, 
                path = $6, 
                is_flow = $7, 
                http_method = $8, 
                static_asset_config = $9, 
                edited_by = $10, 
                email = $11, 
                is_async = $12, 
                authentication_method = $13, 
                edited_at = now(), 
                is_static_website = $14
            WHERE 
                workspace_id = $15 AND 
                path = $16
            "#,
            ct.workspaced_route,
            ct.wrap_body,
            ct.raw_string,
            ct.authentication_resource_path,
            ct.script_path,
            ct.path,
            ct.is_flow,
            ct.http_method as _,
            ct.static_asset_config as _,
            &authed.username,
            &authed.email,
            ct.is_async,
            ct.authentication_method as _,
            ct.is_static_website,
            w_id,
            path,
        )
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "http_triggers.update",
        ActionKind::Create,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    increase_trigger_version_and_commit(tx).await?;

    Ok(path.to_string())
}

async fn delete_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "DELETE FROM http_trigger 
        WHERE workspace_id = $1 
          AND path = $2",
        w_id,
        path
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "http_triggers.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    increase_trigger_version_and_commit(tx).await?;

    Ok(format!("HTTP trigger {path} deleted"))
}

async fn exists_trigger(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(
            SELECT 1 FROM http_trigger 
            WHERE path = $1 AND workspace_id = $2
        )",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

#[derive(Deserialize)]
struct RouteExists {
    route_path: String,
    http_method: HttpMethod,
    trigger_path: Option<String>,
    workspaced_route: Option<bool>,
}

async fn route_path_key_exists(
    route_path_key: &str,
    http_method: &HttpMethod,
    w_id: &str,
    trigger_path: Option<&str>,
    workspaced_route: Option<bool>,
    db: &DB,
) -> error::Result<bool> {
    let exists = if *CLOUD_HOSTED {
        sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 
                FROM http_trigger 
                WHERE 
                    route_path_key = $1
                    AND workspace_id = $2 
                    AND http_method = $3 
                    AND ($4::TEXT IS NULL OR path != $4)
            )
            "#,
            &route_path_key,
            w_id,
            http_method as &HttpMethod,
            trigger_path
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false)
    } else {
        let route_path_key = match workspaced_route {
            Some(true) => Cow::Owned(format!("{}/{}", w_id, route_path_key.trim_matches('/'))),
            _ => Cow::Borrowed(route_path_key),
        };
        sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 
                FROM http_trigger 
                WHERE 
                    ((workspaced_route IS TRUE AND workspace_id || '/' || route_path_key = $1) 
                    OR (workspaced_route IS FALSE AND route_path_key = $1))
                    AND http_method = $2 
                    AND ($3::TEXT IS NULL OR path != $3)
            )
            "#,
            &route_path_key,
            http_method as &HttpMethod,
            trigger_path
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false)
    };

    Ok(exists)
}

async fn exists_route(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(RouteExists { route_path, http_method, trigger_path, workspaced_route }): Json<
        RouteExists,
    >,
) -> JsonResult<bool> {
    let route_path_key = ROUTE_PATH_KEY_RE.replace_all(route_path.as_str(), ":key");

    let exists = route_path_key_exists(
        &route_path_key,
        &http_method,
        &w_id,
        trigger_path.as_deref(),
        workspaced_route,
        &db,
    )
    .await?;

    Ok(Json(exists))
}

#[derive(Debug, Deserialize, Clone)]
pub struct TriggerRoute {
    path: String,
    script_path: String,
    is_flow: bool,
    route_path: String,
    workspace_id: String,
    is_async: bool,
    authentication_method: AuthenticationMethod,
    edited_by: String,
    email: String,
    static_asset_config: Option<sqlx::types::Json<S3Object>>,
    is_static_website: bool,
    authentication_resource_path: Option<String>,
    workspaced_route: bool,
    wrap_body: bool,
    raw_string: bool,
}

pub struct RoutersCache {
    routers: HashMap<HttpMethod, matchit::Router<TriggerRoute>>,
    version: i64,
}

lazy_static::lazy_static! {
    static ref HTTP_ACCESS_CACHE: Cache<(String, String, ApiAuthed), ExpiringCacheEntry<()>> = Cache::new(100);
    static ref HTTP_AUTH_CACHE: Cache<(String, String, ApiAuthed), ExpiringCacheEntry<crate::http_trigger_auth::AuthenticationMethod>> = Cache::new(100);

    static ref HTTP_ROUTERS_CACHE: RwLock<RoutersCache> = RwLock::new(RoutersCache {
        routers: HashMap::new(),
        version: 0,
    });
}

pub async fn refresh_routers_loop(
    db: &DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    match refresh_routers(db).await {
        Ok(_) => {
            tracing::info!("Loaded HTTP routers");
        }
        Err(err) => {
            tracing::error!("Error loading HTTP routers: {err:#}");
        }
    };
    let db = db.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = killpill_rx.recv() => {
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                    match refresh_routers(&db).await {
                        Ok((true, _)) => {
                            tracing::info!("Refreshed HTTP routers");
                        }
                        Err(err) => {
                            tracing::error!("Error refreshing HTTP routers: {err:#}");
                        }
                        _ => {}
                    }
                }
            }
        }
    });
}

pub async fn refresh_routers(db: &DB) -> Result<(bool, RwLockReadGuard<'_, RoutersCache>), Error> {
    let version = sqlx::query_scalar!("SELECT last_value FROM http_trigger_version_seq",)
        .fetch_one(db)
        .await?;
    let routers_cache = HTTP_ROUTERS_CACHE.read().await;
    if routers_cache.version == 0 || version > routers_cache.version {
        drop(routers_cache);
        let mut routers = HashMap::new();

        for http_method in [
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Patch,
            HttpMethod::Delete,
        ] {
            let triggers = sqlx::query_as!(
                TriggerRoute,
                r#"
                    SELECT 
                        path, 
                        script_path, 
                        is_flow, 
                        route_path, 
                        authentication_resource_path,
                        workspace_id, 
                        is_async, 
                        authentication_method  AS "authentication_method: _", 
                        edited_by, 
                        email, 
                        static_asset_config AS "static_asset_config: _",
                        wrap_body,
                        raw_string,
                        workspaced_route,
                        is_static_website
                    FROM 
                        http_trigger 
                    WHERE 
                        http_method = $1
                    "#,
                &http_method as &HttpMethod
            )
            .fetch_all(db)
            .await?;

            let mut router = matchit::Router::new();

            for trigger in triggers {
                let full_path = if trigger.workspaced_route || *CLOUD_HOSTED {
                    format!("/{}/{}", trigger.workspace_id, trigger.route_path)
                } else {
                    format!("/{}", trigger.route_path)
                };

                if trigger.is_static_website {
                    router
                        .insert(format!("{}/*wm_subpath", full_path), trigger.clone())
                        .unwrap_or_else(|e| {
                            tracing::warn!(
                                "Failed to consider http trigger route {}/*wm_subpath: {:?}",
                                full_path,
                                e,
                            );
                        });
                }
                router
                    .insert(full_path.clone(), trigger.clone())
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            "Failed to consider http trigger route {}: {:?}",
                            full_path,
                            e,
                        );
                    });
            }

            routers.insert(http_method, router);
        }

        let mut routers_cache = HTTP_ROUTERS_CACHE.write().await;
        *routers_cache = RoutersCache { routers, version };

        Ok((true, routers_cache.downgrade()))
    } else {
        tracing::debug!("No HTTP routers refresh needed");
        Ok((false, routers_cache))
    }
}

async fn get_http_route_trigger(
    route_path: &str,
    auth_cache: &Arc<AuthCache>,
    token: Option<&String>,
    db: &DB,
    user_db: UserDB,
    method: &http::Method,
) -> error::Result<(TriggerRoute, String, HashMap<String, String>, ApiAuthed)> {
    let http_method: HttpMethod = method.try_into()?;

    let requested_path = format!("/{}", route_path);

    let routers_cache = HTTP_ROUTERS_CACHE.read().await;

    let routers_cache = if routers_cache.routers.is_empty() {
        tracing::warn!("HTTP routers are not loaded, loading from db");
        let (_, routers_cache) = refresh_routers(db).await?;
        routers_cache
    } else {
        routers_cache
    };

    let router = routers_cache
        .routers
        .get(&http_method)
        .ok_or(error::Error::internal_err(
            "HTTP routers could not be loaded".to_string(),
        ))?;

    let trigger_match = router.at(requested_path.as_str()).ok();

    let matchit::Match { value: trigger, params } =
        not_found_if_none(trigger_match, "Trigger", requested_path.as_str())?;

    let params: HashMap<String, String> = params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let username_override = if let AuthenticationMethod::Windmill = trigger.authentication_method {
        let opt_authed = if let Some(token) = token {
            auth_cache
                .get_authed(Some(trigger.workspace_id.clone()), token)
                .await
        } else {
            None
        };
        if let Some(authed) = opt_authed {
            // check that the user has access to the trigger
            let cache_key = (
                trigger.workspace_id.clone(),
                trigger.path.clone(),
                authed.clone(),
            );
            let exists = match HTTP_ACCESS_CACHE.get(&cache_key) {
                Some(cache_entry) if cache_entry.expiry > std::time::Instant::now() => {
                    tracing::debug!("HTTP access cache hit for trigger {}", trigger.path);
                    true
                }
                _ => {
                    tracing::debug!("HTTP access cache miss for trigger {}", trigger.path);
                    let mut tx = user_db.begin(&authed).await?;
                    let exists = sqlx::query_scalar!(
                        r#"
                        SELECT EXISTS(
                            SELECT 1
                            FROM 
                                http_trigger 
                            WHERE 
                                workspace_id = $1 AND 
                                path = $2
                        )
                        "#,
                        trigger.workspace_id,
                        trigger.path
                    )
                    .fetch_one(&mut *tx)
                    .await?
                    .unwrap_or(false);
                    if exists {
                        HTTP_ACCESS_CACHE.insert(
                            cache_key,
                            ExpiringCacheEntry {
                                value: (),
                                expiry: std::time::Instant::now()
                                    + std::time::Duration::from_secs(10),
                            },
                        );
                    }
                    exists
                }
            };
            if exists {
                Some(authed.display_username().to_owned())
            } else {
                return Err(error::Error::NotAuthorized("Unauthorized".to_string()));
            }
        } else {
            return Err(error::Error::NotAuthorized(
                "Requires authentication".to_string(),
            ));
        }
    } else {
        None
    };

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        &db,
        Some(username_override.unwrap_or(format!("http-{}", trigger.path))),
    )
    .await?;

    Ok((trigger.clone(), route_path.to_string(), params, authed))
}

async fn route_job(
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(auth_cache): Extension<Arc<AuthCache>>,
    OptTokened { token }: OptTokened,
    Path(route_path): Path<StripPath>,
    headers: HeaderMap,
    args: RawHttpTriggerArgs,
) -> Result<impl IntoResponse, Response> {
    let route_path = route_path.to_path().trim_end_matches("/");
    let (trigger, called_path, params, authed) = get_http_route_trigger(
        route_path,
        &auth_cache,
        token.as_ref(),
        &db,
        user_db.clone(),
        &args.0.metadata.method,
    )
    .await
    .map_err(|e| e.into_response())?;

    let args = args
        .process_args(
            &authed,
            &db,
            &trigger.workspace_id,
            match trigger.authentication_method {
                AuthenticationMethod::CustomScript | AuthenticationMethod::Signature => true,
                _ => false,
            },
        )
        .await
        .map_err(|e| e.into_response())?;

    match trigger.authentication_method {
        AuthenticationMethod::None
        | AuthenticationMethod::Windmill
        | AuthenticationMethod::CustomScript => {}
        _ => {
            let resource_path = match trigger.authentication_resource_path {
                Some(resource_path) => resource_path,
                None => {
                    return Err(Error::BadRequest(
                        "Missing authentication resource path".to_string(),
                    )
                    .into_response())
                }
            };

            let cache_key = (
                trigger.workspace_id.clone(),
                resource_path.clone(),
                authed.clone(),
            );

            let authentication_method = match HTTP_AUTH_CACHE.get(&cache_key) {
                Some(cache_entry) if cache_entry.expiry > std::time::Instant::now() => {
                    tracing::debug!("HTTP auth method cache hit for trigger {}", trigger.path);
                    cache_entry.value
                }
                _ => {
                    tracing::debug!("HTTP auth method cache miss for trigger {}", trigger.path);
                    let auth_method = try_get_resource_from_db_as::<
                        crate::http_trigger_auth::AuthenticationMethod,
                    >(
                        authed.clone(),
                        Some(user_db.clone()),
                        &db,
                        &resource_path,
                        &trigger.workspace_id,
                    )
                    .await
                    .map_err(|e| e.into_response())?;
                    HTTP_AUTH_CACHE.insert(
                        cache_key,
                        ExpiringCacheEntry {
                            value: auth_method.clone(),
                            expiry: std::time::Instant::now() + std::time::Duration::from_secs(60),
                        },
                    );
                    auth_method
                }
            };

            let raw_payload = args.0.metadata.raw_string.as_ref();

            let response = authentication_method
                .authenticate_http_request(&headers, raw_payload)
                .map_err(|e| e.into_response())?;

            if let Some(response) = response {
                return Ok(response);
            }
        }
    }

    #[cfg(not(feature = "parquet"))]
    if trigger.static_asset_config.is_some() {
        return Err(error::Error::internal_err(
            "Static asset configuration is not supported in this build".to_string(),
        )
        .into_response());
    }

    #[cfg(feature = "parquet")]
    if let Some(sqlx::types::Json(config)) = trigger.static_asset_config {
        let build_static_response_f = async {
            let (_, s3_resource_opt) = get_workspace_s3_resource(
                &authed,
                &db,
                None,
                &"NO_TOKEN".to_string(), // no token is provided in this case
                &trigger.workspace_id,
                config.storage,
            )
            .await?;
            let s3_resource = s3_resource_opt.ok_or(error::Error::internal_err(
                "No files storage resource defined at the workspace level".to_string(),
            ))?;
            let s3_client = build_object_store_client(&s3_resource).await?;

            let path = if trigger.is_static_website {
                let subpath = params
                    .get("wm_subpath")
                    .cloned()
                    .unwrap_or("index.html".to_string());
                tracing::info!("subpath: {}", subpath);
                format!("{}/{}", config.s3.trim_end_matches('/'), subpath)
            } else {
                config.s3.clone()
            };
            let path = object_store::path::Path::from(path);
            let s3_object = s3_client.get(&path).await;

            let s3_object = match s3_object {
                Err(object_store::Error::NotFound { .. }) if trigger.is_static_website => {
                    // fallback to index.html if the file is not found
                    let path = object_store::path::Path::from(format!(
                        "{}/index.html",
                        config.s3.trim_end_matches('/')
                    ));
                    s3_client.get(&path).await
                }
                r => r,
            };

            let s3_object = s3_object.map_err(|err| {
                tracing::warn!("Error retrieving file from S3: {:?}", err);
                error::Error::internal_err(format!("Error retrieving file: {}", err.to_string()))
            })?;

            let mut response_headers = http::HeaderMap::new();
            if let Some(ref e_tag) = s3_object.meta.e_tag {
                if let Some(if_none_match) = headers.get(IF_NONE_MATCH) {
                    if if_none_match == e_tag {
                        return Ok::<_, error::Error>((
                            StatusCode::NOT_MODIFIED,
                            response_headers,
                            axum::body::Body::empty(),
                        ));
                    }
                }
                if let Ok(e_tag) = e_tag.parse() {
                    response_headers.insert("etag", e_tag);
                }
            }
            response_headers.insert(
                "content-type",
                s3_object
                    .attributes
                    .get(&object_store::Attribute::ContentType)
                    .map(|s| s.parse().ok())
                    .flatten()
                    .unwrap_or("application/octet-stream".parse().unwrap()),
            );
            if !trigger.is_static_website {
                response_headers.insert(
                    "content-disposition",
                    config.filename.as_ref().map_or_else(
                        || {
                            s3_object
                                .attributes
                                .get(&object_store::Attribute::ContentDisposition)
                                .map(|s| s.parse().ok())
                                .flatten()
                                .unwrap_or("inline".parse().unwrap())
                        },
                        |filename| {
                            format!("inline; filename=\"{}\"", filename)
                                .parse()
                                .unwrap_or("inline".parse().unwrap())
                        },
                    ),
                );
            }

            let body_stream = axum::body::Body::from_stream(s3_object.into_stream());
            Ok::<_, error::Error>((StatusCode::OK, response_headers, body_stream))
        };
        match build_static_response_f.await {
            Ok((status, headers, body_stream)) => {
                return Ok((status, headers, body_stream).into_response())
            }
            Err(e) => return Err(e.into_response()),
        }
    }

    let runnable_format = get_runnable_format(
        if trigger.is_flow {
            RunnableId::from_flow_path(&trigger.script_path)
        } else {
            RunnableId::from_script_path(&trigger.script_path)
        },
        &trigger.workspace_id,
        &db,
        &TriggerKind::Http,
    )
    .await
    .map_err(|e| e.into_response())?;

    let args = args
        .to_args_from_format(
            &trigger.route_path,
            &called_path,
            &params,
            runnable_format,
            trigger.wrap_body,
        )
        .map_err(|e| e.into_response())?;

    let run_query = RunJobQuery::default();

    let response = if trigger.is_flow {
        if trigger.is_async {
            run_flow_by_path_inner(
                authed,
                db,
                user_db,
                trigger.workspace_id.clone(),
                StripPath(trigger.script_path.to_owned()),
                run_query,
                args,
            )
            .await
            .into_response()
        } else {
            run_wait_result_flow_by_path_internal(
                db,
                run_query,
                StripPath(trigger.script_path.to_owned()),
                authed,
                user_db,
                args,
                trigger.workspace_id.clone(),
            )
            .await
            .into_response()
        }
    } else {
        if trigger.is_async {
            run_script_by_path_inner(
                authed,
                db,
                user_db,
                trigger.workspace_id.clone(),
                StripPath(trigger.script_path.to_owned()),
                run_query,
                args,
            )
            .await
            .into_response()
        } else {
            run_wait_result_script_by_path_internal(
                db,
                run_query,
                StripPath(trigger.script_path.to_owned()),
                authed,
                user_db,
                trigger.workspace_id.clone(),
                args,
            )
            .await
            .into_response()
        }
    };

    Ok(response)
}
