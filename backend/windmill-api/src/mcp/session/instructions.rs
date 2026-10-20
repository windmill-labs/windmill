//! The instructions a client receives on connecting: the session's system prompt as
//! rendered for an external client (`gen/prompt.json`), filled in for the caller. It is the
//! condensed form, since a client may keep only the first 4096 characters; the full one is
//! the `get_instructions` subject "rules". What
//! differs per workspace is cut out of it, so a connection spanning several workspaces
//! gets the rest up front and each workspace's part from `get_workspace_context`.

use std::sync::LazyLock;

use serde::Deserialize;
use windmill_mcp::server::McpAuth;

use super::{guidance::list_skills, Ctx};

/// The prompt with a `{{folder_guidance}}` slot, the guidance once per folder situation,
/// and the sections appended when the workspace has something to put there.
#[derive(Deserialize)]
struct PromptParts {
    /// The condensed prompt a client gets on connecting.
    base: String,
    full_base: String,
    folder_guidance: FolderGuidance,
    skills_header: String,
    skill_line: String,
    workspace_instructions: String,
}

#[derive(Deserialize)]
struct FolderGuidance {
    admin: String,
    admin_no_folders: String,
    member: String,
    member_no_folders: String,
    member_readonly: String,
    member_no_folders_readonly: String,
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

fn folder_guidance(auth: &crate::db::ApiAuthed) -> String {
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

    let guidance = &PARTS.folder_guidance;
    let template = match (auth.is_admin(), writable.is_empty(), readonly.is_empty()) {
        (true, false, _) => &guidance.admin,
        (true, true, _) => &guidance.admin_no_folders,
        (false, false, true) => &guidance.member,
        (false, false, false) => &guidance.member_readonly,
        (false, true, true) => &guidance.member_no_folders,
        (false, true, false) => &guidance.member_no_folders_readonly,
    };
    template
        .replace("{{username}}", auth.username())
        .replace("{{writable_folders}}", &folder_list(&writable))
        .replace("{{readonly_folders}}", &folder_list(&readonly))
}

/// The workspace's skills and its admin-written instructions.
async fn appendix(ctx: &Ctx<'_>) -> String {
    let mut text = String::new();
    // Either lookup failing leaves its section out: a client that cannot connect at all
    // is worse than one that connects without the skills list.
    if let Ok(skills) = list_skills(ctx).await {
        if !skills.is_empty() {
            text.push_str(&PARTS.skills_header);
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
            text.push_str(&lines);
        }
    }
    if let Ok(info) = ctx.get(&ctx.ws("workspaces/get_copilot_info")).await {
        let workspace = info
            .pointer("/custom_prompts/global")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if let Some(workspace) = workspace {
            text.push_str(
                &PARTS
                    .workspace_instructions
                    .replace("{{workspace_instructions}}", workspace),
            );
        }
    }
    text
}

fn render(base: &str, auth: &crate::db::ApiAuthed) -> String {
    base.replace("{{folder_guidance}}", &folder_guidance(auth))
        .replace("{{username}}", auth.username())
}

/// The condensed prompt, for a connection that reaches one workspace.
pub(in crate::mcp) async fn session_instructions(ctx: &Ctx<'_>) -> String {
    let mut prompt = render(&PARTS.base, ctx.auth);
    prompt.push_str(&appendix(ctx).await);
    prompt
}

/// The whole prompt for one workspace, which the condensed one points to as the
/// `get_instructions` subject "rules".
pub(super) async fn full_instructions(ctx: &Ctx<'_>) -> String {
    let mut prompt = render(&PARTS.full_base, ctx.auth);
    prompt.push_str(&appendix(ctx).await);
    prompt
}

/// The prompt without anything specific to a workspace, for a connection that reaches
/// several: usernames are per workspace, so paths read `u/<username>/...`.
pub(in crate::mcp) fn generic_session_instructions() -> String {
    PARTS
        .base
        .replace("{{folder_guidance}}", "")
        .replace("{{username}}", "<username>")
}

/// What `generic_session_instructions` leaves out, for one workspace.
pub(in crate::mcp) async fn workspace_context(ctx: &Ctx<'_>) -> String {
    let username = ctx.auth.username();
    format!(
        "Workspace `{}`. Your username here is \"{username}\", so your personal paths are \
         `u/{username}/<name>`.\n{}{}",
        ctx.w_id,
        folder_guidance(ctx.auth).trim_end(),
        appendix(ctx).await
    )
}
