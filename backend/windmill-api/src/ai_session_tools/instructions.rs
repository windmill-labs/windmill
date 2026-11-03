//! The instructions a client receives on connecting: the session's system prompt as
//! rendered for an external client (`gen/prompt.json`), filled in for the caller. The prompt
//! follows what the caller may do, which is the token's scope and, for a full token, the
//! caller's role in the workspace. What differs per workspace is cut out of it, so a connection
//! spanning several workspaces gets the rest up front and each workspace's part from
//! `get_workspace_context`.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;
use windmill_common::{
    roles::{caller_capabilities, AuthorLevel, RoleCapabilities},
    workspaces::{check_deploy_rules, RuleCheckResult},
    DB,
};
use windmill_mcp::server::McpAuth;

use super::{guidance::list_skills, Ctx, SessionAccess};

/// As generated: each prompt as indexes into one table of distinct lines.
#[derive(Deserialize)]
struct GenParts {
    profiles: HashMap<String, usize>,
    prompts: Vec<FolderSlots<Vec<usize>>>,
    lines: Vec<String>,
    skills_header: String,
    skill_line: String,
    workspace_instructions: String,
}

/// A profile's prompt with a `{{folder_guidance}}` slot, and the guidance once per folder
/// situation.
#[derive(Deserialize)]
struct FolderSlots<T> {
    base: T,
    folder_guidance: FolderGuidance<T>,
}

#[derive(Deserialize)]
struct FolderGuidance<T> {
    admin: T,
    admin_no_folders: T,
    member: T,
    member_no_folders: T,
    member_readonly: T,
    member_no_folders_readonly: T,
}

/// The prompts by profile key (`prompt_profile`), and the sections appended when the
/// workspace has something to put there.
struct PromptParts {
    profiles: HashMap<String, usize>,
    prompts: Vec<FolderSlots<String>>,
    skills_header: String,
    skill_line: String,
    workspace_instructions: String,
}

static PARTS: LazyLock<PromptParts> = LazyLock::new(|| {
    let gen: GenParts =
        serde_json::from_str(include_str!("gen/prompt.json")).expect("gen/prompt.json is valid");
    let text = |ids: &[usize]| {
        ids.iter()
            .map(|&i| gen.lines[i].as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let prompts = gen
        .prompts
        .iter()
        .map(|p| {
            let g = &p.folder_guidance;
            FolderSlots {
                base: text(&p.base),
                folder_guidance: FolderGuidance {
                    admin: text(&g.admin),
                    admin_no_folders: text(&g.admin_no_folders),
                    member: text(&g.member),
                    member_no_folders: text(&g.member_no_folders),
                    member_readonly: text(&g.member_readonly),
                    member_no_folders_readonly: text(&g.member_no_folders_readonly),
                },
            }
        })
        .collect();
    PromptParts {
        profiles: gen.profiles,
        prompts,
        skills_header: gen.skills_header,
        skill_line: gen.skill_line,
        workspace_instructions: gen.workspace_instructions,
    }
});

/// The key of a full-access caller holding every capability: what a connection spanning
/// several workspaces is told before it learns its role in each.
const UNRESTRICTED: &str = "full:11:1:full:full:11";

fn level(level: AuthorLevel) -> &'static str {
    match level {
        AuthorLevel::None => "none",
        AuthorLevel::Compose => "compose",
        AuthorLevel::Full => "full",
    }
}

fn bit(b: bool) -> char {
    if b {
        '1'
    } else {
        '0'
    }
}

/// Which generated prompt the caller reads, keyed as `mcpSessionGen.test.ts` generates them.
/// A failed lookup reads as allowed: the prompt only describes, the API still refuses.
pub(crate) async fn prompt_profile(
    db: &DB,
    auth: &crate::db::ApiAuthed,
    w_id: &str,
    access: SessionAccess,
) -> String {
    match access {
        SessionAccess::ReadOnly => return "read_only".to_string(),
        SessionAccess::ReadAndRun => return "read_run".to_string(),
        SessionAccess::Full => {}
    }
    let caps = caller_capabilities(db, w_id, auth)
        .await
        .unwrap_or(RoleCapabilities::FULL);
    let deploy = !matches!(
        check_deploy_rules(w_id, &auth.username, &auth.groups, auth.is_admin, db).await,
        Ok(RuleCheckResult::Blocked(_))
    );
    full_profile(auth.is_admin, deploy, &caps)
}

fn full_profile(is_admin: bool, deploy: bool, caps: &RoleCapabilities) -> String {
    format!(
        "full:{}{}:{}:{}:{}:{}{}",
        bit(is_admin),
        bit(deploy),
        bit(caps.code),
        level(caps.flows),
        level(caps.apps),
        bit(caps.manage_schedules),
        bit(caps.manage_triggers),
    )
}

fn prompt_for(profile: &str) -> &'static FolderSlots<String> {
    let parts = &*PARTS;
    // A role no profile covers (full flows without code, which writes refuse) reads as the
    // unrestricted one.
    let id = parts
        .profiles
        .get(profile)
        .or_else(|| parts.profiles.get(UNRESTRICTED))
        .expect("the unrestricted profile is generated");
    &parts.prompts[*id]
}

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

fn folder_guidance(auth: &crate::db::ApiAuthed, prompt: &FolderSlots<String>) -> String {
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

    let guidance = &prompt.folder_guidance;
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

/// The whole prompt, for a connection that reaches one workspace.
pub(crate) async fn session_instructions(ctx: &Ctx<'_>, profile: &str) -> String {
    let prompt = prompt_for(profile);
    let mut text = prompt
        .base
        .replace("{{folder_guidance}}", &folder_guidance(ctx.auth, prompt))
        .replace("{{username}}", ctx.auth.username());
    text.push_str(&appendix(ctx).await);
    text
}

/// The prompt without anything specific to a workspace, for a connection that reaches
/// several: usernames and roles are per workspace, so paths read `u/<username>/...` and a full
/// token reads the unrestricted prompt until `workspace_context` narrows it.
pub(crate) fn generic_session_instructions(access: SessionAccess) -> String {
    prompt_for(generic_profile(access))
        .base
        .replace("{{folder_guidance}}", "")
        .replace("{{username}}", "<username>")
}

fn generic_profile(access: SessionAccess) -> &'static str {
    match access {
        SessionAccess::ReadOnly => "read_only",
        SessionAccess::ReadAndRun => "read_run",
        SessionAccess::Full => UNRESTRICTED,
    }
}

/// What `generic_session_instructions(token_access)` leaves out, for one workspace. A role or
/// workspace that narrows the prompt gets its own in full, since its rules replace the general
/// ones; `ctx.access` may be narrower than the token's.
pub(crate) async fn workspace_context(
    ctx: &Ctx<'_>,
    profile: &str,
    token_access: SessionAccess,
) -> String {
    let username = ctx.auth.username();
    let header = format!(
        "Workspace `{}`. Your username here is \"{username}\", so your personal paths are \
         `u/{username}/<name>`.\n",
        ctx.w_id
    );
    let prompt = prompt_for(profile);
    let general = std::ptr::eq(prompt, prompt_for(generic_profile(token_access)));
    let body = if general {
        folder_guidance(ctx.auth, prompt).trim_end().to_string()
    } else {
        format!(
            "Your access in this workspace is narrower than the general instructions assume. \
             Here, these rules replace them:\n\n{}",
            prompt
                .base
                .replace("{{folder_guidance}}", &folder_guidance(ctx.auth, prompt))
                .replace("{{username}}", username)
        )
    };
    format!(
        "{header}{}{}",
        body.trim_start_matches('\n'),
        appendix(ctx).await
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every role a caller can hold reads a prompt generated for it, rather than falling back
    /// to the unrestricted one: the keys here and in `mcpSessionGen.test.ts` must agree.
    #[test]
    fn every_valid_role_has_a_generated_prompt() {
        let levels = [AuthorLevel::None, AuthorLevel::Compose, AuthorLevel::Full];
        for deploy in [true, false] {
            assert!(PARTS.profiles.contains_key(&full_profile(
                true,
                deploy,
                &RoleCapabilities::FULL
            )));
            for code in [true, false] {
                for flows in levels {
                    for apps in levels {
                        if !code && (flows == AuthorLevel::Full || apps == AuthorLevel::Full) {
                            continue;
                        }
                        for (manage_schedules, manage_triggers) in
                            [(true, true), (true, false), (false, true), (false, false)]
                        {
                            let caps = RoleCapabilities {
                                code,
                                flows,
                                apps,
                                manage_schedules,
                                manage_triggers,
                                ..RoleCapabilities::FULL
                            };
                            let key = full_profile(false, deploy, &caps);
                            assert!(PARTS.profiles.contains_key(&key), "{key}");
                        }
                    }
                }
            }
        }
        assert!(
            PARTS.profiles.contains_key("read_only") && PARTS.profiles.contains_key("read_run")
        );
    }
}
