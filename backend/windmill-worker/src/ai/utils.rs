use crate::ai::mcp_client::{McpClient, McpResource, McpToolSource};
use anyhow::Context;
use serde_json::value::RawValue;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
use windmill_common::{
    ai_providers::AIProvider,
    db::DB,
    error::Error,
    flow_conversations::{add_message_to_conversation_tx, MessageType},
    flow_status::AgentAction,
    flows::{FlowValue, Step},
    jobs::JobKind,
    scripts::{ScriptHash, ScriptLang},
    worker::to_raw_value,
};
use windmill_queue::{flow_status::get_step_of_flow_status, MiniPulledJob};

use crate::{ai::types::*, parse_sig_of_lang};

pub fn parse_raw_script_schema(
    content: &str,
    language: &ScriptLang,
) -> Result<Box<RawValue>, Error> {
    let main_arg_signature = parse_sig_of_lang(content, Some(&language), None)?.unwrap(); // safe to unwrap as langauge is some

    let schema = OpenAPISchema {
        r#type: Some(SchemaType::default()),
        properties: Some(
            main_arg_signature
                .args
                .iter()
                .map(|arg| {
                    let name = arg.name.clone();
                    let typ = OpenAPISchema::from_typ(&arg.typ);
                    (name, Box::new(typ))
                })
                .collect(),
        ),
        required: Some(
            main_arg_signature
                .args
                .iter()
                .map(|arg| arg.name.clone())
                .collect(),
        ),
        ..Default::default()
    };

    Ok(to_raw_value(&schema))
}

pub struct FlowJobRunnableIdAndRawFlow {
    pub runnable_id: Option<ScriptHash>,
    pub raw_flow: Option<sqlx::types::Json<Box<RawValue>>>,
    pub kind: JobKind,
}

pub async fn get_flow_job_runnable_and_raw_flow(
    db: &DB,
    job_id: &uuid::Uuid,
) -> windmill_common::error::Result<FlowJobRunnableIdAndRawFlow> {
    let job = sqlx::query_as!(
        FlowJobRunnableIdAndRawFlow,
        "SELECT runnable_id as \"runnable_id: ScriptHash\", raw_flow as \"raw_flow: _\", kind as \"kind: _\" FROM v2_job WHERE id = $1",
        job_id
    )
    .fetch_one(db)
    .await?;
    Ok(job)
}

#[derive(Debug, Clone, Default)]
pub struct FlowChatSettings {
    pub memory_id: Option<Uuid>,
    pub chat_input_enabled: bool,
}

/// Get chat settings (memory_id and chat_input_enabled) from root flow's flow_status
pub async fn get_flow_chat_settings(db: &DB, job: &MiniPulledJob) -> FlowChatSettings {
    let root_job_id = job
        .root_job
        .or(job.flow_innermost_root_job)
        .or(job.parent_job);

    let Some(root_job_id) = root_job_id else {
        return FlowChatSettings::default();
    };

    match sqlx::query!(
        "SELECT
            (flow_status->>'memory_id')::uuid as memory_id,
            (flow_status->>'chat_input_enabled')::boolean as chat_input_enabled
         FROM v2_job_status
         WHERE id = $1",
        root_job_id
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(row)) => FlowChatSettings {
            memory_id: row.memory_id,
            chat_input_enabled: row.chat_input_enabled.unwrap_or(false),
        },
        Ok(None) => FlowChatSettings::default(),
        Err(e) => {
            tracing::warn!(
                "Failed to get chat settings from flow status for job {}: {}",
                job.id,
                e
            );
            FlowChatSettings::default()
        }
    }
}

// Add message to conversation
pub async fn add_message_to_conversation(
    db: &DB,
    conversation_id: &Uuid,
    job_id: Option<Uuid>,
    message_content: &str,
    message_type: MessageType,
    step_name: &Option<String>,
    success: bool,
) -> Result<(), Error> {
    let mut tx = db.begin().await?;
    add_message_to_conversation_tx(
        &mut tx,
        *conversation_id,
        job_id,
        &message_content,
        message_type,
        step_name.as_deref(),
        success,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Find a unique tool name for structured output tool to avoid collisions with user-provided tools
pub fn find_unique_tool_name(base_name: &str, existing_tools: Option<&[ToolDef]>) -> String {
    let Some(tools) = existing_tools else {
        return base_name.to_string();
    };

    if !tools.iter().any(|t| t.function.name == base_name) {
        return base_name.to_string();
    }

    for i in 1..100 {
        let candidate = format!("{}_{}", base_name, i);
        if !tools.iter().any(|t| t.function.name == candidate) {
            return candidate;
        }
    }

    // Fallback with process id if somehow we can't find a unique name
    format!("{}_{}_fallback", base_name, std::process::id())
}

pub async fn update_flow_status_module_with_actions(
    db: &DB,
    parent_job: &Uuid,
    actions: &[AgentAction],
) -> Result<(), Error> {
    let step = get_step_of_flow_status(db, parent_job.to_owned()).await?;
    match step {
        Step::Step { idx: step, .. } => {
            sqlx::query!(
                r#"
                UPDATE v2_job_status SET
                    flow_status = jsonb_set(
                        flow_status,
                        array['modules', $3::TEXT, 'agent_actions'],
                        $2
                    )
                WHERE id = $1
                "#,
                parent_job,
                sqlx::types::Json(actions) as _,
                step as i32
            )
            .execute(db)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

pub async fn update_flow_status_module_with_actions_success(
    db: &DB,
    parent_job: &Uuid,
    action_success: bool,
) -> Result<(), Error> {
    let step = get_step_of_flow_status(db, parent_job.to_owned()).await?;
    match step {
        Step::Step { idx: step, .. } => {
            // Append the new bool to the existing array, or create a new array if it doesn't exist
            sqlx::query!(
                r#"
                UPDATE v2_job_status SET
                    flow_status = jsonb_set(
                        flow_status,
                        array['modules', $2::TEXT, 'agent_actions_success'],
                        COALESCE(
                            flow_status->'modules'->$2->'agent_actions_success',
                            to_jsonb(ARRAY[]::bool[])
                        ) || to_jsonb(ARRAY[$3::bool])
                    )
                WHERE id = $1
                "#,
                parent_job,
                step as i32,
                action_success
            )
            .execute(db)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

/// Get step name from the flow module (summary if exists, else id)
pub fn get_step_name_from_flow(
    flow_value: &FlowValue,
    flow_step_id: Option<&str>,
) -> Option<String> {
    let flow_step_id = flow_step_id?;
    let module = flow_value.modules.iter().find(|m| m.id == flow_step_id)?;
    Some(
        module
            .summary
            .clone()
            .unwrap_or_else(|| format!("AI Agent Step {}", module.id)),
    )
}

/// Check if the provider is Anthropic (either direct or through OpenRouter)
pub fn is_anthropic_provider(provider: &ProviderWithResource) -> bool {
    let provider_is_anthropic = provider.kind.is_anthropic();
    let is_openrouter_anthropic =
        provider.kind == AIProvider::OpenRouter && provider.model.starts_with("anthropic/");
    provider_is_anthropic || is_openrouter_anthropic
}

/// Cleanup MCP clients by gracefully shutting down connections
pub async fn cleanup_mcp_clients(mcp_clients: HashMap<String, Arc<McpClient>>) {
    if mcp_clients.is_empty() {
        return;
    }

    tracing::debug!("Cleaning up {} MCP client(s)", mcp_clients.len());

    for (resource_name, client) in mcp_clients {
        // Try to unwrap the Arc to get the McpClient
        match Arc::try_unwrap(client) {
            Ok(client) => {
                tracing::debug!("Shutting down MCP client for {}", resource_name);
                if let Err(e) = client.shutdown().await {
                    tracing::warn!("Failed to shutdown MCP client for {}: {}", resource_name, e);
                }
            }
            Err(arc) => {
                // Other references still exist (shouldn't happen in normal flow)
                tracing::warn!(
                    "MCP client for {} still has {} references, dropping without graceful shutdown",
                    resource_name,
                    Arc::strong_count(&arc)
                );
            }
        }
    }
}

/// Load tools from MCP servers and return both the clients and tools
/// Returns a map of resource name -> client, and a vector of tools
pub async fn load_mcp_tools(
    db: &DB,
    workspace_id: &str,
    mcp_resource_paths: Vec<String>,
) -> Result<(HashMap<String, Arc<McpClient>>, Vec<Tool>), Error> {
    let mut all_mcp_tools = Vec::new();
    let mut mcp_clients = HashMap::new();

    for resource_path in mcp_resource_paths {
        tracing::debug!("Loading MCP tools from resource: {}", resource_path);

        let path = resource_path.trim_start_matches("$res:");
        let mcp_resource = {
            // Fetch the resource from database
            let resource= sqlx::query_scalar!(
                "SELECT value as \"value: sqlx::types::Json<Box<RawValue>>\" FROM resource WHERE path = $1 AND workspace_id = $2",
                &path,
                &workspace_id
            )
            .fetch_optional(db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Could not find the resource {}, update the resource path in the workspace settings", resource_path)))?
            .ok_or_else(|| Error::BadRequest(format!("Empty resource value for {}", resource_path)))?;

            serde_json::from_str::<McpResource>(resource.0.get())
                .context("Failed to parse MCP resource")?
        };

        let resource_name = mcp_resource.name.clone();

        // Create new MCP client for this execution
        tracing::debug!("Creating fresh MCP client for {}", resource_name);
        let client = McpClient::from_resource(mcp_resource, db, workspace_id, &path)
            .await
            .context("Failed to create MCP client")?;

        let mcp_client = Arc::new(client);

        // Get all available tools
        let available_tools = mcp_client.available_tools();

        all_mcp_tools.extend(available_tools.iter().cloned());

        // Store client for later use and cleanup
        mcp_clients.insert(resource_name, mcp_client);
    }

    Ok((mcp_clients, all_mcp_tools))
}

/// Execute an MCP tool by routing the call to the appropriate MCP client
pub async fn execute_mcp_tool(
    mcp_clients: &HashMap<String, Arc<McpClient>>,
    mcp_source: &McpToolSource,
    arguments_str: &str,
) -> Result<serde_json::Value, Error> {
    // Get the MCP client from the provided map
    let mcp_client = mcp_clients.get(&mcp_source.name).ok_or_else(|| {
        Error::internal_err(format!(
            "MCP client not found for resource: {}",
            mcp_source.name
        ))
    })?;

    // Call the MCP tool
    let result = mcp_client
        .call_tool(&mcp_source.tool_name, arguments_str)
        .await
        .context("MCP tool call failed")?;

    Ok(result)
}
