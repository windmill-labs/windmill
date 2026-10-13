//! The instructions a client receives on connecting: the session's system prompt as
//! rendered for an external client (`gen/prompt.json`), filled in for the caller.

use std::sync::LazyLock;

use serde::Deserialize;
use windmill_mcp::server::McpAuth;

use super::{guidance::list_skills, Ctx};

/// The prompt once per folder situation, since the folder guidance reads differently
/// in each, plus the sections appended when the workspace has something to put there.
#[derive(Deserialize)]
struct PromptParts {
    admin: String,
    admin_no_folders: String,
    member: String,
    member_no_folders: String,
    member_readonly: String,
    member_no_folders_readonly: String,
    skills_header: String,
    skill_line: String,
    workspace_instructions: String,
}

static PARTS: LazyLock<PromptParts> = LazyLock::new(|| {
    serde_json::from_str(include_str!("gen/prompt.json")).expect("gen/prompt.json is valid")
});

/// Folders named in the prompt before the rest are counted rather than listed, as the
/// session does, so a folder-heavy workspace cannot dominate the instructions.
const MAX_LISTED_FOLDERS: usize = 40;

fn folder_list(names: &[&str]) -> String {
    let shown = names
        .iter()
        .take(MAX_LISTED_FOLDERS)
        .map(|n| format!("`f/{n}`"))
        .collect::<Vec<_>>()
        .join(", ");
    if names.len() > MAX_LISTED_FOLDERS {
        format!("{shown} (+{} more)", names.len() - MAX_LISTED_FOLDERS)
    } else {
        shown
    }
}

pub(in crate::mcp) async fn session_instructions(ctx: &Ctx<'_>) -> String {
    let auth = ctx.auth;
    let writable: Vec<&str> = auth
        .folders()
        .iter()
        .filter(|(_, write, _)| *write)
        .map(|(name, _, _)| name.as_str())
        .collect();
    let readonly: Vec<&str> = auth
        .folders()
        .iter()
        .filter(|(_, write, _)| !*write)
        .map(|(name, _, _)| name.as_str())
        .collect();

    let template = match (auth.is_admin(), writable.is_empty(), readonly.is_empty()) {
        (true, false, _) => &PARTS.admin,
        (true, true, _) => &PARTS.admin_no_folders,
        (false, false, true) => &PARTS.member,
        (false, false, false) => &PARTS.member_readonly,
        (false, true, true) => &PARTS.member_no_folders,
        (false, true, false) => &PARTS.member_no_folders_readonly,
    };
    let mut prompt = template
        .replace("{{username}}", auth.username())
        .replace("{{writable_folders}}", &folder_list(&writable))
        .replace("{{readonly_folders}}", &folder_list(&readonly));

    // Either lookup failing leaves its section out: a client that cannot connect at all
    // is worse than one that connects without the skills list.
    if let Ok(skills) = list_skills(ctx).await {
        if !skills.is_empty() {
            prompt.push_str(&PARTS.skills_header);
            let lines = skills
                .iter()
                .map(|skill| {
                    PARTS
                        .skill_line
                        .replace("{{skill_path}}", &skill.path)
                        .replace("{{skill_description}}", &skill.description)
                })
                .collect::<Vec<_>>()
                .join("\n");
            prompt.push_str(&lines);
        }
    }
    if let Ok(info) = ctx.get(&ctx.ws("workspaces/get_copilot_info")).await {
        let workspace = info
            .pointer("/custom_prompts/global")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if let Some(workspace) = workspace {
            prompt.push_str(
                &PARTS
                    .workspace_instructions
                    .replace("{{workspace_instructions}}", workspace),
            );
        }
    }
    prompt
}
