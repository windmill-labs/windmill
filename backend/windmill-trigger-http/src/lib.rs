use std::collections::HashMap;

use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use sqlx::{types::Json as SqlxJson, FromRow};
use tokio::sync::{RwLock, RwLockReadGuard};
use windmill_common::{
    error::{Error, Result},
    flows::Retry,
    utils::ExpiringCacheEntry,
    worker::CLOUD_HOSTED,
    DB,
};
use windmill_types::s3::S3Object;

use windmill_api_auth::ApiAuthed;
use windmill_trigger::TriggerMode;

pub mod handler;
pub mod http_trigger_auth;

lazy_static::lazy_static! {
    pub static ref HTTP_ACCESS_CACHE: Cache<(String, String, ApiAuthed), ExpiringCacheEntry<()>> = Cache::new(100);
    pub static ref HTTP_AUTH_CACHE: Cache<(String, String, ApiAuthed), ExpiringCacheEntry<http_trigger_auth::AuthenticationMethod>> = Cache::new(100);

    pub static ref HTTP_ROUTERS_CACHE: RwLock<RoutersCache> = RwLock::new(RoutersCache {
        routers: HashMap::new(),
        version: 0,
    });
}

#[derive(Debug, Deserialize, Clone)]
pub struct TriggerRoute {
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub route_path: String,
    pub workspace_id: String,
    pub request_type: RequestType,
    pub authentication_method: AuthenticationMethod,
    pub edited_by: String,
    pub email: String,
    pub static_asset_config: Option<sqlx::types::Json<S3Object>>,
    pub is_static_website: bool,
    pub authentication_resource_path: Option<String>,
    pub workspaced_route: bool,
    pub wrap_body: bool,
    pub raw_string: bool,
    pub error_handler_path: Option<String>,
    pub error_handler_args: Option<sqlx::types::Json<HashMap<String, serde_json::Value>>>,
    pub retry: Option<sqlx::types::Json<Retry>>,
    pub mode: TriggerMode,
}

pub struct RoutersCache {
    pub routers: HashMap<HttpMethod, matchit::Router<TriggerRoute>>,
    pub version: i64,
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

#[derive(Serialize, Deserialize, sqlx::Type, Debug, Clone, Copy, PartialEq)]
#[sqlx(type_name = "REQUEST_TYPE", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RequestType {
    Sync,
    Async,
    SyncSse,
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
    pub request_type: RequestType,
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

#[derive(Debug, Clone, Serialize)]
pub struct HttpConfigRequest {
    #[serde(default)]
    pub route_path: String,
    pub request_type: RequestType,
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

#[derive(Deserialize)]
struct HttpConfigRequestHelper {
    #[serde(default)]
    route_path: String,
    request_type: Option<RequestType>,
    is_async: Option<bool>,
    authentication_method: AuthenticationMethod,
    http_method: HttpMethod,
    summary: Option<String>,
    description: Option<String>,
    static_asset_config: Option<SqlxJson<S3Object>>,
    is_static_website: bool,
    authentication_resource_path: Option<String>,
    workspaced_route: Option<bool>,
    wrap_body: Option<bool>,
    raw_string: Option<bool>,
}

impl<'de> Deserialize<'de> for HttpConfigRequest {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = HttpConfigRequestHelper::deserialize(deserializer)?;

        // Determine request_type with backward compatibility
        let request_type = if let Some(mode) = helper.request_type {
            mode
        } else if let Some(is_async) = helper.is_async {
            if is_async {
                RequestType::Async
            } else {
                RequestType::Sync
            }
        } else {
            RequestType::Sync
        };

        Ok(HttpConfigRequest {
            route_path: helper.route_path,
            request_type,
            authentication_method: helper.authentication_method,
            http_method: helper.http_method,
            summary: helper.summary,
            description: helper.description,
            static_asset_config: helper.static_asset_config,
            is_static_website: helper.is_static_website,
            authentication_resource_path: helper.authentication_resource_path,
            workspaced_route: helper.workspaced_route,
            wrap_body: helper.wrap_body,
            raw_string: helper.raw_string,
        })
    }
}

// Regex patterns for route validation
lazy_static::lazy_static! {
    // Matches named params like :id or wildcards like :* or *
    pub static ref ROUTE_PATH_KEY_RE: regex::Regex = regex::Regex::new(r"(/)?(:|\*)[-\w]+").unwrap();
    pub static ref VALID_ROUTE_PATH_RE: regex::Regex = regex::Regex::new(r"^(\*[-\w]+$|:?[-\w]+)(/(\*[-\w]+$|:?[-\w]+))*$").unwrap();
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
                        request_type AS "request_type: _",
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
                        retry as "retry: _",
                        mode as "mode: _"
                    FROM
                        http_trigger
                    WHERE
                        http_method = $1 AND
                        (mode = 'enabled'::TRIGGER_MODE OR mode = 'suspended'::TRIGGER_MODE)
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

pub struct HttpTrigger;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_type_backward_compatibility() {
        // Test with new request_type field
        let json_new = r#"{
            "route_path": "/test",
            "request_type": "sync_sse",
            "authentication_method": "none",
            "http_method": "get",
            "is_static_website": false
        }"#;
        let config: HttpConfigRequest = serde_json::from_str(json_new).unwrap();
        assert_eq!(config.request_type, RequestType::SyncSse);

        // Test with legacy is_async = true
        let json_legacy_async = r#"{
            "route_path": "/test",
            "is_async": true,
            "authentication_method": "none",
            "http_method": "get",
            "is_static_website": false
        }"#;
        let config: HttpConfigRequest = serde_json::from_str(json_legacy_async).unwrap();
        assert_eq!(config.request_type, RequestType::Async);

        // Test with legacy is_async = false
        let json_legacy_sync = r#"{
            "route_path": "/test",
            "is_async": false,
            "authentication_method": "none",
            "http_method": "get",
            "is_static_website": false
        }"#;
        let config: HttpConfigRequest = serde_json::from_str(json_legacy_sync).unwrap();
        assert_eq!(config.request_type, RequestType::Sync);

        // Test with neither field (default to sync)
        let json_default = r#"{
            "route_path": "/test",
            "authentication_method": "none",
            "http_method": "get",
            "is_static_website": false
        }"#;
        let config: HttpConfigRequest = serde_json::from_str(json_default).unwrap();
        assert_eq!(config.request_type, RequestType::Sync);

        // Test that request_type takes precedence over is_async
        let json_both = r#"{
            "route_path": "/test",
            "request_type": "sync_sse",
            "is_async": true,
            "authentication_method": "none",
            "http_method": "get",
            "is_static_website": false
        }"#;
        let config: HttpConfigRequest = serde_json::from_str(json_both).unwrap();
        assert_eq!(config.request_type, RequestType::SyncSse);
    }

    // --- HttpMethod ---

    #[test]
    fn test_http_method_from_http_get() {
        let method = HttpMethod::try_from(&http::Method::GET).unwrap();
        assert_eq!(method, HttpMethod::Get);
    }

    #[test]
    fn test_http_method_from_http_post() {
        let method = HttpMethod::try_from(&http::Method::POST).unwrap();
        assert_eq!(method, HttpMethod::Post);
    }

    #[test]
    fn test_http_method_from_http_put() {
        let method = HttpMethod::try_from(&http::Method::PUT).unwrap();
        assert_eq!(method, HttpMethod::Put);
    }

    #[test]
    fn test_http_method_from_http_delete() {
        let method = HttpMethod::try_from(&http::Method::DELETE).unwrap();
        assert_eq!(method, HttpMethod::Delete);
    }

    #[test]
    fn test_http_method_from_http_patch() {
        let method = HttpMethod::try_from(&http::Method::PATCH).unwrap();
        assert_eq!(method, HttpMethod::Patch);
    }

    #[test]
    fn test_http_method_unsupported() {
        let result = HttpMethod::try_from(&http::Method::HEAD);
        assert!(result.is_err());
    }

    #[test]
    fn test_http_method_options_unsupported() {
        let result = HttpMethod::try_from(&http::Method::OPTIONS);
        assert!(result.is_err());
    }

    // --- HttpMethod serde ---

    #[test]
    fn test_http_method_serde_roundtrip() {
        for method in [HttpMethod::Get, HttpMethod::Post, HttpMethod::Put, HttpMethod::Delete, HttpMethod::Patch] {
            let json = serde_json::to_value(method).unwrap();
            let deserialized: HttpMethod = serde_json::from_value(json).unwrap();
            assert_eq!(method, deserialized);
        }
    }

    #[test]
    fn test_http_method_serialize_lowercase() {
        assert_eq!(serde_json::to_value(HttpMethod::Get).unwrap(), "get");
        assert_eq!(serde_json::to_value(HttpMethod::Post).unwrap(), "post");
    }

    // --- RequestType serde ---

    #[test]
    fn test_request_type_serde_roundtrip() {
        for rt in [RequestType::Sync, RequestType::Async, RequestType::SyncSse] {
            let json = serde_json::to_value(rt).unwrap();
            let deserialized: RequestType = serde_json::from_value(json).unwrap();
            assert_eq!(rt, deserialized);
        }
    }

    #[test]
    fn test_request_type_serialize_values() {
        assert_eq!(serde_json::to_value(RequestType::Sync).unwrap(), "sync");
        assert_eq!(serde_json::to_value(RequestType::Async).unwrap(), "async");
        assert_eq!(serde_json::to_value(RequestType::SyncSse).unwrap(), "sync_sse");
    }

    // --- AuthenticationMethod serde ---

    #[test]
    fn test_authentication_method_serde_roundtrip() {
        for method in [
            AuthenticationMethod::None,
            AuthenticationMethod::Windmill,
            AuthenticationMethod::ApiKey,
            AuthenticationMethod::BasicHttp,
            AuthenticationMethod::CustomScript,
            AuthenticationMethod::Signature,
        ] {
            let json = serde_json::to_value(method).unwrap();
            let deserialized: AuthenticationMethod = serde_json::from_value(json).unwrap();
            assert_eq!(method, deserialized);
        }
    }

    // --- validate_authentication_method ---

    #[test]
    fn test_validate_auth_none_ok() {
        assert!(validate_authentication_method(AuthenticationMethod::None, None).is_ok());
    }

    #[test]
    fn test_validate_auth_windmill_ok() {
        assert!(validate_authentication_method(AuthenticationMethod::Windmill, None).is_ok());
    }

    #[test]
    fn test_validate_auth_custom_script_requires_raw() {
        assert!(validate_authentication_method(AuthenticationMethod::CustomScript, None).is_err());
        assert!(validate_authentication_method(AuthenticationMethod::CustomScript, Some(false)).is_err());
        assert!(validate_authentication_method(AuthenticationMethod::CustomScript, Some(true)).is_ok());
    }

    #[test]
    fn test_validate_auth_signature_without_raw_ok() {
        assert!(validate_authentication_method(AuthenticationMethod::Signature, None).is_ok());
    }

    // --- Route path regex ---

    #[test]
    fn test_valid_route_path() {
        assert!(VALID_ROUTE_PATH_RE.is_match("users"));
        assert!(VALID_ROUTE_PATH_RE.is_match("users/:id"));
        assert!(VALID_ROUTE_PATH_RE.is_match("api/v1/users"));
        assert!(VALID_ROUTE_PATH_RE.is_match("api/v1/:id"));
        assert!(VALID_ROUTE_PATH_RE.is_match("files/*path"));
    }

    #[test]
    fn test_invalid_route_path() {
        assert!(!VALID_ROUTE_PATH_RE.is_match(""));
        assert!(!VALID_ROUTE_PATH_RE.is_match("/leading-slash"));
    }

    #[test]
    fn test_route_path_key_regex() {
        assert!(ROUTE_PATH_KEY_RE.is_match("/:id"));
        assert!(ROUTE_PATH_KEY_RE.is_match("/*path"));
        assert!(ROUTE_PATH_KEY_RE.is_match("/users/:userId/posts/:postId"));
    }

    // --- HttpConfig deserialization ---

    #[test]
    fn test_http_config_request_full() {
        let json = r#"{
            "route_path": "api/v1/users",
            "request_type": "async",
            "authentication_method": "api_key",
            "http_method": "post",
            "is_static_website": false,
            "workspaced_route": true,
            "wrap_body": true,
            "raw_string": false
        }"#;
        let config: HttpConfigRequest = serde_json::from_str(json).unwrap();
        assert_eq!(config.route_path, "api/v1/users");
        assert_eq!(config.request_type, RequestType::Async);
        assert_eq!(config.authentication_method, AuthenticationMethod::ApiKey);
        assert_eq!(config.http_method, HttpMethod::Post);
        assert_eq!(config.workspaced_route, Some(true));
        assert_eq!(config.wrap_body, Some(true));
    }

    #[test]
    fn test_http_config_request_minimal() {
        let json = r#"{
            "authentication_method": "none",
            "http_method": "get",
            "is_static_website": false
        }"#;
        let config: HttpConfigRequest = serde_json::from_str(json).unwrap();
        assert_eq!(config.route_path, "");
        assert_eq!(config.request_type, RequestType::Sync);
        assert!(config.workspaced_route.is_none());
        assert!(config.summary.is_none());
    }
}
