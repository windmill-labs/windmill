/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{ApiAuthed, DB},
    users::{maybe_refresh_folders, require_owner_of_path},
};

use axum::{
    extract::{Extension, Path},
    routing::{delete, get, post},
    Json, Router,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use windmill_common::{
    db::UserDB,
    error::{JsonResult, Result},
    utils::{not_found_if_none, StripPath},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_draft))
        .route("/delete/{kind}/{*path}", delete(delete_draft))
        .route("/get/{kind}/{*path}", get(get_draft))
        .route("/list", get(list_drafts))
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[sqlx(type_name = "DRAFT_TYPE", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum DraftType {
    Script,
    Flow,
    App,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Draft {
    pub path: String,
    pub value: sqlx::types::Json<Box<serde_json::value::Value>>,
    pub typ: DraftType,
}

pub async fn require_writer_of_path(
    authed: &ApiAuthed,
    path: &str,
    w_id: &str,
    db: DB,
    kind: &DraftType,
) -> Result<()> {
    if authed.is_admin {
        return Ok(());
    } else if require_owner_of_path(authed, path).is_ok() {
        return Ok(());
    } else {
        match kind {
            DraftType::Script => crate::scripts::require_is_writer(authed, path, w_id, db).await,
            DraftType::Flow => crate::flows::require_is_writer(authed, path, w_id, db).await,
            DraftType::App => crate::apps::require_is_writer(authed, path, w_id, db).await,
        }
    }
}

async fn create_draft(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(draft): Json<Draft>,
) -> Result<(StatusCode, String)> {
    let authed = maybe_refresh_folders(&draft.path, &w_id, authed, &db).await;

    let mut tx = user_db.begin(&authed).await?;

    require_writer_of_path(&authed, &draft.path, &w_id, db, &draft.typ).await?;

    sqlx::query!(
        "INSERT INTO draft
            (workspace_id, path, value, typ, email)
            VALUES ($1, $2, $3::text::json, $4, $5)
            ON CONFLICT (workspace_id, path, typ, COALESCE(email, ''))
            DO UPDATE SET value = EXCLUDED.value, created_at = now()",
        &w_id,
        draft.path,
        //to preserve key orders
        serde_json::to_string(&draft.value).unwrap(),
        draft.typ as DraftType,
        &authed.email,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, format!("draft {} created", draft.path)))
}

async fn delete_draft(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, DraftType, StripPath)>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "DELETE FROM draft WHERE path = $1 AND typ = $2 AND workspace_id = $3 AND email = $4",
        path.to_path(),
        kind as DraftType,
        w_id,
        &authed.email,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(format!("deleted draft"))
}

// Fetch a single draft (scoped to the requesting user). Used by the editors to
// (re)open a never-deployed item that lives only in the `draft` table.
async fn get_draft(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, DraftType, StripPath)>,
) -> JsonResult<Draft> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let draft_o = sqlx::query_as!(
        Draft,
        r#"SELECT path, value as "value: _", typ as "typ: DraftType"
           FROM draft
           WHERE path = $1 AND typ = $2 AND workspace_id = $3 AND email = $4"#,
        path,
        kind as DraftType,
        w_id,
        &authed.email,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let draft = not_found_if_none(draft_o, "draft", path)?;
    Ok(Json(draft))
}

#[derive(Serialize, Debug)]
pub struct DraftWithMetadata {
    pub path: String,
    pub value: sqlx::types::Json<Box<serde_json::value::Value>>,
    pub typ: DraftType,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// List the requesting user's drafts. The editors/home use this to surface
// never-deployed items (which no longer have a row in script/flow/app).
async fn list_drafts(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<DraftWithMetadata>> {
    let mut tx = user_db.begin(&authed).await?;

    let drafts = sqlx::query_as!(
        DraftWithMetadata,
        r#"SELECT path, value as "value: _", typ as "typ: DraftType", created_at
           FROM draft
           WHERE workspace_id = $1 AND email = $2
           ORDER BY created_at DESC"#,
        w_id,
        &authed.email,
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(drafts))
}
