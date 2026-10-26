//! Tools that show something to the user as a card, for MCP Apps clients. They exist only
//! on the MCP side: the Windmill chat draws its own cards. The model reads and runs through
//! the other tools, which draw nothing, and calls one of these when it means the user to
//! see a result: a host draws a card for every call of a tool that has one.

use serde_json::{json, Value};

use super::{deploy, runs, workspace, Ctx, ToolResult};

pub(super) const TOOLS: &[&str] = &["show_run", "show_runs", "show_deploy_review", "show_item"];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "show_run" => runs::show_run(ctx, args).await,
        "show_runs" => runs::show_runs(ctx, args).await,
        "show_deploy_review" => deploy::show_deploy_review(ctx, args).await,
        "show_item" => workspace::show_item(ctx, args).await,
        _ => unreachable!("{name} is not a view tool"),
    }
}

/// Each tool as `(name, description, input schema)`. All only read: the review card's
/// Deploy button calls `deploy_workspace_item`, which the token's own access gates.
pub(super) fn definitions() -> [(&'static str, &'static str, Value); 4] {
    [
        (
            "show_run",
            "Show a run to the user as a live card in the conversation: status, result, steps and logs, refreshing until it finishes, with a link to it in Windmill and a button to cancel it while it runs. Call it after starting a run the user asked for, or when they ask to see one. Do not call it for runs you start to check your own work: read those with get_run.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "The run (job) id, as run_script, run_flow, test_run_script, test_run_flow or list_runs returned it." }
                },
                "required": ["id"],
                "additionalProperties": false
            }),
        ),
        (
            "show_runs",
            "Show runs to the user as a table in the conversation: status, path, duration, when, and a link to each. Pass ids from list_runs. Call it when the user asks to see runs; to inspect runs yourself, use list_runs and get_run.",
            json!({
                "type": "object",
                "properties": {
                    "ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1,
                        "maxItems": 20,
                        "description": "Run (job) ids from list_runs, in the order to show them."
                    }
                },
                "required": ["ids"],
                "additionalProperties": false
            }),
        ),
        (
            "show_deploy_review",
            "Show the user drafts to review before they are deployed: each draft's diff against the deployed version, with a Deploy button the user presses. Call it when changes are ready and the user should approve them, instead of deploying them yourself. This tool deploys nothing; you are told when the user deploys one. If the user says they see no card, show them the diff and deploy with deploy_workspace_item once they agree.",
            json!({
                "type": "object",
                "properties": {
                    "items": {
                        "type": "array",
                        "maxItems": 10,
                        "description": "The drafts to review. Omit for all of the user's drafts (the first 10).",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["script", "flow", "schedule", "trigger", "resource", "variable", "app"] },
                                "path": { "type": "string" },
                                "trigger_kind": { "type": "string", "description": "Required when type is trigger." }
                            },
                            "required": ["type", "path"],
                            "additionalProperties": false
                        }
                    }
                },
                "additionalProperties": false
            }),
        ),
        (
            "show_item",
            "Show the user a workspace item as a card in the conversation: a script, flow, schedule, trigger, resource or app, with its key settings and a link to it in Windmill (its editor, for a draft). Call it when the user asks to see an item; to read one yourself, use read_workspace_item.",
            json!({
                "type": "object",
                "properties": {
                    "type": { "type": "string", "enum": ["script", "flow", "schedule", "trigger", "resource", "app"] },
                    "path": { "type": "string", "description": "Workspace path of the item." },
                    "trigger_kind": { "type": "string", "description": "Required when type is trigger.", "enum": ["http", "websocket", "kafka", "nats", "postgres", "mqtt", "amqp", "sqs", "gcp", "azure", "email"] }
                },
                "required": ["type", "path"],
                "additionalProperties": false
            }),
        ),
    ]
}
