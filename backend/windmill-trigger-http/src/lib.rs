use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use sqlx::{types::Json as SqlxJson, FromRow};
use tokio::sync::{RwLock, RwLockReadGuard};
use windmill_common::{
    error::{Error, Result},
    flows::Retry,
    global_settings::{allows_any_origin, HTTP_ROUTE_WORKSPACED_ROUTE},
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
        invalidations: 0,
    });
}

static HTTP_ROUTERS_INVALIDATIONS: AtomicU64 = AtomicU64::new(0);

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
    pub permissioned_as: String,
    pub static_asset_config: Option<sqlx::types::Json<S3Object>>,
    pub is_static_website: bool,
    pub authentication_resource_path: Option<String>,
    pub workspaced_route: bool,
    pub wrap_body: bool,
    pub raw_string: bool,
    pub allowed_origins: Option<Vec<String>>,
    pub error_handler_path: Option<String>,
    pub error_handler_args: Option<sqlx::types::Json<HashMap<String, serde_json::Value>>>,
    pub retry: Option<sqlx::types::Json<Retry>>,
    pub mode: TriggerMode,
}

pub struct RoutersCache {
    pub routers: HashMap<HttpMethod, matchit::Router<TriggerRoute>>,
    pub version: i64,
    /// `HTTP_ROUTERS_INVALIDATIONS` as of the moment these rows were read. A rebuild that
    /// started before an invalidation publishes a count behind the current one, which is what
    /// stops it from passing its own stale rows off as covering that invalidation.
    invalidations: u64,
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
    pub allowed_origins: Option<Vec<String>>,
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
    pub allowed_origins: Option<Vec<String>>,
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
    allowed_origins: Option<Vec<String>>,
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
            allowed_origins: helper.allowed_origins,
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

/// The allowlist that governs a route: its own when it has one, otherwise the
/// instance-wide default. `None` means nothing is configured at either level, so
/// the route keeps the historical permissive behaviour.
///
/// A list containing `*` is treated as no restriction, which is how a route opts
/// out of a stricter instance default.
pub fn effective_allowed_origins<'a>(
    route_allowed_origins: Option<&'a [String]>,
    instance_default: &'a [String],
) -> Option<&'a [String]> {
    match route_allowed_origins {
        // `*` is the opt-out, including out of a stricter instance default.
        Some(list) if allows_any_origin(list) => None,
        // Any other stored list restricts, an empty one included: it allows no
        // origin at all. Falling back to the default here would make `[]` more
        // permissive than `NULL`, which is the wrong direction to fail in.
        Some(list) => Some(list),
        None => (!instance_default.is_empty() && !allows_any_origin(instance_default))
            .then_some(instance_default),
    }
}

/// Resolve the `Access-Control-Allow-Origin` value for a request, or `None` to
/// omit the header so the browser blocks the read.
///
/// The request's `Origin` is echoed back only on a match against the allowlist.
/// Reflecting it unchecked is the classic way this feature turns into no
/// restriction at all.
///
/// The comparison ignores ASCII case because a browser lowercases the scheme and
/// host it sends, so a configured `https://App.Example.com` would otherwise name
/// a real origin and still match nothing.
pub fn match_origin(
    allowed_origins: &[String],
    origin: Option<&http::HeaderValue>,
) -> Option<http::HeaderValue> {
    let origin = origin?;
    let origin_str = origin.to_str().ok()?;
    allowed_origins
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(origin_str))
        .then(|| origin.clone())
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

/// `force` rebuilds unconditionally. `nextval` on `http_trigger_version_seq` runs inside the
/// writing transaction and sequences are non-transactional, so another session can cache the
/// bumped version against still-uncommitted rows, after which every version-gated refresh is a
/// no-op. Force when reacting to a bump that could have been observed before its own rows were.
pub async fn refresh_routers(
    db: &DB,
    force: bool,
) -> Result<(bool, RwLockReadGuard<'_, RoutersCache>)> {
    let invalidations = HTTP_ROUTERS_INVALIDATIONS.load(Ordering::Relaxed);
    let version = sqlx::query_scalar!("SELECT last_value FROM http_trigger_version_seq",)
        .fetch_one(db)
        .await?;
    let routers_cache = HTTP_ROUTERS_CACHE.read().await;
    if force
        || routers_cache.version == 0
        || version > routers_cache.version
        || invalidations != routers_cache.invalidations
    {
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
                        permissioned_as,
                        static_asset_config AS "static_asset_config: _",
                        wrap_body,
                        raw_string,
                        allowed_origins,
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
            let http_route_workspaced =
                HTTP_ROUTE_WORKSPACED_ROUTE.load(std::sync::atomic::Ordering::Relaxed);

            for trigger in triggers {
                let full_path =
                    if trigger.workspaced_route || *CLOUD_HOSTED || http_route_workspaced {
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
        *routers_cache = RoutersCache { routers, version, invalidations };

        Ok((true, routers_cache.downgrade()))
    } else {
        tracing::debug!("No HTTP routers refresh needed");
        Ok((false, routers_cache))
    }
}

/// Record that the cache no longer covers everything committed, so the next refresh rebuilds
/// whatever the version says. The routes already loaded keep being served in the meantime. Use
/// after a forced refresh fails: its change is inside the cached version, so nothing else would
/// retry it.
pub fn invalidate_routers() {
    HTTP_ROUTERS_INVALIDATIONS.fetch_add(1, Ordering::Relaxed);
}

pub async fn refresh_routers_loop(
    db: &DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    match refresh_routers(db, false).await {
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
                    match refresh_routers(&db, false).await {
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
    // Not used by the lib itself, only exercised here.
    use windmill_common::global_settings::{
        validate_allowed_origins, MAX_ALLOWED_ORIGINS, MAX_ALLOWED_ORIGIN_LEN,
    };

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
        for method in [
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Delete,
            HttpMethod::Patch,
        ] {
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
        assert_eq!(
            serde_json::to_value(RequestType::SyncSse).unwrap(),
            "sync_sse"
        );
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
        assert!(
            validate_authentication_method(AuthenticationMethod::CustomScript, Some(false))
                .is_err()
        );
        assert!(
            validate_authentication_method(AuthenticationMethod::CustomScript, Some(true)).is_ok()
        );
    }

    #[test]
    fn test_validate_auth_signature_without_raw_ok() {
        assert!(validate_authentication_method(AuthenticationMethod::Signature, None).is_ok());
    }

    // --- CORS allowed origins ---

    fn origin(value: &str) -> http::HeaderValue {
        http::HeaderValue::from_str(value).unwrap()
    }

    #[test]
    fn test_match_origin_exact_match_echoes_request_origin() {
        let allowed = vec!["https://a.com".to_string(), "https://b.com".to_string()];
        assert_eq!(
            match_origin(&allowed, Some(&origin("https://b.com"))),
            Some(origin("https://b.com"))
        );
    }

    #[test]
    fn test_match_origin_ignores_case() {
        let allowed = vec!["https://App.Example.com".to_string()];
        assert_eq!(
            match_origin(&allowed, Some(&origin("https://app.example.com"))),
            Some(origin("https://app.example.com"))
        );
    }

    #[test]
    fn test_match_origin_no_match_omits_header() {
        let allowed = vec!["https://a.com".to_string()];
        assert_eq!(
            match_origin(&allowed, Some(&origin("https://evil.com"))),
            None
        );
        // A prefix of an allowed origin must not match: https://a.com.evil.com
        // is a different site entirely.
        assert_eq!(
            match_origin(&allowed, Some(&origin("https://a.com.evil.com"))),
            None
        );
    }

    #[test]
    fn test_wildcard_entry_means_unrestricted() {
        // `*` is handled before matching: it means "no restriction", which is
        // how a route opts out of a stricter instance default.
        assert!(allows_any_origin(&["*".to_string()]));
        assert!(allows_any_origin(&[
            "https://a.com".to_string(),
            "*".to_string()
        ]));
        assert!(!allows_any_origin(&["https://a.com".to_string()]));
        assert_eq!(
            effective_allowed_origins(Some(&["*".to_string()]), &[]),
            None
        );
    }

    #[test]
    fn test_effective_allowed_origins_prefers_the_route() {
        let route = ["https://a.com".to_string()];
        let default = ["https://default.com".to_string()];
        assert_eq!(
            effective_allowed_origins(Some(&route), &default),
            Some(&route[..])
        );
        // No route list: the instance default applies.
        assert_eq!(
            effective_allowed_origins(None, &default),
            Some(&default[..])
        );
        // No route list and no instance default: nothing is restricted, so the
        // historical permissive behaviour is kept.
        assert_eq!(effective_allowed_origins(None, &[]), None);
        // A route opting out with `*` escapes a stricter instance default.
        assert_eq!(
            effective_allowed_origins(Some(&["*".to_string()]), &default),
            None
        );
        // An empty route list is a restriction that matches nothing, distinct
        // from `NULL` which inherits the instance default. It must never come
        // back as `None`, which the middleware reads as "any origin".
        assert_eq!(
            effective_allowed_origins(Some(&[]), &default),
            Some(&[][..])
        );
        assert_eq!(match_origin(&[], Some(&origin("https://a.com"))), None);
    }

    #[test]
    fn test_match_origin_missing_origin_header_omits_header() {
        let allowed = vec!["https://a.com".to_string()];
        assert_eq!(match_origin(&allowed, None), None);
    }

    #[test]
    fn test_validate_allowed_origins_accepts_anything_comparable() {
        // A shape that cannot match simply matches nothing, so it is the
        // editor's job to warn and not this one's to refuse. Only `null` and
        // values that are not header-comparable are rejected.
        let allowed = vec![
            "https://app.example.com".to_string(),
            "http://localhost:3000".to_string(),
            "http://[::1]:8080".to_string(),
            "chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai".to_string(),
            // Never matches, but that is the caller's problem, not an error.
            "https://app.example.com/".to_string(),
            "https://app.example.com:99999".to_string(),
            "not-an-origin".to_string(),
            "*".to_string(),
        ];
        assert!(validate_allowed_origins(&allowed).is_ok());
        assert!(validate_allowed_origins(&[]).is_ok());
    }

    #[test]
    fn test_validate_allowed_origins_bounds_the_list() {
        // An allowlist is scanned on every request to a restricted route, the
        // unauthenticated preflight included, so its size is a cost anyone can
        // trigger.
        let too_many = vec!["https://a.com".to_string(); MAX_ALLOWED_ORIGINS + 1];
        assert!(validate_allowed_origins(&too_many).is_err());
        assert!(validate_allowed_origins(&too_many[..MAX_ALLOWED_ORIGINS]).is_ok());
        let too_long = format!("https://{}.com", "a".repeat(MAX_ALLOWED_ORIGIN_LEN));
        assert!(validate_allowed_origins(&[too_long]).is_err());
    }

    #[test]
    fn test_validate_allowed_origins_rejects_null_and_uncomparable() {
        for invalid in [
            // Every sandboxed iframe sends `Origin: null`, so allowing it would
            // grant access to any page that can open one.
            "null",
            "NULL", // Cannot be the string an Origin header is compared against.
            "https://a b.com",
            "https://app.example.com ",
            "https://exämple.com",
            // The editor edits the list as one comma-separated field, so an
            // entry carrying a comma would come back as two and widen the list.
            "https://a.com,https://b.com",
            "",
        ] {
            assert!(
                validate_allowed_origins(&[invalid.to_string()]).is_err(),
                "expected {invalid} to be rejected"
            );
        }
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
