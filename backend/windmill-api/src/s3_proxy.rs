use std::sync::Arc;

use axum::{
    extract::{Path, Request},
    response::{IntoResponse, Response},
    routing::{delete, get, put},
    Extension, Router,
};
use object_store::PutMultipartOpts;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    jwt,
    s3_helpers::build_object_store_client,
};

use crate::{
    auth::AuthCache,
    db::DB,
    job_helpers_oss::{
        delete_s3_file_internal, get_workspace_s3_resource, read_object_streamable,
        upload_file_from_req, DeleteS3FileQuery,
    },
};

pub fn workspaced_unauthed_service() -> Router {
    // Most routes are duplicated to support the s3://storage/path syntax
    Router::new()
        .route("/s3%3A//:storage/*key", get(get_object))
        .route("/:storage/*key", get(get_object))
        .route("/s3%3A//:storage/*key", put(put_object))
        .route("/:storage/*key", put(put_object))
        .route("/s3%3A//:storage/*key", delete(delete_object))
        .route("/:storage/*key", delete(delete_object))
}

async fn get_object(
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, storage_str, object_key)): Path<(String, String, String)>,
    Extension(auth_cache): Extension<Arc<AuthCache>>,
    req: Request<axum::body::Body>,
) -> Result<Response> {
    let token = get_token(get_header(&req, "Authorization")).await?;
    let Some(authed) = auth_cache.get_authed(Some(w_id.clone()), &token).await else {
        return Err(Error::NotAuthorized("Invalid token".to_string()));
    };
    let storage = Some(storage_str.clone()).filter(|s| !s.is_empty());

    let (_, s3_resource) =
        get_workspace_s3_resource(&authed, &db, Some(user_db), &token, &w_id, storage).await?;
    let s3_resource = s3_resource.ok_or_else(|| {
        Error::InternalErr(format!(
            "Storage {} not found at the workspace level",
            storage_str
        ))
    })?;
    let s3_client = build_object_store_client(&s3_resource).await?;
    let result = read_object_streamable(s3_client, &object_key).await?;
    let stream = result.into_stream();
    let stream_body = axum::body::Body::from_stream(stream);
    Ok(stream_body.into_response())
}

async fn put_object(
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, storage_str, object_key)): Path<(String, String, String)>,
    Extension(auth_cache): Extension<Arc<AuthCache>>,
    req: Request<axum::body::Body>,
) -> Result<()> {
    let token = get_token(get_header(&req, "Authorization")).await?;
    let Some(authed) = auth_cache.get_authed(Some(w_id.clone()), &token).await else {
        return Err(Error::NotAuthorized("Invalid token".to_string()));
    };
    let storage = Some(storage_str.clone()).filter(|s| !s.is_empty());

    let (_, s3_resource) =
        get_workspace_s3_resource(&authed, &db, Some(user_db), &token, &w_id, storage).await?;
    let s3_resource = s3_resource.ok_or_else(|| {
        Error::InternalErr(format!(
            "Storage {} not found at the workspace level",
            storage_str
        ))
    })?;
    let s3_client = build_object_store_client(&s3_resource).await?;
    upload_file_from_req(
        s3_client,
        &object_key,
        req,
        PutMultipartOpts { ..Default::default() },
    )
    .await
}

async fn delete_object(
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, storage_str, object_key)): Path<(String, String, String)>,
    Extension(auth_cache): Extension<Arc<AuthCache>>,
    req: Request<axum::body::Body>,
) -> Result<()> {
    let token = get_token(get_header(&req, "Authorization")).await?;
    let Some(authed) = auth_cache.get_authed(Some(w_id.clone()), &token).await else {
        return Err(Error::NotAuthorized("Invalid token".to_string()));
    };
    let storage = Some(storage_str.clone()).filter(|s| !s.is_empty());

    delete_s3_file_internal(
        &authed,
        &db,
        Some(user_db),
        &token,
        &w_id,
        DeleteS3FileQuery { file_key: object_key, storage },
    )
    .await
}

async fn get_token(authorization_header: Result<&str>) -> Result<String> {
    // Access key is the first two parts of the token JWT (header and payload)
    // Secret key is the third part of the JWT (req_signature)

    // The secret key is never passed in cleartext with the S3 protocol.
    // It is used as a key to sign the request, we only receive a req_signature.
    // So what we do is we derive the jwt_signature from the (cleartext) header and payload,
    // and then we sign the request with that jwt_signature to check that
    // it corresponds to the received req_signature

    let (access_key, req_signature) = authorization_header
        .map_err(|e| Error::InternalErr(format!("Failed to get token: {}", e)))
        .and_then(|authorization| {
            if authorization.is_empty() {
                Err(Error::InternalErr("Authorization missing".to_string()))
            } else {
                let access_key = authorization
                    .split_once("Credential=")
                    .and_then(|(_, t)| t.split_once('/').map(|(t, _)| t))
                    .ok_or_else(|| Error::InternalErr("Couldn't parse credential".to_string()))?;
                let signature = authorization
                    .split_once("Signature=")
                    .map(|(_, t)| t)
                    .ok_or_else(|| Error::InternalErr("Couldn't parse signature".to_string()))?;
                Ok((access_key, signature))
            }
        })?;

    let jwt_signature = jwt::generate_signature(access_key).await?;
    let full_token = format!("{}.{}", access_key, jwt_signature);
    Ok(full_token)
}

fn get_header<'a>(req: &'a Request<axum::body::Body>, header_name: &str) -> Result<&'a str> {
    req.headers()
        .get(header_name)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| Error::InternalErr(format!("{} missing in header", header_name)))
}
