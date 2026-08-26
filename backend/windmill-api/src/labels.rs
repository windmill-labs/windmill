/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use windmill_api_auth::{check_scopes, ApiAuthed};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::DB,
    error::{Error, JsonResult, Result},
};

/// Mirrors the `LabelColor` enum in `openapi.yaml`, from which
/// `frontend/src/lib/components/labels/labelColors.ts` is generated. Kept as a
/// list rather than an enum so an unknown value coming back from an older row is
/// a rejected write, not a failed deserialization of the whole row.
const PALETTE: [&str; 10] = [
    "yellow", "blue", "green", "purple", "pink", "orange", "red", "cyan", "lime", "gray",
];

/// The column is TEXT so the registry never caps what a label may be called, but
/// a write still has to be bounded: the handler runs on the privileged pool, and
/// every row it accepts is returned to every member by `labels/list`. Well above
/// the 50 characters the label input allows.
const MAX_NAME_LEN: usize = 255;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_workspace_labels))
        .route("/update", post(update_label))
}

#[derive(Serialize)]
pub struct Label {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Every label the workspace knows about: the ones actually in use on an item,
/// plus the ones that only exist in `label_settings` because someone coloured
/// them and then removed the last item carrying them. Keeping those means a
/// colour survives the label falling out of use, and it is what lets labels that
/// live only on jobs or triggers — neither of which is in the union below — be
/// coloured at all.
async fn list_workspace_labels(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<Label>> {
    let labels = sqlx::query_as!(
        Label,
        "SELECT t.name AS \"name!\", ls.color AS \"color?\" FROM (
            SELECT DISTINCT unnest(labels) as name FROM (
                SELECT labels FROM script WHERE workspace_id = $1 AND labels IS NOT NULL
                UNION ALL SELECT labels FROM flow WHERE workspace_id = $1 AND labels IS NOT NULL
                UNION ALL SELECT labels FROM resource WHERE workspace_id = $1 AND labels IS NOT NULL
                UNION ALL SELECT labels FROM variable WHERE workspace_id = $1 AND labels IS NOT NULL
                UNION ALL SELECT labels FROM schedule WHERE workspace_id = $1 AND labels IS NOT NULL
                UNION ALL SELECT labels FROM app WHERE workspace_id = $1 AND labels IS NOT NULL
                UNION ALL SELECT labels FROM folder WHERE workspace_id = $1 AND labels IS NOT NULL
            ) u
            UNION
            SELECT name FROM label_settings WHERE workspace_id = $1
        ) t
        LEFT JOIN label_settings ls ON ls.workspace_id = $1 AND ls.name = t.name
        ORDER BY 1",
        &w_id
    )
    .fetch_all(&db)
    .await?;
    Ok(Json(labels))
}

#[derive(Deserialize)]
pub struct UpdateLabel {
    pub name: String,
    /// `None` clears the colour, dropping the row.
    pub color: Option<String>,
}

/// Colouring a label is deliberately as unprivileged as creating one: labels
/// appear the moment anyone types a new one into an item, so gating the colour
/// behind admin would leave a label anyone can make and only an admin can
/// finish. Operators are the exception — they write nothing.
async fn update_label(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(nl): Json<UpdateLabel>,
) -> Result<String> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot edit labels".to_string(),
        ));
    }
    check_scopes(&authed, || "labels:write".to_string())?;

    // The name keys a label that already exists verbatim in some `labels text[]`,
    // which stores whatever was typed. Trimming here would colour a *different*
    // label — `" release "` would write a row for `"release"` — so the raw name is
    // the key and the trim only answers whether it is blank.
    let name = nl.name.as_str();
    if name.trim().is_empty() {
        return Err(Error::BadRequest("Label name cannot be empty".to_string()));
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(Error::BadRequest(format!(
            "Label name cannot exceed {} characters",
            MAX_NAME_LEN
        )));
    }

    match nl.color.as_deref() {
        Some(color) => {
            if !PALETTE.contains(&color) {
                return Err(Error::BadRequest(format!(
                    "Unknown label color '{}', expected one of: {}",
                    color,
                    PALETTE.join(", ")
                )));
            }
            sqlx::query!(
                "INSERT INTO label_settings (workspace_id, name, color) VALUES ($1, $2, $3)
                 ON CONFLICT (workspace_id, name) DO UPDATE SET color = EXCLUDED.color",
                &w_id,
                name,
                color
            )
            .execute(&db)
            .await?;
        }
        None => {
            sqlx::query!(
                "DELETE FROM label_settings WHERE workspace_id = $1 AND name = $2",
                &w_id,
                name
            )
            .execute(&db)
            .await?;
        }
    }

    audit_log(
        &db,
        &authed,
        "labels.update",
        ActionKind::Update,
        &w_id,
        Some(name),
        Some([("color", nl.color.as_deref().unwrap_or("none"))].into()),
    )
    .await?;

    Ok(format!("Updated label {}", name))
}
