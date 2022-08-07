use axum::{
    extract::{Extension, Path},
    routing::{get, post, put},
    Json, Router,
};
use hyper::StatusCode;

use crate::{
    db::{UserDB, DB},
    error::{JsonResult, Result},
    users::Authed,
    utils::{not_found_if_none, StripPath},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/*path", put(new_payload))
        .route("/*path", get(get_payload))
}

pub fn global_service() -> Router {
    Router::new().route("/*path", post(update_payload))
}

pub async fn new_payload(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<StatusCode> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "
       INSERT INTO capture
                   (workspace_id, path)
            VALUES ($1, $2)
       ON CONFLICT (workspace_id, path)
     DO UPDATE SET created_at = now()
        ",
        &w_id,
        &path.to_path(),
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::CREATED)
}

pub async fn update_payload(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<serde_json::Value>,
) -> Result<StatusCode> {
    let mut tx = db.begin().await?;

    sqlx::query!(
        "
       UPDATE capture
          SET payload = $3
        WHERE workspace_id = $1
          AND path = $2
        ",
        &w_id,
        &path.to_path(),
        &payload,
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_payload(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<serde_json::Value> {
    let mut tx = user_db.begin(&authed).await?;

    let payload = sqlx::query_scalar!(
        "
       SELECT payload
         FROM capture
        WHERE workspace_id = $1
          AND path = $2
        ",
        &w_id,
        &path.to_path(),
    )
    .fetch_optional(&mut tx)
    .await?;

    tx.commit().await?;

    not_found_if_none(payload, "capture", path.to_path()).map(axum::Json)
}
