use std::{sync::Arc, time::SystemTime};

use aws_sdk_config::config::Credentials;
use aws_sigv4::http_request::{SignableBody, SigningSettings};
use axum::{
    body::Body,
    extract::{Path, Request},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Extension, Router,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::StreamExt;
use http::{HeaderMap, HeaderValue, StatusCode, Uri};
use object_store::PutMultipartOpts;
use windmill_common::{
    error::{to_anyhow, Error, Result},
    jwt,
    s3_helpers::{
        build_object_store_client, check_lfs_object_path_permissions, ObjectStoreResource,
    },
};

use crate::{
    auth::AuthCache,
    db::DB,
    job_helpers_ee::{
        get_large_file_storage, get_workspace_s3_resource_and_check_paths, upload_file_from_req,
    },
    job_helpers_oss::{delete_s3_file_internal, read_object_streamable, DeleteS3FileQuery},
};
use async_stream::stream;

pub fn workspaced_unauthed_service() -> Router {
    // Most routes are duplicated to support the s3://storage/path syntax
    Router::new()
        .route("/:storage/*key", get(get_object))
        .route("/:storage/*key", put(put_object))
        .route("/:storage/*key", post(post_object))
        .route("/:storage/*key", delete(delete_object))
}

async fn get_object(
    Extension(db): Extension<DB>,
    Path((w_id, storage_str, object_key)): Path<(String, String, String)>,
    Extension(auth_cache): Extension<Arc<AuthCache>>,
    req: Request<Body>,
) -> Result<Response> {
    let s3_resource = get_object_store_resource(
        &w_id,
        &req.uri().to_string(),
        auth_cache,
        req.method().as_str(),
        req.headers(),
        &storage_str,
        &db,
        &object_key,
    )
    .await?;

    let s3_client = build_object_store_client(&s3_resource).await?;
    let result = read_object_streamable(s3_client, &object_key).await?;
    let stream = result.into_stream();
    let stream_body = Body::from_stream(stream);
    Ok(stream_body.into_response())
}

async fn put_object(
    Extension(db): Extension<DB>,
    Path((w_id, storage_str, object_key)): Path<(String, String, String)>,
    Extension(auth_cache): Extension<Arc<AuthCache>>,
    req: Request<Body>,
) -> Result<Response> {
    let s3_resource = get_object_store_resource(
        &w_id,
        &req.uri().to_string(),
        auth_cache,
        req.method().as_str(),
        req.headers(),
        &storage_str,
        &db,
        &object_key,
    )
    .await?;

    if matches!(s3_resource, ObjectStoreResource::S3(_)) {
        direct_s3_proxy(
            http::Method::PUT,
            req.headers().clone(),
            s3_resource,
            &object_key,
            req.uri().clone(),
            req.into_body(),
        )
        .await
    } else {
        let s3_client = build_object_store_client(&s3_resource).await?;
        upload_file_from_req(
            s3_client,
            &object_key,
            req,
            PutMultipartOpts { ..Default::default() },
        )
        .await?;
        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .map_err(to_anyhow)?)
    }
}

// Only for DuckDB to work with S3
async fn post_object(
    Extension(db): Extension<DB>,
    Path((w_id, storage_str, object_key)): Path<(String, String, String)>,
    Extension(auth_cache): Extension<Arc<AuthCache>>,
    req: Request<Body>,
) -> Result<Response> {
    let s3_resource = get_object_store_resource(
        &w_id,
        &req.uri().to_string(),
        auth_cache,
        req.method().as_str(),
        req.headers(),
        &storage_str,
        &db,
        &object_key,
    )
    .await?;
    direct_s3_proxy(
        http::Method::POST,
        req.headers().clone(),
        s3_resource,
        &object_key,
        req.uri().clone(),
        req.into_body(),
    )
    .await
}

async fn delete_object(
    Extension(db): Extension<DB>,
    Path((w_id, storage_str, object_key)): Path<(String, String, String)>,
    Extension(auth_cache): Extension<Arc<AuthCache>>,
    req: Request<Body>,
) -> Result<()> {
    let uri = format!("/api/w/{}/s3_proxy/{}", w_id, req.uri().to_string());
    let token = get_token(req.headers(), req.method().as_str(), &uri).await?;
    let Some(authed) = auth_cache.get_authed(Some(w_id.clone()), &token).await else {
        return Err(Error::NotAuthorized("Invalid token".to_string()));
    };
    let storage = match storage_str.as_str() {
        "_default_" => None,
        _ => Some(storage_str.clone()),
    };

    let lfs = get_large_file_storage(&db, &w_id, storage.clone())
        .await?
        .ok_or_else(|| Error::InternalErr(format!("Large file storage not found",)))?;
    check_lfs_object_path_permissions(&lfs, &object_key, &authed.clone().into())?;

    delete_s3_file_internal(
        &authed,
        &db,
        &token,
        &w_id,
        DeleteS3FileQuery { file_key: object_key, storage },
    )
    .await
}

async fn direct_s3_proxy(
    method: http::Method,
    header_map: HeaderMap,
    s3_resource: ObjectStoreResource,
    object_key: &str,
    uri: Uri,
    body: axum::body::Body,
) -> Result<Response> {
    let authorization = get_header(&header_map, "Authorization").map_err(to_anyhow)?;
    let mut header_map = header_map.clone();
    let mut authorization = AuthorizationHeader::parse(authorization)
        .ok_or_else(|| Error::InternalErr("Invalid Authorization header format".to_string()))?;

    let rendered_endpoint = s3_resource.get_endpoint_url()?;
    let s3_resource = match s3_resource {
        ObjectStoreResource::S3(s3_resource) => s3_resource,
        _ => {
            return Err(Error::InternalErr(
                "Direct proxy is only supported for S3".to_string(),
            ));
        }
    };
    let remote_uri = format!(
        "{}/{}{}",
        rendered_endpoint,
        object_key,
        uri.query()
            .map(|q| format!("?{q}"))
            .as_deref()
            .unwrap_or("")
    );
    let host = format!(
        "{}.s3.{}.amazonaws.com",
        s3_resource.bucket, s3_resource.region
    );
    header_map.insert("host", HeaderValue::from_str(&host).map_err(to_anyhow)?);
    authorization.credential.region = &s3_resource.region;
    authorization.credential.access_key = s3_resource.access_key.as_deref().unwrap_or("");
    let signature = generate_s3_signature(
        s3_resource.secret_key.as_deref().unwrap_or(""),
        &header_map,
        method.as_str(),
        &remote_uri,
        &authorization,
    )?;
    authorization.signature = &signature;
    let authorization = authorization.to_string();
    header_map.insert(
        "Authorization",
        HeaderValue::from_str(&authorization).map_err(to_anyhow)?,
    );

    // Send request
    let client = reqwest::Client::new();
    let response = match method {
        http::Method::POST => client.post(remote_uri),
        http::Method::PUT => client.put(remote_uri),
        http::Method::DELETE => client.delete(remote_uri),
        http::Method::GET => client.get(remote_uri),
        _ => {
            return Err(Error::InternalErr(format!(
                "Unsupported HTTP method: {}",
                method
            )));
        }
    }
    .headers(header_map)
    .body(axum_body_to_reqwest_stream(body))
    .send()
    .await
    .map_err(to_anyhow)?;

    Ok(convert_reqwest_to_axum_response(response).await?)
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
    let authorization = AuthorizationHeader::parse(authorization)
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
    let signature = generate_s3_signature(secret_key, header_map, http_method, uri, authorization)?;
    if signature != authorization.signature {
        return Err(Error::NotAuthorized("Signature mismatch".to_string()));
    }
    Ok(())
}

fn generate_s3_signature(
    secret_key: &str,
    header_map: &HeaderMap<HeaderValue>,
    http_method: &str,
    uri: &str,
    authorization: &AuthorizationHeader<'_>,
) -> Result<String> {
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

    Ok(signature.to_string())
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

impl AuthorizationHeader<'_> {
    fn parse(value: &str) -> Option<AuthorizationHeader> {
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
    fn to_string(&self) -> String {
        format!(
            "{} Credential={}/{}/{}/{}/aws4_request, SignedHeaders={}, Signature={}",
            self.sign_method,
            self.credential.access_key,
            self.credential.date,
            self.credential.region,
            self.credential.service,
            self.signed_headers,
            self.signature
        )
    }
}

pub async fn convert_reqwest_to_axum_response(res: reqwest::Response) -> Result<Response<Body>> {
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.bytes_stream();

    let mut builder = Response::builder()
        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    for (key, value) in headers.iter() {
        builder = builder.header(key, value);
    }
    let response = builder.body(Body::from_stream(bytes)).map_err(to_anyhow)?;
    Ok(response)
}

fn axum_body_to_reqwest_stream(axum_body: Body) -> reqwest::Body {
    let stream = stream! {
        let mut axum_stream = axum_body.into_data_stream();
        #[allow(unused_variables)]
        let mut size = 0;
        while let Some(result) = axum_stream.next().await {
            match result {
                Ok(bytes) => {
                    size += bytes.len();
                    #[cfg(not(feature = "enterprise"))]
                    if size > 50 * 1024 * 1024 {
                        // consume the rest of the body
                        while let Some(_) = axum_stream.next().await {}
                        yield Err(std::io::Error::new(std::io::ErrorKind::FileTooLarge, "Uploading files bigger than 50Mb is only permitted in Windmill EE"));
                        break;
                    }
                    yield Ok(bytes);
                }
                Err(err) => yield Err(std::io::Error::new(std::io::ErrorKind::Other, err))
            }
        }
    };

    reqwest::Body::wrap_stream(stream)
}

async fn get_object_store_resource(
    w_id: &str,
    uri: &str,
    auth_cache: Arc<AuthCache>,
    method: &str,
    header_map: &HeaderMap<HeaderValue>,
    storage_str: &str,
    db: &DB,
    object_key: &str,
) -> Result<ObjectStoreResource> {
    let uri = format!("/api/w/{}/s3_proxy/{}", w_id, uri);
    let token = get_token(header_map, method, &uri).await?;
    let Some(authed) = auth_cache.get_authed(Some(w_id.to_string()), &token).await else {
        return Err(Error::NotAuthorized("Invalid token".to_string()));
    };
    let storage = match storage_str {
        "_default_" => None,
        _ => Some(storage_str.to_string()),
    };

    let (_, s3_resource) = get_workspace_s3_resource_and_check_paths(
        &authed,
        &db,
        None,
        &token,
        &w_id,
        storage.clone(),
        &[object_key],
    )
    .await?;
    let s3_resource = s3_resource.ok_or_else(|| {
        Error::InternalErr(format!(
            "Storage {} not found at the workspace level",
            storage_str
        ))
    })?;
    Ok(s3_resource)
}
