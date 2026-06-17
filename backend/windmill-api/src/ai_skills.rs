/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::{ApiAuthed, DB};
use axum::{
    extract::{Extension, Json, Path},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::require_admin,
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_skills))
        .route("/get/{name}", get(get_skill))
        .route("/upload", post(upload_skills))
        .route("/delete/{name}", delete(delete_skill))
}

/// Cheap listing surfaced in the AI chat system prompt — no `instructions` body.
#[derive(Serialize)]
pub struct SkillListItem {
    pub name: String,
    pub description: String,
}

/// Full skill, including the SKILL.md body, fetched on demand by `read_skill`.
#[derive(Serialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub instructions: String,
}

#[derive(Deserialize)]
pub struct UploadSkills {
    pub skills: Vec<SkillUpload>,
}

#[derive(Deserialize)]
pub struct SkillUpload {
    pub name: String,
    pub description: String,
    pub instructions: String,
}

/// Skill names become the model-facing id and are interpolated into the system
/// prompt, so keep them to a conservative slug charset.
fn validate_skill(skill: &SkillUpload) -> Result<()> {
    let name = skill.name.trim();
    if name.is_empty() || name.len() > 255 {
        return Err(Error::BadRequest(format!(
            "skill name must be between 1 and 255 characters, got {:?}",
            skill.name
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(Error::BadRequest(format!(
            "skill name {name:?} must only contain letters, digits, '_' or '-'"
        )));
    }
    if skill.description.trim().is_empty() {
        return Err(Error::BadRequest(format!(
            "skill {name:?} is missing a description (the SKILL.md frontmatter `description`)"
        )));
    }
    if skill.instructions.trim().is_empty() {
        return Err(Error::BadRequest(format!(
            "skill {name:?} has an empty SKILL.md body"
        )));
    }
    Ok(())
}

async fn list_skills(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<SkillListItem>> {
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query!(
        "SELECT name, description FROM ai_skill WHERE workspace_id = $1 ORDER BY name",
        &w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| SkillListItem { name: r.name, description: r.description })
            .collect(),
    ))
}

async fn get_skill(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<Skill> {
    let mut tx = user_db.begin(&authed).await?;
    let row = sqlx::query!(
        "SELECT name, description, instructions FROM ai_skill WHERE workspace_id = $1 AND name = $2",
        &w_id,
        &name
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    row.map(|r| {
        Json(Skill { name: r.name, description: r.description, instructions: r.instructions })
    })
    .ok_or_else(|| Error::NotFound(format!("no skill named {name:?} in workspace {w_id}")))
}

/// Bulk upsert the uploaded skills by name. Existing skills not in the payload
/// are left untouched — removal goes through `delete_skill`.
async fn upload_skills(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(payload): Json<UploadSkills>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    if payload.skills.is_empty() {
        return Err(Error::BadRequest("no skills to upload".to_string()));
    }
    for skill in &payload.skills {
        validate_skill(skill)?;
    }

    let mut tx = db.begin().await?;
    for skill in &payload.skills {
        sqlx::query!(
            r#"INSERT INTO ai_skill (workspace_id, name, description, instructions, edited_at, edited_by)
               VALUES ($1, $2, $3, $4, now(), $5)
               ON CONFLICT (workspace_id, name) DO UPDATE
               SET description = EXCLUDED.description,
                   instructions = EXCLUDED.instructions,
                   edited_at = now(),
                   edited_by = EXCLUDED.edited_by"#,
            &w_id,
            skill.name.trim(),
            skill.description,
            skill.instructions,
            &authed.username,
        )
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "ai_skills.upload",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("skill_count", &payload.skills.len().to_string()[..])].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(format!(
        "Uploaded {} skill(s) to workspace {}",
        payload.skills.len(),
        &w_id
    ))
}

async fn delete_skill(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = db.begin().await?;
    let deleted = sqlx::query_scalar!(
        "DELETE FROM ai_skill WHERE workspace_id = $1 AND name = $2 RETURNING name",
        &w_id,
        &name
    )
    .fetch_optional(&mut *tx)
    .await?;

    if deleted.is_none() {
        tx.commit().await?;
        return Err(Error::NotFound(format!(
            "no skill named {name:?} in workspace {w_id}"
        )));
    }

    audit_log(
        &mut *tx,
        &authed,
        "ai_skills.delete",
        ActionKind::Delete,
        &w_id,
        Some(&name),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Deleted skill {name} from workspace {w_id}"))
}
