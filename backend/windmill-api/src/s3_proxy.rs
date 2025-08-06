use axum::{
    extract::Path,
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
    auth::Tokened,
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
) -> Result<Response> {
    let storage = if storage.is_empty() {
        None
    } else {
        Some(storage)
    };

    // temp values
    let Tokened { token } = Tokened { token: "".to_string() };
    let user_db: Option<UserDB> = None;
    let authed = ApiAuthed {
        email: "s3_proxy".to_string(),
        username: "s3_proxy".to_string(),
        is_admin: true,
        ..Default::default()
    };

    let (_, s3_resource) =
        get_workspace_s3_resource(&authed, &db, user_db, &token, &w_id, storage).await?;
    let s3_resource = s3_resource.ok_or(Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_object_store_client(&s3_resource).await?;
    let result = read_object_streamable(s3_client, &object_key).await?;
    let stream = result.into_stream();
    let stream_body = axum::body::Body::from_stream(stream);
    Ok(stream_body.into_response())
}
