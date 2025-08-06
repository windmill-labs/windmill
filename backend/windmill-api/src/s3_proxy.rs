use std::sync::Arc;

use axum::{
    extract::{Path, Request},
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    s3_helpers::build_object_store_client,
};

use crate::{
    auth::AuthCache,
    db::{ApiAuthed, DB},
    job_helpers_ee::{get_workspace_s3_resource, read_object_streamable},
};

pub fn workspaced_unauthed_service() -> Router {
    Router::new()
        .route("/s3%3A//:storage/*key", get(get_object))
        .route("/:storage/*key", get(get_object))
}

async fn get_object(
    Extension(db): Extension<DB>,
    Path((w_id, storage, object_key)): Path<(String, String, String)>,
    Extension(auth_cache): Extension<Arc<AuthCache>>,
    req: Request<axum::body::Body>,
) -> Result<Response> {
    let token = get_token(&req)?;
    let Some(authed) = auth_cache.get_authed(Some(w_id.clone()), token).await else {
        return Err(Error::NotAuthorized("Invalid token".to_string()));
    };

    let storage = if storage.is_empty() {
        None
    } else {
        Some(storage)
    };

    // temp values
    let user_db: Option<UserDB> = None;
    let authed = ApiAuthed {
        email: "s3_proxy".to_string(),
        username: "s3_proxy".to_string(),
        is_admin: true,
        ..Default::default()
    };

    let (_, s3_resource) =
        get_workspace_s3_resource(&authed, &db, user_db, token, &w_id, storage).await?;
    let s3_resource = s3_resource.ok_or(Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_object_store_client(&s3_resource).await?;
    let result = read_object_streamable(s3_client, &object_key).await?;
    let stream = result.into_stream();
    let stream_body = axum::body::Body::from_stream(stream);
    Ok(stream_body.into_response())
}

fn get_token(req: &Request<axum::body::Body>) -> Result<&str> {
    get_header(&req, "Authorization")
        .map_err(|e| Error::InternalErr(format!("Failed to get token: {}", e)))
        .and_then(|token| {
            if token.is_empty() {
                Err(Error::InternalErr("Missing token".to_string()))
            } else {
                let token = token
                    .split_once(' ')
                    .map(|(_, t)| t.trim_start_matches("Credential="))
                    .and_then(|t| t.split_once('/').map(|(t, _)| t))
                    .ok_or_else(|| Error::InternalErr("Invalid S3 authorization".to_string()))?;
                Ok(token)
            }
        })
}

fn get_header<'a>(req: &'a Request<axum::body::Body>, header_name: &str) -> Result<&'a str> {
    req.headers()
        .get(header_name)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| Error::InternalErr(format!("{} missing in header", header_name)))
}
