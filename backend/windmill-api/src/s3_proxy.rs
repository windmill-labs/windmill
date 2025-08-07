use std::{sync::Arc, time::SystemTime};

use aws_sdk_config::config::Credentials;
use aws_sigv4::http_request::{SignableBody, SigningSettings};
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
    routing::{delete, get, put},
    Extension, Router,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use http::{HeaderMap, HeaderValue};
use object_store::PutMultipartOpts;
use windmill_common::{
    db::UserDB,
    error::{to_anyhow, Error, Result},
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
    req: Request<Body>,
) -> Result<Response> {
    let uri = format!("/api/w/{}/s3_proxy{}", w_id, req.uri().to_string());
    let token = get_token(req.headers(), req.method().as_str(), &uri).await?;
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
    let stream_body = Body::from_stream(stream);
    Ok(stream_body.into_response())
}

async fn put_object(
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, storage_str, object_key)): Path<(String, String, String)>,
    Extension(auth_cache): Extension<Arc<AuthCache>>,
    req: Request<Body>,
) -> Result<()> {
    let uri = format!("/api/w/{}/s3_proxy{}", w_id, req.uri().to_string());
    let token = get_token(req.headers(), req.method().as_str(), &uri).await?;
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
    req: Request<Body>,
) -> Result<()> {
    let uri = format!("/api/w/{}/s3_proxy{}", w_id, req.uri().to_string());
    let token = get_token(req.headers(), req.method().as_str(), &uri).await?;
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

async fn get_token(
    header_map: &HeaderMap<HeaderValue>,
    http_method: &str,
    uri: &str,
) -> Result<String> {
    // Access key is the first two parts of the token JWT (header and payload)
    // Secret key is the third part of the JWT (authorization.signature)

    // The secret key is never passed in cleartext with the S3 protocol.
    // It is used as a key to sign the request, we only receive a (authorization.signature).
    // So what we do is we derive the jwt_signature from the (cleartext) header and payload,
    // and then we sign the request with that jwt_signature to check that
    // it corresponds to the received (authorization.signature)

    let authorization = get_header(header_map, "Authorization").map_err(to_anyhow)?;
    let authorization = parse_authorization_header(authorization)
        .ok_or_else(|| Error::InternalErr("Invalid Authorization header format".to_string()))?;

    let access_key = authorization.credential.access_key;

    let jwt_signature = jwt::generate_signature(access_key).await?;
    let full_token = format!("{}.{}", access_key, jwt_signature);

    check_s3_signature(&jwt_signature, header_map, http_method, uri, &authorization)?;

    Ok(full_token)
}

fn get_header<'a>(header_map: &'a HeaderMap<HeaderValue>, header_name: &str) -> Result<&'a str> {
    header_map
        .get(header_name)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| Error::InternalErr(format!("{} missing in header", header_name)))
}

fn check_s3_signature(
    secret_key: &str,
    header_map: &HeaderMap<HeaderValue>,
    http_method: &str,
    uri: &str,
    authorization: &AuthorizationHeader<'_>,
) -> Result<()> {
    if authorization.sign_method != "AWS4-HMAC-SHA256" {
        return Err(Error::InternalErr("Unsupported sign method".to_string()));
    }
    let datetime = get_header(header_map, "x-amz-date")?;
    let datetime = NaiveDateTime::parse_from_str(datetime, "%Y%m%dT%H%M%SZ").map_err(to_anyhow)?;
    let datetime = DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc);
    let datetime =
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(datetime.timestamp() as u64);

    let content_sha256 = get_header(header_map, "x-amz-content-sha256").map_err(to_anyhow)?;

    let mut headers = vec![];
    for header in authorization.signed_headers.split(';') {
        if let Some(value) = header_map.get(header) {
            headers.push((header, value.to_str().map_err(to_anyhow)?));
        }
    }

    let signable_request = aws_sigv4::http_request::SignableRequest::new(
        http_method,
        uri,
        headers.into_iter(),
        SignableBody::Precomputed(content_sha256.to_string()),
    )
    .map_err(to_anyhow)?;

    let credentials = Credentials::new(
        authorization.credential.access_key,
        secret_key,
        None,
        None,
        "S3 Proxy",
    )
    .into();

    let params = aws_sigv4::sign::v4::SigningParams::builder()
        .name(authorization.credential.service)
        .region(authorization.credential.region)
        .time(datetime)
        .identity(&credentials)
        .settings(SigningSettings::default())
        .build()
        .map_err(to_anyhow)?;
    let params = aws_sigv4::http_request::SigningParams::V4(params);

    let signing_output =
        aws_sigv4::http_request::sign(signable_request, &params).map_err(to_anyhow)?;
    let signature = signing_output.signature();
    if signature != authorization.signature {
        return Err(Error::NotAuthorized("Signature mismatch".to_string()));
    }
    Ok(())
}

struct AuthorizationHeader<'a> {
    sign_method: &'a str, // AWS4-HMAC-SHA256
    credential: AuthorizationHeaderCredential<'a>,
    signed_headers: &'a str, // "host;x-amz-content-sha256;x-amz-date"
    signature: &'a str,
}

struct AuthorizationHeaderCredential<'a> {
    access_key: &'a str, // Access key ID
    #[allow(unused)]
    date: &'a str, // Date in YYYYMMDD format
    region: &'a str,     // AWS region
    service: &'a str,    // Service name (e.g., "s3")
}

fn parse_authorization_header(value: &str) -> Option<AuthorizationHeader> {
    let mut split = value.split_whitespace();
    let sign_method = split.next()?;
    let credential = split
        .next()?
        .trim_end_matches(',')
        .trim_start_matches("Credential=");
    let signed_headers = split
        .next()?
        .trim_end_matches(',')
        .trim_start_matches("SignedHeaders=");
    let signature = split.next()?.trim_start_matches("Signature=");

    let mut credential_split = credential.split('/');
    let credential = AuthorizationHeaderCredential {
        // order matters
        access_key: credential_split.next()?,
        date: credential_split.next()?,
        region: credential_split.next()?,
        service: credential_split.next()?,
    };

    Some(AuthorizationHeader { sign_method, credential, signed_headers, signature })
}
