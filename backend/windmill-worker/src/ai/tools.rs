use crate::ai::query_builder::StreamEventProcessor;
use crate::ai::types::McpToolSource;
use crate::ai::types::*;
use crate::ai::utils::{
    add_message_to_conversation, execute_mcp_tool, get_step_name_from_flow,
    is_completed_input_transform, update_flow_status_module_with_actions,
    update_flow_status_module_with_actions_success, FlowContext,
};
use crate::common::OccupancyMetrics;
use crate::result_processor::handle_non_flow_job_error;
use crate::worker_flow::{
    evaluate_input_transform, raw_script_to_payload, script_to_payload, JobPayloadWithTag,
};
use crate::{
    create_job_dir, handle_queued_job, JobCompletedReceiver, JobCompletedSender, SendResult,
    SendResultPayload,
};
use anyhow::Context;
use mappable_rc::Marc;
use serde_json::value::RawValue;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
use windmill_common::ai_types::OpenAIToolCall;
use windmill_common::jobs::JobPayload;

#[cfg(feature = "mcp")]
use windmill_mcp::McpClient;

#[cfg(not(feature = "mcp"))]
pub struct McpClientStub;

#[cfg(not(feature = "mcp"))]
type McpClient = McpClientStub;
use windmill_common::{
    client::AuthedClient,
    db::DB,
    error::Error,
    flow_conversations::MessageType,
    flow_status::AgentAction,
    flows::FlowModuleValue,
    worker::{to_raw_value, Connection},
};
use windmill_queue::{
    add_completed_job, add_completed_job_error, get_mini_pulled_job, push, MiniCompletedJob,
    MiniPulledJob, PushArgs, PushIsolationLevel,
};

/// Context for tool execution containing all required references and state
pub struct ToolExecutionContext<'a> {
    // Database & connections
    pub db: &'a DB,
    pub conn: &'a Connection,

    // Job context
    pub job: &'a MiniPulledJob,
    pub parent_job: Option<&'a Uuid>,
    pub summary: &'a Option<&'a str>,
    pub flow_step_id_override: Option<&'a str>,

    // Execution parameters
    pub client: &'a AuthedClient,
    pub worker_dir: &'a str,
    pub base_internal_url: &'a str,
    pub worker_name: &'a str,
    pub hostname: &'a str,

    // Runtime state
    pub occupancy_metrics: &'a mut OccupancyMetrics,
    pub killpill_rx: &'a mut tokio::sync::broadcast::Receiver<()>,

    // Optional streaming & chat
    pub stream_event_processor: Option<&'a StreamEventProcessor>,
    pub flow_context: &'a mut FlowContext,
    pub previous_result: &'a Option<Box<RawValue>>,
    pub id_context: &'a Option<crate::js_eval::IdContext>,
}

/// Execute all tool calls from an AI response
pub async fn execute_tool_calls(
    mut ctx: ToolExecutionContext<'_>,
    tool_calls: &[OpenAIToolCall],
    tools: &[Tool],
    mcp_clients: &HashMap<String, Arc<McpClient>>,
    actions: &mut Vec<AgentAction>,
    final_events_str: &mut String,
    structured_output_tool_name: &Option<String>,
) -> Result<(Vec<OpenAIMessage>, Option<OpenAIContent>, bool), Error> {
    let mut messages = Vec::new();
    let mut used_structured_output_tool = false;
    let mut final_content = None;

    for tool_call in tool_calls.iter() {
        // Stream tool call progress
        if let Some(stream_event_processor) = ctx.stream_event_processor {
            let event = StreamingEvent::ToolExecution {
                call_id: tool_call.id.clone(),
                function_name: tool_call.function.name.clone(),
            };
            stream_event_processor.send(event, final_events_str).await?;
        }

        // Check if this is the structured output tool
        if structured_output_tool_name
            .as_ref()
            .map_or(false, |name| tool_call.function.name == *name)
        {
            used_structured_output_tool = true;
            messages.push(OpenAIMessage {
                role: "tool".to_string(),
                content: Some(OpenAIContent::Text(
                    "Successfully ran structured_output tool".to_string(),
                )),
                tool_call_id: Some(tool_call.id.clone()),
                ..Default::default()
            });
            messages.push(OpenAIMessage {
                role: "assistant".to_string(),
                content: Some(OpenAIContent::Text(tool_call.function.arguments.clone())),
                agent_action: Some(AgentAction::Message {}),
                ..Default::default()
            });
            final_content = Some(OpenAIContent::Text(tool_call.function.arguments.clone()));
            break;
        }

        let tool = tools
            .iter()
            .find(|t| t.def.function.name == tool_call.function.name);

        if let Some(tool) = tool {
            // Check if this is an MCP tool
            if let Some(mcp_source) = &tool.mcp_source {
                execute_mcp_tool_call(
                    &mut ctx,
                    tool_call,
                    mcp_clients,
                    mcp_source,
                    actions,
                    &mut messages,
                    final_events_str,
                )
                .await?;
            } else if tool.module.is_some() {
                execute_windmill_tool(
                    &mut ctx,
                    tool_call,
                    tool,
                    actions,
                    &mut messages,
                    final_events_str,
                )
                .await?;
            } else {
                return Err(Error::internal_err(format!(
                    "Tool type not supported: {}",
                    tool_call.function.name
                )));
            }
        } else {
            return Err(Error::internal_err(format!(
                "Tool not found: {}",
                tool_call.function.name
            )));
        }
    }

    Ok((messages, final_content, used_structured_output_tool))
}

/// Execute an MCP tool call
async fn execute_mcp_tool_call(
    ctx: &mut ToolExecutionContext<'_>,
    tool_call: &OpenAIToolCall,
    mcp_clients: &HashMap<String, Arc<McpClient>>,
    mcp_source: &McpToolSource,
    actions: &mut Vec<AgentAction>,
    messages: &mut Vec<OpenAIMessage>,
    final_events_str: &mut String,
) -> Result<(), Error> {
    let tool_result =
        execute_mcp_tool(mcp_clients, mcp_source, &tool_call.function.arguments).await;

    let call_id = ulid::Ulid::new().into();
    let resource_path = &mcp_source.resource_path;
    let tool_name = &tool_call.function.name;
    let arguments = serde_json::from_str(&tool_call.function.arguments).ok();

    actions.push(AgentAction::McpToolCall {
        call_id,
        function_name: tool_name.clone(),
        resource_path: resource_path.clone(),
        arguments: arguments.clone(),
    });

    match tool_result {
        Ok(result) => {
            let result_str =
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string());

            messages.push(OpenAIMessage {
                role: "tool".to_string(),
                content: Some(OpenAIContent::Text(result_str.clone())),
                tool_call_id: Some(tool_call.id.clone()),
                agent_action: Some(AgentAction::McpToolCall {
                    call_id,
                    function_name: tool_name.clone(),
                    resource_path: resource_path.clone(),
                    arguments: arguments.clone(),
                }),
                ..Default::default()
            });

            // Stream tool result
            if let Some(stream_event_processor) = ctx.stream_event_processor {
                let event = StreamingEvent::ToolResult {
                    call_id: tool_call.id.clone(),
                    function_name: tool_call.function.name.clone(),
                    result: result.to_string(),
                    success: true,
                };
                stream_event_processor.send(event, final_events_str).await?;
            }

            // Add tool message to conversation if chat_input_enabled
            let content = format!("Used {} tool", tool_call.function.name);
            add_tool_message_to_chat(ctx, None, &content, true).await;
        }
        Err(e) => {
            let error_msg = format!("MCP tool error: {}", e);
            tracing::error!("{}", error_msg);

            messages.push(OpenAIMessage {
                role: "tool".to_string(),
                content: Some(OpenAIContent::Text(error_msg.clone())),
                tool_call_id: Some(tool_call.id.clone()),
                agent_action: Some(AgentAction::McpToolCall {
                    call_id,
                    function_name: tool_name.clone(),
                    resource_path: resource_path.clone(),
                    arguments: arguments.clone(),
                }),
                ..Default::default()
            });

            // Stream tool error
            if let Some(stream_event_processor) = ctx.stream_event_processor {
                let event = StreamingEvent::ToolResult {
                    call_id: tool_call.id.clone(),
                    function_name: tool_name.clone(),
                    result: error_msg.clone(),
                    success: false,
                };
                stream_event_processor.send(event, final_events_str).await?;
            }

            // Add tool message to conversation if chat_input_enabled
            add_tool_message_to_chat(ctx, None, &error_msg, false).await;
        }
    }

    Ok(())
}

/// Execute a Windmill tool (script or flow)
async fn execute_windmill_tool(
    ctx: &mut ToolExecutionContext<'_>,
    tool_call: &OpenAIToolCall,
    tool: &Tool,
    actions: &mut Vec<AgentAction>,
    messages: &mut Vec<OpenAIMessage>,
    final_events_str: &mut String,
) -> Result<(), Error> {
    // Regular Windmill tools must have a module
    let tool_module = tool.module.as_ref().ok_or_else(|| {
        Error::internal_err(format!("Tool {} has no module", tool_call.function.name))
    })?;

    let job_id = ulid::Ulid::new().into();
    actions.push(AgentAction::ToolCall {
        job_id,
        function_name: tool_call.function.name.clone(),
        module_id: tool_module.id.clone(),
    });

    if let Some(parent_job) = ctx.parent_job {
        update_flow_status_module_with_actions(ctx.db, parent_job, actions).await?;
    }

    let raw_tool_call_args = if tool_call.function.arguments.is_empty() {
        "{}".to_string()
    } else {
        tool_call.function.arguments.clone()
    };

    let mut tool_call_args = serde_json::from_str::<HashMap<String, Box<RawValue>>>(
        &raw_tool_call_args,
    )
    .with_context(|| {
        format!(
            "Failed to parse tool call arguments for tool call {}: {}",
            tool_call.function.name, tool_call.function.arguments
        )
    })?;

    let tool_value = tool_module.get_value()?;

    // Get input transforms given by the user and merge them with AI given args
    let input_transforms = match &tool_value {
        FlowModuleValue::Script { input_transforms, .. }
        | FlowModuleValue::RawScript { input_transforms, .. }
        | FlowModuleValue::FlowScript { input_transforms, .. }
        | FlowModuleValue::AIAgent { input_transforms, .. } => input_transforms,
        _ => {
            return Err(Error::internal_err(format!(
                "Unsupported tool: {}",
                tool_call.function.name
            )));
        }
    };

    // Prepare context for transform evaluation
    let last_result = Arc::new(
        ctx.previous_result
            .as_ref()
            .cloned()
            .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null)),
    );

    let flow_inputs = ctx
        .flow_context
        .flow_inputs
        .as_ref()
        .map(|args| Marc::new(args.clone()));

    // Evaluate each input transform and merge with AI-provided args
    for (key, transform) in input_transforms.iter() {
        // We skip static empty / null values, those are the one the AI will fill in
        if !is_completed_input_transform(transform) {
            continue;
        }
        let result = evaluate_input_transform::<Box<RawValue>>(
            transform,
            last_result.clone(),
            flow_inputs.clone(),
            None,
            Some(ctx.client),
            ctx.id_context.as_ref(),
        )
        .await?;

        tool_call_args.insert(key.clone(), result);
    }

    let job_payload = match tool_value {
        FlowModuleValue::Script { path: script_path, hash: script_hash, tag_override, .. } => {
            script_to_payload(
                script_hash,
                script_path,
                ctx.db,
                ctx.job,
                tool_module,
                tag_override,
                tool_module.apply_preprocessor,
            )
            .await?
        }
        FlowModuleValue::RawScript {
            path,
            content,
            language,
            lock,
            tag,
            concurrency_settings,
            ..
        } => {
            let path = path
                .unwrap_or_else(|| format!("{}/tools/{}", ctx.job.runnable_path(), tool_module.id));
            raw_script_to_payload(
                path,
                content,
                language,
                lock,
                concurrency_settings,
                tool_module,
                tag,
                tool_module.delete_after_use.unwrap_or(false),
            )
        }
        FlowModuleValue::FlowScript { id, language, concurrency_settings, tag, .. } => {
            let path = format!("{}/tools/{}", ctx.job.runnable_path(), tool_module.id);
            JobPayloadWithTag {
                payload: JobPayload::FlowScript {
                    id,
                    language,
                    concurrency_settings: concurrency_settings.into(),
                    cache_ttl: tool_module.cache_ttl.map(|x| x as i32),
                    cache_ignore_s3_path: tool_module.cache_ignore_s3_path.clone(),
                    dedicated_worker: None,
                    path,
                },
                tag: tag.clone(),
                delete_after_use: tool_module.delete_after_use.unwrap_or(false),
                timeout: None,
                on_behalf_of: None,
            }
        }
        FlowModuleValue::AIAgent { tools: sub_tools, .. } => {
            let has_nested_agent_tools = sub_tools.iter().any(|t| {
                matches!(
                    t.value,
                    windmill_common::flows::ToolValue::FlowModule(FlowModuleValue::AIAgent { .. })
                )
            });
            if has_nested_agent_tools {
                return Err(Error::internal_err(
                    "AI agent tools cannot be nested beyond 2 levels. The nested agent tool contains \
                     AIAgent sub-tools, which would exceed the maximum nesting depth.".to_string()
                ));
            }
            let path = format!("{}/tools/{}", ctx.job.runnable_path(), tool_module.id);
            JobPayloadWithTag {
                payload: JobPayload::AIAgent { path },
                tag: None,
                delete_after_use: tool_module.delete_after_use.unwrap_or(false),
                timeout: None,
                on_behalf_of: None,
            }
        }
        _ => {
            return Err(Error::internal_err(format!(
                "Unsupported tool: {}",
                tool_call.function.name
            )));
        }
    };

    let mut tx = ctx.db.begin().await?;

    let job_perms =
        windmill_common::auth::get_job_perms(&mut *tx, &ctx.job.id, &ctx.job.workspace_id)
            .await?
            .map(|x| x.into());

    let (email, permissioned_as) = if let Some(on_behalf_of) = job_payload.on_behalf_of.as_ref() {
        (&on_behalf_of.email, on_behalf_of.permissioned_as.clone())
    } else {
        (
            &ctx.job.permissioned_as_email,
            ctx.job.permissioned_as.to_owned(),
        )
    };

    let job_priority = tool_module.priority.or(ctx.job.priority);

    let tx = PushIsolationLevel::Transaction(tx);
    let (uuid, tx) = push(
        ctx.db,
        tx,
        &ctx.job.workspace_id,
        job_payload.payload,
        PushArgs { args: &tool_call_args, extra: None },
        &ctx.job.created_by,
        email,
        permissioned_as,
        Some(&format!("job-span-{}", ctx.job.id)),
        None,
        ctx.job.schedule_path(),
        Some(ctx.job.id),
        ctx.job.root_job.or(Some(ctx.job.id)),
        ctx.job.flow_innermost_root_job.or(Some(ctx.job.id)),
        Some(job_id),
        false,
        false,
        None,
        ctx.job.visible_to_owner,
        Some(ctx.job.tag.clone()),
        job_payload.timeout,
        None,
        job_priority,
        job_perms.as_ref(),
        true,
        None,
        None,
        None,
    )
    .await?;

    tx.commit().await?;

    let tool_job = get_mini_pulled_job(ctx.db, &uuid).await?;

    let Some(tool_job) = tool_job else {
        return Err(Error::internal_err("Tool job not found".to_string()));
    };

    let tool_job = Arc::new(tool_job);

    let (inner_job_completed_tx, inner_job_completed_rx) = JobCompletedSender::new(ctx.conn, 1);

    let inner_job_completed_rx = inner_job_completed_rx.expect(
        "inner_job_completed_tx should be set as agent jobs are not supported on agent workers",
    );

    // Spawn handle_queued_job on separate task to prevent tokio stack overflow
    // Clone everything needed for the spawned task
    let tool_job_spawn = tool_job.clone();
    let conn_spawn = ctx.conn.clone();
    let client_spawn = ctx.client.clone();
    let hostname_spawn = ctx.hostname.to_string();
    let worker_name_spawn = ctx.worker_name.to_string();
    let worker_dir_spawn = ctx.worker_dir.to_string();
    let base_internal_url_spawn = ctx.base_internal_url.to_string();
    let inner_job_completed_tx_spawn = inner_job_completed_tx.clone();
    let mut occupancy_metrics_spawn = ctx.occupancy_metrics.clone();
    let mut killpill_rx_spawn = ctx.killpill_rx.resubscribe();

    // Spawn on separate tokio task with fresh stack
    let join_handle = tokio::task::spawn(async move {
        #[cfg(feature = "benchmark")]
        let mut bench_spawn = windmill_common::bench::BenchmarkIter::new();

        let job_dir = create_job_dir(&worker_dir_spawn, tool_job_spawn.id).await;

        let result = handle_queued_job(
            tool_job_spawn,
            None,
            None,
            None,
            None,
            &conn_spawn,
            &client_spawn,
            &hostname_spawn,
            &worker_name_spawn,
            &worker_dir_spawn,
            &job_dir,
            None,
            &base_internal_url_spawn,
            inner_job_completed_tx_spawn,
            &mut occupancy_metrics_spawn,
            &mut killpill_rx_spawn,
            None,
            None,
            #[cfg(feature = "benchmark")]
            &mut bench_spawn,
        )
        .await;

        // Return both result and updated metrics
        (result, occupancy_metrics_spawn)
    });

    // Await the spawned task
    let (handle_result, updated_occupancy) = join_handle
        .await
        .map_err(|e| Error::internal_err(format!("Tool execution task failed: {}", e)))?;

    // Merge occupancy metrics back
    ctx.occupancy_metrics.total_duration_of_running_jobs =
        updated_occupancy.total_duration_of_running_jobs;

    match handle_result {
        Err(err) => {
            handle_tool_execution_error(
                ctx,
                tool_call,
                tool_module,
                &MiniCompletedJob::from(tool_job),
                job_id,
                err,
                messages,
                final_events_str,
            )
            .await?;
        }
        Ok(success) => {
            handle_tool_execution_success(
                ctx,
                tool_call,
                tool_module,
                job_id,
                success,
                inner_job_completed_rx,
                messages,
                final_events_str,
            )
            .await?;
        }
    }

    Ok(())
}

/// Handle tool execution error
async fn handle_tool_execution_error(
    ctx: &mut ToolExecutionContext<'_>,
    tool_call: &OpenAIToolCall,
    tool_module: &windmill_common::flows::FlowModule,
    tool_job: &MiniCompletedJob,
    job_id: Uuid,
    err: Error,
    messages: &mut Vec<OpenAIMessage>,
    final_events_str: &mut String,
) -> Result<(), Error> {
    let err_string = format!("{}: {}", err.name(), err.to_string());
    let err_json = windmill_common::worker::error_to_value(&err);
    let _ = handle_non_flow_job_error(
        ctx.db,
        tool_job,
        0,
        None,
        err_string.clone(),
        err_json,
        ctx.worker_name,
    )
    .await;

    let error_message = format!("Error running tool: {}", err_string);
    messages.push(OpenAIMessage {
        role: "tool".to_string(),
        content: Some(OpenAIContent::Text(error_message.clone())),
        tool_call_id: Some(tool_call.id.clone()),
        agent_action: Some(AgentAction::ToolCall {
            job_id,
            function_name: tool_call.function.name.clone(),
            module_id: tool_module.id.clone(),
        }),
        ..Default::default()
    });

    // Stream tool result (error case)
    if let Some(stream_event_processor) = ctx.stream_event_processor {
        let tool_result_event = StreamingEvent::ToolResult {
            call_id: tool_call.id.clone(),
            function_name: tool_call.function.name.clone(),
            result: error_message.clone(),
            success: false,
        };
        stream_event_processor
            .send(tool_result_event, final_events_str)
            .await?;
    }

    if let Some(parent_job) = ctx.parent_job {
        update_flow_status_module_with_actions_success(ctx.db, parent_job, false).await?;
    }

    // Add tool message to conversation if chat_input_enabled (error case)
    add_tool_message_to_chat(ctx, Some(job_id), &error_message, false).await;

    Ok(())
}

/// Handle tool execution success
async fn handle_tool_execution_success(
    ctx: &mut ToolExecutionContext<'_>,
    tool_call: &OpenAIToolCall,
    tool_module: &windmill_common::flows::FlowModule,
    job_id: Uuid,
    success: bool,
    inner_job_completed_rx: JobCompletedReceiver,
    messages: &mut Vec<OpenAIMessage>,
    final_events_str: &mut String,
) -> Result<(), Error> {
    let send_result = inner_job_completed_rx.bounded_rx.try_recv().ok();

    let result = if let Some(SendResult {
        result: SendResultPayload::JobCompleted(ref jc), ..
    }) = send_result
    {
        let result = jc.result.clone();
        // Write tool completion to the DB inline instead of forwarding through
        // the parent channel. Forwarding would deadlock for nested agents: the
        // sub-tool result would fill the parent's bounded(1) channel, leaving
        // no room for the agent's own completion from process_result.
        if jc.success {
            add_completed_job(
                ctx.db,
                &jc.job,
                true,
                false,
                sqlx::types::Json(&*jc.result),
                jc.result_columns.clone(),
                jc.mem_peak,
                jc.canceled_by.clone(),
                false,
                jc.duration,
                jc.from_cache.unwrap_or(false),
            )
            .await
            .map_err(|e| Error::internal_err(format!("Failed to add completed job: {e}")))?;
        } else {
            let error_value: serde_json::Value =
                serde_json::from_str(jc.result.get()).unwrap_or_else(|_| {
                    serde_json::json!({ "message": format!("Non serializable error: {}", jc.result.get()) })
                });
            add_completed_job_error(
                ctx.db,
                &jc.job,
                jc.mem_peak,
                jc.canceled_by.clone(),
                error_value,
                ctx.worker_name,
                false,
                jc.duration,
            )
            .await
            .map_err(|e| Error::internal_err(format!("Failed to add completed job error: {e}")))?;
        }
        result
    } else {
        return Err(Error::internal_err(
            "Tool job completed but no result".to_string(),
        ));
    };

    messages.push(OpenAIMessage {
        role: "tool".to_string(),
        content: Some(OpenAIContent::Text(result.get().to_string())),
        tool_call_id: Some(tool_call.id.clone()),
        agent_action: Some(AgentAction::ToolCall {
            job_id,
            function_name: tool_call.function.name.clone(),
            module_id: tool_module.id.clone(),
        }),
        ..Default::default()
    });

    // Stream tool result (success case)
    if let Some(stream_event_processor) = ctx.stream_event_processor {
        let tool_result_event = StreamingEvent::ToolResult {
            call_id: tool_call.id.clone(),
            function_name: tool_call.function.name.clone(),
            result: result.get().to_string(),
            success: true,
        };
        stream_event_processor
            .send(tool_result_event, final_events_str)
            .await?;
    }

    if let Some(parent_job) = ctx.parent_job {
        update_flow_status_module_with_actions_success(ctx.db, parent_job, success).await?;
    }

    // Add tool message to conversation if chat_input_enabled
    let content = if success {
        format!("Used {} tool", tool_call.function.name)
    } else {
        format!("Error executing {}", tool_call.function.name)
    };

    add_tool_message_to_chat(ctx, Some(job_id), &content, success).await;

    Ok(())
}

/// Add tool message to conversation if chat is enabled
async fn add_tool_message_to_chat(
    ctx: &mut ToolExecutionContext<'_>,
    tool_job_id: Option<Uuid>,
    content: &str,
    success: bool,
) {
    let chat_enabled = ctx
        .flow_context
        .flow_status
        .as_ref()
        .and_then(|fs| fs.chat_input_enabled)
        .unwrap_or(false);
    if chat_enabled {
        if let Some(memory_id) = ctx
            .flow_context
            .flow_status
            .as_ref()
            .and_then(|fs| fs.memory_id)
        {
            let db_clone = ctx.db.clone();
            let effective_step_id = ctx
                .flow_step_id_override
                .or(ctx.job.flow_step_id.as_deref());
            let step_name = get_step_name_from_flow(ctx.summary.as_deref(), effective_step_id);
            let content = content.to_string();

            // Spawn task because we do not need to wait for the result
            tokio::spawn(async move {
                if let Err(e) = add_message_to_conversation(
                    &db_clone,
                    &memory_id,
                    tool_job_id,
                    &content,
                    MessageType::Tool,
                    &step_name,
                    success,
                )
                .await
                {
                    tracing::warn!(
                        "Failed to add tool message to conversation {}: {}",
                        memory_id,
                        e
                    );
                }
            });
        }
    }
}
