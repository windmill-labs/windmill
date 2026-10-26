//! Deploying and deleting workspace items, and diffing drafts against what is deployed.

use std::sync::LazyLock;

use futures::{stream, StreamExt};
use regex::Regex;
use serde_json::{json, Map, Value};

use super::flows::drop_stale_step_locks;
use super::workspace::{
    api_message, app_source_to_draft_value, delete_draft, deployed_item_exists, draft_kind,
    draft_page, get_deployed, get_draft_item, item_exists, item_type_arg, path_enc, service,
    unified_patch, ITEM_TYPES, TRIGGER_KINDS,
};
use super::{
    array, bool_arg, is_not_found, js_str, matches_glob, non_null, opt_str_arg, pretty, str_arg,
    truthy, Ctx, ToolResult,
};

pub(super) const TOOLS: &[&str] = &["deploy_workspace_item", "delete_workspace_item", "diff"];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "deploy_workspace_item" => deploy_item(ctx, args).await,
        "delete_workspace_item" => delete_item(ctx, args).await,
        "diff" => diff(ctx, args).await,
        _ => unreachable!("{name} is not a deploy tool"),
    }
}

/// An object without `keys`; `{}` for anything that is not an object.
fn without(value: &Value, keys: &[&str]) -> Value {
    let mut out = value.as_object().cloned().unwrap_or_default();
    for key in keys {
        out.shift_remove(*key);
    }
    Value::Object(out)
}

fn uint_arg(args: &Value, name: &str) -> Option<usize> {
    args.get(name)
        .and_then(Value::as_u64)
        .map(|n| usize::try_from(n).unwrap_or(usize::MAX))
}

// ============================================================================
// deploy_workspace_item
// ============================================================================

fn assert_draft_based_on_latest(
    item_type: &str,
    path: &str,
    base: Option<&Value>,
    head: Option<&Value>,
    force: bool,
) -> Result<(), String> {
    let (Some(base), Some(head)) = (base.filter(|v| !v.is_null()), head.filter(|v| !v.is_null()))
    else {
        return Ok(());
    };
    if force || base == head {
        return Ok(());
    }
    Err(format!(
        "This {item_type} draft \"{path}\" was started from an older deployed version (forked from {}, \
         latest is {}). Deploying now would overwrite the version deployed since. Call rebase_draft to \
         discard the stale draft and get your changes back as a diff, then re-apply them (the new draft \
         re-bases onto the latest version) and deploy. To deploy as-is and replace the newer version, call \
         deploy_workspace_item again with force: true.",
        js_str(base),
        js_str(head)
    ))
}

/// An editor draft of a script or flow can stage trigger edits, which the editors deploy
/// through per-kind forms. Deploying the item alone deletes the draft and those edits
/// with it, so it is refused instead.
fn refuse_staged_triggers(item_type: &str, path: &str, draft: &Value) -> Result<(), String> {
    let staged = draft
        .get("draft_triggers")
        .and_then(Value::as_array)
        .is_some_and(|triggers| triggers.iter().any(|t| truthy(t.get("draftConfig"))));
    if staged {
        return Err(format!(
            "Draft {item_type} \"{path}\" also stages trigger changes made in the Windmill editor, which \
             cannot be deployed from here. Deploy it from the editor so they are not lost."
        ));
    }
    Ok(())
}

async fn deploy_script(ctx: &Ctx<'_>, path: &str, message: Option<&str>) -> Result<(), String> {
    let body_of = |e: String| api_message(&e).to_string();
    let row = ctx
        .get(&ctx.ws(&format!("scripts/get/p/{}?get_draft=true", path_enc(path))))
        .await
        .map_err(body_of)?;
    let draft = non_null(row.get("draft")).unwrap_or(&row);
    refuse_staged_triggers("script", path, draft)?;
    let mut body = without(
        draft,
        &[
            "draft_triggers",
            "draft_only",
            "parent_hash",
            "deployment_message",
            "preserve_on_behalf_of",
            // A draft seeded from the deployed script carries its lock, and the server skips
            // the dependency job whenever one is set.
            "lock",
            "lock_error_logs",
        ],
    );
    body["path"] = non_null(draft.get("path")).cloned().unwrap_or(json!(path));
    if let Some(hash) = row.get("hash") {
        body["parent_hash"] = hash.clone();
    }
    if let Some(message) = message {
        body["deployment_message"] = json!(message);
    }
    // Without the flag the server resets the identity to the deploying user.
    if truthy(draft.get("on_behalf_of_email")) {
        body["preserve_on_behalf_of"] = json!(true);
    }
    ctx.post(&ctx.ws("scripts/create"), body)
        .await
        .map_err(body_of)?;
    Ok(())
}

async fn deploy_flow(
    ctx: &Ctx<'_>,
    path: &str,
    draft_only: bool,
    message: Option<&str>,
) -> Result<(), String> {
    let body_of = |e: String| api_message(&e).to_string();
    let row = ctx
        .get(&ctx.ws(&format!("flows/get/{}?get_draft=true", path_enc(path))))
        .await
        .map_err(body_of)?;
    let draft = non_null(row.get("draft")).unwrap_or(&row);
    refuse_staged_triggers("flow", path, draft)?;
    let or_empty = |key: &str| non_null(draft.get(key)).cloned().unwrap_or(json!(""));
    let mut body = Map::new();
    body.insert(
        "path".into(),
        non_null(draft.get("draft_path"))
            .or_else(|| non_null(draft.get("path")))
            .cloned()
            .unwrap_or(json!(path)),
    );
    body.insert("summary".into(), or_empty("summary"));
    body.insert("description".into(), or_empty("description"));
    for key in [
        "value",
        "schema",
        "tag",
        "dedicated_worker",
        "ws_error_handler_muted",
        "visible_to_runner_only",
        "on_behalf_of_email",
        "on_behalf_of",
        "labels",
        "job_token_scopes",
    ] {
        if let Some(value) = draft.get(key) {
            body.insert(key.into(), value.clone());
        }
    }
    if let Some(Value::Object(value)) = body.get_mut("value") {
        // With nothing deployed the server fills `value` from the draft itself.
        let deployed = row
            .get("value")
            .filter(|_| !draft_only && !truthy(row.get("no_deployed")));
        drop_stale_step_locks(value, deployed);
    }
    if truthy(draft.get("on_behalf_of_email")) {
        body.insert("preserve_on_behalf_of".into(), json!(true));
    }
    if let Some(message) = message {
        body.insert("deployment_message".into(), json!(message));
    }
    // A draft-only flow has no flow row to update.
    let route = if draft_only {
        "flows/create".to_string()
    } else {
        format!("flows/update/{}", path_enc(path))
    };
    ctx.post(&ctx.ws(&route), Value::Object(body))
        .await
        .map_err(body_of)?;
    Ok(())
}

async fn upsert(
    ctx: &Ctx<'_>,
    item_type: &str,
    trigger_kind: Option<&str>,
    path: &str,
    body: Value,
) -> Result<(), String> {
    let service = service(item_type, trigger_kind)?;
    let route = if item_exists(ctx, item_type, trigger_kind, path).await? {
        format!("{service}/update/{}", path_enc(path))
    } else {
        format!("{service}/create")
    };
    ctx.post(&ctx.ws(&route), body).await?;
    Ok(())
}

/// An agent's draft never carries the identity it runs as, and without the flag the server
/// resets it to the deploying user, so an update restates the deployed one.
async fn deploy_resource(ctx: &Ctx<'_>, path: &str, draft: &Value) -> Result<(), String> {
    if !item_exists(ctx, "resource", None, path).await? {
        ctx.post(&ctx.ws("resources/create"), draft.clone()).await?;
        return Ok(());
    }
    let deployed = get_deployed(ctx, "resource", None, path).await?;
    let mut body = draft.clone();
    let agent_obo = deployed
        .get("value")
        .and_then(|value| value.get("on_behalf_of"))
        .filter(|obo| {
            deployed.get("resource_type").and_then(Value::as_str) == Some("ai_agent")
                && truthy(Some(obo))
        });
    if let (Some(obo), Some(body)) = (agent_obo, body.as_object_mut()) {
        let mut value = body
            .get("value")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        value.insert("on_behalf_of".into(), obo.clone());
        body.insert("value".into(), Value::Object(value));
        body.insert("preserve_on_behalf_of".into(), json!(true));
    }
    ctx.post(
        &ctx.ws(&format!("resources/update/{}", path_enc(path))),
        body,
    )
    .await?;
    Ok(())
}

/// Deploys a variable draft; the note says when a secret's stored value was left alone.
async fn deploy_variable(
    ctx: &Ctx<'_>,
    path: &str,
    draft: &Value,
) -> Result<Option<String>, String> {
    let is_secret = truthy(draft.get("is_secret"));
    let stages_no_value = draft.get("value").and_then(Value::as_str) == Some("");
    if !item_exists(ctx, "variable", None, path).await? {
        if is_secret && stages_no_value {
            return Err(format!(
                "Draft variable \"{}\" is secret but stages no value, so it cannot be created. Run write_variable with its value first.",
                draft.get("path").map(js_str).unwrap_or_default()
            ));
        }
        ctx.post(&ctx.ws("variables/create"), draft.clone()).await?;
        return Ok(None);
    }
    // A draft never carries a secret's or an OAuth-managed value, so `''` there means
    // "none staged": sending it would blank the stored value or wipe a rotating token.
    let protected = is_secret
        || draft.get("is_oauth") == Some(&Value::Bool(true))
        || non_null(draft.get("account")).is_some();
    let keep_stored = protected && stages_no_value;
    let mut body = if keep_stored {
        without(draft, &["value"])
    } else {
        draft.clone()
    };
    // The draft holds the full variable, so an absent expiry was cleared, while the update
    // endpoint reads an absent one as unchanged.
    if let Some(body) = body.as_object_mut() {
        body.entry("value_valid_until").or_insert(Value::Null);
    }
    ctx.post(
        &ctx.ws(&format!("variables/update/{}", path_enc(path))),
        body,
    )
    .await?;
    Ok((is_secret && keep_stored).then(|| {
        "Its secret value was left unchanged, because the draft staged only metadata. If a new value was meant to be set, run write_variable again with it.".to_string()
    }))
}

/// A path runnable runs the DEPLOYED item at its path, so one pointing at a draft-only
/// script or flow fails at runtime.
async fn undeployed_runnable_targets(ctx: &Ctx<'_>, runnables: &Map<String, Value>) -> Vec<String> {
    let mut targets = Vec::new();
    for (key, runnable) in runnables {
        let kind = runnable.get("type").and_then(Value::as_str);
        let target_type = if matches!(kind, Some("path" | "runnableByPath")) {
            runnable.get("runType").and_then(Value::as_str)
        } else {
            kind
        };
        let (Some(target_type @ ("script" | "flow")), Some(path)) = (
            target_type,
            runnable
                .get("path")
                .and_then(Value::as_str)
                .filter(|p| !p.is_empty()),
        ) else {
            continue;
        };
        // Advisory: a failed probe must not block the deploy it annotates.
        if let Ok(false) = deployed_item_exists(ctx, target_type, None, path).await {
            targets.push(format!("\"{key}\" → {target_type} \"{path}\""));
        }
    }
    targets
}

async fn guest_access_is_live(ctx: &Ctx<'_>) -> bool {
    let (Ok(usage), Ok(settings)) = (
        ctx.get(&ctx.ws("workspaces/guest_usage")).await,
        ctx.get(&ctx.ws("workspaces/get_public_settings")).await,
    ) else {
        return false;
    };
    truthy(usage.get("available"))
        && truthy(usage.get("instance_enabled"))
        && truthy(settings.get("guest_access_enabled"))
}

/// Deploys a raw app draft from its sources: the server bundles them on a worker and
/// derives the policy's triggerables from the runnables. Returns the note for the result.
async fn deploy_app(
    ctx: &Ctx<'_>,
    path: &str,
    item: &Value,
    draft: &Value,
    message: Option<&str>,
    force: bool,
) -> Result<Option<String>, String> {
    if let Some(base) = non_null(item.get("parentVersionId")) {
        let head = if item_exists(ctx, "app", None, path).await? {
            get_deployed(ctx, "app", None, path)
                .await?
                .get("versions")
                .and_then(Value::as_array)
                .and_then(|versions| versions.last())
                .cloned()
        } else {
            None
        };
        assert_draft_based_on_latest("app", path, Some(base), head.as_ref(), force)?;
    }
    let object_or_empty = |key: &str| {
        draft
            .get(key)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default()
    };
    let runnables = object_or_empty("runnables");
    let mut policy = object_or_empty("policy");
    if !truthy(policy.get("execution_mode")) {
        policy.insert("execution_mode".into(), json!("publisher"));
    }

    let mut notes = Vec::new();
    let undeployed = undeployed_runnable_targets(ctx, &runnables).await;
    if !undeployed.is_empty() {
        notes.push(format!(
            "These backend runnables point at items that are NOT deployed, so they fail at runtime: \
             {}. Deploy those items too, and tell the user the app is not working until they are.",
            undeployed.join(", ")
        ));
    }
    // What a runnable runs as is the server's to decide, so the note names who can
    // reach the app and never an identity.
    match policy.get("execution_mode").and_then(Value::as_str) {
        Some("anonymous") => notes.push(
            "This app is deployed as anonymous: its backend runnables are now reachable by anyone \
             with the URL, without logging in. Tell the user plainly what is now reachable and by whom."
                .to_string(),
        ),
        // `guest` is stored whether or not a guest is admitted: silent unless it is live.
        Some("guest") if guest_access_is_live(ctx).await => notes.push(
            "This app is deployed as guest: anyone the instance's identity provider authenticates \
             can now open it and run its backend runnables, member of this workspace or not. Tell \
             the user plainly what is now reachable and by whom."
                .to_string(),
        ),
        _ => {}
    }

    // An editor-created draft is parked at a synthetic storage key, its chosen path in
    // the stored draft. Only a missing item justifies falling back to the storage path.
    let target = match ctx
        .get(&ctx.ws(&format!(
            "apps/get/p/{}?get_draft=true&raw_app=true",
            path_enc(path)
        )))
        .await
    {
        Ok(row) => non_null(row.get("draft"))
            .and_then(|d| non_null(d.get("draft_path")).or_else(|| non_null(d.get("path"))))
            .or_else(|| non_null(row.get("draft_path")))
            .and_then(Value::as_str)
            .unwrap_or(path)
            .to_string(),
        Err(e) if is_not_found(&e) => path.to_string(),
        Err(e) => return Err(e),
    };

    let mut app = Map::new();
    app.insert("path".into(), json!(target));
    app.insert(
        "value".into(),
        json!({
            "files": object_or_empty("files"),
            "runnables": runnables,
            "data": non_null(draft.get("data")).cloned().unwrap_or_else(|| json!({ "tables": [] })),
        }),
    );
    app.insert(
        "summary".into(),
        non_null(draft.get("summary"))
            .or_else(|| non_null(item.get("summary")))
            .cloned()
            .unwrap_or(json!("")),
    );
    if truthy(policy.get("on_behalf_of")) {
        app.insert("preserve_on_behalf_of".into(), json!(true));
    }
    app.insert("policy".into(), Value::Object(policy));
    if let Some(message) = message {
        app.insert("deployment_message".into(), json!(message));
    }
    if item_exists(ctx, "app", None, &target).await? {
        // `custom_path` stays out of an update: the server keeps it when absent, and
        // sending it takes admin rights.
        ctx.post(
            &ctx.ws(&format!("apps/update_raw_source/{}", path_enc(&target))),
            Value::Object(app),
        )
        .await?;
    } else {
        if let Some(custom_path) = draft.get("custom_path") {
            app.insert("custom_path".into(), custom_path.clone());
        }
        ctx.post(&ctx.ws("apps/create_raw_source"), Value::Object(app))
            .await?;
    }
    Ok((!notes.is_empty()).then(|| notes.join(" ")))
}

fn mutation_result(
    message: String,
    item_type: &str,
    path: &str,
    trigger_kind: Option<&str>,
    url: Option<String>,
) -> String {
    let mut out = Map::new();
    out.insert("success".into(), json!(true));
    out.insert("message".into(), json!(message));
    out.insert("type".into(), json!(item_type));
    out.insert("path".into(), json!(path));
    if let Some(trigger_kind) = trigger_kind {
        out.insert("triggerKind".into(), json!(trigger_kind));
    }
    if let Some(url) = url {
        out.insert("url".into(), json!(url));
    }
    pretty(&Value::Object(out))
}

/// Where a deployed item is seen in Windmill.
fn deployed_page(item_type: &str, path: &str, value: &Value) -> Option<String> {
    Some(match item_type {
        "script" => format!("/scripts/get/{path}"),
        "flow" => format!("/flows/get/{path}"),
        "app" => format!("/apps_raw/get/{path}"),
        "variable" => format!("/variables#{path}"),
        "resource" if super::workspace::is_agent(value) => format!("/agents/get/{path}"),
        "resource" => format!("/resources#/resource/{path}"),
        "schedule" => format!("/schedules#{path}"),
        _ => return None,
    })
}

async fn deploy_item(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let item_type = item_type_arg(args)?;
    let path = str_arg(args, "path")?;
    let trigger_kind = opt_str_arg(args, "trigger_kind");
    let message = opt_str_arg(args, "deployment_message");
    let force = bool_arg(args, "force");
    if item_type == "trigger" && trigger_kind.is_none() {
        return Err("trigger_kind is required when deploying a trigger.".to_string());
    }
    let no_draft = || format!("No draft found for {item_type} \"{path}\".");
    let kind = draft_kind(item_type, trigger_kind).ok_or_else(no_draft)?;
    let (item, draft) = get_draft_item(ctx, item_type, path, trigger_kind)
        .await?
        .ok_or_else(no_draft)?;
    let value = item
        .get("value")
        .ok_or_else(|| format!("Draft {item_type} \"{path}\" has no value to deploy."))?;

    let mut note = None;
    match item_type {
        "script" => {
            let head = if item_exists(ctx, "script", None, path).await? {
                get_deployed(ctx, "script", None, path)
                    .await?
                    .get("hash")
                    .cloned()
            } else {
                None
            };
            assert_draft_based_on_latest(
                "script",
                path,
                item.get("parentHash"),
                head.as_ref(),
                force,
            )?;
            deploy_script(ctx, path, message).await?;
        }
        "flow" => {
            let exists = item_exists(ctx, "flow", None, path).await?;
            let head = if exists {
                get_deployed(ctx, "flow", None, path)
                    .await?
                    .get("version_id")
                    .cloned()
            } else {
                None
            };
            assert_draft_based_on_latest(
                "flow",
                path,
                item.get("parentVersionId"),
                head.as_ref(),
                force,
            )?;
            deploy_flow(ctx, path, !exists, message).await?;
        }
        "variable" => note = deploy_variable(ctx, path, value).await?,
        "resource" => deploy_resource(ctx, path, value).await?,
        "app" => note = deploy_app(ctx, path, &item, value, message, force).await?,
        _ => upsert(ctx, item_type, trigger_kind, path, value.clone()).await?,
    }

    // The baseline keeps a draft saved since the read above from being deleted undeployed.
    delete_draft(ctx, &kind, path, draft.created_at.as_deref()).await?;

    let note = note.map(|note| format!(" {note}")).unwrap_or_default();
    Ok(mutation_result(
        format!("Deployed draft {item_type} \"{path}\" to the workspace. Draft removed.{note}"),
        item_type,
        path,
        trigger_kind,
        deployed_page(item_type, path, value).map(|page| ctx.page_url(&page)),
    ))
}

// ============================================================================
// delete_workspace_item
// ============================================================================

async fn delete_item(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let item_type = item_type_arg(args)?;
    let path = str_arg(args, "path")?;
    let trigger_kind = opt_str_arg(args, "trigger_kind");
    if item_type == "trigger" && trigger_kind.is_none() {
        return Err("trigger_kind is required when deleting a trigger.".to_string());
    }
    // Undoing a draft is a discard, not a delete: say so before the delete route 404s.
    if !deployed_item_exists(ctx, item_type, trigger_kind, path).await? {
        let draft = get_draft_item(ctx, item_type, path, trigger_kind).await?;
        return Err(if draft.is_some() {
            format!(
                "No deployed {item_type} at \"{path}\" — it only exists as a draft, so there is nothing to delete \
                 from the workspace. Call discard_local_draft with the same arguments to remove the draft."
            )
        } else {
            format!("No {item_type} at \"{path}\": neither a deployed item nor a draft. Nothing to delete.")
        });
    }

    let path_in_url = path_enc(path);
    if item_type == "script" {
        ctx.request(
            "POST",
            &ctx.ws(&format!("scripts/delete/p/{path_in_url}")),
            None,
        )
        .await?;
    } else {
        ctx.delete(&ctx.ws(&format!(
            "{}/delete/{path_in_url}",
            service(item_type, trigger_kind)?
        )))
        .await?;
    }
    if let Some(kind) = draft_kind(item_type, trigger_kind) {
        delete_draft(ctx, &kind, path, None).await?;
    }
    Ok(mutation_result(
        format!(
            "Deleted {item_type} \"{path}\" from the workspace. Any matching draft was also cleared."
        ),
        item_type,
        path,
        trigger_kind,
        None,
    ))
}

// ============================================================================
// Stable YAML: what the session's `yaml` library prints for
// `stringify(value, { sortMapEntries: true, aliasDuplicateObjects: false, lineWidth: 0 })`,
// so a diff reads the same here as in a session.
// ============================================================================

static YAML_CONTROL_CHARS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\x00-\x08\x0b-\x1f\x7f-\u{9f}]").expect("valid regex"));
static YAML_NOT_PLAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^[\n\t ,\[\]{}#&*!|>'"%@`]|^[?-]$|^[?-][ \t]|[\n:][ \t]|[ \t]\n|[\n\t ]#|[\n\t :]$"#,
    )
    .expect("valid regex")
});
/// Plain scalars the core schema would read back as null, a boolean or a number.
static YAML_NOT_A_STRING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"^(?:~|[Nn]ull|NULL)?$",
        r"|^(?:[Tt]rue|TRUE|[Ff]alse|FALSE)$",
        r"|^0o[0-7]+$|^[-+]?[0-9]+$|^0x[0-9a-fA-F]+$",
        r"|^(?:[-+]?\.(?:inf|Inf|INF)|\.nan|\.NaN|\.NAN)$",
        r"|^[-+]?(?:\.[0-9]+|[0-9]+(?:\.[0-9]*)?)[eE][-+]?[0-9]+$",
        r"|^[-+]?(?:\.[0-9]+|[0-9]+\.[0-9]*)$",
    ))
    .expect("valid regex")
});
static YAML_DOCUMENT_MARKER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^(%|---|\.\.\.)").expect("valid regex"));

/// Null object properties are dropped so `field: null` never diffs against an absent
/// field; array elements are kept so positions stay aligned.
fn prune_nulls(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(prune_nulls).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), prune_nulls(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn to_stable_yaml(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(value) => yaml_node(&prune_nulls(value), "") + "\n",
    }
}

fn yaml_node(value: &Value, indent: &str) -> String {
    let nested = format!("{indent}  ");
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => yaml_number(n),
        Value::String(s) => yaml_string(s, indent, false),
        Value::Array(items) if items.is_empty() => "[]".to_string(),
        Value::Object(map) if map.is_empty() => "{}".to_string(),
        Value::Array(items) => items
            .iter()
            .map(|item| format!("- {}", yaml_node(item, &nested)))
            .collect::<Vec<_>>()
            .join(&format!("\n{indent}")),
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            entries
                .into_iter()
                .map(|(key, value)| {
                    let key = yaml_string(key, &nested, true);
                    let rendered = yaml_node(value, &nested);
                    let is_block = match value {
                        Value::Array(items) => !items.is_empty(),
                        Value::Object(map) => !map.is_empty(),
                        _ => false,
                    };
                    if is_block {
                        format!("{key}:\n{nested}{rendered}")
                    } else {
                        format!("{key}: {rendered}")
                    }
                })
                .collect::<Vec<_>>()
                .join(&format!("\n{indent}"))
        }
    }
}

// ponytail: prints what Rust prints for a float. JS switches to exponent notation
// below 1e-6 and from 1e21, and rounds integers past 2^53; port Number#toString if a
// diff of such numbers ever has to match a session's byte for byte.
fn yaml_number(n: &serde_json::Number) -> String {
    match n.as_f64() {
        Some(f) if !n.is_i64() && !n.is_u64() && f.fract() == 0.0 && f.abs() < 1e21 => {
            format!("{f:.0}")
        }
        _ => n.to_string(),
    }
}

fn yaml_string(value: &str, indent: &str, implicit_key: bool) -> String {
    if YAML_CONTROL_CHARS.is_match(value) {
        return yaml_double_quoted(value, indent, implicit_key);
    }
    let multiline = value.contains('\n');
    if implicit_key && multiline {
        return yaml_quoted(value, indent, implicit_key);
    }
    if multiline {
        return yaml_block(value, indent);
    }
    let top_level_key = implicit_key && indent.len() == 2;
    if YAML_NOT_PLAIN.is_match(value)
        || YAML_NOT_A_STRING.is_match(value)
        || (top_level_key && YAML_DOCUMENT_MARKER.is_match(value))
    {
        return yaml_quoted(value, indent, implicit_key);
    }
    value.to_string()
}

fn yaml_quoted(value: &str, indent: &str, implicit_key: bool) -> String {
    let single = value.contains('"') && !value.contains('\'') && !value.contains('\n');
    if single {
        format!("'{}'", value.replace('\'', "''"))
    } else {
        yaml_double_quoted(value, indent, implicit_key)
    }
}

/// Lines of a double-quoted scalar shorter than this stay on one line, newlines escaped.
const YAML_MIN_MULTILINE_LENGTH: usize = 40;

fn yaml_double_quoted(value: &str, indent: &str, implicit_key: bool) -> String {
    let json: Vec<char> = serde_json::to_string(value)
        .unwrap_or_default()
        .chars()
        .collect();
    let at = |i: usize| json.get(i).copied();
    let mut out = String::new();
    let mut start = 0;
    let mut i = 0;
    while i < json.len() {
        let mut ch = json[i];
        // A space before a line break would be folded away unless escaped.
        if ch == ' ' && at(i + 1) == Some('\\') && at(i + 2) == Some('n') {
            out.extend(&json[start..i]);
            out.push_str("\\ ");
            i += 1;
            start = i;
            ch = '\\';
        }
        if ch == '\\' {
            match at(i + 1) {
                Some('u') => {
                    out.extend(&json[start..i]);
                    let code: String = json[i + 2..(i + 6).min(json.len())].iter().collect();
                    match code.as_str() {
                        "0000" => out.push_str("\\0"),
                        "0007" => out.push_str("\\a"),
                        "000b" => out.push_str("\\v"),
                        "001b" => out.push_str("\\e"),
                        _ if code.starts_with("00") => out.push_str(&format!("\\x{}", &code[2..])),
                        _ => out.push_str(&format!("\\u{code}")),
                    }
                    i += 5;
                    start = i + 1;
                }
                Some('n') => {
                    if implicit_key
                        || at(i + 2) == Some('"')
                        || json.len() < YAML_MIN_MULTILINE_LENGTH
                    {
                        i += 1;
                    } else {
                        // A real line break; the reader folds the first one away.
                        out.extend(&json[start..i]);
                        out.push_str("\n\n");
                        while at(i + 2) == Some('\\')
                            && at(i + 3) == Some('n')
                            && at(i + 4) != Some('"')
                        {
                            out.push('\n');
                            i += 2;
                        }
                        out.push_str(indent);
                        if at(i + 2) == Some(' ') {
                            out.push('\\');
                        }
                        i += 1;
                        start = i + 1;
                    }
                }
                _ => i += 1,
            }
        }
        i += 1;
    }
    if start == 0 {
        return json.into_iter().collect();
    }
    out.extend(&json[start.min(json.len())..]);
    out
}

/// Appends `indent` after every run of line breaks; `inner_only` leaves a run that
/// ends the text alone.
fn indent_after_breaks(text: &str, indent: &str, inner_only: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        out.push(ch);
        if ch == '\n' && chars.peek() != Some(&'\n') && !(inner_only && chars.peek().is_none()) {
            out.push_str(indent);
        }
    }
    out
}

/// A literal block scalar, so multi-line text (code most of all) diffs line by line.
fn yaml_block(value: &str, indent: &str) -> String {
    let is_space = |c: char| matches!(c, '\n' | '\t' | ' ');
    let body_len = value.trim_end_matches(is_space).len();
    let (mut body, mut end) = value.split_at(body_len);
    // A block cannot end in whitespace-only text after its last line break.
    if end
        .rfind('\n')
        .is_some_and(|last_break| last_break + 1 < end.len())
    {
        return yaml_quoted(value, indent, false);
    }
    let chomp = match end.find('\n') {
        None => "-",
        Some(first_break) if body.is_empty() || first_break != end.len() - 1 => "+",
        Some(_) => "",
    };
    end = end.strip_suffix('\n').unwrap_or(end);
    let end = indent_after_breaks(end, indent, true);

    // Leading spaces need an explicit indentation indicator; leading line breaks are
    // kept ahead of the body.
    let lead_len = body.len() - body.trim_start_matches([' ', '\n']).len();
    let lead = &body[..lead_len];
    let starts_with_space = lead.contains(' ');
    let start = lead.rfind('\n').map(|i| &lead[..=i]).unwrap_or("");
    body = &body[start.len()..];
    let indicator = match (starts_with_space, indent.is_empty()) {
        (false, _) => "",
        (true, true) => "1",
        (true, false) => "2",
    };
    format!(
        "|{indicator}{chomp}\n{indent}{}{}{end}",
        indent_after_breaks(start, indent, false),
        indent_after_breaks(body, indent, false),
    )
}

// ============================================================================
// Patches
// ============================================================================

fn line_count(patch: &str) -> usize {
    if patch.is_empty() {
        0
    } else {
        patch.split('\n').count()
    }
}

/// Unified patch between two sides of an item, `""` when they render the same.
fn yaml_patch(before: Option<&Value>, after: Option<&Value>, labels: (&str, &str)) -> String {
    let (before, after) = (to_stable_yaml(before), to_stable_yaml(after));
    if before == after {
        return String::new();
    }
    unified_patch(labels.0, labels.1, &before, &after, 3)
}

/// Unified patch between two raw files; an absent side reads as empty.
fn text_patch(before: Option<&str>, after: Option<&str>, labels: (&str, &str)) -> String {
    let (before, after) = (before.unwrap_or(""), after.unwrap_or(""));
    if before == after {
        return String::new();
    }
    unified_patch(labels.0, labels.1, before, after, 3)
}

/// Indices of the added and removed lines of a patch. The `---`/`+++` file labels only
/// occur ahead of the first hunk, while a changed source line may itself start with
/// `++` or `--`.
fn changed_line_indices(patch: &str) -> Vec<usize> {
    let mut in_hunk = false;
    let mut indices = Vec::new();
    for (i, line) in patch.split('\n').enumerate() {
        if line.starts_with("@@") {
            in_hunk = true;
        } else if in_hunk && (line.starts_with('+') || line.starts_with('-')) {
            indices.push(i);
        }
    }
    indices
}

const MAX_DIFF_PATCH_CHARS: usize = 50_000;
const DIFF_READ_DEFAULT_LINES: usize = 500;

/// A window of a patch by lines, with a character backstop. When the backstop cuts
/// inside the window the continuation offset advances past the complete lines
/// delivered only, or the rest of the window would be skipped for good.
fn window_patch(patch: &str, offset: usize, limit: usize, max_chars: usize) -> String {
    let lines: Vec<&str> = patch.split('\n').collect();
    let total = lines.len();
    if offset >= total {
        return format!(
            "(no lines at offset {offset} — the patch has {total} lines; call again with a smaller offset)"
        );
    }
    let requested_end = total.min(offset.saturating_add(limit));
    let mut body = lines[offset..requested_end].join("\n");
    let mut effective_end = requested_end;
    let mut note = String::new();
    if body.chars().count() > max_chars {
        body = body.chars().take(max_chars).collect();
        match body.rfind('\n') {
            // One line over the whole budget: deliver its head and step past it.
            None => {
                effective_end = offset + 1;
                note = format!("\n… [line truncated at {max_chars} chars]");
            }
            Some(last_break) => {
                body.truncate(last_break);
                effective_end = offset + body.split('\n').count();
                note = format!("\n… window truncated at {max_chars} chars.");
            }
        }
    }
    if offset > 0 || effective_end < total {
        let rest = if effective_end < total {
            format!("; call diff again with offset={effective_end} for the rest")
        } else {
            String::new()
        };
        note.push_str(&format!(
            "\n(lines {}-{effective_end} of {total}{rest})",
            offset + 1
        ));
    }
    body + note.as_str()
}

struct FileDiff {
    status: &'static str,
    patch: String,
    line_count: usize,
}

#[derive(Default)]
struct DiffParts {
    /// The whole-value patch, or the config-only patch of a multi-file app.
    patch: String,
    /// Changed files of a multi-file app, by name.
    files: Option<Vec<(String, FileDiff)>>,
    line_count: usize,
    has_changes: bool,
}

/// A map of string contents marks a multi-file app value.
fn app_files(value: Option<&Value>) -> Option<&Map<String, Value>> {
    value?
        .get("files")?
        .as_object()
        .filter(|files| !files.is_empty() && files.values().all(Value::is_string))
}

/// Per-file text patches plus a config-only patch for a multi-file app, one whole-value
/// YAML patch for anything else.
fn diff_parts(before: Option<&Value>, after: Option<&Value>, labels: (&str, &str)) -> DiffParts {
    let (before_files, after_files) = (app_files(before), app_files(after));
    if before_files.is_none() && after_files.is_none() {
        let patch = yaml_patch(before, after, labels);
        return DiffParts {
            line_count: line_count(&patch),
            has_changes: !patch.is_empty(),
            patch,
            files: None,
        };
    }
    let mut names: Vec<&String> = before_files
        .into_iter()
        .chain(after_files)
        .flat_map(|files| files.keys())
        .collect();
    names.sort();
    names.dedup();
    fn content<'a>(files: Option<&'a Map<String, Value>>, name: &str) -> Option<&'a str> {
        files?.get(name)?.as_str()
    }
    let mut files = Vec::new();
    let mut total = 0;
    for name in names {
        let (old, new) = (content(before_files, name), content(after_files, name));
        let patch = text_patch(old, new, labels);
        // An empty file appearing or disappearing yields no patch, but its presence
        // is a change in its own right.
        if patch.is_empty() && old.is_some() == new.is_some() {
            continue;
        }
        let line_count = line_count(&patch);
        total += line_count;
        let status = match (old, new) {
            (None, _) => "added",
            (_, None) => "deleted",
            _ => "modified",
        };
        files.push((name.clone(), FileDiff { status, patch, line_count }));
    }
    let config = |side: Option<&Value>| {
        side.map(|v| {
            if v.is_object() {
                without(v, &["files"])
            } else {
                v.clone()
            }
        })
    };
    let patch = yaml_patch(config(before).as_ref(), config(after).as_ref(), labels);
    DiffParts {
        line_count: total + line_count(&patch),
        has_changes: !files.is_empty() || !patch.is_empty(),
        patch,
        files: Some(files),
    }
}

// ============================================================================
// The two sides of a draft
// ============================================================================

/// The per-user markers a `get_draft=true` read adds beside the deployed item.
const OVERLAY_KEYS: &[&str] = &[
    "draft",
    "is_draft",
    "draft_saved_at",
    "draft_base",
    "no_deployed",
    "other_drafts_users",
];

/// Server-managed script fields: identical noise, or recomputed at every deploy.
const SCRIPT_RUNTIME_KEYS: &[&str] = &[
    "workspace_id",
    "hash",
    "parent_hash",
    "parent_hashes",
    "created_at",
    "created_by",
    "archived",
    "deleted",
    "extra_perms",
    "lock",
    "lock_error_logs",
    "starred",
    "has_draft",
    "draft_only",
    "assets",
    "marked",
];

/// Runtime fields of a schedule or trigger, which the workspace comparison ignores too.
const TRIGGER_RUNTIME_KEYS: &[&str] = &[
    "workspace_id",
    "edited_by",
    "edited_at",
    "email",
    "error",
    "enabled",
    "mode",
    "server_id",
    "last_server_ping",
    "extra_perms",
    "permissioned_as",
    "subscription_id",
    "push_auth_config",
];

struct DraftSides {
    deployed: Value,
    draft: Value,
    /// False when the read carried no draft, and the item's own value stood in for it.
    has_draft: bool,
    /// True when the item was never deployed.
    no_deployed: bool,
}

/// A raw app on the flat draft shape, without the inline-script locks the server
/// recomputes at every deploy.
fn canonical_raw_app(source: &Value, path: Option<&Value>) -> Value {
    let mut value = app_source_to_draft_value(source, None);
    if let Some(runnables) = value.get_mut("runnables").and_then(Value::as_object_mut) {
        for runnable in runnables.values_mut() {
            if let Some(inline) = runnable
                .get_mut("inlineScript")
                .and_then(Value::as_object_mut)
            {
                inline.shift_remove("lock");
            }
        }
    }
    // A staged rename changes where a deploy lands the app, so the path is compared too.
    if let (Some(path), Some(value)) = (path, value.as_object_mut()) {
        value.insert("path".into(), path.clone());
    }
    value
}

/// A low-code app draft split into its grid and the metadata the editor mirrors into
/// it: `(value, summary, draft_path)`.
fn classic_app_parts(json: Option<&Value>) -> (Value, Option<Value>, Option<Value>) {
    let Some(Value::Object(obj)) = json else {
        return (json.cloned().unwrap_or(Value::Null), None, None);
    };
    // A legacy draft wraps the grid; a bare app always carries `grid` at the top.
    let inner = obj.get("value").filter(|v| v.get("grid").is_some());
    if let (false, Some(inner)) = (obj.contains_key("grid"), inner) {
        let (value, summary, draft_path) = classic_app_parts(Some(inner));
        let outer = |key: &str| obj.get(key).filter(|v| !v.is_null()).cloned();
        return (
            value,
            outer("summary").or(summary),
            outer("draft_path").or(draft_path),
        );
    }
    let mut value = obj.clone();
    value.shift_remove("parent_version");
    let draft_path = value.shift_remove("draft_path").filter(|v| !v.is_null());
    let summary = value.shift_remove("summary").filter(|v| !v.is_null());
    (Value::Object(value), summary, draft_path)
}

/// A variable, resource, schedule or trigger on one field set, whether it comes as the
/// deployed row or as the editor state a draft stores.
fn canonical_drawer_value(kind: &str, raw: &Value, is_draft: bool) -> Value {
    if !raw.is_object() {
        return json!({});
    }
    let or_empty =
        |value: Option<&Value>| value.filter(|v| !v.is_null()).cloned().unwrap_or(json!(""));
    let ws_specific = if is_draft {
        "wsSpecific"
    } else {
        "ws_specific"
    };
    let mut out = Map::new();
    match kind {
        "variable" => {
            let empty = json!({});
            let variable = if is_draft {
                non_null(raw.get("variable")).unwrap_or(&empty)
            } else {
                raw
            };
            let is_secret = truthy(variable.get("is_secret"));
            out.insert(
                "value".into(),
                if is_secret {
                    json!("<secret>")
                } else {
                    or_empty(variable.get("value"))
                },
            );
            out.insert("is_secret".into(), json!(is_secret));
            out.insert("description".into(), or_empty(variable.get("description")));
        }
        "resource" => {
            let key = if is_draft { "args" } else { "value" };
            let mut value = non_null(raw.get(key)).cloned().unwrap_or(json!({}));
            // An agent's identity is in its deployed value and never in a draft: not an edit.
            if raw.get("resource_type").and_then(Value::as_str) == Some("ai_agent") {
                if let Some(value) = value.as_object_mut() {
                    value.shift_remove("on_behalf_of");
                }
            }
            out.insert("value".into(), value);
            out.insert("description".into(), or_empty(raw.get("description")));
            if let Some(resource_type) = non_null(raw.get("resource_type")) {
                out.insert("resource_type".into(), resource_type.clone());
            }
        }
        _ => return without(raw, TRIGGER_RUNTIME_KEYS),
    }
    if let Some(labels) = non_null(raw.get("labels")) {
        out.insert("labels".into(), labels.clone());
    }
    if let Some(ws_specific) = non_null(raw.get(ws_specific)) {
        out.insert("ws_specific".into(), ws_specific.clone());
    }
    Value::Object(out)
}

/// The deployed value and the draft value of an item, each normalised so that only a
/// real change differs. `kind` is a draft kind; an item that is neither deployed nor
/// drafted is the API's 404.
async fn draft_sides(ctx: &Ctx<'_>, kind: &str, path: &str) -> Result<DraftSides, String> {
    let path_in_url = path_enc(path);
    let route = match kind {
        "script" => format!("scripts/get/p/{path_in_url}?get_draft=true"),
        "flow" => format!("flows/get/{path_in_url}?get_draft=true"),
        // The server resolves a never-deployed app's draft kind from `raw_app`.
        "app" | "raw_app" => format!(
            "apps/get/p/{path_in_url}?get_draft=true&raw_app={}",
            kind == "raw_app"
        ),
        "variable" => format!("variables/get/{path_in_url}?decrypt_secret=false&get_draft=true"),
        "resource" => format!("resources/get/{path_in_url}?get_draft=true"),
        "trigger_schedule" => format!("schedules/get/{path_in_url}?get_draft=true"),
        _ => {
            let trigger_kind = kind
                .strip_prefix("trigger_")
                .filter(|k| TRIGGER_KINDS.contains(k))
                .ok_or_else(|| format!("Draft diff not supported for kind {kind}"))?;
            format!("{trigger_kind}_triggers/get/{path_in_url}?get_draft=true")
        }
    };
    Ok(sides_of_row(kind, &ctx.get(&ctx.ws(&route)).await?))
}

/// Splits a `get_draft=true` read into its two sides.
fn sides_of_row(kind: &str, row: &Value) -> DraftSides {
    let draft = non_null(row.get("draft"));
    let has_draft = draft.is_some();
    let no_deployed = row.get("no_deployed") == Some(&Value::Bool(true));
    let deployed = without(row, OVERLAY_KEYS);
    let (deployed, draft) = match kind {
        "script" => (
            without(&deployed, SCRIPT_RUNTIME_KEYS),
            without(draft.unwrap_or(&deployed), SCRIPT_RUNTIME_KEYS),
        ),
        // The base a draft pinned differs from the head when it is stale: not a change.
        "flow" => (
            without(&deployed, &["version_id"]),
            without(draft.unwrap_or(&deployed), &["version_id"]),
        ),
        "app" | "raw_app"
            if kind == "raw_app" || row.get("raw_app") == Some(&Value::Bool(true)) =>
        {
            let row_path = row.get("path");
            let staged_path = draft
                .and_then(|d| non_null(d.get("draft_path")))
                .or(row_path);
            (
                canonical_raw_app(row, row_path),
                canonical_raw_app(draft.unwrap_or(row), staged_path),
            )
        }
        "app" => {
            let row_summary = non_null(row.get("summary")).cloned().unwrap_or(json!(""));
            let deployed_parts = classic_app_parts(row.get("value"));
            let value = deployed_parts.0.clone();
            let (draft_value, summary, draft_path) = match draft {
                Some(draft) => classic_app_parts(Some(draft)),
                None => deployed_parts,
            };
            (
                json!({ "summary": row_summary, "value": value, "path": row.get("path") }),
                json!({
                    "summary": summary.unwrap_or(row_summary.clone()),
                    "value": draft_value,
                    "path": draft_path.or_else(|| row.get("path").cloned()),
                }),
            )
        }
        _ => (
            canonical_drawer_value(kind, &deployed, false),
            canonical_drawer_value(kind, draft.unwrap_or(&deployed), true),
        ),
    };
    DraftSides { deployed, draft, has_draft, no_deployed }
}

const VARIABLE_VALUE_PLACEHOLDER: &str = "<variable value — never shown in chat>";
const VARIABLE_VALUE_CHANGED_PLACEHOLDER: &str = "<variable value — never shown in chat (changed)>";
const VARIABLE_MASKED_NOTE: &str = "Note: variable values are never shown in chat — the diff marks whether the value changed without revealing it.\n\n";
const SECRET_UNCOMPARABLE_NOTE: &str = "Note: this is a SECRET variable — its value is never shown and cannot be compared, so it may ALSO have changed beyond what this diff shows.\n\n";

/// A variable's value never reaches a tool result, secret or not: both sides get a
/// placeholder that still shows WHETHER a readable value changed. The flag is set for a
/// secret, whose sides are never comparable — the deployed value is not decrypted and a
/// draft holds a marker — so equal sides prove nothing.
fn mask_variable_sides(before: &mut Option<Value>, after: &mut Option<Value>) -> bool {
    let object = |side: &Option<Value>| side.as_ref().is_some_and(Value::is_object);
    let both = object(before) && object(after);
    let is_secret = |side: &Option<Value>| {
        side.as_ref().and_then(|v| v.get("is_secret")) == Some(&Value::Bool(true))
    };
    let secret = is_secret(before) || is_secret(after);
    let value = |side: &Option<Value>| side.as_ref().and_then(|v| v.get("value")).cloned();
    let changed = !secret && both && value(before) != value(after);
    if let Some(Value::Object(before)) = before {
        before.insert("value".into(), json!(VARIABLE_VALUE_PLACEHOLDER));
    }
    if let Some(Value::Object(after)) = after {
        let placeholder = if changed {
            VARIABLE_VALUE_CHANGED_PLACEHOLDER
        } else {
            VARIABLE_VALUE_PLACEHOLDER
        };
        after.insert("value".into(), json!(placeholder));
    }
    secret && both
}

// ============================================================================
// diff
// ============================================================================

/// One of the caller's drafts, as the drafts listing reports it.
struct DraftRow {
    kind: String,
    /// The path the draft is stored under.
    path: String,
    /// The path the item is known by: a staged name when there is one.
    display_path: String,
    /// The item type and trigger kind a tool addresses the draft by, if any does.
    addressing: Option<(&'static str, Option<&'static str>)>,
}

fn addressing_of(kind: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match kind {
        "script" => ("script", None),
        "flow" => ("flow", None),
        "resource" => ("resource", None),
        "variable" => ("variable", None),
        "app" | "raw_app" => ("app", None),
        "trigger_schedule" => ("schedule", None),
        _ => {
            let trigger_kind = kind.strip_prefix("trigger_")?;
            let trigger_kind = TRIGGER_KINDS.iter().find(|k| **k == trigger_kind)?;
            ("trigger", Some(*trigger_kind))
        }
    })
}

/// The caller's drafts, and how many drafts other users hold in the workspace.
async fn list_draft_rows(ctx: &Ctx<'_>) -> Result<(Vec<DraftRow>, usize), String> {
    let rows = ctx.get(&ctx.ws("drafts/list?all_users=true")).await?;
    let rows = array(Some(&rows));
    let mine: Vec<DraftRow> = rows
        .iter()
        .filter(|row| row.get("mine") != Some(&Value::Bool(false)))
        .filter_map(|row| {
            let kind = row.get("kind")?.as_str()?;
            let path = row.get("path")?.as_str()?;
            let display_path = row
                .get("draft_path")
                .and_then(Value::as_str)
                .filter(|p| !p.is_empty())
                .unwrap_or(path);
            Some(DraftRow {
                kind: kind.to_string(),
                path: path.to_string(),
                display_path: display_path.to_string(),
                addressing: addressing_of(kind),
            })
        })
        .collect();
    let others = rows.len() - mine.len();
    Ok((mine, others))
}

/// A draft's computed diff against what is deployed.
#[derive(Default)]
struct Materialized {
    parts: DiffParts,
    /// Variable content was masked: the patch marks a value change without showing it.
    value_masked: bool,
    value_uncomparable: bool,
    no_deployed: bool,
    error: Option<String>,
}

async fn materialize(ctx: &Ctx<'_>, kind: &str, path: &str) -> Materialized {
    match draft_sides(ctx, kind, path).await {
        Ok(sides) => materialized_of(kind, sides),
        Err(e) => {
            let error = if is_not_found(&e) {
                "item not found (the draft may reference a deleted item)".to_string()
            } else {
                e
            };
            Materialized { error: Some(error), ..Default::default() }
        }
    }
}

fn materialized_of(kind: &str, sides: DraftSides) -> Materialized {
    let mut before = (!sides.no_deployed).then_some(sides.deployed);
    let mut after = Some(sides.draft);
    let value_masked = kind == "variable";
    let value_uncomparable = value_masked && mask_variable_sides(&mut before, &mut after);
    Materialized {
        parts: diff_parts(before.as_ref(), after.as_ref(), ("deployed", "draft")),
        value_masked,
        value_uncomparable,
        no_deployed: sides.no_deployed,
        error: None,
    }
}

/// What an item read prints for a multi-file app: one file's patch when `file` is
/// given, otherwise the per-file summary plus the config changes.
fn render_files(
    files: &[(String, FileDiff)],
    config_patch: &str,
    file: Option<&str>,
    offset: usize,
    limit: usize,
) -> ToolResult {
    let window = |patch: &str| window_patch(patch, offset, limit, MAX_DIFF_PATCH_CHARS);
    if let Some(requested) = file {
        // App files are keyed with a leading slash: the slash-less spelling and a
        // unique base name are accepted too.
        let suffix = format!("/{}", requested.strip_prefix('/').unwrap_or(requested));
        let by_suffix: Vec<_> = files.iter().filter(|(n, _)| n.ends_with(&suffix)).collect();
        let resolved = files
            .iter()
            .find(|(name, _)| name == requested)
            .or_else(|| {
                files
                    .iter()
                    .find(|(name, _)| *name == format!("/{requested}"))
            })
            .or_else(|| (by_suffix.len() == 1).then(|| by_suffix[0]));
        let Some((name, file_diff)) = resolved else {
            let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
            let changed = if names.is_empty() {
                "(none)".to_string()
            } else {
                names.join(", ")
            };
            return Err(format!(
                "No changes in file \"{requested}\". Changed files: {changed}."
            ));
        };
        if file_diff.patch.is_empty() {
            return Ok(format!(
                "File \"{name}\" was {} with empty content.",
                file_diff.status
            ));
        }
        return Ok(window(&file_diff.patch));
    }
    let mut sections = Vec::new();
    if files.is_empty() {
        sections.push("No file contents changed.".to_string());
    } else {
        let lines: Vec<String> = files
            .iter()
            .map(|(name, file_diff)| {
                let size = match (file_diff.status, file_diff.line_count) {
                    ("deleted", _) => String::new(),
                    (_, 0) => " (empty file)".to_string(),
                    (_, n) => format!(" ({n} diff lines)"),
                };
                format!("- {name} — {}{size}", file_diff.status)
            })
            .collect();
        sections.push(format!(
            "{} file(s) changed:\n{}\nRead one with file=\"<name>\".",
            lines.len(),
            lines.join("\n")
        ));
    }
    if !config_patch.is_empty() {
        sections.push(format!("Config changes:\n{}", window(config_patch)));
    }
    Ok(sections.join("\n\n"))
}

async fn diff_item(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let non_empty = |name: &str| opt_str_arg(args, name).filter(|s| !s.is_empty());
    let (Some(item_type), Some(path)) = (non_empty("type"), non_empty("path")) else {
        return Err("type is required when path is provided.".to_string());
    };
    if !ITEM_TYPES.contains(&item_type) {
        return Err(format!("Invalid type \"{item_type}\""));
    }
    let kind = draft_kind(item_type, opt_str_arg(args, "trigger_kind"))
        .ok_or("trigger_kind is required when type is trigger.")?;
    let file = opt_str_arg(args, "file");
    let offset = uint_arg(args, "offset").unwrap_or(0);
    let limit = uint_arg(args, "limit").unwrap_or(DIFF_READ_DEFAULT_LINES);

    // The listing is what knows a draft parked under a synthetic key by the name staged
    // for it, and which of the two app kinds a draft at this path is.
    let kinds: &[&str] = if item_type == "app" {
        &["raw_app", "app"]
    } else {
        &[kind.as_str()]
    };
    let (rows, _) = list_draft_rows(ctx).await?;
    let listed = kinds
        .iter()
        .find_map(|kind| rows.iter().find(|r| r.kind == *kind && r.path == path))
        .or_else(|| {
            rows.iter().find(|r| {
                kinds.contains(&r.kind.as_str()) && (r.display_path == path || r.path == path)
            })
        });

    let mut caveat = "";
    let (parts, no_deployed, value_uncomparable) = match listed {
        Some(row) => {
            let m = materialize(ctx, &row.kind, &row.path).await;
            if let Some(error) = m.error {
                return Err(format!("Could not diff {item_type} \"{path}\": {error}"));
            }
            if m.value_masked {
                caveat = if m.value_uncomparable {
                    SECRET_UNCOMPARABLE_NOTE
                } else {
                    VARIABLE_MASKED_NOTE
                };
            }
            (m.parts, m.no_deployed, m.value_uncomparable)
        }
        // Not listed: no draft at all, nothing at the path, or a draft the listing
        // does not show. The item's own read settles which.
        None => {
            let sides = draft_sides(ctx, &kind, path).await.map_err(|e| {
                if is_not_found(&e) {
                    format!(
                        "No {item_type} found at \"{path}\" — it has neither a deployed version nor a draft."
                    )
                } else {
                    e
                }
            })?;
            if !sides.no_deployed && !sides.has_draft {
                return Ok(format!(
                    "No draft exists for {item_type} \"{path}\" — the deployed version is current."
                ));
            }
            let mut before = (!sides.no_deployed).then_some(sides.deployed);
            let mut after = Some(sides.draft);
            let mut value_uncomparable = false;
            if kind == "variable" {
                value_uncomparable = mask_variable_sides(&mut before, &mut after);
                caveat = if value_uncomparable {
                    SECRET_UNCOMPARABLE_NOTE
                } else {
                    VARIABLE_MASKED_NOTE
                };
            }
            let patch = yaml_patch(before.as_ref(), after.as_ref(), ("deployed", "draft"));
            (
                DiffParts { patch, ..Default::default() },
                sides.no_deployed,
                value_uncomparable,
            )
        }
    };

    let changed_files = parts.files.as_ref().map_or(0, Vec::len);
    if parts.patch.is_empty() && changed_files == 0 {
        // A secret is masked on both ends: an empty patch cannot prove it unchanged.
        let message = if value_uncomparable {
            format!(
                "No visible changes for {item_type} \"{path}\" — but a secret's value cannot be compared and may have been updated in the draft."
            )
        } else {
            format!("Draft matches the deployed version of {item_type} \"{path}\" — no changes.")
        };
        return Ok(format!("{caveat}{message}"));
    }
    let header = if no_deployed {
        format!("{item_type} \"{path}\" has no deployed version yet — the entire draft is new.\n\n")
    } else {
        format!("Draft changes vs deployed for {item_type} \"{path}\":\n\n")
    };
    let body = match &parts.files {
        Some(files) => render_files(files, &parts.patch, file, offset, limit)?,
        None if file.is_some() => {
            return Err(format!(
                "file only applies to multi-file apps; {item_type} \"{path}\" diffs as a single document — call again without file."
            ))
        }
        None => window_patch(&parts.patch, offset, limit, MAX_DIFF_PATCH_CHARS),
    };
    Ok(format!("{caveat}{header}{body}"))
}

/// Patches computed per index read, so a workspace with hundreds of drafts still
/// answers; the rest report as pending and compute on an item read.
const EAGER_MATERIALIZE_CAP: usize = 50;
const FETCH_CONCURRENCY: usize = 6;
const DIFF_INDEX_DEFAULT_ITEMS: usize = 50;
const DIFF_INDEX_MAX_ITEMS: usize = 100;
const DIFF_INDEX_MAX_FILE_CHILDREN: usize = 20;

struct IndexEntry {
    row: DraftRow,
    materialized: Option<Materialized>,
}

/// Every draft of the caller with its diff, computed for the first `cap` addressable ones.
async fn diff_index_entries(ctx: &Ctx<'_>, cap: usize) -> Result<(Vec<IndexEntry>, usize), String> {
    let (rows, others) = list_draft_rows(ctx).await?;
    let eager: Vec<usize> = (0..rows.len())
        .filter(|i| rows[*i].addressing.is_some())
        .take(cap)
        .collect();
    // Built ahead of the stream: a closure borrowing `rows` inside it does not make a
    // `Send` future.
    let fetches: Vec<_> = eager
        .iter()
        .map(|i| materialize(ctx, &rows[*i].kind, &rows[*i].path))
        .collect();
    let computed: Vec<Materialized> = stream::iter(fetches)
        .buffered(FETCH_CONCURRENCY)
        .collect()
        .await;
    let mut entries: Vec<IndexEntry> = rows
        .into_iter()
        .map(|row| IndexEntry { row, materialized: None })
        .collect();
    for (i, materialized) in eager.into_iter().zip(computed) {
        entries[i].materialized = Some(materialized);
    }
    Ok((entries, others))
}

fn index_line(entry: &IndexEntry) -> String {
    let row = &entry.row;
    let Some((item_type, trigger_kind)) = row.addressing else {
        return format!(
            "- {} draft \"{}\" — not addressable in this chat",
            row.kind, row.display_path
        );
    };
    let label = match trigger_kind {
        Some(trigger_kind) => format!("{trigger_kind} trigger"),
        None => item_type.to_string(),
    };
    let name = format!("{label} \"{}\"", row.display_path);
    let Some(m) = &entry.materialized else {
        return format!("- {name} — draft present (diff not computed yet; read it with type+path)");
    };
    let lines = m.parts.line_count;
    match (
        &m.error,
        m.no_deployed,
        m.parts.has_changes,
        m.value_uncomparable,
    ) {
        (Some(error), ..) => format!("- {name} — diff failed: {error}"),
        (None, true, ..) => format!("- {name} — new, never deployed ({lines} lines)"),
        (None, false, true, true) => {
            format!("- {name} — modified ({lines} diff lines; secret value may also differ)")
        }
        (None, false, true, false) => format!("- {name} — modified ({lines} diff lines)"),
        (None, false, false, true) => {
            format!("- {name} — no visible changes (secret value cannot be compared; may differ)")
        }
        (None, false, false, false) => format!("- {name} — draft matches deployed"),
    }
}

/// The indented per-file rows under a multi-file app's index line.
fn file_children_lines(entry: &IndexEntry) -> Vec<String> {
    let Some(parts) = entry.materialized.as_ref().map(|m| &m.parts) else {
        return Vec::new();
    };
    let Some(files) = &parts.files else {
        return Vec::new();
    };
    let mut lines: Vec<String> = files
        .iter()
        .take(DIFF_INDEX_MAX_FILE_CHILDREN)
        .map(|(name, file_diff)| {
            let size = match (file_diff.status, file_diff.line_count) {
                ("deleted", _) => String::new(),
                (_, 0) => " (empty file)".to_string(),
                (_, n) => format!(" ({n} lines)"),
            };
            format!("    · {name} — {}{size}", file_diff.status)
        })
        .collect();
    if files.len() > DIFF_INDEX_MAX_FILE_CHILDREN {
        lines.push(format!(
            "    · … {} more files",
            files.len() - DIFF_INDEX_MAX_FILE_CHILDREN
        ));
    }
    if !parts.patch.is_empty() {
        lines.push(format!(
            "    · (config) — modified ({} lines)",
            line_count(&parts.patch)
        ));
    }
    lines
}

async fn diff_index(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let (entries, others) = diff_index_entries(ctx, EAGER_MATERIALIZE_CAP).await?;
    let types: Vec<&str> = args
        .get("types")
        .and_then(Value::as_array)
        .map(|types| types.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let prefix = opt_str_arg(args, "path_prefix").filter(|p| !p.is_empty());
    let matching: Vec<&IndexEntry> = entries
        .iter()
        .filter(|e| {
            types.is_empty()
                || e.row
                    .addressing
                    .is_some_and(|(item_type, _)| types.contains(&item_type))
        })
        .filter(|e| {
            prefix.is_none_or(|prefix| {
                e.row.display_path.starts_with(prefix) || e.row.path.starts_with(prefix)
            })
        })
        .collect();
    let total = matching.len();
    let limit = uint_arg(args, "limit")
        .unwrap_or(DIFF_INDEX_DEFAULT_ITEMS)
        .min(DIFF_INDEX_MAX_ITEMS);
    let shown = &matching[..total.min(limit)];

    let filtered = !types.is_empty() || prefix.is_some();
    let mut lines = vec![if total > 0 {
        format!("{total} draft(s) vs deployed:")
    } else if filtered && !entries.is_empty() {
        format!(
            "No drafts match your filters ({} draft(s) exist in the workspace).",
            entries.len()
        )
    } else {
        "No drafts in this workspace — nothing differs from the deployed state.".to_string()
    }];
    for entry in shown {
        lines.push(index_line(entry));
        lines.extend(file_children_lines(entry));
    }
    if total > shown.len() {
        lines.push(format!(
            "Showing {} of {total} drafts — narrow with types/path_prefix or raise limit.",
            shown.len()
        ));
    }
    if others > 0 {
        lines.push(format!(
            "{others} draft(s) by other users exist in this workspace (not shown — drafts are per-user)."
        ));
    }
    Ok(lines.join("\n"))
}

const DIFF_SEARCH_DEFAULT_MAX_MATCHES: usize = 50;
const DIFF_SEARCH_MAX_MATCHES_CEILING: usize = 200;
const SEARCH_CONTEXT_LINES: usize = 2;
const SEARCH_MAX_LINE_CHARS: usize = 200;
const SEARCH_TOTAL_CHAR_BUDGET: usize = 12_000;

/// Literal, case-insensitive search over the changed lines of every draft's diff.
async fn diff_search(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let query = opt_str_arg(args, "search").unwrap_or_default();
    if query.is_empty() {
        return Err("search requires a non-empty string.".to_string());
    }
    if opt_str_arg(args, "against") == Some("parent_workspace") {
        return Err(parent_comparison_unavailable(ctx).await);
    }
    let (entries, _) = diff_index_entries(ctx, usize::MAX).await?;
    let file_glob = opt_str_arg(args, "file_glob").filter(|g| !g.is_empty());
    let max_matches = uint_arg(args, "max_matches")
        .unwrap_or(DIFF_SEARCH_DEFAULT_MAX_MATCHES)
        .min(DIFF_SEARCH_MAX_MATCHES_CEILING);
    Ok(search_entries(&entries, query, file_glob, max_matches))
}

fn search_entries(
    entries: &[IndexEntry],
    query: &str,
    file_glob: Option<&str>,
    max_matches: usize,
) -> String {
    // `(subject, patch)`: an item path, or `<item path>/<file>` for a file inside an app.
    let mut units: Vec<(String, &str)> = Vec::new();
    let mut failed: Vec<&str> = Vec::new();
    for entry in entries {
        let Some(m) = &entry.materialized else {
            continue;
        };
        let path = entry.row.display_path.as_str();
        // A diff that could not be computed has no patch: reporting "no matches" for
        // it would present an incomplete search as a definitive one.
        if m.error.is_some() {
            failed.push(path);
            continue;
        }
        for (name, file_diff) in m.parts.files.iter().flatten() {
            units.push((
                format!("{path}/{}", name.trim_start_matches('/')),
                &file_diff.patch,
            ));
        }
        if !m.parts.patch.is_empty() {
            units.push((path.to_string(), &m.parts.patch));
        }
    }
    let mut warning = String::new();
    if !failed.is_empty() {
        let one = failed.len() == 1;
        warning = format!(
            "\nWarning: {} could not be computed and {} NOT searched (matches may be missing): {}. Retry, or read the item{} directly for the error.",
            if one { "this diff" } else { "these diffs" },
            if one { "was" } else { "were" },
            failed.join(", "),
            if one { "" } else { "s" }
        );
    }
    if let Some(glob) = file_glob {
        units.retain(|(subject, _)| matches_glob(subject, glob));
    }
    // ponytail: byte order, where the session sorts with the locale's collation; the
    // two only differ in which diff's matches come first.
    units.sort_by(|a, b| a.0.cmp(&b.0));

    let needle = query.to_lowercase();
    // `(unit, line index)` of every line to print: the matches and their context.
    let mut rows: Vec<(usize, usize)> = Vec::new();
    let mut unit_lines: Vec<Vec<&str>> = Vec::new();
    let (mut total, mut rendered, mut subjects) = (0, 0, 0);
    for (unit, (_, patch)) in units.iter().enumerate() {
        let lines: Vec<&str> = patch.split('\n').collect();
        let mut unit_matched = false;
        for i in changed_line_indices(patch) {
            if !lines[i].to_lowercase().contains(&needle) {
                continue;
            }
            total += 1;
            unit_matched = true;
            if rendered >= max_matches {
                continue;
            }
            rendered += 1;
            let last = (i + SEARCH_CONTEXT_LINES).min(lines.len() - 1);
            rows.extend((i.saturating_sub(SEARCH_CONTEXT_LINES)..=last).map(|j| (unit, j)));
        }
        subjects += usize::from(unit_matched);
        unit_lines.push(lines);
    }
    if total == 0 {
        return format!(
            "No changed lines match. Try a broader or differently-spelled term{}.{warning}",
            if file_glob.is_some() {
                ", or drop the file_glob"
            } else {
                ""
            }
        );
    }

    let plural = |n: usize| if n == 1 { "" } else { "s" };
    let header = format!(
        "{total} changed line{} match in {subjects} diff{}{}",
        plural(total),
        plural(subjects),
        if total > rendered {
            format!(
                " (showing the first {max_matches}; narrow with file_glob or a more specific query)"
            )
        } else {
            String::new()
        }
    );
    let mut spent = header.chars().count();
    let mut out = vec![header];
    let mut current_unit = None;
    let mut seen = std::collections::HashSet::new();
    for row @ (unit, line) in rows {
        if !seen.insert(row) {
            continue;
        }
        let text = unit_lines[unit][line];
        let text = if text.chars().count() > SEARCH_MAX_LINE_CHARS {
            let head: String = text.chars().take(SEARCH_MAX_LINE_CHARS).collect();
            format!("{head}… [line truncated]")
        } else {
            text.to_string()
        };
        let subject = if current_unit == Some(unit) {
            String::new()
        } else {
            format!("{}\n", units[unit].0)
        };
        let row = format!("{subject}  {}: {text}", line + 1);
        let cost = row.chars().count() + 1;
        if spent + cost > SEARCH_TOTAL_CHAR_BUDGET {
            out.push(
                "… output truncated at the context budget — narrow with file_glob or a more specific query."
                    .to_string(),
            );
            break;
        }
        current_unit = Some(unit);
        spent += cost;
        out.push(row);
    }
    out.join("\n") + warning.as_str()
}

/// Why `against="parent_workspace"` fails. A tool reaches the API with a credential
/// that is only valid in its own workspace, and the comparison reads the parent's
/// items under the caller's rights there.
async fn parent_comparison_unavailable(ctx: &Ctx<'_>) -> String {
    let unavailable = "Comparing a fork with its parent workspace is not available on this server. Use diff without 'against' to compare drafts vs deployed versions, or the fork's compare page in Windmill for the comparison with the parent.";
    let Ok(user) = ctx.get("/workspaces/users").await else {
        return unavailable.to_string();
    };
    let workspace = user
        .get("workspaces")
        .and_then(Value::as_array)
        .and_then(|workspaces| {
            workspaces
                .iter()
                .find(|w| w.get("id").and_then(Value::as_str) == Some(ctx.w_id))
        });
    match workspace {
        Some(workspace) if non_null(workspace.get("parent_workspace_id")).is_none() => format!(
            "Workspace \"{}\" is not a fork — it has no parent workspace to compare against. Use diff without 'against' to compare drafts vs deployed versions.",
            ctx.w_id
        ),
        _ => unavailable.to_string(),
    }
}

/// Items `show_deploy_review` draws at most, each one diff.
const MAX_REVIEW_ITEMS: usize = 10;

/// Drafts with their diffs, each with a Deploy button the user presses: the card calls
/// `deploy_workspace_item` itself, so that click is the confirmation.
pub(super) async fn show_deploy_review(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let requested: Vec<(String, Option<String>, String)> =
        match args.get("items").and_then(Value::as_array) {
            Some(items) if !items.is_empty() => items
                .iter()
                .filter_map(|item| {
                    Some((
                        opt_str_arg(item, "type")?.to_string(),
                        opt_str_arg(item, "trigger_kind").map(str::to_string),
                        opt_str_arg(item, "path")?.to_string(),
                    ))
                })
                .collect(),
            _ => list_draft_rows(ctx)
                .await?
                .0
                .into_iter()
                .filter_map(|row| {
                    let (item_type, trigger_kind) = row.addressing?;
                    Some((
                        item_type.to_string(),
                        trigger_kind.map(str::to_string),
                        row.display_path,
                    ))
                })
                .collect(),
        };
    if requested.is_empty() {
        return Ok("No drafts to review: nothing differs from the deployed state.".to_string());
    }
    let shown = &requested[..requested.len().min(MAX_REVIEW_ITEMS)];
    let mut items = Vec::new();
    for (item_type, trigger_kind, path) in shown {
        let diff = diff_item(
            ctx,
            &json!({ "type": item_type, "path": path, "trigger_kind": trigger_kind }),
        )
        .await;
        let page = draft_page(item_type, path, &Value::Null)
            .or_else(|| deployed_page(item_type, path, &Value::Null));
        items.push(json!({
            "type": item_type,
            "trigger_kind": trigger_kind,
            "path": path,
            "patch": diff.as_ref().ok(),
            "error": diff.as_ref().err().map(|e| match api_message(e) { "" => e.as_str(), m => m }),
            "url": page.map(|page| ctx.page_url(&page)),
        }));
    }
    ctx.set_view(
        "windmill/review",
        json!({ "workspace": ctx.w_id, "items": items }),
    );
    let mut text = format!(
        "Showing {} draft(s) to the user with their diffs. Nothing was deployed: the user deploys each from the card, and you are told when they do.",
        items.len()
    );
    if requested.len() > shown.len() {
        text.push_str(&format!(
            " Only the first {MAX_REVIEW_ITEMS} of {} are shown.",
            requested.len()
        ));
    }
    let unreadable: Vec<&str> = items
        .iter()
        .filter(|item| !item["error"].is_null())
        .filter_map(|item| item["path"].as_str())
        .collect();
    if !unreadable.is_empty() {
        text.push_str(&format!(
            " These could not be diffed and offer no Deploy: {}.",
            unreadable.join(", ")
        ));
    }
    Ok(text)
}

async fn diff(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    if args.get("search").is_some_and(|s| !s.is_null()) {
        return diff_search(ctx, args).await;
    }
    if opt_str_arg(args, "against") == Some("parent_workspace") {
        return Err(parent_comparison_unavailable(ctx).await);
    }
    if args.get("path").is_some_and(|p| !p.is_null()) {
        diff_item(ctx, args).await
    } else {
        diff_index(ctx, args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_matches_the_session_emitter() {
        // Expected texts are what the session's `yaml` library prints for these values.
        let scalars = json!({ "s": [
            "true", "~", "12", "1e3", "yes", "-", "- a", "a: b", "a:b", "#x", "a #b", "x ",
            "\"q\"", "it's", "say \"hi\"", "@at", "a,b", "[x]", "?q", "---", " lead",
            "0 0 12 * * *", "$encrypted:abc==", "", null, 1.0, -2.5e-3, [], {}, [[1, 2], ["a"]]
        ], "gone": null });
        assert_eq!(
            to_stable_yaml(Some(&scalars)),
            "s:\n  - \"true\"\n  - \"~\"\n  - \"12\"\n  - \"1e3\"\n  - yes\n  - \"-\"\n  - \"- a\"\n  - \"a: b\"\n  - a:b\n  - \"#x\"\n  - \"a #b\"\n  - \"x \"\n  - '\"q\"'\n  - it's\n  - say \"hi\"\n  - \"@at\"\n  - a,b\n  - \"[x]\"\n  - ?q\n  - ---\n  - \" lead\"\n  - 0 0 12 * * *\n  - $encrypted:abc==\n  - \"\"\n  - null\n  - 1\n  - -0.0025\n  - []\n  - {}\n  - - - 1\n      - 2\n    - - a\n"
        );

        let blocks = json!({
            "content": "export async function main() {\n  return 1\n}\n",
            "one": "a\nb", "keep": "a\n\n", "keeps": "a\n\n\nb\n\n\n",
            "lead": "  indented\nnext", "leadnl": "\n\nx\n", "blank": "a\n\n\nb",
            "wsend": "a\n  ", "onlynl": "\n", "nlnl": "\n\n", "crlf": "a\r\nb\r\n", "ctrl": "a\u{7}b",
            "longcrlf": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\nbbbbbbbbbbbbbbbbbbbbbbbb\r\n\r\n cccc \r\nd",
            "z": "  \nfoo", "x": "a\n\n \nb\n",
            "seq": [{ "a": "x\ny\n", "b": [{ "c": "p\nq" }] }, "m\nn"],
        });
        assert_eq!(
            to_stable_yaml(Some(&blocks)),
            "blank: |-\n  a\n\n\n  b\ncontent: |\n  export async function main() {\n    return 1\n  }\ncrlf: \"a\\r\\nb\\r\\n\"\nctrl: \"a\\ab\"\nkeep: |+\n  a\n\nkeeps: |+\n  a\n\n\n  b\n\n\nlead: |2-\n    indented\n  next\nleadnl: |\n  \n\n  x\nlongcrlf: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\\r\n\n  bbbbbbbbbbbbbbbbbbbbbbbb\\r\n\n  \\r\n\n  \\ cccc \\r\n\n  d\"\nnlnl: |+\n  \n\none: |-\n  a\n  b\nonlynl: |+\n  \nseq:\n  - a: |\n      x\n      y\n    b:\n      - c: |-\n          p\n          q\n  - |-\n    m\n    n\nwsend: \"a\\n  \"\nx: |\n  a\n\n   \n  b\nz: |2-\n    \n  foo\n"
        );

        let keys = json!({
            "": 1, "12": 1, "true": 1, "a b": 1, "a:b": 1, "a: b": 1, "#k": 1, "-": 1,
            "x\ny": 1, "say \"hi\"": 1, "$schema": 1, "/App.tsx": 1, "---": 1, "~": 1,
        });
        assert_eq!(
            to_stable_yaml(Some(&keys)),
            "\"\": 1\n\"#k\": 1\n$schema: 1\n\"-\": 1\n\"---\": 1\n/App.tsx: 1\n\"12\": 1\na b: 1\n\"a: b\": 1\na:b: 1\nsay \"hi\": 1\n\"true\": 1\n\"x\\ny\": 1\n\"~\": 1\n"
        );
        assert_eq!(to_stable_yaml(None), "");
        assert_eq!(to_stable_yaml(Some(&json!({}))), "{}\n");
    }

    #[test]
    fn yaml_patch_ignores_key_order_and_null_fields() {
        let labels = ("deployed", "draft");
        let patch = |a: Value, b: Value| yaml_patch(Some(&a), Some(&b), labels);
        assert_eq!(patch(json!({"a": 1, "b": 2}), json!({"b": 2, "a": 1})), "");
        assert_eq!(patch(json!({"a": 1, "tag": null}), json!({"a": 1})), "");
        assert!(patch(json!({"a": 1, "tag": "prod"}), json!({"a": 1})).contains("-tag: prod"));

        let code = patch(
            json!({"content": "line1\nline2\nline3\n", "language": "bun"}),
            json!({"content": "line1\nchanged\nline3\n", "language": "bun"}),
        );
        assert!(code.contains("-  line2") && code.contains("+  changed"));
        assert!(!code.contains("-  line1"));

        let new = yaml_patch(None, Some(&json!({"summary": "new item"})), labels);
        assert!(new.contains("+summary: new item"));
    }

    #[test]
    fn window_patch_resumes_at_the_first_undelivered_line() {
        let patch = (0..20)
            .map(|i| format!("line-{i:02}"))
            .collect::<Vec<_>>()
            .join("\n");
        // The cap fits five complete lines.
        assert_eq!(
            window_patch(&patch, 0, 20, 40),
            "line-00\nline-01\nline-02\nline-03\nline-04\n… window truncated at 40 chars.\n(lines 1-5 of 20; call diff again with offset=5 for the rest)"
        );
        assert!(window_patch(&patch, 5, 20, 40).starts_with("line-05\n"));
        assert!(window_patch("a\nb", 10, 5, 100).contains("no lines at offset 10"));

        let long = format!("{}\nafter", "x".repeat(100));
        assert!(window_patch(&long, 0, 10, 40).contains("offset=1"));
        assert_eq!(window_patch(&long, 1, 10, 40), "after\n(lines 2-2 of 2)");
    }

    #[test]
    fn changed_lines_exclude_the_file_labels_only() {
        let patch = text_patch(
            Some("a\n--counter\nb\n"),
            Some("a\n++counter\nb\n"),
            ("deployed", "draft"),
        );
        let lines: Vec<&str> = patch.split('\n').collect();
        let changed: Vec<&str> = changed_line_indices(&patch)
            .into_iter()
            .map(|i| lines[i])
            .collect();
        assert_eq!(changed, ["---counter", "+++counter"]);
    }

    #[test]
    fn variable_values_are_masked_on_both_sides() {
        let mut before = Some(json!({"value": "old-plaintext", "is_secret": false}));
        let mut after = Some(json!({"value": "new-plaintext", "is_secret": false}));
        assert!(!mask_variable_sides(&mut before, &mut after));
        let patch = yaml_patch(before.as_ref(), after.as_ref(), ("deployed", "draft"));
        assert!(!patch.contains("plaintext") && patch.contains("(changed)"));

        // Equal masked sides prove nothing about a secret.
        let mut before = Some(json!({"value": "<secret>", "is_secret": true}));
        let mut after = before.clone();
        assert!(mask_variable_sides(&mut before, &mut after));
        assert_eq!(before, after);
    }

    #[test]
    fn multi_file_apps_split_into_file_patches_and_a_config_patch() {
        let before = json!({
            "summary": "Dash",
            "files": { "src/App.tsx": "a\nb\nc\n", "old.js": "legacy\n", "gone.css": "", "kept.ts": "same\n" },
        });
        let after = json!({
            "summary": "Dash v2",
            "files": { "src/App.tsx": "a\nB\nc\n", "new.ts": "fresh\n", "added.ts": "", "kept.ts": "same\n" },
        });
        let parts = diff_parts(Some(&before), Some(&after), ("deployed", "draft"));
        let files = parts.files.unwrap();
        let statuses: Vec<(&str, &str, usize)> = files
            .iter()
            .map(|(name, f)| (name.as_str(), f.status, f.line_count))
            .collect();
        assert_eq!(
            statuses,
            [
                ("added.ts", "added", 0),
                ("gone.css", "deleted", 0),
                ("new.ts", "added", 6),
                ("old.js", "deleted", 6),
                ("src/App.tsx", "modified", 9),
            ]
        );
        assert!(parts.patch.contains("+summary: Dash v2") && !parts.patch.contains("App.tsx"));
        assert!(parts.has_changes);

        let same = diff_parts(Some(&before), Some(&before), ("deployed", "draft"));
        assert!(!same.has_changes && same.files.unwrap().is_empty());
    }

    #[test]
    fn globs_match_file_names_or_full_paths() {
        assert!(matches_glob("f/dash/main/src/App.tsx", "*.tsx"));
        assert!(matches_glob("f/dash/main/src/App.tsx", "f/dash/**"));
        assert!(!matches_glob("f/dash/main/src/App.tsx", "f/dash/*.tsx"));
        assert!(!matches_glob("f/dash/main", "*.tsx"));
    }
}
