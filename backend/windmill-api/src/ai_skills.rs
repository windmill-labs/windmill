/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::{ApiAuthed, DB};
use std::collections::HashSet;
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

const MAX_SKILLS_PER_UPLOAD: usize = 50;
// Every stored skill's name + description is advertised in the global AI chat
// system prompt, so bound the total a workspace can accumulate across uploads.
const MAX_SKILLS_PER_WORKSPACE: usize = 100;
// `name` and `description` follow the Claude SKILL.md spec
// (https://platform.claude.com/docs/en/agents-and-tools/agent-skills): both are
// loaded into the AI chat system prompt and `name` is the model-facing skill id,
// so matching the upstream limits keeps skills portable with Claude Code.
const MAX_SKILL_NAME_CHARS: usize = 64;
const MAX_SKILL_DESCRIPTION_CHARS: usize = 1_024;
// Not a spec field — a payload bound on the SKILL.md body, so measured in bytes.
const MAX_SKILL_INSTRUCTIONS_BYTES: usize = 64 * 1024;

fn validate_skill(skill: &SkillUpload) -> Result<()> {
    let name = skill.name.trim();
    if name.is_empty() || name.chars().count() > MAX_SKILL_NAME_CHARS {
        return Err(Error::BadRequest(format!(
            "skill name must be between 1 and {MAX_SKILL_NAME_CHARS} characters, got {:?}",
            skill.name
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(Error::BadRequest(format!(
            "skill name {name:?} must only contain lowercase letters, digits or '-'"
        )));
    }
    if skill.description.trim().is_empty() {
        return Err(Error::BadRequest(format!(
            "skill {name:?} is missing a description (the SKILL.md frontmatter `description`)"
        )));
    }
    if skill.description.chars().count() > MAX_SKILL_DESCRIPTION_CHARS {
        return Err(Error::BadRequest(format!(
            "skill {name:?} description must be at most {MAX_SKILL_DESCRIPTION_CHARS} characters"
        )));
    }
    if skill.instructions.trim().is_empty() {
        return Err(Error::BadRequest(format!(
            "skill {name:?} has an empty SKILL.md body"
        )));
    }
    if skill.instructions.len() > MAX_SKILL_INSTRUCTIONS_BYTES {
        return Err(Error::BadRequest(format!(
            "skill {name:?} instructions must be at most {MAX_SKILL_INSTRUCTIONS_BYTES} bytes"
        )));
    }
    Ok(())
}

/// Collect the trimmed skill names, rejecting duplicates within a single upload.
/// The insert upserts by name, so a duplicate would silently keep only the last
/// and make the reported/audited count wrong.
fn collect_upload_names(skills: &[SkillUpload]) -> Result<Vec<String>> {
    let mut names = Vec::with_capacity(skills.len());
    let mut seen = HashSet::with_capacity(skills.len());
    for skill in skills {
        let name = skill.name.trim().to_string();
        if !seen.insert(name.clone()) {
            return Err(Error::BadRequest(format!(
                "duplicate skill name {name:?} in upload"
            )));
        }
        names.push(name);
    }
    Ok(names)
}

/// Reject an upload that would push the workspace past `MAX_SKILLS_PER_WORKSPACE`.
/// Uploads upsert, so names already present (`replacing`) don't count as new.
fn check_workspace_skill_capacity(
    existing_total: i64,
    replacing: i64,
    upload_count: usize,
) -> Result<()> {
    let new_count = upload_count as i64 - replacing;
    if existing_total + new_count > MAX_SKILLS_PER_WORKSPACE as i64 {
        return Err(Error::BadRequest(format!(
            "workspace cannot store more than {MAX_SKILLS_PER_WORKSPACE} skills"
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
    if payload.skills.len() > MAX_SKILLS_PER_UPLOAD {
        return Err(Error::BadRequest(format!(
            "cannot upload more than {MAX_SKILLS_PER_UPLOAD} skills at a time"
        )));
    }
    for skill in &payload.skills {
        validate_skill(skill)?;
    }
    let names = collect_upload_names(&payload.skills)?;

    let mut tx = db.begin().await?;
    let counts = sqlx::query!(
        r#"SELECT
               COUNT(*)::bigint AS "total!",
               COUNT(*) FILTER (WHERE name = ANY($2::text[]))::bigint AS "replacing!"
           FROM ai_skill
           WHERE workspace_id = $1"#,
        &w_id,
        &names
    )
    .fetch_one(&mut *tx)
    .await?;
    check_workspace_skill_capacity(counts.total, counts.replacing, names.len())?;

    for (skill, name) in payload.skills.iter().zip(names.iter()) {
        sqlx::query!(
            r#"INSERT INTO ai_skill (workspace_id, name, description, instructions, edited_at, edited_by)
               VALUES ($1, $2, $3, $4, now(), $5)
               ON CONFLICT (workspace_id, name) DO UPDATE
               SET description = EXCLUDED.description,
                   instructions = EXCLUDED.instructions,
                   edited_at = now(),
                   edited_by = EXCLUDED.edited_by"#,
            &w_id,
            name,
            skill.description,
            skill.instructions,
            &authed.username,
        )
        .execute(&mut *tx)
        .await?;
    }

    let audit_resource = names.join(",");
    audit_log(
        &mut *tx,
        &authed,
        "ai_skills.upload",
        ActionKind::Update,
        &w_id,
        Some(&audit_resource),
        Some([("skill_count", &names.len().to_string()[..])].into()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn skill() -> SkillUpload {
        SkillUpload {
            name: "test-skill".to_string(),
            description: "Useful for tests".to_string(),
            instructions: "# Test\n\nDo the thing.".to_string(),
        }
    }

    #[test]
    fn validate_skill_rejects_oversized_description() {
        let mut skill = skill();
        skill.description = "x".repeat(MAX_SKILL_DESCRIPTION_CHARS + 1);

        assert!(matches!(validate_skill(&skill), Err(Error::BadRequest(_))));
    }

    #[test]
    fn validate_skill_rejects_oversized_instructions() {
        let mut skill = skill();
        skill.instructions = "x".repeat(MAX_SKILL_INSTRUCTIONS_BYTES + 1);

        assert!(matches!(validate_skill(&skill), Err(Error::BadRequest(_))));
    }

    #[test]
    fn validate_skill_rejects_oversized_name() {
        let mut skill = skill();
        skill.name = "a".repeat(MAX_SKILL_NAME_CHARS + 1);

        assert!(matches!(validate_skill(&skill), Err(Error::BadRequest(_))));
    }

    #[test]
    fn validate_skill_rejects_non_slug_name() {
        // Uppercase, underscore, space and punctuation are all outside the
        // Claude SKILL.md `[a-z0-9-]` name charset.
        for bad in ["My-Skill", "my_skill", "my skill", "skill!"] {
            let mut skill = skill();
            skill.name = bad.to_string();

            assert!(
                matches!(validate_skill(&skill), Err(Error::BadRequest(_))),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn validate_skill_counts_description_in_characters() {
        // 1024 two-byte chars exceed the byte limit but sit exactly on the
        // character limit, so they must be accepted.
        let mut skill = skill();
        skill.description = "é".repeat(MAX_SKILL_DESCRIPTION_CHARS);

        assert!(validate_skill(&skill).is_ok());
    }

    #[test]
    fn workspace_capacity_allows_replacement_at_cap() {
        // Already at the cap, but the upload only replaces an existing skill.
        let at_cap = MAX_SKILLS_PER_WORKSPACE as i64;
        assert!(check_workspace_skill_capacity(at_cap, 1, 1).is_ok());
    }

    #[test]
    fn workspace_capacity_rejects_new_skill_over_cap() {
        let at_cap = MAX_SKILLS_PER_WORKSPACE as i64;
        assert!(matches!(
            check_workspace_skill_capacity(at_cap, 0, 1),
            Err(Error::BadRequest(_))
        ));
    }

    #[test]
    fn collect_upload_names_trims_and_collects() {
        let names = collect_upload_names(&[skill()]).unwrap();
        assert_eq!(names, vec!["test-skill".to_string()]);
    }

    #[test]
    fn collect_upload_names_rejects_duplicates() {
        // Names are compared after trimming, so whitespace can't smuggle a dup in.
        let dup = SkillUpload { name: " test-skill ".to_string(), ..skill() };
        assert!(matches!(
            collect_upload_names(&[skill(), dup]),
            Err(Error::BadRequest(_))
        ));
    }
}
