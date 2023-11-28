use crate::{
    db::{ApiAuthed, DB},
    variables::build_crypt,
    HTTP_CLIENT,
};

use axum::{
    body::{Bytes, StreamBody},
    extract::{Extension, Path},
    response::IntoResponse,
    routing::post,
    Router,
};
use magic_crypt::MagicCryptTrait;
use serde_json::json;
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
    organization_id: Option<String>,
}

fn create_openai_json_error(msg: String) -> Error {
    Error::OpenAIError(
        serde_json::to_string(&json!({
            "error": {
                "message": msg
            }
        }))
        .unwrap(),
    )
}

struct Variable {
    value: String,
    is_secret: bool,
}
async fn get_variable(path: String, db: &DB, w_id: &String) -> Result<String, Error> {
    let mut tx = db.begin().await?;
    let mut variable = sqlx::query_as!(
        Variable,
        "SELECT value, is_secret
        FROM variable
        WHERE path = $1 AND workspace_id = $2",
        &path,
        &w_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if variable.is_secret {
        let mc = build_crypt(&mut tx, &w_id).await?;
        variable.value = mc
            .decrypt_base64_to_string(variable.value)
            .map_err(|e| Error::InternalErr(e.to_string()))?;
    }
    tx.commit().await?;
    Ok(variable.value)
}

lazy_static::lazy_static! {
    pub static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();
}

async fn proxy(
    authed: ApiAuthed,
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
        return Err(create_openai_json_error(
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
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        create_openai_json_error(format!(
            "Could not find the OpenAI resource at path {openai_resource_path}, update the resource path in the workspace settings"
        ))
    })?;

    if resource.is_none() {
        return Err(create_openai_json_error(
            "OpenAI resource missing value".to_string(),
        ));
    }

    let mut resource: OpenaiResource = serde_json::from_value(resource.unwrap())
        .map_err(|e| Error::InternalErr(format!("validating openai resource {e}")))?;

    if resource.api_key.starts_with("$var:") {
        let openai_api_key_path = resource.api_key.strip_prefix("$var:").unwrap().to_string();
        resource.api_key = get_variable(openai_api_key_path, &db, &w_id).await?;
    }

    let base_url = if let Some(base_url) = &*OPENAI_AZURE_BASE_PATH {
        base_url
    } else {
        "https://api.openai.com/v1"
    };

    let url = format!("{}/{}", base_url, openai_path);
    let mut request = HTTP_CLIENT
        .post(url)
        .header("content-type", "application/json")
        .body(body);

    if base_url != "https://api.openai.com/v1" {
        request = request
            .header("api-key", resource.api_key)
            .query(&[("api-version", "2023-05-15")])
    } else {
        request = request.header("authorization", format!("Bearer {}", resource.api_key))
    }

    if let Some(mut org_id) = resource.organization_id {
        tracing::info!("org_id: {:?}", org_id);
        if org_id.starts_with("$var:") {
            let openai_organisation_path = org_id.strip_prefix("$var:").unwrap().to_string();
            org_id = get_variable(openai_organisation_path, &db, &w_id).await?;
        }
        request = request.header("OpenAI-Organization", org_id);
    }

    let response = request.send().await.map_err(to_anyhow)?;

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

    if response.error_for_status_ref().is_err() {
        return Err(Error::OpenAIError(
            response.text().await.unwrap_or("".to_string()),
        ));
    }

    let status_code = response.status();
    let headers = response.headers().clone();
    let stream = response.bytes_stream();

    Ok((status_code, headers, StreamBody::new(stream)))
}
