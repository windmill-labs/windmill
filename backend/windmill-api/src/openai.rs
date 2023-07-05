use crate::{db::DB, users::Authed, HTTP_CLIENT};

use axum::{
    body::{Bytes, StreamBody},
    extract::{Extension, Path},
    response::IntoResponse,
    routing::post,
    Router,
};

use windmill_audit::{audit_log, ActionKind};
use windmill_common::error::{to_anyhow, Error};

pub fn workspaced_service() -> Router {
    let router = Router::new().route("/proxy/*openai_path", post(proxy));

    router
}

struct OpenAIKey {
    openai_key: Option<String>,
}
async fn proxy(
    authed: Authed,
    Extension(db): Extension<DB>,
    Path((w_id, openai_path)): Path<(String, String)>,
    body: Bytes,
) -> impl IntoResponse {
    tracing::info!("reached openai endpoint");

    let mut tx = db.begin().await?;
    let settings = sqlx::query_as!(
        OpenAIKey,
        "SELECT openai_key FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("getting openai_key: {e}")))?;
    tx.commit().await?;

    let openai_key = match settings.openai_key {
        Some(key) => key,
        None => {
            return Err(Error::BadRequest(
                "openai_key is not set for this workspace".to_string(),
            ))
        }
    };

    let resp = HTTP_CLIENT
        .post(String::from("https://api.openai.com/v1/") + &openai_path)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", openai_key))
        .body(body)
        .send()
        .await
        .map_err(to_anyhow)?;

    let mut tx = db.begin().await?;
    audit_log(
        &mut tx,
        &authed.username,
        "openai.request",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("openai_path", &format!("{:?}", openai_path)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    let stream = resp.bytes_stream();

    Ok(StreamBody::new(stream))
}
