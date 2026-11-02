//! The workspace settings document, in the layout of the `settings.yaml` the CLI syncs.

use serde_json::{json, Map, Value};

use super::chat::Effect;
use super::workspace::api_message;
use super::{array, js_head, js_len, js_str, str_arg, Ctx, Surface, ToolResult};

pub(super) const TOOLS: &[&str] = &["get_workspace_settings", "set_workspace_settings"];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "get_workspace_settings" => get_workspace_settings(ctx, args).await,
        "set_workspace_settings" => set_workspace_settings(ctx, args).await,
        _ => unreachable!("{name} is not a settings tool"),
    }
}

const MAX_READ_CHARS: usize = 20_000;

async fn get_workspace_settings(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let doc = ctx
        .get(&ctx.ws("workspaces/settings_document?secrets=redacted"))
        .await?;
    let keys: Vec<&str> = array(args.get("keys"))
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let mut picked = doc.clone();
    let mut note = String::new();
    if !keys.is_empty() {
        picked = Value::Object(
            keys.iter()
                .filter_map(|k| Some((k.to_string(), doc.get(k)?.clone())))
                .collect::<Map<_, _>>(),
        );
        let unset: Vec<&str> = keys
            .iter()
            .copied()
            .filter(|k| doc.get(k).is_none())
            .collect();
        if !unset.is_empty() {
            note = format!(
                "\n# Not in the document (unset, or not a setting): {}",
                unset.join(", ")
            );
        }
    }
    let mut text = serde_yml::to_string(&picked).map_err(|e| e.to_string())?;
    text.push_str(&note);
    Ok(if js_len(&text) > MAX_READ_CHARS {
        format!(
            "{}\n… truncated at {MAX_READ_CHARS} chars; read the keys you need with `keys`.",
            js_head(&text, MAX_READ_CHARS)
        )
    } else {
        text
    })
}

fn parse_settings_yaml(text: &str) -> Result<Value, String> {
    match serde_yml::from_str::<Value>(text) {
        Ok(doc @ Value::Object(_)) => Ok(doc),
        Ok(_) => Err("The yaml must be a mapping of setting keys to values.".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// The dry run reports a no-op or an invalid document before anything is written: for MCP
/// it stands in for the session's confirmation card, and the chat runs it alone, as a
/// check-only call, before showing that card.
async fn set_workspace_settings(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let route = ctx.ws("workspaces/settings_document");
    let refuse = |label: String, text: String| {
        ctx.facts(json!({ "refused": label }));
        Ok(text)
    };
    let document = match parse_settings_yaml(str_arg(args, "yaml")?) {
        Ok(document) => document,
        Err(e) if ctx.surface == Surface::Chat && !ctx.check_only() => return Err(e),
        Err(e) => {
            return refuse(
                format!("Invalid settings: {e}"),
                format!("Nothing was changed: {e}"),
            )
        }
    };
    // The chat's own check ran before its card, and the user confirmed what it showed.
    if ctx.surface == Surface::Mcp || ctx.check_only() {
        let dry_run = json!({ "document": document, "replace": false, "dry_run": true });
        let dry_run = match ctx.post(&route, dry_run).await {
            Ok(result) => result,
            Err(e) => {
                let e = api_message(&e);
                return refuse(
                    format!("Invalid settings: {e}"),
                    format!("Nothing was changed: {e}"),
                );
            }
        };
        if array(dry_run.get("changes")).is_empty() {
            let warnings: Vec<String> = array(dry_run.get("warnings")).iter().map(js_str).collect();
            let warnings = if warnings.is_empty() {
                String::new()
            } else {
                format!(" Warnings: {}", warnings.join(" "))
            };
            return refuse(
                "No settings change".to_string(),
                format!("The settings already hold these values: nothing to change.{warnings}"),
            );
        }
        if ctx.check_only() {
            ctx.facts(json!({ "changes": dry_run["changes"] }));
            return Ok(String::new());
        }
    }
    let applied = match ctx
        .post(&route, json!({ "document": document, "replace": false }))
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let error = api_message(&e);
            let mut failed = Map::new();
            failed.insert(
                "content".into(),
                json!("Failed to change workspace settings"),
            );
            failed.insert("error".into(), json!(error));
            ctx.effect(Effect::Status(failed));
            return Ok(json!({ "success": false, "error": error }).to_string());
        }
    };
    ctx.facts(json!({ "changes": applied["changes"] }));
    let keys: Vec<String> = array(applied.get("changes"))
        .iter()
        .map(|c| js_str(c.get("key").unwrap_or(&Value::Null)))
        .collect();
    let mut out = Map::new();
    out.insert("success".into(), json!(true));
    out.insert(
        "message".into(),
        json!(format!(
            "{} now applied: {}.",
            match ctx.surface {
                Surface::Chat => "The user approved and these settings are",
                Surface::Mcp => "These settings are",
            },
            keys.join(", ")
        )),
    );
    if let Some(warnings) = applied
        .get("warnings")
        .filter(|w| !array(Some(w)).is_empty())
    {
        out.insert("warnings".into(), warnings.clone());
    }
    Ok(Value::Object(out).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_yaml_must_be_a_mapping() {
        assert_eq!(
            parse_settings_yaml("webhook: null\ncolor: red").unwrap(),
            json!({ "webhook": null, "color": "red" })
        );
        for text in ["- a\n- b", "just text", "42"] {
            assert_eq!(
                parse_settings_yaml(text).unwrap_err(),
                "The yaml must be a mapping of setting keys to values.",
                "{text}"
            );
        }
        assert!(parse_settings_yaml("a: [").is_err());
    }
}
