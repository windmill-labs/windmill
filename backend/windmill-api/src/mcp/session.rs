//! The AI session's toolset (`crate::ai_session_tools`) as MCP serves it.

use serde_json::{Map, Value};
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

pub(super) fn session_tools(session_access: SessionAccess) -> Vec<Tool> {
    tools::session_tools(access(session_access))
        .map(|tool| {
            Tool::new(tool.name, tool.description, tool.parameters.clone())
                .with_annotations(
                    ToolAnnotations::new()
                        .read_only(tool.read_only)
                        .destructive(tool.destructive),
                )
                .with_meta(result_size_meta())
        })
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
    Ok(if outcome.is_error {
        CallToolResult::error(content)
    } else {
        CallToolResult::success(content)
    })
}
