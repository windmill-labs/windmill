/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{UserDB, DB},
    users::{maybe_refresh_folders, require_owner_of_path, Authed},
};

use axum::{
    extract::{Extension, Path},
    routing::post,
    Json, Router,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use windmill_common::error::Result;

pub fn workspaced_service() -> Router {
    Router::new().route("/create", post(create_draft))
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
    pub value: serde_json::Value,
    pub typ: DraftType,
}

async fn create_draft(
    authed: Authed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(draft): Json<Draft>,
) -> Result<(StatusCode, String)> {
    let authed = maybe_refresh_folders(&draft.path, &w_id, authed, &db).await;

    let mut tx = user_db.begin(&authed).await?;

    require_owner_of_path(&authed, &draft.path)?;

    sqlx::query!(
        "INSERT INTO draft
            (workspace_id, path, value, typ)
            VALUES ($1, $2, $3::text::json, $4)
            ON CONFLICT (workspace_id, path, typ) DO UPDATE SET value = $3::text::json",
        &w_id,
        draft.path,
        //to preserve key orders
        serde_json::to_string(&draft.value).unwrap(),
        draft.typ: DraftType,
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, format!("draft {} created", draft.path)))
}

// async fn get_draft(
//     authed: Authed,
//     Extension(user_db): Extension<UserDB>,
//     Path((w_id, path)): Path<(String, StripPath)>,
// ) -> JsonResult<Draft> {
//     let path = path.to_path();
//     let mut tx = user_db.begin(&authed).await?;

//     let script_o = sqlx::query_as!(
//         Draft,
//         r#"SELECT path, value, typ as "typ: DraftType" FROM draft WHERE path = $1 AND workspace_id = $2"#,
//         path,
//         w_id
//     )
//     .fetch_optional(&mut tx)
//     .await?;
//     tx.commit().await?;

//     let draft = not_found_if_none(script_o, "draft", path)?;
//     Ok(Json(draft))
// }
