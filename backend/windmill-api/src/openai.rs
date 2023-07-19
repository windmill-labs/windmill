use crate::{db::DB, users::Authed, variables::build_crypt, HTTP_CLIENT};

use axum::{
    body::{Bytes, StreamBody},
    extract::{Extension, Path},
    http::HeaderMap,
    response::IntoResponse,
    routing::post,
    Router,
};
use magic_crypt::MagicCryptTrait;
use windmill_audit::{audit_log, ActionKind};
use windmill_common::error::{to_anyhow, Error};

use serde::Deserialize;

pub fn workspaced_service() -> Router {
    let router = Router::new().route("/proxy/*openai_path", post(proxy));

    router
}

#[derive(Deserialize)]
struct OpenaiResource {
    api_key: String,
    organisation: Option<String>,
}
async fn proxy(
    authed: Authed,
    Extension(db): Extension<DB>,
    Path((w_id, openai_path)): Path<(String, String)>,
    body: Bytes,
) -> impl IntoResponse {
    let mut tx = db.begin().await?;
    let openai_resource_path = sqlx::query_scalar!(
        "SELECT openai_resource_path FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    if openai_resource_path.is_none() {
        return Err(Error::InternalErr(
            "OpenAI resource not configured".to_string(),
        ));
    }

    let openai_resource_path = openai_resource_path.unwrap();

    tx = db.begin().await?;
    let resource = sqlx::query_scalar!(
        "SELECT value
        FROM resource
        WHERE path = $1 AND workspace_id = $2",
        &openai_resource_path,
        &w_id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    if resource.is_none() {
        return Err(Error::InternalErr(
            "OpenAI resource missing value".to_string(),
        ));
    }

    let mut resource: OpenaiResource = serde_json::from_value(resource.unwrap())
        .map_err(|e| Error::InternalErr(format!("validating openai resource {e}")))?;

    let openai_api_key_path = if resource.api_key.starts_with("$var:") {
        resource.api_key.strip_prefix("$var:").unwrap().to_string()
    } else {
        return Err(Error::InternalErr(
            "OpenAI resource api key must be a variable".to_string(),
        ));
    };

    tx = db.begin().await?;
    resource.api_key = sqlx::query_scalar!(
        "SELECT value
        FROM variable
        WHERE path = $1 AND workspace_id = $2",
        &openai_api_key_path,
        &w_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let mc = build_crypt(&mut tx, &w_id).await?;
    tx.commit().await?;
    resource.api_key = mc
        .decrypt_base64_to_string(resource.api_key)
        .map_err(|e| Error::InternalErr(e.to_string()))?;

    let mut request = HTTP_CLIENT
        .post(String::from("https://api.openai.com/v1/") + &openai_path)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", resource.api_key))
        .body(body);

    if resource.organisation.is_some() {
        request = request.header("OpenAI-Organization", resource.organisation.unwrap());
    }

    let resp = request.send().await.map_err(to_anyhow)?;

    tx = db.begin().await?;
    audit_log(
        &mut *tx,
        &authed.username,
        "openai.request",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("openai_path", &format!("{:?}", openai_path)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    let mut headers = HeaderMap::new();
    for (k, v) in resp.headers().iter() {
        headers.insert(k, v.clone());
    }

    let stream = resp.bytes_stream();

    Ok((headers, StreamBody::new(stream)))
}
