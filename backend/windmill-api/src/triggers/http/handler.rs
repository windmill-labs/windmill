use super::{
    http_trigger_args::RawHttpTriggerArgs, AuthenticationMethod, HttpMethod, RequestType,
    TriggerRoute, HTTP_ACCESS_CACHE, HTTP_AUTH_CACHE, HTTP_ROUTERS_CACHE,
};
use crate::{
    auth::{AuthCache, OptTokened},
    db::{ApiAuthed, DB},
    jobs::start_job_update_sse_stream,
    resources::try_get_resource_from_db_as,
    triggers::{
        http::{
            refresh_routers, validate_authentication_method, HttpConfig, HttpConfigRequest,
            RouteExists, ROUTE_PATH_KEY_RE, VALID_ROUTE_PATH_RE,
        },
        trigger_helpers::{
            get_runnable_format, trigger_runnable, trigger_runnable_and_wait_for_result,
            trigger_runnable_inner, RunnableId,
        },
        Trigger, TriggerCrud, TriggerData, TriggerMode,
    },
    users::fetch_api_authed,
    utils::ExpiringCacheEntry,
};
use axum::{
    async_trait,
    extract::Path,
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use futures::StreamExt;
use http::{HeaderMap, StatusCode};
use sqlx::PgConnection;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    jobs::JobTriggerKind,
    triggers::{TriggerKind, TriggerMetadata},
    utils::{not_found_if_none, require_admin, StripPath},
    worker::CLOUD_HOSTED,
};
use windmill_git_sync::handle_deployment_metadata;

#[cfg(feature = "parquet")]
use {
    crate::job_helpers_oss::get_workspace_s3_resource,
    windmill_common::s3_helpers::build_object_store_client,
};

use windmill_git_sync::DeployedObject;

pub async fn increase_trigger_version(tx: &mut PgConnection) -> Result<()> {
    sqlx::query!("SELECT nextval('http_trigger_version_seq')")
        .fetch_one(tx)
        .await?;
    Ok(())
}

pub fn generate_route_path_key(route_path: &str) -> String {
    ROUTE_PATH_KEY_RE
        .replace_all(route_path, "${1}${2}key")
        .to_string()
}

pub async fn route_path_key_exists(
    route_path_key: &str,
    http_method: &HttpMethod,
    w_id: &str,
    trigger_path: Option<&str>,
    workspaced_route: Option<bool>,
    db: &DB,
) -> Result<bool> {
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

pub async fn exists_route(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(RouteExists { route_path, http_method, trigger_path, workspaced_route }): Json<
        RouteExists,
    >,
) -> Result<Json<bool>> {
    let route_path_key = generate_route_path_key(&route_path);

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

fn check_no_duplicates<'trigger>(
    new_http_triggers: &[TriggerData<HttpConfigRequest>],
    route_path_key: &[String],
) -> Result<()> {
    let mut seen = HashSet::with_capacity(new_http_triggers.len());

    for (i, trigger) in new_http_triggers.iter().enumerate() {
        if !seen.insert((
            &route_path_key[i],
            trigger.config.http_method,
            trigger.config.workspaced_route,
        )) {
            return Err(Error::BadRequest(format!(
            "Duplicate HTTP route detected: '{}'. Each HTTP route must have a unique 'route_path'.",
            &trigger.config.route_path
        )));
        }
    }

    Ok(())
}

pub async fn insert_new_trigger_into_db(
    authed: &ApiAuthed,
    tx: &mut PgConnection,
    w_id: &str,
    trigger: &TriggerData<HttpConfigRequest>,
    route_path_key: &str,
) -> Result<()> {
    require_admin(authed.is_admin, &authed.username)?;

    let request_type = trigger.config.request_type;

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
                summary,
                description,
                is_flow,
                mode,
                request_type,
                authentication_method,
                http_method,
                static_asset_config,
                edited_by,
                email,
                edited_at,
                is_static_website,
                error_handler_path,
                error_handler_args,
                retry
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, now(), $20, $21, $22, $23
            )
            "#,
            w_id,
            trigger.base.path,
            trigger.config.route_path,
            route_path_key,
            trigger.config.workspaced_route.unwrap_or(false),
            trigger.config.authentication_resource_path,
            trigger.config.wrap_body.unwrap_or(false),
            trigger.config.raw_string.unwrap_or(false),
            trigger.base.script_path,
            trigger.config.summary,
            trigger.config.description,
            trigger.base.is_flow,
            trigger.base.mode() as _,
            request_type as _,
            trigger.config.authentication_method as _,
            trigger.config.http_method as _,
            trigger.config.static_asset_config as _,
            &authed.username,
            &authed.email,
            trigger.config.is_static_website,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _
        )
        .execute(&mut *tx)
        .await?;
    Ok(())
}

pub async fn create_many_http_triggers(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(new_http_triggers): Json<Vec<TriggerData<HttpConfigRequest>>>,
) -> Result<(StatusCode, String)> {
    require_admin(authed.is_admin, &authed.username)?;

    let handler = HttpTrigger;

    let error_wrapper = |route_path: &str, error: Error| -> Error {
        anyhow::anyhow!(
            "Error occurred for HTTP route at route path: {}, error: {}",
            route_path,
            error
        )
        .into()
    };

    let mut route_path_keys = Vec::with_capacity(new_http_triggers.len());

    for new_http_trigger in new_http_triggers.iter() {
        handler
            .validate_new(&db, &w_id, &new_http_trigger.config)
            .await
            .map_err(|err| error_wrapper(&new_http_trigger.config.route_path, err))?;

        let route_path_key =
            check_if_route_exist(&db, &new_http_trigger.config, &w_id, None).await?;

        route_path_keys.push(route_path_key.clone());
    }

    check_no_duplicates(&new_http_triggers, &route_path_keys)?;

    let mut tx = user_db.begin(&authed).await?;

    for (new_http_trigger, route_path_key) in new_http_triggers.iter().zip(route_path_keys.iter()) {
        insert_new_trigger_into_db(&authed, &mut tx, &w_id, new_http_trigger, route_path_key)
            .await
            .map_err(|err| error_wrapper(&new_http_trigger.config.route_path, err))?;

        audit_log(
            &mut *tx,
            &authed,
            "http_trigger.create",
            ActionKind::Create,
            &w_id,
            Some(&new_http_trigger.base.path),
            None,
        )
        .await
        .map_err(|err| error_wrapper(&new_http_trigger.config.route_path, err.into()))?;

        increase_trigger_version(&mut tx)
            .await
            .map_err(|err| error_wrapper(&new_http_trigger.config.route_path, err.into()))?;
    }

    tx.commit().await?;

    for http_trigger in new_http_triggers.into_iter() {
        handle_deployment_metadata(
            &authed.email,
            &authed.username,
            &db,
            &w_id,
            windmill_git_sync::DeployedObject::HttpTrigger { path: http_trigger.base.path.clone() },
            Some(format!("HTTP trigger '{}' created", http_trigger.base.path)),
            true,
        )
        .await
        .map_err(|err| error_wrapper(&http_trigger.config.route_path, err.into()))?;
    }
    Ok((StatusCode::CREATED, "Created all HTTP routes".to_string()))
}

async fn check_if_route_exist(
    db: &DB,
    config: &HttpConfigRequest,
    workspace_id: &str,
    trigger_path: Option<&str>,
) -> Result<String> {
    let route_path_key = generate_route_path_key(&config.route_path);

    let exists = route_path_key_exists(
        &route_path_key,
        &config.http_method,
        workspace_id,
        trigger_path,
        config.workspaced_route,
        db,
    )
    .await?;

    if exists {
        return Err(Error::BadRequest(
            "A route already exists with this path".to_string(),
        ));
    }

    Ok(route_path_key)
}

pub struct HttpTrigger;

#[async_trait]
impl TriggerCrud for HttpTrigger {
    type TriggerConfig = HttpConfig;
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfigRequest = HttpConfigRequest;
    type TestConnectionConfig = ();

    const TABLE_NAME: &'static str = "http_trigger";
    const TRIGGER_TYPE: &'static str = "http";
    const SUPPORTS_SERVER_STATE: bool = false;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str = "/http_triggers";
    const DEPLOYMENT_NAME: &'static str = "HTTP trigger";
    const IS_ALLOWED_ON_CLOUD: bool = true;
    const ADDITIONAL_SELECT_FIELDS: &[&'static str] = &[
        "route_path",
        "route_path_key",
        "request_type",
        "authentication_method",
        "http_method",
        "summary",
        "description",
        "static_asset_config",
        "is_static_website",
        "authentication_resource_path",
        "workspaced_route",
        "wrap_body",
        "raw_string",
    ];

    fn get_deployed_object(path: String) -> DeployedObject {
        DeployedObject::HttpTrigger { path }
    }

    fn additional_routes(&self) -> Router {
        Router::new()
            .route("/create_many", post(create_many_http_triggers))
            .route("/route_exists", post(exists_route))
    }

    async fn validate_new(
        &self,
        _db: &DB,
        _workspace_id: &str,
        new: &Self::TriggerConfigRequest,
    ) -> Result<()> {
        if *CLOUD_HOSTED && (new.is_static_website || new.static_asset_config.is_some()) {
            return Err(Error::BadRequest(
                "Static website and static asset are not supported on cloud".to_string(),
            ));
        }

        if !VALID_ROUTE_PATH_RE.is_match(&new.route_path) {
            return Err(Error::BadRequest("Invalid route path".to_string()));
        }

        validate_authentication_method(new.authentication_method, new.raw_string)?;

        Ok(())
    }

    async fn validate_edit(
        &self,
        _db: &DB,
        _workspace_id: &str,
        edit: &Self::TriggerConfigRequest,
        _path: &str,
    ) -> Result<()> {
        if *CLOUD_HOSTED && (edit.is_static_website || edit.static_asset_config.is_some()) {
            return Err(Error::BadRequest(
                "Static website and static asset are not supported on cloud".to_string(),
            ));
        }

        validate_authentication_method(edit.authentication_method, edit.raw_string)?;

        Ok(())
    }

    async fn create_trigger(
        &self,
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let route_path_key = check_if_route_exist(db, &trigger.config, &w_id, None).await?;

        insert_new_trigger_into_db(authed, tx, w_id, &trigger, &route_path_key).await?;

        increase_trigger_version(tx).await?;

        Ok(())
    }

    async fn update_trigger(
        &self,
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        workspace_id: &str,
        path: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        if authed.is_admin {
            if trigger.config.route_path.is_empty() {
                return Err(Error::BadRequest("route_path is required".to_string()));
            };

            let route_path = &trigger.config.route_path;
            if !VALID_ROUTE_PATH_RE.is_match(route_path) {
                return Err(Error::BadRequest("Invalid route path".to_string()));
            }

            let route_path_key =
                check_if_route_exist(db, &trigger.config, workspace_id, Some(path)).await?;

            let request_type = trigger.config.request_type;

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
                mode = $10,
                http_method = $11,
                static_asset_config = $12,
                edited_by = $13,
                email = $14,
                request_type = $15,
                authentication_method = $16,
                summary = $17,
                description = $18,
                edited_at = now(),
                is_static_website = $19,
                error_handler_path = $20,
                error_handler_args = $21,
                retry = $22
            WHERE
                workspace_id = $23 AND
                path = $24
            "#,
                route_path,
                &route_path_key,
                trigger.config.workspaced_route,
                trigger.config.wrap_body,
                trigger.config.raw_string,
                trigger.config.authentication_resource_path,
                trigger.base.script_path,
                trigger.base.path,
                trigger.base.is_flow,
                trigger.base.mode() as _,
                trigger.config.http_method as _,
                trigger.config.static_asset_config as _,
                &authed.username,
                &authed.email,
                request_type as _,
                trigger.config.authentication_method as _,
                trigger.config.summary,
                trigger.config.description,
                trigger.config.is_static_website,
                trigger.error_handling.error_handler_path,
                trigger.error_handling.error_handler_args as _,
                trigger.error_handling.retry as _,
                workspace_id,
                path,
            )
            .execute(&mut *tx)
            .await?;
        } else {
            let request_type = trigger.config.request_type;

            sqlx::query!(
                r#"
            UPDATE
                http_trigger
            SET
                wrap_body = $1,
                raw_string = $2,
                authentication_resource_path = $3,
                script_path = $4,
                path = $5,
                is_flow = $6,
                mode = $7,
                http_method = $8,
                static_asset_config = $9,
                edited_by = $10,
                email = $11,
                request_type = $12,
                authentication_method = $13,
                summary = $14,
                description = $15,
                edited_at = now(),
                is_static_website = $16,
                error_handler_path = $17,
                error_handler_args = $18,
                retry = $19
            WHERE
                workspace_id = $20 AND
                path = $21
            "#,
                trigger.config.wrap_body,
                trigger.config.raw_string,
                trigger.config.authentication_resource_path,
                trigger.base.script_path,
                trigger.base.path,
                trigger.base.is_flow,
                trigger.base.mode() as _,
                trigger.config.http_method as _,
                trigger.config.static_asset_config as _,
                &authed.username,
                &authed.email,
                request_type as _,
                trigger.config.authentication_method as _,
                trigger.config.summary,
                trigger.config.description,
                trigger.config.is_static_website,
                trigger.error_handling.error_handler_path,
                trigger.error_handling.error_handler_args as _,
                trigger.error_handling.retry as _,
                workspace_id,
                path,
            )
            .execute(&mut *tx)
            .await?;
        }

        increase_trigger_version(tx).await?;

        Ok(())
    }

    async fn set_trigger_mode_extra_action(&self, tx: &mut PgConnection) -> Result<()> {
        increase_trigger_version(tx).await
    }

    async fn delete_by_path(
        &self,
        tx: &mut PgConnection,
        workspace_id: &str,
        path: &str,
    ) -> Result<bool> {
        let deleted = sqlx::query(&format!(
            "DELETE FROM {} WHERE workspace_id = $1 AND path = $2",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(path)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        increase_trigger_version(tx).await?;

        Ok(deleted > 0)
    }
}

async fn conditional_cors_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let mut response = next.run(req).await;

    let headers = response.headers_mut();

    // Check existing headers first to determine what not to insert
    let mut not_insert_origin = false;
    let mut not_insert_methods = false;
    let mut not_insert_headers = false;

    for key in headers.keys() {
        if !not_insert_origin && key == http::header::ACCESS_CONTROL_ALLOW_ORIGIN {
            not_insert_origin = true;
        }
        if !not_insert_methods && key == http::header::ACCESS_CONTROL_ALLOW_METHODS {
            not_insert_methods = true;
        }
        if !not_insert_headers && key == http::header::ACCESS_CONTROL_ALLOW_HEADERS {
            not_insert_headers = true;
        }

        // Early exit if all headers are already present
        if not_insert_origin && not_insert_methods && not_insert_headers {
            break;
        }
    }

    // Insert only the missing headers
    if !not_insert_origin {
        headers.insert(
            http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
            http::HeaderValue::from_static("*"),
        );
    }

    if !not_insert_methods {
        headers.insert(
            http::header::ACCESS_CONTROL_ALLOW_METHODS,
            http::HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS"),
        );
    }

    if !not_insert_headers {
        headers.insert(
            http::header::ACCESS_CONTROL_ALLOW_HEADERS,
            http::HeaderValue::from_static("content-type, authorization"),
        );
    }

    response
}

pub fn http_route_trigger_handler() -> Router {
    Router::new()
        .route(
            "/*path",
            get(route_job)
                .post(route_job)
                .delete(route_job)
                .put(route_job)
                .patch(route_job)
                .head(|| async { "" })
                .options(|| async { "" }),
        )
        .layer(axum::middleware::from_fn(conditional_cors_middleware))
}

async fn get_http_route_trigger(
    route_path: &str,
    auth_cache: &Arc<AuthCache>,
    token: Option<&String>,
    db: &DB,
    user_db: UserDB,
    method: &http::Method,
) -> Result<(TriggerRoute, String, HashMap<String, String>, ApiAuthed)> {
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
        .ok_or(Error::internal_err(
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
                    tracing::debug!("HTTP access cache hit for route {}", trigger.path);
                    true
                }
                _ => {
                    tracing::debug!("HTTP access cache miss for route {}", trigger.path);
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
                return Err(Error::NotAuthorized("Unauthorized".to_string()));
            }
        } else {
            return Err(Error::NotAuthorized("Requires authentication".to_string()));
        }
    } else {
        None
    };

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        &db,
        Some(username_override.unwrap_or(format!("HTTP-{}", trigger.path))),
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
) -> std::result::Result<impl IntoResponse, Response> {
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

    if trigger.script_path.is_empty() && trigger.static_asset_config.is_none() {
        return Err(Error::NotFound(format!(
            "Runnable path of HTTP route at path: {}",
            trigger.path
        ))
        .into_response());
    }

    let args = args
        .process_args(
            &authed,
            &db,
            &trigger.workspace_id,
            match trigger.authentication_method {
                AuthenticationMethod::CustomScript | AuthenticationMethod::Signature => true,
                _ => trigger.raw_string,
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
                    tracing::debug!("HTTP auth method cache hit for route {}", trigger.path);
                    cache_entry.value
                }
                _ => {
                    tracing::debug!("HTTP auth method cache miss for route {}", trigger.path);
                    let auth_method = try_get_resource_from_db_as::<
                        super::http_trigger_auth::AuthenticationMethod,
                    >(
                        &authed,
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
        return Err(Error::internal_err(
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
                &trigger.workspace_id,
                config.storage,
            )
            .await?;
            let s3_resource = s3_resource_opt.ok_or(Error::internal_err(
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
                Error::internal_err(format!("Error retrieving file: {}", err.to_string()))
            })?;

            let mut response_headers = http::HeaderMap::new();
            if let Some(ref e_tag) = s3_object.meta.e_tag {
                if let Some(if_none_match) = headers.get(http::header::IF_NONE_MATCH) {
                    if if_none_match == e_tag {
                        return Ok::<_, Error>((
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
            Ok::<_, Error>((StatusCode::OK, response_headers, body_stream))
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

    let trigger_info = TriggerMetadata::new(Some(trigger.path.clone()), JobTriggerKind::Http);
    if trigger.mode == TriggerMode::Suspended {
        let _ = trigger_runnable(
            &db,
            Some(user_db),
            authed,
            &trigger.workspace_id,
            &trigger.script_path,
            trigger.is_flow,
            args,
            trigger.retry.as_ref(),
            trigger.error_handler_path.as_deref(),
            trigger.error_handler_args.as_ref(),
            format!("http_trigger/{}", trigger.path),
            None,
            true,
            trigger_info,
        )
        .await
        .map_err(|e| e.into_response())?;

        return Ok((
            StatusCode::OK,
            format!(
                "Trigger {} is in suspended mode, jobs are added to the queue but suspended",
                &trigger.path
            ),
        )
            .into_response());
    }

    // Handle execution based on the execution mode
    match trigger.request_type {
        RequestType::SyncSse => {
            // Trigger the job (always async when streaming)
            let (uuid, _, _, _) = trigger_runnable_inner(
                &db,
                None,
                Some(user_db.clone()),
                authed.clone(),
                &trigger.workspace_id,
                &trigger.script_path,
                trigger.is_flow,
                args,
                trigger.retry.as_ref(),
                trigger.error_handler_path.as_deref(),
                trigger.error_handler_args.as_ref(),
                format!("http_trigger/{}", trigger.path),
                None,
                trigger_info,
                None,
            )
            .await
            .map_err(|e| e.into_response())?;

            // Set up SSE stream
            let opt_authed = Some(authed.clone());
            let opt_tokened = OptTokened { token: None };
            let (tx, rx) = tokio::sync::mpsc::channel(32);

            let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|x| {
                format!(
                    "data: {}\n\n",
                    serde_json::to_string(&x).unwrap_or_default()
                )
            });

            start_job_update_sse_stream(
                opt_authed,
                opt_tokened,
                db.clone(),
                trigger.workspace_id.clone(),
                uuid,
                None,
                None,
                None,
                None,
                Some(true),
                Some(true),
                None,
                None,
                tx,
                None,
            );

            let body = axum::body::Body::from_stream(
                stream.map(std::result::Result::<_, std::convert::Infallible>::Ok),
            );

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "text/event-stream")
                .header("Cache-Control", "no-cache")
                .body(body)
                .map_err(|e| Error::internal_err(e.to_string()).into_response())?)
        }
        RequestType::Async => trigger_runnable(
            &db,
            Some(user_db),
            authed,
            &trigger.workspace_id,
            &trigger.script_path,
            trigger.is_flow,
            args,
            trigger.retry.as_ref(),
            trigger.error_handler_path.as_deref(),
            trigger.error_handler_args.as_ref(),
            format!("http_trigger/{}", trigger.path),
            None,
            false,
            trigger_info,
        )
        .await
        .map_err(|e| e.into_response()),
        RequestType::Sync => trigger_runnable_and_wait_for_result(
            &db,
            Some(user_db),
            authed,
            &trigger.workspace_id,
            &trigger.script_path,
            trigger.is_flow,
            args,
            trigger.retry.as_ref(),
            trigger.error_handler_path.as_deref(),
            trigger.error_handler_args.as_ref(),
            format!("http_trigger/{}", trigger.path),
            trigger_info,
        )
        .await
        .map_err(|e| e.into_response()),
    }
}
