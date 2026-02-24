#[cfg(feature = "bedrock")]
use crate::ai::providers::bedrock::check_env_credentials;
use crate::ai::tools::{execute_tool_calls, ToolExecutionContext};
use crate::ai::utils::{
    add_message_to_conversation, any_tool_needs_previous_result, cleanup_mcp_clients,
    filter_schema_by_input_transforms, find_unique_tool_name, get_flow_context,
    get_flow_job_runnable_and_raw_flow, get_step_name_from_flow, load_mcp_tools,
    parse_raw_script_schema, should_use_structured_output_tool,
    update_flow_status_module_with_actions, update_flow_status_module_with_actions_success,
};
use crate::memory_oss::{read_from_memory, write_to_memory};
use crate::worker_flow::{get_previous_job_result, get_transform_context};
use async_recursion::async_recursion;
use regex::Regex;
use serde_json::value::RawValue;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
#[cfg(feature = "mcp")]
use windmill_mcp::McpClient;

#[cfg(not(feature = "mcp"))]
use crate::ai::tools::McpClientStub as McpClient;
use windmill_common::{
    ai_providers::AIProvider,
    cache,
    client::AuthedClient,
    db::DB,
    error::{self, Error},
    flow_conversations::MessageType,
    flow_status::AgentAction,
    flows::{FlowModule, FlowModuleValue, ToolValue},
    get_latest_hash_for_path,
    jobs::JobKind,
    scripts::get_full_hub_script_by_path,
    utils::{StripPath, HTTP_CLIENT},
    worker::{to_raw_value, Connection},
};
use windmill_queue::{CanceledBy, MiniPulledJob};

use crate::{
    ai::{
        image_handler::upload_image_to_s3,
        query_builder::{
            create_query_builder, BuildRequestArgs, ParsedResponse, StreamEventProcessor,
        },
        types::*,
    },
    common::{build_args_map, resolve_job_timeout, OccupancyMetrics, StreamNotifier},
    handle_child::run_future_with_polling_update_job_poller,
};

lazy_static::lazy_static! {
    static ref TOOL_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();

    /// Parse AI_HTTP_HEADERS environment variable into a vector of (header_name, header_value) tuples
    /// Format: "header1: value1, header2: value2"
    static ref AI_HTTP_HEADERS: Vec<(String, String)> = {
        std::env::var("AI_HTTP_HEADERS")
            .ok()
            .map(|headers_str| {
                headers_str
                    .split(',')
                    .filter_map(|header| {
                        let parts: Vec<&str> = header.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let name = parts[0].trim().to_string();
                            let value = parts[1].trim().to_string();
                            if !name.is_empty() && !value.is_empty() {
                                Some((name, value))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    static ref AI_AGENT_TOOL_SCHEMA: Box<RawValue> = to_raw_value(&serde_json::json!({
        "type": "object",
        "properties": {
            "user_message": { "type": "string" },
        },
        "required": ["user_message"],
        "additionalProperties": false,
    }));
}

const DEFAULT_MAX_AGENT_ITERATIONS: usize = 10;
const HARD_MAX_AGENT_ITERATIONS: usize = 1000;

fn find_module_by_id(
    modules: &Vec<FlowModule>,
    target_id: &str,
) -> Result<Option<FlowModule>, Error> {
    let mut found: Option<FlowModule> = None;
    FlowModule::traverse_modules(modules, &mut |module| {
        if found.is_none() && module.id == target_id {
            found = Some(module.clone());
        }
        Ok(())
    })
    .map_err(|e| Error::internal_err(format!("Failed to traverse flow modules: {e}")))?;
    Ok(found)
}

fn find_ai_agent_tool_module_in_parent_agent(
    modules: &Vec<FlowModule>,
    parent_agent_step_id: &str,
    tool_module_id: &str,
) -> Result<Option<FlowModule>, Error> {
    let Some(parent_agent_module) = find_module_by_id(modules, parent_agent_step_id)? else {
        return Ok(None);
    };

    let FlowModuleValue::AIAgent { tools, .. } = parent_agent_module.get_value()? else {
        return Ok(None);
    };

    for tool in tools {
        if tool.id == tool_module_id {
            return Ok(Option::<FlowModule>::from(&tool));
        }
    }

    Ok(None)
}

pub async fn handle_ai_agent_job(
    // connection
    conn: &Connection,
    db: &DB,

    // agent job
    job: &MiniPulledJob,

    // job execution context
    client: &AuthedClient,
    canceled_by: &mut Option<CanceledBy>,
    mem_peak: &mut i32,
    occupancy_metrics: &mut OccupancyMetrics,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
    has_stream: &mut bool,
) -> Result<Box<RawValue>, Error> {
    // build_args_map returns None if no $res:/$var: transforms needed, in which case use original args
    let args = match build_args_map(job, client, conn).await? {
        Some(transformed) => transformed,
        None => job.args.as_ref().map(|a| a.0.clone()).unwrap_or_default(),
    };
    let args = serde_json::from_str::<AIAgentArgs>(&serde_json::to_string(&args)?)?;

    // Handle dry_run mode - check credentials without making API calls
    if args.credentials_check {
        return handle_credentials_check(&args.provider).await;
    }

    // flow_step_id is set by the flow executor for top-level AI agents.
    // For nested AI agent tools, it's not set (to avoid triggering flow step
    // machinery on a parent that has no v2_job_status row), so we extract the
    // tool module ID from the runnable_path which has the form ".../tools/{id}".
    let flow_step_id = job
        .flow_step_id
        .as_deref()
        .or_else(|| job.runnable_path().rsplit_once("/tools/").map(|(_, id)| id))
        .ok_or_else(|| Error::internal_err("AI agent job has no flow step id".to_string()))?
        .to_string();
    let flow_step_id = &flow_step_id;

    let Some(immediate_parent_job) = &job.parent_job else {
        return Err(Error::internal_err(
            "AI agent job has no parent job".to_string(),
        ));
    };

    let mut flow_job_id = *immediate_parent_job;
    let mut flow_job = get_flow_job_runnable_and_raw_flow(db, &flow_job_id).await?;
    let direct_parent_job_kind = flow_job.kind;
    let direct_parent_job_flow_step_id = flow_job.flow_step_id.clone();

    // If the direct parent is an AI agent (nested tool case), go one level up to the flow.
    if flow_job.kind == JobKind::AIAgent {
        let Some(parent_job_id) = flow_job.parent_job else {
            return Err(Error::internal_err(
                "AI agent parent has no parent job".to_string(),
            ));
        };
        flow_job_id = parent_job_id;
        flow_job = get_flow_job_runnable_and_raw_flow(db, &flow_job_id).await?;

        if !matches!(
            flow_job.kind,
            JobKind::Flow | JobKind::FlowNode | JobKind::FlowPreview
        ) {
            return Err(Error::internal_err(
                "AI agent nesting beyond 2 levels is not supported. \
                 Only flow → agent → nested agent tool is allowed."
                    .to_string(),
            ));
        }
    }

    let flow_data = match flow_job.kind {
        JobKind::Flow | JobKind::FlowNode => {
            cache::job::fetch_flow(db, &flow_job.kind, flow_job.runnable_id).await?
        }
        JobKind::FlowPreview => {
            cache::job::fetch_preview_flow(db, &flow_job_id, flow_job.raw_flow).await?
        }
        _ => {
            return Err(Error::internal_err(
                "expected parent flow, flow preview or flow node for ai agent job".to_string(),
            ));
        }
    };

    let value = flow_data.value();

    let module = if direct_parent_job_kind == JobKind::AIAgent {
        let parent_agent_step_id = direct_parent_job_flow_step_id.as_deref().ok_or_else(|| {
            Error::internal_err("Parent AI agent job has no flow_step_id".to_string())
        })?;
        find_ai_agent_tool_module_in_parent_agent(
            &value.modules,
            parent_agent_step_id,
            flow_step_id,
        )?
    } else {
        find_module_by_id(&value.modules, flow_step_id)?
    };

    let Some(module) = module else {
        return Err(Error::internal_err(
            "AI agent module not found in flow".to_string(),
        ));
    };

    let summary = module.summary.clone();

    let FlowModuleValue::AIAgent { tools, .. } = module.get_value()? else {
        return Err(Error::internal_err(
            "AI agent module is not an AI agent".to_string(),
        ));
    };

    // Separate Windmill tools from MCP tools, websearch, and extract MCP resource configs
    let mut windmill_modules: Vec<FlowModule> = Vec::new();
    #[allow(unused_mut)]
    let mut mcp_configs: Vec<crate::ai::utils::McpResourceConfig> = Vec::new();
    let mut has_websearch = false;

    for tool in tools {
        match &tool.value {
            #[allow(unused_variables)]
            ToolValue::Mcp(mcp_config) => {
                #[cfg(feature = "mcp")]
                {
                    // This is an MCP tool - extract config
                    tracing::debug!(
                        "MCP server module: path={}, include={:?}, exclude={:?}",
                        mcp_config.resource_path,
                        mcp_config.include_tools,
                        mcp_config.exclude_tools
                    );
                    mcp_configs.push(crate::ai::utils::McpResourceConfig {
                        resource_path: mcp_config.resource_path.clone(),
                        include_tools: Some(mcp_config.include_tools.clone()),
                        exclude_tools: Some(mcp_config.exclude_tools.clone()),
                    });
                }

                #[cfg(not(feature = "mcp"))]
                {
                    tracing::warn!("MCP tool detected but MCP feature is not enabled");
                }
            }
            ToolValue::FlowModule(_) => {
                // Regular Windmill flow module (script, flow, etc.) - convert to FlowModule
                tracing::debug!("Windmill module: {:?}", tool.id);
                if let Some(flow_module) = Option::<FlowModule>::from(&tool) {
                    windmill_modules.push(flow_module);
                }
            }
            ToolValue::Websearch(_) => {
                // WebSearch tool - mark as enabled
                tracing::debug!("WebSearch tool enabled");
                has_websearch = true;
            }
        }
    }

    // Process Windmill flow modules into Tool definitions
    let tools = futures::future::try_join_all(windmill_modules.into_iter().map(|mut t| {
        let conn = conn;
        let db = db;
        let job = job;
        async move {
            let Some(summary) = t.summary.as_ref().filter(|s| TOOL_NAME_REGEX.is_match(s)) else {
                return Err(Error::internal_err(format!(
                    "Invalid tool name: {:?}",
                    t.summary
                )));
            };

            // Extract schema and input_transforms from the module value
            let module_value = t.get_value()?;
            let (schema, input_transforms) = match &module_value {
                FlowModuleValue::Script {
                    hash,
                    path,
                    tag_override,
                    input_transforms,
                    is_trigger,
                    pass_flow_input_directly,
                } => {
                    let schema = match hash {
                        Some(hash) => {
                            let (_, metadata) = cache::script::fetch(conn, hash.clone()).await?;
                            Ok::<_, Error>(
                                metadata
                                    .schema
                                    .clone()
                                    .map(|s| RawValue::from_string(s).ok())
                                    .flatten(),
                            )
                        }
                        None => {
                            if path.starts_with("hub/") {
                                let hub_script = get_full_hub_script_by_path(
                                    StripPath(path.to_string()),
                                    &HTTP_CLIENT,
                                    None,
                                )
                                .await?;
                                Ok(Some(hub_script.schema))
                            } else {
                                let hash = get_latest_hash_for_path(
                                    db,
                                    &job.workspace_id,
                                    path.as_str(),
                                    true,
                                )
                                .await?
                                .0;
                                // update module definition to use a fixed hash so all tool calls match the same schema
                                t.value = to_raw_value(&FlowModuleValue::Script {
                                    hash: Some(hash),
                                    path: path.clone(),
                                    tag_override: tag_override.clone(),
                                    input_transforms: input_transforms.clone(),
                                    is_trigger: *is_trigger,
                                    pass_flow_input_directly: *pass_flow_input_directly,
                                });
                                let (_, metadata) = cache::script::fetch(conn, hash).await?;
                                Ok(metadata
                                    .schema
                                    .clone()
                                    .map(|s| RawValue::from_string(s).ok())
                                    .flatten())
                            }
                        }
                    }?;
                    (schema, input_transforms)
                }
                FlowModuleValue::RawScript { content, language, input_transforms, .. } => {
                    let schema = Some(parse_raw_script_schema(&content, &language)?);
                    (schema, input_transforms)
                }
                FlowModuleValue::AIAgent { input_transforms, .. } => {
                    // By convention for AIAgent tools, only user_message is expected to be AI-filled.
                    (
                        Some(
                            RawValue::from_string(AI_AGENT_TOOL_SCHEMA.get().to_string())
                                .expect("AI_AGENT_TOOL_SCHEMA should always be valid JSON"),
                        ),
                        input_transforms,
                    )
                }
                _ => {
                    return Err(Error::internal_err(format!(
                        "Unsupported tool: {}",
                        summary
                    )));
                }
            };

            // Filter schema based on user given input transforms
            let schema = if let Some(s) = schema {
                Some(filter_schema_by_input_transforms(s, input_transforms)?)
            } else {
                None
            };

            Ok(Tool {
                def: ToolDef {
                    r#type: "function".to_string(),
                    function: ToolDefFunction {
                        name: summary.clone(),
                        description: Some(summary.clone()),
                        parameters: schema.unwrap_or_else(|| {
                            to_raw_value(&serde_json::json!({
                                "type": "object",
                                "properties": {},
                                "required": [],
                            }))
                        }),
                    },
                },
                module: Some(t),
                mcp_source: None,
            })
        }
    }))
    .await?;

    // Load MCP tools if configured
    let mut tools = tools;

    let mcp_clients = if !mcp_configs.is_empty() {
        let (clients, mcp_tools) =
            load_mcp_tools(db, &job.workspace_id, mcp_configs, &client.token).await?;
        tools.extend(mcp_tools);
        clients
    } else {
        HashMap::new()
    };

    let mut inner_occupancy_metrics = occupancy_metrics.clone();

    let stream_notifier = StreamNotifier::new(conn, job);

    if let Some(stream_notifier) = stream_notifier {
        stream_notifier.update_flow_status_with_stream_job();
    }

    let flow_status_job = if direct_parent_job_kind == JobKind::AIAgent {
        None
    } else {
        Some(flow_job_id)
    };

    let agent_fut = run_agent(
        db,
        conn,
        job,
        flow_status_job.as_ref(),
        Some(flow_step_id.as_str()),
        &args,
        &tools,
        &mcp_clients,
        summary.as_deref(),
        client,
        &mut inner_occupancy_metrics,
        worker_dir,
        base_internal_url,
        worker_name,
        hostname,
        killpill_rx,
        has_stream,
        has_websearch,
    );

    let result = run_future_with_polling_update_job_poller(
        job.id,
        job.timeout,
        conn,
        mem_peak,
        canceled_by,
        agent_fut,
        worker_name,
        &job.workspace_id,
        &mut Some(occupancy_metrics),
        Box::pin(futures::stream::once(async { 0 })),
    )
    .await?;

    // Cleanup MCP clients
    cleanup_mcp_clients(mcp_clients).await;

    Ok(result)
}

#[async_recursion]
pub async fn run_agent(
    // connection
    db: &DB,
    conn: &Connection,

    // agent job and flow data
    job: &MiniPulledJob,
    parent_job: Option<&Uuid>,
    flow_step_id_override: Option<&str>,
    args: &AIAgentArgs,
    tools: &[Tool],
    mcp_clients: &HashMap<String, Arc<McpClient>>,
    summary: Option<&str>,

    // job execution context
    client: &AuthedClient,
    occupancy_metrics: &mut OccupancyMetrics,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
    has_stream: &mut bool,
    has_websearch: bool,
) -> error::Result<Box<RawValue>> {
    let output_type = args.output_type.as_ref().unwrap_or(&OutputType::Text);
    // Skip get_base_url for Bedrock - it uses SDK directly, not HTTP
    let base_url = if args.provider.kind == AIProvider::AWSBedrock {
        String::new()
    } else {
        args.provider.get_base_url(db).await?
    };
    let api_key = args.provider.get_api_key().unwrap_or("");

    // Create the query builder for the provider
    let query_builder = create_query_builder(&args.provider);

    // Initialize messages
    let mut messages =
        if let Some(system_prompt) = args.system_prompt.clone().filter(|s| !s.is_empty()) {
            vec![OpenAIMessage {
                role: "system".to_string(),
                content: Some(OpenAIContent::Text(system_prompt)),
                ..Default::default()
            }]
        } else {
            vec![]
        };

    // Effective flow_step_id: override for nested agents, otherwise from job
    let effective_flow_step_id: Option<&str> =
        flow_step_id_override.or(job.flow_step_id.as_deref());

    // Fetch flow context for input transforms context, chat and memory
    let mut flow_context = get_flow_context(db, job).await;

    // Determine if we're using manual messages (which bypasses memory)
    let use_manual_messages = matches!(args.memory, Some(Memory::Manual { .. }));

    // Check if user_message is provided and non-empty
    let has_user_message = args
        .user_message
        .as_ref()
        .map(|m| !m.is_empty())
        .unwrap_or(false);

    // Validate: at least one of memory with manual messages or user_message must be provided
    if !use_manual_messages && !has_user_message {
        return Err(Error::internal_err(
            "Either 'memory' with manual messages or 'user_message' must be provided".to_string(),
        ));
    }

    let is_text_output = output_type == &OutputType::Text;

    // Flow-level memory_id (from chat mode) takes precedence over step-level memory_id
    let memory_id = flow_context
        .flow_status
        .as_ref()
        .and_then(|fs| fs.memory_id)
        .or_else(|| {
            // Extract memory_id from Memory::Auto if present
            match &args.memory {
                Some(Memory::Auto { memory_id, .. }) => *memory_id,
                _ => None,
            }
        });

    // Load messages based on history mode
    if matches!(output_type, OutputType::Text) {
        match &args.memory {
            Some(Memory::Manual { messages: manual_messages }) => {
                // Use explicitly provided messages (bypass memory)
                if !manual_messages.is_empty() {
                    messages.extend(manual_messages.clone());
                }
            }
            Some(Memory::Auto { context_length, .. }) => {
                // Auto mode: load from memory
                if let Some(step_id) = effective_flow_step_id {
                    if let Some(memory_id) = memory_id {
                        // Read messages from memory
                        match read_from_memory(db, &job.workspace_id, memory_id, step_id).await {
                            Ok(Some(loaded_messages)) => {
                                // Take the last n messages
                                let start_idx =
                                    loaded_messages.len().saturating_sub(*context_length);
                                let mut messages_to_load = loaded_messages[start_idx..].to_vec();
                                let first_non_tool_message_index =
                                    messages_to_load.iter().position(|m| m.role != "tool");

                                // Remove the first messages if their role is "tool" to avoid OpenAI API error
                                if let Some(index) = first_non_tool_message_index {
                                    messages_to_load = messages_to_load[index..].to_vec();
                                }

                                messages.extend(messages_to_load);
                            }
                            Ok(None) => {}
                            Err(e) => {
                                tracing::error!(
                                    "Failed to read memory for step {}: {}",
                                    step_id,
                                    e
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Extract previous step result only if any tool needs it
    let previous_result = {
        if any_tool_needs_previous_result(&tools) {
            if let Some(ref flow_status) = flow_context.flow_status {
                get_previous_job_result(db, &job.workspace_id, flow_status)
                    .await
                    .ok()
                    .flatten()
            } else {
                None
            }
        } else {
            None
        }
    };

    // Build IdContext for results.stepId syntax
    let id_context = {
        if let Some(ref flow_status) = flow_context.flow_status {
            // Get the step ID from the AI agent's flow step
            let previous_id = effective_flow_step_id
                .map(str::to_string)
                .unwrap_or_else(|| "unknown".to_string());

            Some(get_transform_context(job, &previous_id, flow_status))
        } else {
            None
        }
    };

    // Add user message if provided and non-empty
    if let Some(ref user_message) = args.user_message {
        if !user_message.is_empty() {
            messages.push(OpenAIMessage {
                role: "user".to_string(),
                content: Some(OpenAIContent::Text(user_message.clone())),
                ..Default::default()
            });
        }
    }

    // Add user images if provided
    if let Some(ref user_images) = args.user_images {
        if !user_images.is_empty() {
            let mut parts = vec![];
            for image in user_images.iter() {
                if !image.s3.is_empty() {
                    parts.push(ContentPart::S3Object { s3_object: image.clone() });
                }
            }
            messages.push(OpenAIMessage {
                role: "user".to_string(),
                content: Some(OpenAIContent::Parts(parts)),
                ..Default::default()
            });
        }
    }

    let mut actions = vec![];
    let mut content = None;
    let mut final_usage: Option<crate::ai::types::TokenUsage> = None;

    // Check if this provider supports tools with the current output type
    let supports_tools = query_builder.supports_tools_with_output_type(output_type);

    let mut tool_defs: Option<Vec<ToolDef>> = if tools.is_empty() || !supports_tools {
        None
    } else {
        Some(tools.iter().map(|t| t.def.clone()).collect())
    };

    // Handle structured output schema
    let has_output_properties = args
        .output_schema
        .as_ref()
        .and_then(|schema| schema.properties.as_ref())
        .map(|props| !props.is_empty())
        .unwrap_or(false);

    let should_use_structured_output_tool =
        should_use_structured_output_tool(&args.provider.kind, &args.provider.model);
    let mut used_structured_output_tool = false;
    let mut structured_output_tool_name: Option<String> = None;

    // For text output with schema, handle structured output
    if has_output_properties && is_text_output {
        let schema = args.output_schema.as_ref().unwrap();
        if should_use_structured_output_tool {
            // Anthropic uses a tool for structured output
            let unique_tool_name = find_unique_tool_name("structured_output", tool_defs.as_deref());
            structured_output_tool_name = Some(unique_tool_name.clone());

            let output_tool = ToolDef {
                r#type: "function".to_string(),
                function: ToolDefFunction {
                    name: unique_tool_name,
                    description: Some(
                        "This tool MUST be used last to return a structured JSON object as the final output."
                            .to_string(),
                    ),
                    parameters: to_raw_value(&schema),
                },
            };
            if let Some(ref mut existing_tools) = tool_defs {
                existing_tools.push(output_tool);
            } else {
                tool_defs = Some(vec![output_tool]);
            }
        }
        // For non-Anthropic providers, response_format is handled by the query builder
    }

    let user_wants_streaming = args.streaming.unwrap_or(false);
    *has_stream = user_wants_streaming && is_text_output;

    let mut final_events_str = String::new();

    // Always create a StreamEventProcessor for text output (use silent mode if user doesn't want streaming)
    let stream_event_processor = if is_text_output {
        if user_wants_streaming {
            Some(StreamEventProcessor::new(conn, job))
        } else {
            Some(StreamEventProcessor::new_silent())
        }
    } else {
        None
    };

    let chat_enabled = flow_context
        .flow_status
        .as_ref()
        .and_then(|fs| fs.chat_input_enabled)
        .unwrap_or(false);

    let step_name = get_step_name_from_flow(summary.as_deref(), effective_flow_step_id);

    let max_iterations = args
        .max_iterations
        .map(|m| m.clamp(1, HARD_MAX_AGENT_ITERATIONS))
        .unwrap_or(DEFAULT_MAX_AGENT_ITERATIONS);

    // Main agent loop
    for i in 0..max_iterations {
        if used_structured_output_tool {
            break;
        }

        // Handle AWS Bedrock provider specially using the official SDK
        let parsed = if args.provider.kind == AIProvider::AWSBedrock {
            #[cfg(feature = "bedrock")]
            {
                let region = args
                    .provider
                    .get_region()
                    .unwrap_or(windmill_common::ai_providers::USE_ENV_REGION);
                // Use Bedrock SDK via dedicated query builder
                crate::ai::providers::bedrock::BedrockQueryBuilder::default()
                    .execute_request(
                        &messages,
                        tool_defs.as_deref(),
                        args.provider.get_model(),
                        args.temperature,
                        args.max_completion_tokens,
                        api_key,
                        region,
                        stream_event_processor.clone(),
                        client,
                        &job.workspace_id,
                        structured_output_tool_name.as_deref(),
                        args.provider.get_aws_access_key_id(),
                        args.provider.get_aws_secret_access_key(),
                        args.provider.get_aws_session_token(),
                    )
                    .await?
            }
            #[cfg(not(feature = "bedrock"))]
            {
                return Err(Error::internal_err(
                    "AWS Bedrock support is not enabled. Build with 'bedrock' feature.".to_string(),
                ));
            }
        } else {
            // For all other providers, use the HTTP client approach
            let build_args = BuildRequestArgs {
                messages: &messages,
                tools: tool_defs.as_deref(),
                model: args.provider.get_model(),
                temperature: args.temperature,
                max_tokens: args.max_completion_tokens,
                output_schema: args.output_schema.as_ref(),
                output_type,
                system_prompt: args.system_prompt.as_deref(),
                user_message: args.user_message.as_deref().unwrap_or(""),
                images: args.user_images.as_deref(),
                has_websearch,
            };

            let request_body = query_builder
                .build_request(&build_args, client, &job.workspace_id)
                .await?;

            let endpoint =
                query_builder.get_endpoint(&base_url, args.provider.get_model(), output_type);
            let auth_headers = query_builder.get_auth_headers(api_key, &base_url, output_type);

            let timeout = resolve_job_timeout(conn, &job.workspace_id, job.id, job.timeout)
                .await
                .0;

            // Helper to build HTTP request with headers
            let build_http_request = |body: String| {
                let mut req = HTTP_CLIENT
                    .post(&endpoint)
                    .timeout(timeout)
                    .header("Content-Type", "application/json");

                for (header_name, header_value) in &auth_headers {
                    req = req.header(*header_name, header_value.clone());
                }

                for (header_name, header_value) in AI_HTTP_HEADERS.iter() {
                    req = req.header(header_name.as_str(), header_value.as_str());
                }

                req.body(body)
            };

            let resp = build_http_request(request_body.clone())
                .send()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to call API: {}", e)))?;

            // Check if request failed and we should retry without stream_options
            let resp = match resp.error_for_status_ref() {
                Ok(_) => resp,
                Err(e) => {
                    let status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());

                    // Retry without stream_options if provider supports it and error suggests incompatibility
                    // Common error patterns: 400 Bad Request with mentions of stream_options or include_usage
                    let should_retry = query_builder.supports_retry_without_usage()
                        && status.as_u16() == 400
                        && (text.contains("stream_options")
                            || text.contains("include_usage")
                            || text.contains("Additional properties are not allowed"));

                    if should_retry {
                        tracing::info!(
                            "Retrying request without stream_options due to provider incompatibility"
                        );

                        let retry_body = query_builder
                            .build_request_without_usage(&build_args, client, &job.workspace_id)
                            .await?;

                        let retry_resp =
                            build_http_request(retry_body).send().await.map_err(|e| {
                                Error::internal_err(format!("Failed to call API on retry: {}", e))
                            })?;

                        match retry_resp.error_for_status_ref() {
                            Ok(_) => retry_resp,
                            Err(retry_e) => {
                                let retry_text = retry_resp
                                    .text()
                                    .await
                                    .unwrap_or_else(|_| "<failed to read body>".to_string());
                                return Err(Error::internal_err(format!(
                                    "API error on retry: {} - {}",
                                    retry_e, retry_text
                                )));
                            }
                        }
                    } else {
                        return Err(Error::internal_err(format!("API error: {} - {}", e, text)));
                    }
                }
            };

            if let Some(ref stream_event_processor) = stream_event_processor {
                query_builder
                    .parse_streaming_response(resp, stream_event_processor.clone())
                    .await?
            } else {
                query_builder.parse_image_response(resp).await?
            }
        };

        match parsed {
            ParsedResponse::Text {
                content: response_content,
                tool_calls,
                events_str,
                annotations,
                used_websearch,
                usage,
            } => {
                // Accumulate usage from this iteration
                if let Some(u) = usage {
                    match &mut final_usage {
                        Some(existing) => existing.accumulate(&u),
                        None => final_usage = Some(u),
                    }
                }
                if let Some(events_str) = events_str {
                    final_events_str.push_str(&events_str);
                }

                // Add websearch tool message if websearch was used
                if used_websearch {
                    actions.push(AgentAction::WebSearch {});
                    messages.push(OpenAIMessage {
                        role: "tool".to_string(),
                        content: Some(OpenAIContent::Text(
                            "Used websearch tool successfully".to_string(),
                        )),
                        agent_action: Some(AgentAction::WebSearch {}),
                        ..Default::default()
                    });
                    if chat_enabled {
                        if let Some(memory_id) = memory_id {
                            let agent_job_id = job.id;
                            let db_clone = db.clone();
                            let message_content = "Used websearch tool successfully".to_string();
                            let step_name = step_name.clone();
                            tokio::spawn(async move {
                                if let Err(e) = add_message_to_conversation(
                                    &db_clone,
                                    &memory_id,
                                    Some(agent_job_id),
                                    &message_content,
                                    MessageType::Tool,
                                    &step_name,
                                    true,
                                )
                                .await
                                {
                                    tracing::warn!(
                                        "Failed to add websearch tool message to conversation {}: {}",
                                        memory_id,
                                        e
                                    );
                                }
                            });
                        }
                    }
                }

                if let Some(ref response_content) = response_content {
                    actions.push(AgentAction::Message {});
                    messages.push(OpenAIMessage {
                        role: "assistant".to_string(),
                        content: Some(OpenAIContent::Text(response_content.clone())),
                        agent_action: Some(AgentAction::Message {}),
                        annotations: if annotations.is_empty() {
                            None
                        } else {
                            Some(annotations.clone())
                        },
                        ..Default::default()
                    });

                    if let Some(parent_job) = parent_job {
                        update_flow_status_module_with_actions(db, parent_job, &actions).await?;
                        update_flow_status_module_with_actions_success(db, parent_job, true)
                            .await?;
                    }

                    content = Some(OpenAIContent::Text(response_content.clone()));

                    // Add assistant message to conversation if chat_input_enabled
                    if chat_enabled && !response_content.is_empty() {
                        if let Some(memory_id) = memory_id {
                            let agent_job_id = job.id;
                            let db_clone = db.clone();
                            let message_content = response_content.clone();
                            let step_name = step_name.clone();

                            // Spawn task because we do not need to wait for the result
                            tokio::spawn(async move {
                                if let Err(e) = add_message_to_conversation(
                                    &db_clone,
                                    &memory_id,
                                    Some(agent_job_id),
                                    &message_content,
                                    MessageType::Assistant,
                                    &step_name,
                                    true,
                                )
                                .await
                                {
                                    tracing::warn!(
                                        "Failed to add assistant message to conversation {}: {}",
                                        memory_id,
                                        e
                                    );
                                }
                            });
                        }
                    }
                }

                if tool_calls.is_empty() {
                    break;
                } else if i == max_iterations - 1 {
                    return Err(Error::internal_err(
                        "AI agent reached max iterations, but there are still tool calls"
                            .to_string(),
                    ));
                }

                messages.push(OpenAIMessage {
                    role: "assistant".to_string(),
                    tool_calls: Some(tool_calls.clone()),
                    ..Default::default()
                });

                // Handle tool calls using extracted tools module
                let tool_execution_ctx = ToolExecutionContext {
                    db,
                    conn,
                    job,
                    parent_job,
                    summary: &summary,
                    flow_step_id_override,
                    client,
                    worker_dir,
                    base_internal_url,
                    worker_name,
                    hostname,
                    occupancy_metrics,
                    killpill_rx,
                    stream_event_processor: stream_event_processor.as_ref(),
                    flow_context: &mut flow_context,
                    previous_result: &previous_result,
                    id_context: &id_context,
                };

                let (tool_messages, tool_content, tool_used_structured_output) =
                    execute_tool_calls(
                        tool_execution_ctx,
                        &tool_calls,
                        &tools,
                        mcp_clients,
                        &mut actions,
                        &mut final_events_str,
                        &structured_output_tool_name,
                    )
                    .await?;

                messages.extend(tool_messages);
                if let Some(tc) = tool_content {
                    content = Some(tc);
                }
                used_structured_output_tool = tool_used_structured_output;
            }
            ParsedResponse::Image { base64_data } => {
                // For image output, upload to S3 and track in conversation
                let s3_object = upload_image_to_s3(&base64_data, job, client).await?;

                let content = to_raw_value(&s3_object);

                // Add assistant message to conversation if chat_input_enabled
                if chat_enabled {
                    if let Some(memory_id) = memory_id {
                        let agent_job_id = job.id;
                        let db_clone = db.clone();

                        // Create extended version with type discriminator for conversation storage
                        // This avoids conflicts with outputs that are of the same format as S3 objects
                        let s3_with_type = S3ObjectWithType {
                            s3_object: s3_object.clone(),
                            r#type: "windmill_s3_object".to_string(),
                        };

                        let message_content = serde_json::to_string(&s3_with_type)
                            .unwrap_or_else(|_| content.get().to_string());

                        // Spawn task because we do not need to wait for the result
                        tokio::spawn(async move {
                            if let Err(e) = add_message_to_conversation(
                                &db_clone,
                                &memory_id,
                                Some(agent_job_id),
                                &message_content,
                                MessageType::Assistant,
                                &step_name,
                                true,
                            )
                            .await
                            {
                                tracing::warn!(
                                    "Failed to add assistant message to conversation {}: {}",
                                    memory_id,
                                    e
                                );
                            }
                        });
                    }
                }

                // Return early since image generation is complete
                return Ok(content);
            }
        }
    }

    // Return the final result
    let final_messages: Vec<Message> = messages
        .iter()
        .map(|m| Message { message: m, agent_action: m.agent_action.as_ref() })
        .collect();

    // Parse content as JSON for structured output, fallback to string if it fails
    let output_value = match content {
        Some(content_str) => match has_output_properties {
            true => match content_str {
                OpenAIContent::Text(text) => {
                    serde_json::from_str::<Box<RawValue>>(&text).map_err(|_e| {
                        Error::internal_err(format!("Failed to parse structured output: {}", text))
                    })
                }
                OpenAIContent::Parts(_parts) => Err(Error::internal_err(
                    "Failed to parse structured output".to_string(),
                )),
            },
            false => Ok(match content_str {
                OpenAIContent::Text(text) => to_raw_value(&text),
                OpenAIContent::Parts(parts) => to_raw_value(&parts),
            }),
        }?,
        None => to_raw_value(&""),
    };

    // Wait for stream event processor to finish persisting events (if any)
    if let Some(handle) = {
        if let Some(stream_event_processor) = stream_event_processor {
            stream_event_processor.to_handle()
        } else {
            None
        }
    } {
        if let Err(e) = handle.await {
            return Err(Error::internal_err(format!(
                "Error waiting for stream event processor: {}",
                e
            )));
        }
    }

    // Persist complete conversation to memory at the end (only if in auto mode with context length)
    // Skip memory persistence if using manual messages (bypass memory entirely)
    // final_messages contains the complete history (old messages + new ones)
    if matches!(output_type, OutputType::Text) && !use_manual_messages {
        if let Some(Memory::Auto { context_length, .. }) = &args.memory {
            if let Some(step_id) = effective_flow_step_id {
                // Extract OpenAIMessages from final_messages
                let all_messages: Vec<OpenAIMessage> =
                    final_messages.iter().map(|m| m.message.clone()).collect();

                if !all_messages.is_empty() {
                    // Keep only the last n messages
                    let start_idx = all_messages.len().saturating_sub(*context_length);
                    let messages_to_persist = all_messages[start_idx..].to_vec();

                    if let Some(memory_id) = memory_id {
                        if let Err(e) = write_to_memory(
                            db,
                            &job.workspace_id,
                            memory_id,
                            step_id,
                            &messages_to_persist,
                        )
                        .await
                        {
                            tracing::error!(
                                "Failed to persist {} messages to memory for step {}: {}",
                                messages_to_persist.len(),
                                step_id,
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(to_raw_value(&AIAgentResult {
        output: output_value,
        messages: final_messages,
        wm_stream: if !final_events_str.is_empty() {
            Some(final_events_str)
        } else {
            None
        },
        usage: if final_usage.as_ref().map(|u| u.is_empty()).unwrap_or(true) {
            None
        } else {
            final_usage
        },
    }))
}

/// Handle credentials check mode - check credentials without making API calls
async fn handle_credentials_check(provider: &ProviderWithResource) -> Result<Box<RawValue>, Error> {
    let result = match &provider.kind {
        #[cfg(feature = "bedrock")]
        AIProvider::AWSBedrock => {
            let check = check_env_credentials().await;
            serde_json::json!({
                "credentials_check": true,
                "provider": "aws_bedrock",
                "credentials": {
                    "available": check.available,
                    "access_key_id_prefix": check.access_key_id_prefix,
                    "region": check.region,
                    "error": check.error
                }
            })
        }
        #[cfg(not(feature = "bedrock"))]
        AIProvider::AWSBedrock => {
            serde_json::json!({
                "credentials_check": true,
                "provider": "aws_bedrock",
                "error": "AWS Bedrock support is not enabled. Build with 'bedrock' feature."
            })
        }
        other => {
            serde_json::json!({
                "credentials_check": true,
                "provider": format!("{:?}", other),
                "message": "Credentials check not implemented for this provider"
            })
        }
    };

    serde_json::value::to_raw_value(&result).map_err(|e| Error::internal_err(e.to_string()))
}
