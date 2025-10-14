use crate::ai::mcp_client::McpClient;
use crate::ai::tools::{execute_tool_calls, ToolExecutionContext};
use crate::ai::utils::{
    add_message_to_conversation, cleanup_mcp_clients, find_unique_tool_name,
    get_flow_chat_settings, get_flow_job_runnable_and_raw_flow, get_step_name_from_flow,
    is_anthropic_provider, load_mcp_tools, parse_raw_script_schema,
    update_flow_status_module_with_actions, update_flow_status_module_with_actions_success,
    FlowChatSettings,
};
use crate::memory_oss::{read_from_memory, write_to_memory};
use async_recursion::async_recursion;
use regex::Regex;
use serde_json::value::RawValue;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
use windmill_common::{
    ai_providers::AZURE_API_VERSION,
    cache,
    client::AuthedClient,
    db::DB,
    error::{self, Error},
    flow_conversations::MessageType,
    flow_status::AgentAction,
    flows::{FlowModule, FlowModuleValue, FlowValue},
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
}

const MAX_AGENT_ITERATIONS: usize = 10;

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
    let mut args = build_args_map(job, client, conn).await?;

    // Extract resolved MCP paths from args (added by flow executor)
    let mcp_resource_paths: Vec<String> = if let Some(ref args_map) = args {
        if let Some(paths_raw) = args_map.get("_wm_mcp_resource_paths") {
            serde_json::from_str(paths_raw.get())
                .map_err(|e| Error::internal_err(format!("Failed to parse MCP paths: {}", e)))?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Remove internal field before converting to AIAgentArgs
    if let Some(ref mut args_map) = args {
        args_map.remove("_wm_mcp_resource_paths");
    }

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

    let Some(module) = module else {
        return Err(Error::internal_err(
            "AI agent module not found in flow".to_string(),
        ));
    };

    let FlowModuleValue::AIAgent { tools, input_transforms, .. } = module.get_value()? else {
        return Err(Error::internal_err(
            "AI agent module is not an AI agent".to_string(),
        ));
    };

    // Separate Windmill tools from MCP tools
    let mut windmill_modules: Vec<FlowModule> = Vec::new();

    for tool in tools {
        match tool {
            windmill_common::flows::Tool::Windmill(module) => {
                windmill_modules.push(*module);
            }
            windmill_common::flows::Tool::Mcp(_) => {
                // MCP paths already resolved by flow executor
                // (they're in mcp_resource_paths extracted earlier)
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

            let schema = match &t.get_value() {
                Ok(FlowModuleValue::Script {
                    hash,
                    path,
                    tag_override,
                    input_transforms,
                    is_trigger,
                    pass_flow_input_directly,
                }) => match hash {
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
                            let hash = get_latest_hash_for_path(db, &job.workspace_id, path.as_str(), true)
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
                },
                Ok(FlowModuleValue::RawScript { content, language, .. }) => {
                    Ok(Some(parse_raw_script_schema(&content, &language)?))
                }
                Err(e) => {
                    return Err(Error::internal_err(format!(
                        "Invalid tool {}: {}",
                        summary,
                        e.to_string()
                    )));
                }
                _ => {
                    return Err(Error::internal_err(format!(
                        "Unsupported tool: {}",
                        summary
                    )));
                }
            }?;

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
    let mcp_clients = if !mcp_resource_paths.is_empty() {
        let (clients, mcp_tools) = load_mcp_tools(db, &job.workspace_id, mcp_resource_paths).await?;
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
        value,
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
    flow_value: &FlowValue,

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

    let mut chat_settings: Option<FlowChatSettings> = None;

    // Load previous messages from memory for text output mode (only if context length is set)
    if matches!(output_type, OutputType::Text) {
        if let Some(context_length) = args.messages_context_length.filter(|&n| n > 0) {
            if let Some(step_id) = job.flow_step_id.as_deref() {
                // Fetch chat settings from root flow
                chat_settings = Some(get_flow_chat_settings(db, job).await);
                if let Some(memory_id) = chat_settings.as_ref().and_then(|s| s.memory_id) {
                    // Read messages from memory
                    match read_from_memory(&job.workspace_id, memory_id, step_id).await {
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

    let is_anthropic = is_anthropic_provider(&args.provider);
    let mut used_structured_output_tool = false;
    let mut structured_output_tool_name: Option<String> = None;

    // For text output with schema, handle structured output
    if has_output_properties && output_type == &OutputType::Text {
        let schema = args.output_schema.as_ref().unwrap();
        if is_anthropic {
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

    // Main agent loop
    for i in 0..MAX_AGENT_ITERATIONS {
        if used_structured_output_tool {
            break;
        }

        // For text output or image output with tools
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

        let endpoint =
            query_builder.get_endpoint(&base_url, args.provider.get_model(), output_type);
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

        if args.provider.kind.is_azure_openai(&base_url) {
            request = request.query(&[("api-version", AZURE_API_VERSION)])
        }

        let resp = request
            .body(request_body)
            .send()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to call API: {}", e)))?;

        match resp.error_for_status_ref() {
            Ok(_) => {
                let parsed = if let Some(stream_event_processor) = stream_event_processor.clone() {
                    query_builder
                        .parse_streaming_response(resp, stream_event_processor)
                        .await?
                } else {
                    // Handle non-streaming response
                    query_builder.parse_response(resp).await?
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

                            update_flow_status_module_with_actions(db, parent_job, &actions)
                                .await?;
                            update_flow_status_module_with_actions_success(db, parent_job, true)
                                .await?;

                            content = Some(OpenAIContent::Text(response_content.clone()));

                            // Add assistant message to conversation if chat_input_enabled
                            if chat_settings.is_none() {
                                chat_settings = Some(get_flow_chat_settings(db, job).await);
                            }
                            let chat_enabled = chat_settings
                                .as_ref()
                                .map(|s| s.chat_input_enabled)
                                .unwrap_or(false);

                            if chat_enabled && !response_content.is_empty() {
                                if let Some(mid) = chat_settings.as_ref().and_then(|s| s.memory_id)
                                {
                                    let agent_job_id = job.id;
                                    let db_clone = db.clone();
                                    let message_content = response_content.clone();
                                    let step_name = get_step_name_from_flow(
                                        flow_value,
                                        job.flow_step_id.as_deref(),
                                    );

                                    // Spawn task because we do not need to wait for the result
                                    tokio::spawn(async move {
                                        if let Err(e) = add_message_to_conversation(
                                            &db_clone,
                                            &mid,
                                            Some(agent_job_id),
                                            &message_content,
                                            MessageType::Assistant,
                                            &step_name,
                                            true,
                                        )
                                        .await
                                        {
                                            tracing::warn!("Failed to add assistant message to conversation {}: {}", mid, e);
                                        }
                                    });
                                }
                            }
                        }

                        if tool_calls.is_empty() {
                            break;
                        } else if i == MAX_AGENT_ITERATIONS - 1 {
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
                            flow_value,
                            client,
                            worker_dir,
                            base_internal_url,
                            worker_name,
                            hostname,
                            occupancy_metrics,
                            job_completed_tx,
                            killpill_rx,
                            stream_event_processor: stream_event_processor.as_ref(),
                            chat_settings: &mut chat_settings,
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
                        // For image output with tools, we got an image response
                        let s3_object = upload_image_to_s3(&base64_data, job, client).await?;
                        return Ok(to_raw_value(&s3_object));
                    }
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

                    if let Some(memory_id) = chat_settings.as_ref().and_then(|s| s.memory_id) {
                        if let Err(e) = write_to_memory(
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
