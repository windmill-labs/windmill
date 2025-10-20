use crate::ai::types::{ToolDef, ToolDefFunction};
use anyhow::Context;
use serde_json::value::RawValue;
use sqlx::Row;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
use windmill_common::mcp_client::{McpClient, McpResource, McpToolSource};
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

/// Filters out properties from a JSON schema that have completed input transforms.
/// This allows AI agents to only see and fill parameters that don't have user-configured values.
///
/// A transform is considered "completed" if:
/// - Static transform: value is defined and not empty/null
/// - JavaScript transform: expr is defined and not empty
pub fn filter_schema_by_input_transforms(
    schema: Box<RawValue>,
    input_transforms: &HashMap<String, InputTransform>,
) -> Result<Box<RawValue>, Error> {
    // Parse the schema JSON
    let mut schema_value: serde_json::Value = serde_json::from_str(schema.get())
        .context("Failed to parse schema JSON")
        .map_err(|e| Error::ExecutionErr(e.to_string()))?;

    tracing::info!(
        "HERE Filtering schema with {} input transforms defined",
        input_transforms.len()
    );

    // Collect keys to remove (parameters with completed input transforms)
    let keys_to_remove: Vec<String> = input_transforms
        .iter()
        .filter_map(|(key, transform)| {
            let is_completed = match transform {
                InputTransform::Static { value } => {
                    // Completed if value is defined and not null/empty
                    let val_str = value.get().trim();
                    let is_complete = !val_str.is_empty() && val_str != "null";
                    tracing::info!(
                        "HERE Parameter '{}': Static transform, value='{}', completed={}",
                        key,
                        if val_str.len() > 50 {
                            format!("{}...", &val_str[..50])
                        } else {
                            val_str.to_string()
                        },
                        is_complete
                    );
                    is_complete
                }
                InputTransform::Javascript { expr } => {
                    // Completed if expr is defined and not empty
                    let is_complete = !expr.trim().is_empty();
                    let expr_display = if expr.len() > 50 {
                        format!("{}...", &expr[..50])
                    } else {
                        expr.to_string()
                    };
                    tracing::info!(
                        "HERE Parameter '{}': JavaScript transform, expr='{}', completed={}",
                        key,
                        expr_display,
                        is_complete
                    );
                    is_complete
                }
            };
            if is_completed {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect();

    if !keys_to_remove.is_empty() {
        tracing::info!(
            "HERE Removing {} completed parameters from schema: {:?}",
            keys_to_remove.len(),
            keys_to_remove
        );

        let original_property_count = schema_value
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|p| p.len())
            .unwrap_or(0);

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
            let original_required_count = required.len();
            required.retain(|item| {
                if let Some(key) = item.as_str() {
                    !keys_to_remove.contains(&key.to_string())
                } else {
                    true
                }
            });
            tracing::info!(
                "HERE Filtered required array: {} -> {} items",
                original_required_count,
                required.len()
            );
        }

        let final_property_count = schema_value
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|p| p.len())
            .unwrap_or(0);

        tracing::info!(
            "HERE Schema filtering complete: {} -> {} properties remaining for AI to fill",
            original_property_count,
            final_property_count
        );
    } else {
        tracing::info!("No completed input transforms found, schema unchanged");
    }

    // Convert back to RawValue
    Ok(to_raw_value(&schema_value))
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
pub struct FlowContext {
    pub memory_id: Option<Uuid>,
    pub chat_input_enabled: bool,
    pub args: Option<HashMap<String, Box<RawValue>>>,
    pub flow_status: Option<windmill_common::flow_status::FlowStatus>,
}

/// Get flow context (chat settings + args + flow_status) from root flow's job data
/// This extended query provides all necessary context for AI tool input transforms
pub async fn get_flow_context(db: &DB, job: &MiniPulledJob) -> FlowContext {
    let root_job_id = job
        .root_job
        .or(job.flow_innermost_root_job)
        .or(job.parent_job);

    let Some(root_job_id) = root_job_id else {
        tracing::debug!("HERE No root job found for job {}, returning default FlowContext", job.id);
        return FlowContext::default();
    };

    tracing::info!(
        "HERE Fetching flow context for root job {} (from agent job {})",
        root_job_id,
        job.id
    );

    // Use untyped query to handle complex JSON types
    let result = sqlx::query(
        r#"
        SELECT
            (js.flow_status->>'memory_id')::uuid as memory_id,
            (js.flow_status->>'chat_input_enabled')::boolean as chat_input_enabled,
            j.args,
            js.flow_status
        FROM v2_job_status js
        INNER JOIN v2_job j ON j.id = js.id
        WHERE js.id = $1
        "#,
    )
    .bind(root_job_id)
    .fetch_optional(db)
    .await;

    match result {
        Ok(Some(row)) => {
            let memory_id: Option<Uuid> = row.try_get("memory_id").ok();
            let chat_input_enabled: bool = row.try_get("chat_input_enabled").ok().unwrap_or(false);

            let args = row
                .try_get::<sqlx::types::Json<HashMap<String, Box<RawValue>>>, _>("args")
                .ok()
                .map(|j| j.0);

            let flow_status = row
                .try_get::<sqlx::types::Json<windmill_common::flow_status::FlowStatus>, _>("flow_status")
                .ok()
                .map(|j| j.0);

            let args_count = args.as_ref().map(|a| a.len()).unwrap_or(0);
            let has_flow_status = flow_status.is_some();

            tracing::info!(
                "HERE Flow context loaded: memory_id={}, chat_enabled={}, args_count={}, has_flow_status={}",
                memory_id.map(|id| id.to_string()).unwrap_or_else(|| "none".to_string()),
                chat_input_enabled,
                args_count,
                has_flow_status
            );

            FlowContext {
                memory_id,
                chat_input_enabled,
                args,
                flow_status,
            }
        }
        Ok(None) => {
            tracing::warn!(
                "HERE No flow context found for root job {} (agent job {}), returning default",
                root_job_id,
                job.id
            );
            FlowContext::default()
        }
        Err(e) => {
            tracing::error!(
                "HERE Failed to get flow context for job {}: {}",
                job.id,
                e
            );
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

/// Convert raw MCP tools to Windmill Tool format with source tracking
fn convert_mcp_tools_to_windmill_tools(
    mcp_tools: &[rmcp::model::Tool],
    resource_name: &str,
    resource_path: &str,
) -> Result<Vec<Tool>, Error> {
    mcp_tools
        .iter()
        .map(|mcp_tool| {
            let tool_name = format!("mcp_{}_{}", resource_name, mcp_tool.name);

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

/// Load tools from MCP servers and return both the clients and tools
/// Returns a map of resource name -> client, and a vector of tools
pub async fn load_mcp_tools(
    db: &DB,
    workspace_id: &str,
    mcp_configs: Vec<McpResourceConfig>,
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

/// Merge static input transforms into AI-provided arguments.
/// Static transforms take precedence over AI arguments.
///
/// This allows users to pre-configure certain tool parameters via static input transforms
/// while the AI fills in the remaining dynamic parameters.
pub fn merge_static_transforms(
    ai_args: &mut HashMap<String, Box<RawValue>>,
    input_transforms: &HashMap<String, InputTransform>,
) -> Result<(), Error> {
    let mut merge_count = 0;

    for (key, transform) in input_transforms {
        if let InputTransform::Static { value } = transform {
            // Skip empty/null values
            let val_str = value.get().trim();
            if !val_str.is_empty() && val_str != "null" {
                tracing::info!(
                    "  Merging static transform for parameter '{}': {}",
                    key,
                    if val_str.len() > 100 {
                        format!("{}...", &val_str[..100])
                    } else {
                        val_str.to_string()
                    }
                );

                // Insert/overwrite with static value
                ai_args.insert(key.clone(), value.clone());
                merge_count += 1;
            }
        }
        // Skip JavaScript transforms for now (future enhancement)
    }

    if merge_count > 0 {
        tracing::info!(
            "Merged {} static transform(s) into AI arguments. Final argument count: {}",
            merge_count,
            ai_args.len()
        );
    } else {
        tracing::info!("No static transforms to merge. Using AI arguments as-is.");
    }

    Ok(())
}

/// Evaluate both static and JavaScript input transforms for AI tools.
/// Supports flow_input, previous_result, result, params, and results.stepId references.
///
/// Phase 1: flow_input, previous_result, result, params
/// Phase 2: results.stepId (requires id_context)
pub async fn evaluate_input_transforms(
    tool_args: &mut HashMap<String, Box<RawValue>>,
    input_transforms: &HashMap<String, InputTransform>,
    flow_context: &FlowContext,
    previous_result: Option<&Box<RawValue>>,
    id_context: Option<&crate::js_eval::IdContext>,
    client: &windmill_common::client::AuthedClient,
) -> Result<(), Error> {
    // Early return if no transforms
    if input_transforms.is_empty() {
        tracing::debug!("HERE No input transforms to evaluate");
        return Ok(());
    }

    let static_count = input_transforms.iter().filter(|(_, t)| matches!(t, InputTransform::Static { .. })).count();
    let js_count = input_transforms.iter().filter(|(_, t)| matches!(t, InputTransform::Javascript { .. })).count();
    let has_previous_result = previous_result.is_some();
    let has_flow_args = flow_context.args.is_some();
    let has_id_context = id_context.is_some();

    tracing::info!(
        "HERE Evaluating {} input transforms ({} static, {} JavaScript) - has_previous_result={}, has_flow_args={}, has_id_context={}",
        input_transforms.len(),
        static_count,
        js_count,
        has_previous_result,
        has_flow_args,
        has_id_context
    );

    // Pre-build base context once for all JavaScript transforms
    let mut base_context = HashMap::new();
    if let Some(prev) = previous_result {
        let arc_result = Arc::new(prev.clone());
        base_context.insert("previous_result".to_string(), arc_result.clone());
        base_context.insert("result".to_string(), arc_result);
        tracing::debug!("HERE Added previous_result and result to transform context");
    }

    let mut static_applied = 0;
    let mut js_applied = 0;

    for (key, transform) in input_transforms {
        match transform {
            InputTransform::Static { value } => {
                // Skip empty/null values - optimized single check
                let val_str = value.get().trim();
                if !val_str.is_empty() && val_str != "null" {
                    tracing::debug!(
                        "HERE Applied static transform for '{}': {}",
                        key,
                        if val_str.len() > 100 {
                            format!("{}...", &val_str[..100])
                        } else {
                            val_str.to_string()
                        }
                    );
                    tool_args.insert(key.clone(), value.clone());
                    static_applied += 1;
                } else {
                    tracing::debug!("HERE Skipped empty/null static transform for '{}'", key);
                }
            }
            InputTransform::Javascript { expr } => {
                tracing::info!(
                    "HERE Evaluating JavaScript transform for '{}': {}",
                    key,
                    if expr.len() > 100 {
                        format!("{}...", &expr[..100])
                    } else {
                        expr.clone()
                    }
                );

                // Clone base context and add current params
                let mut ctx = base_context.clone();
                ctx.insert("params".to_string(), Arc::new(to_raw_value(&tool_args)));

                // Prepare flow_input as Marc for eval_timeout
                let flow_input = flow_context.args.as_ref().map(|args| {
                    mappable_rc::Marc::new(args.clone())
                });

                // Evaluate JavaScript expression with optional IdContext for results.stepId
                let result = crate::js_eval::eval_timeout(
                    expr.clone(),
                    ctx,
                    flow_input,
                    Some(client),
                    id_context,  // Phase 2: enables results.stepId syntax
                    None,  // No additional ctx
                )
                .await
                .map_err(|e| {
                    tracing::error!(
                        "HERE JavaScript transform failed for '{}': {:#}",
                        key,
                        e
                    );
                    Error::ExecutionErr(format!(
                        "Failed to evaluate JavaScript transform for '{}': {:#}",
                        key, e
                    ))
                })?;

                let result_preview = result.get();
                tracing::info!(
                    "HERE JavaScript transform for '{}' succeeded: {}",
                    key,
                    if result_preview.len() > 100 {
                        format!("{}...", &result_preview[..100])
                    } else {
                        result_preview.to_string()
                    }
                );

                tool_args.insert(key.clone(), result);
                js_applied += 1;
            }
        }
    }

    tracing::info!(
        "HERE Input transforms complete: {} static applied, {} JavaScript applied, final args count: {}",
        static_applied,
        js_applied,
        tool_args.len()
    );

    Ok(())
}
