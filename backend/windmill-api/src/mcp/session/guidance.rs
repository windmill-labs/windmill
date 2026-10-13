//! `get_instructions` and `read_skill`: guidance the model pulls on demand.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;
use serde_json::Value;

use super::{opt_str_arg, str_arg, Ctx, ToolResult};

pub(super) const TOOLS: &[&str] = &["get_instructions", "read_skill"];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "get_instructions" => get_instructions(args),
        "read_skill" => read_skill(ctx, args).await,
        _ => unreachable!("{name} is not a guidance tool"),
    }
}

/// Each subject's text per language, as the session renders it; `""` is the text for
/// a call that names no language.
#[derive(Deserialize)]
struct Instructions {
    index: HashMap<String, HashMap<String, String>>,
    texts: HashMap<String, String>,
}

static INSTRUCTIONS: LazyLock<Instructions> = LazyLock::new(|| {
    serde_json::from_str(include_str!("gen/instructions.json"))
        .expect("gen/instructions.json is valid")
});

fn get_instructions(args: &Value) -> ToolResult {
    let subject = str_arg(args, "subject")?;
    let by_language = INSTRUCTIONS
        .index
        .get(subject)
        .ok_or_else(|| format!("Unknown subject `{subject}`"))?;
    let language = opt_str_arg(args, "language").unwrap_or("");
    let id = by_language
        .get(language)
        .ok_or_else(|| format!("Unknown language `{language}`"))?;
    Ok(INSTRUCTIONS.texts[id].clone())
}

const SKILL_RESOURCE_TYPE: &str = "ai_skill";
const SKILLS_PAGE_SIZE: usize = 100;
/// Guard against a paging bug looping forever, not a product cap.
const MAX_SKILLS_PAGES: usize = 100;
const MAX_SKILL_DESCRIPTION_CHARS: usize = 1024;
const MAX_SKILL_INSTRUCTIONS_BYTES: usize = 64 * 1024;
const TRUNCATED: &str = "… [truncated]";

pub(super) struct Skill {
    pub path: String,
    pub description: String,
}

/// Every skill the caller can read. The session lists only the ones its user left
/// switched on, a preference kept in the browser; here they are all in play.
pub(super) async fn list_skills(ctx: &Ctx<'_>) -> Result<Vec<Skill>, String> {
    let mut skills = Vec::new();
    for page in 1..=MAX_SKILLS_PAGES {
        let rows = ctx
            .get(&ctx.ws(&format!(
                "resources/list?resource_type={SKILL_RESOURCE_TYPE}&page={page}&per_page={SKILLS_PAGE_SIZE}"
            )))
            .await?;
        let rows = rows.as_array().map(Vec::as_slice).unwrap_or_default();
        for row in rows {
            let Some(path) = row.get("path").and_then(Value::as_str) else {
                continue;
            };
            let description = row
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let description = if description.chars().count() > MAX_SKILL_DESCRIPTION_CHARS {
                let cut: String = description
                    .chars()
                    .take(MAX_SKILL_DESCRIPTION_CHARS)
                    .collect();
                format!("{cut}{TRUNCATED}")
            } else {
                description.to_string()
            };
            skills.push(Skill { path: path.to_string(), description });
        }
        if rows.len() < SKILLS_PAGE_SIZE {
            break;
        }
    }
    Ok(skills)
}

async fn read_skill(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    // Checked against the listing: the path is the model's, and without this the tool
    // reads any resource holding a string `content`.
    let listed = match list_skills(ctx).await {
        Ok(skills) => skills.iter().any(|skill| skill.path == path),
        Err(e) => return Ok(read_failed(path, &e)),
    };
    if !listed {
        return Ok(format!(
            "\"{path}\" is not one of the skills available here. Only the paths listed under \"Skills\" in the instructions can be read."
        ));
    }
    let value = match ctx
        .get(&ctx.ws(&format!("resources/get_value/{path}")))
        .await
    {
        Ok(value) => value,
        Err(e) => return Ok(read_failed(path, &e)),
    };
    let Some(body) = value.get("content").and_then(Value::as_str) else {
        return Ok(read_failed(
            path,
            &format!("resource {path} has no string \"content\" — is it an {SKILL_RESOURCE_TYPE}?"),
        ));
    };
    let body = if body.len() > MAX_SKILL_INSTRUCTIONS_BYTES {
        let mut end = MAX_SKILL_INSTRUCTIONS_BYTES;
        while !body.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}{TRUNCATED}", &body[..end])
    } else {
        body.to_string()
    };
    Ok(format!("Skill: {path}\n\nInstructions:\n{body}"))
}

fn read_failed(path: &str, message: &str) -> String {
    format!(
        "Failed to read skill \"{path}\": {message}. Check the path against the Skills list in the instructions."
    )
}
