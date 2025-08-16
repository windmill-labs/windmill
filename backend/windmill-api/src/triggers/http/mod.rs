use std::collections::HashMap;

use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, FromRow, PgConnection};
use tokio::sync::{RwLock, RwLockReadGuard};
use windmill_common::{
    error::{Error, Result},
    flows::Retry,
    s3_helpers::S3Object,
    worker::CLOUD_HOSTED,
    DB,
};

use crate::{db::ApiAuthed, utils::ExpiringCacheEntry};

pub mod handler;
pub mod http_trigger_args;
pub mod http_trigger_auth;

lazy_static::lazy_static! {
    static ref HTTP_ACCESS_CACHE: Cache<(String, String, ApiAuthed), ExpiringCacheEntry<()>> = Cache::new(100);
    static ref HTTP_AUTH_CACHE: Cache<(String, String, ApiAuthed), ExpiringCacheEntry<http_trigger_auth::AuthenticationMethod>> = Cache::new(100);

    static ref HTTP_ROUTERS_CACHE: RwLock<RoutersCache> = RwLock::new(RoutersCache {
        routers: HashMap::new(),
        version: 0,
    });
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
    error_handler_path: Option<String>,
    error_handler_args: Option<sqlx::types::Json<HashMap<String, Box<RawValue>>>>,
    retry: Option<sqlx::types::Json<Retry>>,
}

pub struct RoutersCache {
    routers: HashMap<HttpMethod, matchit::Router<TriggerRoute>>,
    version: i64,
}

#[derive(Serialize, Deserialize, sqlx::Type, Debug, Clone, Copy, Hash, Eq, PartialEq)]
#[sqlx(type_name = "HTTP_METHOD", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl TryFrom<&http::Method> for HttpMethod {
    type Error = Error;
    fn try_from(method: &http::Method) -> Result<Self> {
        match method {
            &http::Method::GET => Ok(HttpMethod::Get),
            &http::Method::POST => Ok(HttpMethod::Post),
            &http::Method::PUT => Ok(HttpMethod::Put),
            &http::Method::DELETE => Ok(HttpMethod::Delete),
            &http::Method::PATCH => Ok(HttpMethod::Patch),
            _ => Err(Error::BadRequest("Invalid HTTP method".to_string())),
        }
    }
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

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct HttpConfig {
    pub route_path: String,
    pub route_path_key: String,
    pub is_async: bool,
    pub authentication_method: AuthenticationMethod,
    pub http_method: HttpMethod,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub static_asset_config: Option<SqlxJson<S3Object>>,
    pub is_static_website: bool,
    pub authentication_resource_path: Option<String>,
    pub workspaced_route: bool,
    pub wrap_body: bool,
    pub raw_string: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewHttpConfig {
    pub route_path: String,
    pub is_async: bool,
    pub authentication_method: AuthenticationMethod,
    pub http_method: HttpMethod,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub static_asset_config: Option<SqlxJson<S3Object>>,
    pub is_static_website: bool,
    pub authentication_resource_path: Option<String>,
    pub workspaced_route: Option<bool>,
    pub wrap_body: Option<bool>,
    pub raw_string: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditHttpConfig {
    pub route_path: Option<String>,
    pub is_async: bool,
    pub authentication_method: AuthenticationMethod,
    pub http_method: HttpMethod,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub static_asset_config: Option<SqlxJson<S3Object>>,
    pub is_static_website: bool,
    pub authentication_resource_path: Option<String>,
    pub workspaced_route: Option<bool>,
    pub wrap_body: Option<bool>,
    pub raw_string: Option<bool>,
}

// Regex patterns for route validation
lazy_static::lazy_static! {
    static ref ROUTE_PATH_KEY_RE: regex::Regex = regex::Regex::new(r"/?:[-\w]+").unwrap();
    static ref VALID_ROUTE_PATH_RE: regex::Regex = regex::Regex::new(r"^:?[-\w]+(/:?[-\w]+)*$").unwrap();
}

#[derive(Deserialize)]
pub struct RouteExists {
    pub route_path: String,
    pub http_method: HttpMethod,
    pub trigger_path: Option<String>,
    pub workspaced_route: Option<bool>,
}

pub fn validate_authentication_method(
    authentication_method: AuthenticationMethod,
    raw_string: Option<bool>,
) -> Result<()> {
    match (authentication_method, raw_string) {
        (AuthenticationMethod::CustomScript, raw) if !raw.unwrap_or(false) => {
            Err(Error::BadRequest(
                "To use custom script authentication, please enable the raw body option."
                    .to_string(),
            ))
        }
        _ => Ok(()),
    }
}
        

pub async fn increase_trigger_version(tx: &mut PgConnection) -> Result<()> {
    sqlx::query!("SELECT nextval('http_trigger_version_seq')")
        .fetch_one(tx)
        .await?;
    Ok(())
}

pub async fn route_path_key_exists(
    route_path_key: &str,
    http_method: &HttpMethod,
    w_id: &str,
    trigger_path: Option<&str>,
    workspaced_route: Option<bool>,
    db: &DB,
) -> Result<bool> {
    let existing = if workspaced_route.unwrap_or(false) {
        sqlx::query_scalar!(
            "SELECT path FROM http_trigger WHERE route_path_key = $1 AND http_method = $2 AND workspace_id = $3 AND workspaced_route = true",
            route_path_key,
            http_method as _,
            w_id
        )
        .fetch_optional(db)
        .await?
    } else {
        sqlx::query_scalar!(
            "SELECT path FROM http_trigger WHERE route_path_key = $1 AND http_method = $2",
            route_path_key,
            http_method as _,
        )
        .fetch_optional(db)
        .await?
    };

    if let Some(existing_path) = existing {
        if let Some(trigger_path) = trigger_path {
            Ok(existing_path != trigger_path)
        } else {
            Ok(true)
        }
    } else {
        Ok(false)
    }
}

pub fn generate_route_path_key(route_path: &str) -> String {
    ROUTE_PATH_KEY_RE.replace_all(route_path, "/*").to_string()
}

pub async fn refresh_routers(db: &DB) -> Result<(bool, RwLockReadGuard<'_, RoutersCache>)> {
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
                        is_static_website,
                        error_handler_path,
                        error_handler_args as "error_handler_args: _",
                        retry as "retry: _"
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
                                "Failed to consider HTTP route {}/*wm_subpath: {:?}",
                                full_path,
                                e,
                            );
                        });
                }
                router
                    .insert(full_path.clone(), trigger.clone())
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to consider HTTP route {}: {:?}", full_path, e,);
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
