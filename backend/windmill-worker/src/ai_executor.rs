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
use windmill_common::mcp_client::McpClient;
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
    JobCompletedSender,
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
}

const DEFAULT_MAX_AGENT_ITERATIONS: usize = 10;
const HARD_MAX_AGENT_ITERATIONS: usize = 1000;

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
    job_completed_tx: &JobCompletedSender,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
    has_stream: &mut bool,
) -> Result<Box<RawValue>, Error> {
    let args = build_args_map(job, client, conn).await?;
    let args = serde_json::from_str::<AIAgentArgs>(&serde_json::to_string(&args)?)?;

    let Some(flow_step_id) = &job.flow_step_id else {
        return Err(Error::internal_err(
            "AI agent job has no flow step id".to_string(),
        ));
    };

    let Some(parent_job) = &job.parent_job else {
        return Err(Error::internal_err(
            "AI agent job has no parent job".to_string(),
        ));
    };

    let flow_job = get_flow_job_runnable_and_raw_flow(db, &parent_job).await?;

    let flow_data = match flow_job.kind {
        JobKind::Flow | JobKind::FlowNode => {
            cache::job::fetch_flow(db, &flow_job.kind, flow_job.runnable_id).await?
        }
        JobKind::FlowPreview => {
            cache::job::fetch_preview_flow(db, &parent_job, flow_job.raw_flow).await?
        }
        _ => {
            return Err(Error::internal_err(
                "expected parent flow, flow preview or flow node for ai agent job".to_string(),
            ));
        }
    };

    let value = flow_data.value();

    let module = value.modules.iter().find(|m| m.id == *flow_step_id);
    let summary = module.as_ref().and_then(|m| m.summary.clone());

    let Some(module) = module else {
        return Err(Error::internal_err(
            "AI agent module not found in flow".to_string(),
        ));
    };

    let FlowModuleValue::AIAgent { tools, .. } = module.get_value()? else {
        return Err(Error::internal_err(
            "AI agent module is not an AI agent".to_string(),
        ));
    };

    // Separate Windmill tools from MCP tools and extract MCP resource configs
    let mut windmill_modules: Vec<FlowModule> = Vec::new();
    let mut mcp_configs: Vec<crate::ai::utils::McpResourceConfig> = Vec::new();

    for tool in tools {
        match &tool.value {
            ToolValue::Mcp(mcp_config) => {
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
            ToolValue::FlowModule(_) => {
                // Regular Windmill flow module (script, flow, etc.) - convert to FlowModule
                tracing::debug!("Windmill module: {:?}", tool.id);
                if let Some(flow_module) = Option::<FlowModule>::from(&tool) {
                    windmill_modules.push(flow_module);
                }
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
                        description: None,
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
        let (clients, mcp_tools) = load_mcp_tools(db, &job.workspace_id, mcp_configs).await?;
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

    let agent_fut = run_agent(
        db,
        conn,
        job,
        parent_job,
        &args,
        &tools,
        &mcp_clients,
        summary.as_deref(),
        client,
        &mut inner_occupancy_metrics,
        job_completed_tx,
        worker_dir,
        base_internal_url,
        worker_name,
        hostname,
        killpill_rx,
        has_stream,
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
    parent_job: &Uuid,
    args: &AIAgentArgs,
    tools: &[Tool],
    mcp_clients: &HashMap<String, Arc<McpClient>>,
    summary: Option<&str>,

    // job execution context
    client: &AuthedClient,
    occupancy_metrics: &mut OccupancyMetrics,
    job_completed_tx: &JobCompletedSender,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
    has_stream: &mut bool,
) -> error::Result<Box<RawValue>> {
    let output_type = args.output_type.as_ref().unwrap_or(&OutputType::Text);
    let base_url = args.provider.get_base_url(db).await?;
    let api_key = args.provider.get_api_key();
    let region = args.provider.get_region();

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

    // Fetch flow context for input transforms context, chat and memory
    let mut flow_context = get_flow_context(db, job).await;

    // Load previous messages from memory for text output mode (only if context length is set)
    if matches!(output_type, OutputType::Text) {
        if let Some(context_length) = args.messages_context_length.filter(|&n| n > 0) {
            if let Some(step_id) = job.flow_step_id.as_deref() {
                if let Some(memory_id) = flow_context
                    .flow_status
                    .as_ref()
                    .and_then(|fs| fs.memory_id)
                {
                    // Read messages from memory
                    match read_from_memory(db, &job.workspace_id, memory_id, step_id).await {
                        Ok(Some(loaded_messages)) => {
                            // Take the last n messages
                            let start_idx = loaded_messages.len().saturating_sub(context_length);
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
                            tracing::error!("Failed to read memory for step {}: {}", step_id, e);
                        }
                    }
                }
            }
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
            let previous_id = job
                .flow_step_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string());

            Some(get_transform_context(job, &previous_id, flow_status).await?)
        } else {
            None
        }
    };

    // Create user message with optional images
    let mut parts = vec![ContentPart::Text { text: args.user_message.clone() }];
    if let Some(images) = &args.user_images {
        for image in images.iter() {
            if !image.s3.is_empty() {
                parts.push(ContentPart::S3Object { s3_object: image.clone() });
            }
        }
    }
    let user_content = OpenAIContent::Parts(parts);

    messages.push(OpenAIMessage {
        role: "user".to_string(),
        content: Some(user_content),
        ..Default::default()
    });

    let mut actions = vec![];
    let mut content = None;

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
    if has_output_properties && output_type == &OutputType::Text {
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

    // Check if streaming is enabled and supported
    let should_stream = args.streaming.unwrap_or(false)
        && query_builder.supports_streaming()
        && output_type == &OutputType::Text;

    *has_stream = should_stream;

    let mut final_events_str = String::new();

    let stream_event_processor = if should_stream {
        Some(StreamEventProcessor::new(conn, job))
    } else {
        None
    };

    let max_iterations = args
        .max_iterations
        .map(|m| m.clamp(1, HARD_MAX_AGENT_ITERATIONS))
        .unwrap_or(DEFAULT_MAX_AGENT_ITERATIONS);

    // Main agent loop
    for i in 0..max_iterations {
        if used_structured_output_tool {
            break;
        }

        // Special handling for AWS Bedrock using the official SDK
        let parsed = if args.provider.kind == AIProvider::AWSBedrock {
            let Some(region) = region else {
                return Err(Error::internal_err(
                    "AWS Bedrock region is required".to_string(),
                ));
            };
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
                    should_stream,
                    stream_event_processor.clone(),
                    client,
                    &job.workspace_id,
                    structured_output_tool_name.as_deref(),
                )
                .await?
        } else {
            // For non-Bedrock providers, use HTTP client
            let build_args = BuildRequestArgs {
                messages: &messages,
                tools: tool_defs.as_deref(),
                model: args.provider.get_model(),
                temperature: args.temperature,
                max_tokens: args.max_completion_tokens,
                output_schema: args.output_schema.as_ref(),
                output_type,
                system_prompt: args.system_prompt.as_deref(),
                user_message: &args.user_message,
                images: args.user_images.as_deref(),
            };

            let request_body = query_builder
                .build_request(&build_args, client, &job.workspace_id, should_stream)
                .await?;

            let endpoint = query_builder.get_endpoint(
                &base_url,
                args.provider.get_model(),
                output_type,
                should_stream,
            );
            let auth_headers = query_builder.get_auth_headers(api_key, &base_url, output_type);

            let timeout = resolve_job_timeout(conn, &job.workspace_id, job.id, job.timeout)
                .await
                .0;

            let mut request = HTTP_CLIENT
                .post(&endpoint)
                .timeout(timeout)
                .header("Content-Type", "application/json");

            // Apply authentication headers
            for (header_name, header_value) in &auth_headers {
                request = request.header(*header_name, header_value.clone());
            }

            // Apply custom headers from AI_HTTP_HEADERS environment variable
            for (header_name, header_value) in AI_HTTP_HEADERS.iter() {
                request = request.header(header_name.as_str(), header_value.as_str());
            }

            let resp = request
                .body(request_body)
                .send()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to call API: {}", e)))?;

            match resp.error_for_status_ref() {
                Ok(_) => {
                    if let Some(stream_event_processor) = stream_event_processor.clone() {
                        query_builder
                            .parse_streaming_response(resp, stream_event_processor)
                            .await?
                    } else {
                        // Handle non-streaming response
                        query_builder.parse_response(resp).await?
                    }
                }
                Err(e) => {
                    let _status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    return Err(Error::internal_err(format!("API error: {} - {}", e, text)));
                }
            }
        };

        match parsed {
            ParsedResponse::Text { content: response_content, tool_calls, events_str } => {
                if let Some(events_str) = events_str {
                    final_events_str.push_str(&events_str);
                }

                if let Some(ref response_content) = response_content {
                    actions.push(AgentAction::Message {});
                    messages.push(OpenAIMessage {
                        role: "assistant".to_string(),
                        content: Some(OpenAIContent::Text(response_content.clone())),
                        agent_action: Some(AgentAction::Message {}),
                        ..Default::default()
                    });

                    update_flow_status_module_with_actions(db, parent_job, &actions).await?;
                    update_flow_status_module_with_actions_success(db, parent_job, true).await?;

                    content = Some(OpenAIContent::Text(response_content.clone()));

                    // Add assistant message to conversation if chat_input_enabled
                    let chat_enabled = flow_context
                        .flow_status
                        .as_ref()
                        .and_then(|fs| fs.chat_input_enabled)
                        .unwrap_or(false);
                    if chat_enabled && !response_content.is_empty() {
                        if let Some(memory_id) = flow_context
                            .flow_status
                            .as_ref()
                            .and_then(|fs| fs.memory_id)
                        {
                            let agent_job_id = job.id;
                            let db_clone = db.clone();
                            let message_content = response_content.clone();
                            let step_name = get_step_name_from_flow(
                                summary.as_deref(),
                                job.flow_step_id.as_deref(),
                            );

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
                    client,
                    worker_dir,
                    base_internal_url,
                    worker_name,
                    hostname,
                    occupancy_metrics,
                    job_completed_tx,
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
                let chat_enabled = flow_context
                    .flow_status
                    .as_ref()
                    .and_then(|fs| fs.chat_input_enabled)
                    .unwrap_or(false);
                if chat_enabled {
                    if let Some(memory_id) = flow_context
                        .flow_status
                        .as_ref()
                        .and_then(|fs| fs.memory_id)
                    {
                        let agent_job_id = job.id;
                        let db_clone = db.clone();
                        let flow_step_id_owned = job.flow_step_id.clone();
                        let summary_owned = summary.map(|s| s.to_string());

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
                            let step_name = get_step_name_from_flow(
                                summary_owned.as_deref(),
                                flow_step_id_owned.as_deref(),
                            );

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

    if let Some(stream_event_processor) = stream_event_processor {
        if let Some(handle) = stream_event_processor.to_handle() {
            if let Err(e) = handle.await {
                return Err(Error::internal_err(format!(
                    "Error waiting for stream event processor: {}",
                    e
                )));
            }
        }
    }

    // Persist complete conversation to memory at the end (only if context length is set)
    // final_messages contains the complete history (old messages + new ones)
    if matches!(output_type, OutputType::Text) {
        if let Some(context_length) = args.messages_context_length.filter(|&n| n > 0) {
            if let Some(step_id) = job.flow_step_id.as_deref() {
                // Extract OpenAIMessages from final_messages
                let all_messages: Vec<OpenAIMessage> =
                    final_messages.iter().map(|m| m.message.clone()).collect();

                if !all_messages.is_empty() {
                    // Keep only the last n messages
                    let start_idx = all_messages.len().saturating_sub(context_length);
                    let messages_to_persist = all_messages[start_idx..].to_vec();

                    if let Some(memory_id) = flow_context.flow_status.and_then(|fs| fs.memory_id) {
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
    }))
}
