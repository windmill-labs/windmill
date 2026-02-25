pub use crate::ai::types::McpToolSource;
use crate::ai::types::ToolDef;
use anyhow::Context;
use serde_json::value::RawValue;
use sqlx::types::Json;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use uuid::Uuid;
use windmill_common::flows::FlowModuleValue;
use windmill_common::{
    ai_providers::AIProvider,
    db::DB,
    error::Error,
    flow_conversations::{add_message_to_conversation_tx, MessageType},
    flow_status::AgentAction,
    flows::{InputTransform, Step},
    jobs::JobKind,
    scripts::{ScriptHash, ScriptLang},
    worker::to_raw_value,
};
#[cfg(feature = "mcp")]
use windmill_mcp::{McpClient, McpResource, McpTool};
use windmill_queue::{flow_status::get_step_of_flow_status, MiniPulledJob};

use crate::{ai::types::*, parse_sig_of_lang};

pub fn parse_raw_script_schema(
    content: &str,
    language: &ScriptLang,
) -> Result<Box<RawValue>, Error> {
    let main_arg_signature = parse_sig_of_lang(content, Some(&language), None)?
        .ok_or_else(|| Error::BadConfig(format!(
            "Cannot parse signature for language {:?}. The language parser may not be enabled in this build.",
            language
        )))?;

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

pub fn is_completed_input_transform(transform: &InputTransform) -> bool {
    match transform {
        InputTransform::Static { value } => {
            let val = value.get().trim();
            !val.is_empty() && val != "null"
        }
        InputTransform::Javascript { expr } => !expr.trim().is_empty(),
        InputTransform::Ai => false,
    }
}

/// Filters out properties from a JSON schema that have completed input transforms.
/// This allows AI agents to only see and fill parameters that don't have user-configured values.
pub fn filter_schema_by_input_transforms(
    schema: Box<RawValue>,
    input_transforms: &HashMap<String, InputTransform>,
) -> Result<Box<RawValue>, Error> {
    // Parse the schema JSON
    let mut schema_value: serde_json::Value = serde_json::from_str(schema.get())
        .context("Failed to parse schema JSON")
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    // Collect keys to remove (parameters with completed input transforms)
    let keys_to_remove: HashSet<String> = input_transforms
        .iter()
        .filter_map(|(key, transform)| {
            let is_completed = is_completed_input_transform(transform);
            if is_completed {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect();

    if !keys_to_remove.is_empty() {
        // Remove completed parameters from properties
        if let Some(properties) = schema_value
            .get_mut("properties")
            .and_then(|p| p.as_object_mut())
        {
            for key in &keys_to_remove {
                properties.remove(key);
            }
        }

        // Also remove from required array
        if let Some(required) = schema_value
            .get_mut("required")
            .and_then(|r| r.as_array_mut())
        {
            required.retain(|item| {
                if let Some(key) = item.as_str() {
                    !keys_to_remove.contains(key)
                } else {
                    true
                }
            });
        }
    }

    // Convert back to RawValue
    Ok(to_raw_value(&schema_value))
}

#[derive(Clone)]
pub struct FlowJobRunnableIdAndRawFlow {
    pub runnable_id: Option<ScriptHash>,
    pub raw_flow: Option<sqlx::types::Json<Box<RawValue>>>,
    pub kind: JobKind,
    pub parent_job: Option<Uuid>,
    pub flow_step_id: Option<String>,
}

pub async fn get_flow_job_runnable_and_raw_flow(
    db: &DB,
    job_id: &uuid::Uuid,
) -> windmill_common::error::Result<FlowJobRunnableIdAndRawFlow> {
    let job = sqlx::query_as!(
        FlowJobRunnableIdAndRawFlow,
        "SELECT runnable_id as \"runnable_id: ScriptHash\", raw_flow as \"raw_flow: _\", kind as \"kind: _\", parent_job, flow_step_id FROM v2_job WHERE id = $1",
        job_id
    )
    .fetch_one(db)
    .await?;
    Ok(job)
}

#[derive(Debug, Clone, Default)]
pub struct FlowContext {
    pub flow_inputs: Option<HashMap<String, Box<RawValue>>>,
    pub flow_status: Option<windmill_common::flow_status::FlowStatus>,
}

/// Get flow context (chat settings + args + flow_status) from root flow's job data
pub async fn get_flow_context(db: &DB, job: &MiniPulledJob) -> FlowContext {
    let root_job_id = job
        .root_job
        .or(job.flow_innermost_root_job)
        .or(job.parent_job);

    let Some(root_job_id) = root_job_id else {
        return FlowContext::default();
    };

    match sqlx::query!(
        r#"
        SELECT
            j.args as "args: Json<HashMap<String, Box<RawValue>>>",
            js.flow_status as "flow_status: Json<windmill_common::flow_status::FlowStatus>"
        FROM v2_job_status js
        INNER JOIN v2_job j ON j.id = js.id
        WHERE js.id = $1
        "#,
        root_job_id
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(row)) => FlowContext {
            flow_inputs: row.args.map(|j| j.0),
            flow_status: row.flow_status.map(|j| j.0),
        },
        Ok(None) => {
            tracing::warn!(
                "No flow context found for root job {} (agent job {}), returning default",
                root_job_id,
                job.id
            );
            FlowContext::default()
        }
        Err(e) => {
            tracing::error!("Failed to get flow context for job {}: {}", job.id, e);
            FlowContext::default()
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
    summary: Option<&str>,
    flow_step_id: Option<&str>,
) -> Option<String> {
    let flow_step_id = flow_step_id?;
    Some(
        summary
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("AI Agent Step {}", flow_step_id)),
    )
}

/// AWS Bedrock do not handle structured output query param, so we use a tool for structured output. Same for every Claude models.
pub fn should_use_structured_output_tool(provider: &AIProvider, model: &str) -> bool {
    model.contains("claude") || provider == &AIProvider::AWSBedrock
}

/// Cleanup MCP clients by gracefully shutting down connections
#[cfg(feature = "mcp")]
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

#[cfg(feature = "mcp")]
fn sanitize_tool_name_part(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Convert raw MCP tools to Windmill Tool format with source tracking
#[cfg(feature = "mcp")]
fn convert_mcp_tools_to_windmill_tools(
    mcp_tools: &[McpTool],
    resource_name: &str,
    resource_path: &str,
) -> Result<Vec<Tool>, Error> {
    mcp_tools
        .iter()
        .map(|mcp_tool| {
            let sanitized_resource_name = sanitize_tool_name_part(resource_name);
            let tool_name = format!("mcp_{}_{}", sanitized_resource_name, mcp_tool.name);

            let mut schema_value = serde_json::to_value(&*mcp_tool.input_schema)
                .context("Failed to convert MCP schema to JSON value")?;
            McpClient::fix_array_schemas(&mut schema_value);
            let parameters = to_raw_value(&schema_value);

            // Build the description from title and description
            let description = if let Some(title) = &mcp_tool.title {
                if let Some(desc) = &mcp_tool.description {
                    Some(format!("{}: {}", title, desc))
                } else {
                    Some(title.to_string())
                }
            } else {
                mcp_tool.description.as_ref().map(|d| d.to_string())
            };

            let tool_def_function =
                ToolDefFunction { name: tool_name.clone(), description, parameters };

            let tool_def = ToolDef { r#type: "function".to_string(), function: tool_def_function };

            Ok(Tool {
                def: tool_def,
                module: None,
                mcp_source: Some(McpToolSource {
                    name: resource_name.to_string(),
                    tool_name: mcp_tool.name.to_string(),
                    resource_path: resource_path.to_string(),
                }),
            })
        })
        .collect()
}

/// Configuration for loading tools from an MCP server resource
#[cfg(feature = "mcp")]
#[derive(Debug, Clone)]
pub struct McpResourceConfig {
    pub resource_path: String,
    pub include_tools: Option<Vec<String>>,
    pub exclude_tools: Option<Vec<String>>,
}

/// Apply include/exclude filters to a list of tools
/// Priority: include_tools > exclude_tools > all
/// - If include_tools is Some and non-empty: whitelist approach (keep only listed tools)
/// - Else if exclude_tools is Some and non-empty: blacklist approach (remove listed tools)
/// - Otherwise: no filtering (keep all tools)
#[cfg(feature = "mcp")]
fn apply_tool_filters(
    tools: Vec<Tool>,
    include_tools: &Option<Vec<String>>,
    exclude_tools: &Option<Vec<String>>,
) -> Vec<Tool> {
    // If include_tools is specified and non-empty, use whitelist approach
    if let Some(include_list) = include_tools {
        if !include_list.is_empty() {
            return tools
                .into_iter()
                .filter(|tool| {
                    tool.mcp_source
                        .as_ref()
                        .map(|src| include_list.contains(&src.tool_name))
                        .unwrap_or(false)
                })
                .collect();
        }
    }

    // If exclude_tools is specified and non-empty, use blacklist approach
    if let Some(exclude_list) = exclude_tools {
        if !exclude_list.is_empty() {
            return tools
                .into_iter()
                .filter(|tool| {
                    tool.mcp_source
                        .as_ref()
                        .map(|src| !exclude_list.contains(&src.tool_name))
                        .unwrap_or(true)
                })
                .collect();
        }
    }

    // No filtering - return all tools
    tools
}

/// Check if a token variable is expired and refresh it if needed via API call
#[cfg(feature = "mcp")]
async fn refresh_token_if_expired(
    db: &DB,
    workspace_id: &str,
    token_path: &str,
    auth_token: &str,
) -> Result<(), Error> {
    // Query variable with account join to check expiration
    let token_info = sqlx::query!(
        r#"
        SELECT
            variable.path,
            variable.account as account_id,
            (now() > account.expires_at) as "is_expired: bool"
        FROM variable
        LEFT JOIN account ON variable.account = account.id AND account.workspace_id = $2
        WHERE variable.path = $1 AND variable.workspace_id = $2
        "#,
        token_path,
        workspace_id
    )
    .fetch_optional(db)
    .await?;

    let Some(token_info) = token_info else {
        return Ok(());
    };

    let Some(account_id) = token_info.account_id else {
        return Ok(());
    };

    if !token_info.is_expired.unwrap_or(false) {
        return Ok(());
    }

    tracing::debug!(
        "Token variable {} is expired, triggering refresh",
        token_path
    );

    // Call the API refresh endpoint
    let base_url = windmill_common::BASE_URL.read().await.clone();
    let refresh_url = format!(
        "{}/api/w/{}/oauth/refresh_token/{}",
        base_url, workspace_id, account_id
    );

    #[derive(serde::Serialize)]
    struct RefreshRequest {
        path: String,
    }

    let response = windmill_common::utils::HTTP_CLIENT
        .post(&refresh_url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&RefreshRequest { path: token_path.to_string() })
        .send()
        .await
        .map_err(|e| {
            Error::internal_err(format!("Failed to call token refresh endpoint: {}", e))
        })?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(Error::internal_err(format!(
            "Token refresh failed: {}",
            error_text
        )));
    }
    Ok(())
}

/// Load tools from MCP servers and return both the clients and tools
/// Returns a map of resource name -> client, and a vector of tools
#[cfg(feature = "mcp")]
pub async fn load_mcp_tools(
    db: &DB,
    workspace_id: &str,
    mcp_configs: Vec<McpResourceConfig>,
    auth_token: &str,
) -> Result<(HashMap<String, Arc<McpClient>>, Vec<Tool>), Error> {
    let mut all_mcp_tools = Vec::new();
    let mut mcp_clients = HashMap::new();

    for config in mcp_configs {
        tracing::debug!("Loading MCP tools from resource: {}", config.resource_path);

        let path = config.resource_path.trim_start_matches("$res:");
        let mcp_resource = {
            // Fetch the resource from database
            let resource= sqlx::query_scalar!(
                "SELECT value as \"value: sqlx::types::Json<Box<RawValue>>\" FROM resource WHERE path = $1 AND workspace_id = $2",
                &path,
                &workspace_id
            )
            .fetch_optional(db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Could not find the resource {}, update the resource path in the workspace settings", config.resource_path)))?
            .ok_or_else(|| Error::BadRequest(format!("Empty resource value for {}", config.resource_path)))?;

            serde_json::from_str::<McpResource>(resource.0.get())
                .context("Failed to parse MCP resource")?
        };

        let resource_name = mcp_resource.name.clone();

        // Check if token needs refresh before creating MCP client
        if let Some(ref token_path) = mcp_resource.token {
            let token_var_path = token_path.trim_start_matches("$var:");
            if let Err(e) =
                refresh_token_if_expired(db, workspace_id, token_var_path, auth_token).await
            {
                tracing::warn!(
                    "Failed to refresh token for MCP resource {}: {}. Proceeding with possibly expired token.",
                    resource_name, e
                );
            }
        }

        // Create new MCP client for this execution
        tracing::debug!("Creating fresh MCP client for {}", resource_name);
        let client = McpClient::from_resource(mcp_resource, db, workspace_id)
            .await
            .context("Failed to create MCP client")?;

        // Get raw MCP tools from client
        let raw_mcp_tools = client.available_tools();

        // Convert to Windmill Tool format
        let converted_tools =
            convert_mcp_tools_to_windmill_tools(raw_mcp_tools, &resource_name, &path)?;

        // Apply include/exclude filters
        let filtered_tools = apply_tool_filters(
            converted_tools,
            &config.include_tools,
            &config.exclude_tools,
        );

        tracing::info!(
            "Loaded {} tools from MCP server '{}' (filtered from {} available tools)",
            filtered_tools.len(),
            resource_name,
            raw_mcp_tools.len()
        );

        all_mcp_tools.extend(filtered_tools);

        // Store client for later use and cleanup
        let mcp_client = Arc::new(client);
        mcp_clients.insert(resource_name, mcp_client);
    }

    Ok((mcp_clients, all_mcp_tools))
}

/// Execute an MCP tool by routing the call to the appropriate MCP client
#[cfg(feature = "mcp")]
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

// Stub implementations when mcp feature is not enabled
#[cfg(not(feature = "mcp"))]
pub struct McpResourceConfig {}

/// Stub for cleanup_mcp_clients when mcp is not enabled
#[cfg(not(feature = "mcp"))]
pub async fn cleanup_mcp_clients<T>(_mcp_clients: HashMap<String, Arc<T>>) {
    // No-op when MCP is disabled
}

/// Stub for load_mcp_tools when mcp is not enabled
#[cfg(not(feature = "mcp"))]
pub async fn load_mcp_tools<T>(
    _db: &DB,
    _workspace_id: &str,
    _mcp_configs: Vec<McpResourceConfig>,
    _auth_token: &str,
) -> Result<(HashMap<String, Arc<T>>, Vec<Tool>), Error> {
    Ok((HashMap::new(), Vec::new()))
}

/// Stub for execute_mcp_tool when mcp is not enabled
#[cfg(not(feature = "mcp"))]
pub async fn execute_mcp_tool<T>(
    _mcp_clients: &HashMap<String, Arc<T>>,
    mcp_source: &McpToolSource,
    _arguments_str: &str,
) -> Result<serde_json::Value, Error> {
    Err(Error::internal_err(format!(
        "MCP support is not enabled. Cannot execute MCP tool: {}",
        mcp_source.tool_name
    )))
}

/// Check if any tool's input transforms reference previous_result
pub fn any_tool_needs_previous_result(tools: &[Tool]) -> bool {
    tools.iter().any(|tool| {
        if let Some(module) = &tool.module {
            if let Ok(module_value) = module.get_value() {
                let input_transforms = match module_value {
                    FlowModuleValue::Script { input_transforms, .. } => input_transforms,
                    FlowModuleValue::RawScript { input_transforms, .. } => input_transforms,
                    FlowModuleValue::FlowScript { input_transforms, .. } => input_transforms,
                    FlowModuleValue::AIAgent { input_transforms, .. } => input_transforms,
                    _ => return false,
                };

                return input_transforms.iter().any(|(_, transform)| {
                    if let windmill_common::flows::InputTransform::Javascript { expr } = transform {
                        expr.contains("previous_result")
                    } else {
                        false
                    }
                });
            }
        }
        false
    })
}

/// Extract text content from OpenAIContent, joining parts with space if multiple
pub fn extract_text_content(content: &OpenAIContent) -> String {
    match content {
        OpenAIContent::Text(text) => text.clone(),
        OpenAIContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| {
                if let ContentPart::Text { text } = p {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}

/// Parse a data URL to extract media type and base64 data
/// Format: data:mime_type;base64,data
/// Returns (media_type, data) tuple if successful
pub fn parse_data_url(url: &str) -> Option<(String, String)> {
    if !url.starts_with("data:") {
        return None;
    }
    let rest = url.strip_prefix("data:")?;
    let (header, data) = rest.split_once(",")?;
    let media_type = header.strip_suffix(";base64")?;
    Some((media_type.to_string(), data.to_string()))
}
