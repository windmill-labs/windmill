use super::{
    http_trigger_args::RawHttpTriggerArgs, refresh_routers, AuthenticationMethod, HttpMethod,
    RequestType, TriggerRoute, HTTP_ACCESS_CACHE, HTTP_AUTH_CACHE, HTTP_ROUTERS_CACHE,
};
use crate::{
    auth::{AuthCache, OptTokened},
    db::{ApiAuthed, DB},
    jobs::start_job_update_sse_stream,
    triggers::trigger_helpers::{
        get_runnable_format, trigger_runnable, trigger_runnable_and_wait_for_result,
        trigger_runnable_inner, RunnableId,
    },
    users::fetch_api_authed,
    utils::{check_scopes, ExpiringCacheEntry},
};
use axum::{
    extract::Path,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use futures::StreamExt;
use http::{HeaderMap, StatusCode};
use std::{collections::HashMap, sync::Arc};
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    jobs::JobTriggerKind,
    triggers::{TriggerKind, TriggerMetadata},
    utils::{not_found_if_none, StripPath},
};
use windmill_store::resources::try_get_resource_from_db_as;
use windmill_trigger::TriggerMode;

#[cfg(feature = "parquet")]
use {
    crate::job_helpers_oss::get_workspace_s3_resource,
    windmill_object_store::build_object_store_client,
};

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
            check_scopes(&authed, || format!("http_triggers:read:{}", &trigger.path))?;

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
            let path = windmill_object_store::object_store_reexports::Path::from(path);
            let s3_object = s3_client.get(&path).await;

            let s3_object = match s3_object {
                Err(windmill_object_store::object_store_reexports::ObjectStoreError::NotFound { .. }) if trigger.is_static_website => {
                    // fallback to index.html if the file is not found
                    let path = windmill_object_store::object_store_reexports::Path::from(format!(
                        "{}/index.html",
                        config.s3.trim_end_matches('/')
                    ));
                    s3_client.get(&path).await
                }
                r => r,
            };

            let s3_object = s3_object.map_err(|err| {
                tracing::warn!("Error retrieving file from S3: {:?}", err);
                let mut msg = format!("Error retrieving file: {err}");
                let mut source = std::error::Error::source(&err);
                while let Some(e) = source {
                    msg.push_str(&format!("\n  caused by: {e}"));
                    source = e.source();
                }
                Error::internal_err(msg)
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
                    .get(&windmill_object_store::object_store_reexports::Attribute::ContentType)
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
                                .get(&windmill_object_store::object_store_reexports::Attribute::ContentDisposition)
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
            let (uuid, _, early_return, _) = trigger_runnable_inner(
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
                early_return,
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
