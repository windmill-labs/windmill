//! The AI session's toolset (`crate::ai_session_tools`) as MCP serves it.

use serde_json::{json, Map, Value};
use windmill_mcp::server::{
    BackendResult, CallToolResult, ContentBlock, ErrorData, MetaObject, SessionAccess, Tool,
    ToolAnnotations,
};

use crate::ai_session_tools::{self as tools, CallRefused};

pub(super) use tools::{
    generic_session_instructions, session_instructions, workspace_context, Ctx,
};

pub(super) fn access(access: SessionAccess) -> tools::SessionAccess {
    match access {
        SessionAccess::ReadOnly => tools::SessionAccess::ReadOnly,
        SessionAccess::ReadAndRun => tools::SessionAccess::ReadAndRun,
        SessionAccess::Full => tools::SessionAccess::Full,
    }
}

/// Claude clients keep an MCP result in context only up to a token budget well under the
/// authoring references `get_instructions` returns, unless the tool raises it. Above the
/// runner's own cap on a result, so that cap is the one that applies.
fn result_size_meta() -> MetaObject {
    let mut meta = Map::new();
    meta.insert("anthropic/maxResultSizeChars".to_string(), 100_000.into());
    MetaObject(meta)
}

/// The destructive tools a card may call, each from a button the user presses. Every
/// other destructive tool is hidden from cards: a card acts as the user with no one
/// confirming.
const CARD_ACTIONS: &[&str] = &["deploy_workspace_item", "cancel_job"];

/// A tool's display name, which Claude shows in its tool list and approval prompts.
fn tool_title(name: &str) -> String {
    let words: Vec<String> = name
        .split('_')
        .map(|word| match word {
            "db" | "sql" | "json" => word.to_uppercase(),
            _ => word.to_string(),
        })
        .collect();
    let title = words.join(" ");
    let mut chars = title.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// The MCP Apps view a client that renders them draws these tools' results in. It draws
/// what the tool put in its result's `_meta` (`Ctx::view`), and acts by calling tools.
/// Hosts cache a view by its URI: bump the version when a change breaks the `_meta` shape
/// the deployed card reads.
pub(super) const CARD_URI: &str = "ui://windmill/card-v1";
const CARD_HTML: &str = include_str!("card.html");

/// The `ui://` views the session's tools render in, as `(uri, html)`.
pub(super) fn session_views() -> &'static [(&'static str, &'static str)] {
    &[(CARD_URI, CARD_HTML)]
}

pub(super) fn session_tools(session_access: SessionAccess) -> Vec<Tool> {
    let access = access(session_access);
    tools::session_tools(access)
        .map(|tool| {
            let mut meta = result_size_meta();
            if tool.destructive && !CARD_ACTIONS.contains(&tool.name) {
                meta.0
                    .insert("ui".into(), json!({ "visibility": ["model"] }));
            }
            let title = tool_title(tool.name);
            Tool::new(tool.name, tool.description, tool.parameters.clone())
                .with_title(title.clone())
                .with_annotations(
                    ToolAnnotations::with_title(title)
                        .read_only(tool.read_only)
                        .destructive(tool.destructive)
                        .open_world(tool.open_world),
                )
                .with_meta(meta)
        })
        .chain(
            tools::view_definitions()
                .into_iter()
                .filter(|(name, ..)| tools::view_allowed(name, access))
                .map(|(name, description, schema)| {
                    let mut meta = result_size_meta();
                    meta.0
                        .insert("ui".into(), json!({ "resourceUri": CARD_URI }));
                    let Value::Object(schema) = schema else {
                        unreachable!("a view tool's schema is an object")
                    };
                    let title = tool_title(name);
                    Tool::new(name, description, schema)
                        .with_title(title.clone())
                        .with_annotations(
                            ToolAnnotations::with_title(title)
                                .read_only(true)
                                .destructive(false)
                                .open_world(false),
                        )
                        .with_meta(meta)
                }),
        )
        .collect()
}

pub(super) async fn call_session_tool(
    ctx: &Ctx<'_>,
    name: &str,
    args: Value,
) -> BackendResult<CallToolResult> {
    let outcome =
        tools::call_session_tool(ctx, name, args)
            .await
            .map_err(|refused| match refused {
                CallRefused::NotAllowed(message) => ErrorData::invalid_request(message, None),
                CallRefused::UnknownTool(message) | CallRefused::NotAvailable(message) => {
                    ErrorData::invalid_params(message, None)
                }
            })?;
    windmill_common::feature_usage::log_feature_usage("mcp_workspace", "tool_call", name);
    let content = vec![ContentBlock::text(outcome.text)];
    let mut result = if outcome.is_error {
        CallToolResult::error(content)
    } else {
        CallToolResult::success(content)
    };
    if let Some((key, view)) = ctx.view.lock().unwrap().take() {
        result.meta = Some(MetaObject(Map::from_iter([(key.to_string(), view)])));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_card_can_call_a_destructive_tool_but_deploy_and_cancel() {
        for tool in session_tools(SessionAccess::Full) {
            let name = tool.name.as_ref();
            let destructive = tool.annotations.as_ref().and_then(|a| a.destructive_hint);
            if destructive == Some(true) {
                let ui = tool.meta.as_ref().and_then(|m| m.0.get("ui"));
                let model_only = ui == Some(&json!({ "visibility": ["model"] }));
                assert_eq!(
                    model_only,
                    !matches!(name, "deploy_workspace_item" | "cancel_job"),
                    "{name}"
                );
            }
        }
    }

    /// The ChatGPT directory wants every hint explicit, and Claude's a title on every tool.
    #[test]
    fn every_session_tool_has_a_title_and_explicit_hints() {
        for tool in session_tools(SessionAccess::Full) {
            let a = tool.annotations.as_ref().expect("annotations");
            assert!(tool.title.is_some() && a.title.is_some(), "{}", tool.name);
            assert!(
                a.read_only_hint.is_some()
                    && a.destructive_hint.is_some()
                    && a.open_world_hint.is_some(),
                "{}",
                tool.name
            );
        }
    }
}
