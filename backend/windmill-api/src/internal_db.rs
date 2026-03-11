use crate::db::ApiAuthed;
use axum::{extract::Path, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use windmill_common::{
    error::{Error, Result},
    query_builders::try_expand_internal_db_query,
    scripts::ScriptLang,
};

pub fn workspaced_service() -> Router {
    Router::new().route("/expand_marker", post(expand_marker))
}

#[derive(Deserialize)]
struct ExpandMarkerRequest {
    language: ScriptLang,
    content: String,
}

#[derive(Serialize)]
struct ExpandMarkerResponse {
    code: String,
}

async fn expand_marker(
    _authed: ApiAuthed,
    Path(_w_id): Path<String>,
    Json(req): Json<ExpandMarkerRequest>,
) -> Result<Json<ExpandMarkerResponse>> {
    match try_expand_internal_db_query(&req.content, &req.language) {
        Some(Ok(expanded)) => Ok(Json(ExpandMarkerResponse { code: expanded.code })),
        Some(Err(msg)) => Err(Error::BadRequest(msg)),
        None => Err(Error::BadRequest(
            "Content is not a WM_INTERNAL_DB marker".to_string(),
        )),
    }
}
