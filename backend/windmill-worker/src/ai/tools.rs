use crate::ai::stream_event_processor::StreamEventProcessor;
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
use crate::{create_job_dir, handle_queued_job, JobCompletedSender};
use anyhow::Context;
use mappable_rc::Marc;
use serde_json::value::RawValue;
use sqlx::types::Json;
use std::{collections::HashMap, sync::Arc, time::Duration};
use uuid::Uuid;
use windmill_ai::{ai_types::OpenAIToolCall, query_builder::StreamEventSink, types::*};
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
    flow_conversations::{MessageExtras, MessageType},
    flow_status::AgentAction,
    flows::FlowModuleValue,
    worker::{make_tool_job_pull_query, to_raw_value, Connection},
};
use windmill_queue::{
    get_mini_pulled_job, pull, push, try_admit_owned_job, MiniCompletedJob, MiniPulledJob,
    PushArgs, PushIsolationLevel,
};

/// Shared collection of abort handles for spawned tool tasks.
/// Used to abort in-flight tasks when the parent agent is force-cancelled.
pub type ToolAbortHandles = Arc<std::sync::Mutex<Vec<tokio::task::AbortHandle>>>;

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
    pub omit_output_from_conversation: bool,
    /// The thinking that led to this round's calls, stored on the first tool row written.
    /// None when the round wrote text, whose row carries it.
    pub reasoning: Option<String>,
    pub previous_result: &'a Option<Box<RawValue>>,
    pub id_context: &'a Option<crate::js_eval::IdContext>,

    // Abort handles for spawned tool tasks (used for force-cancel cleanup)
    pub tool_abort_handles: ToolAbortHandles,
    pub job_completed_tx: JobCompletedSender,
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

    let mut calls = tool_calls.iter().peekable();
    while let Some(tool_call) = calls.next() {
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
                let mut batch = vec![(tool_call, tool)];
                while let Some(next_call) = calls.peek() {
                    if structured_output_tool_name.as_deref()
                        == Some(next_call.function.name.as_str())
                    {
                        break;
                    }
                    let Some(next_tool) = tools.iter().find(|t| {
                        t.def.function.name == next_call.function.name
                            && t.mcp_source.is_none()
                            && t.module.is_some()
                    }) else {
                        break;
                    };
                    batch.push((calls.next().unwrap(), next_tool));
                }
                execute_windmill_tools(&mut ctx, &batch, actions, &mut messages, final_events_str)
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

    if let Some(parent_job) = ctx.parent_job {
        update_flow_status_module_with_actions(ctx.db, parent_job, actions).await?;
    }

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

            if let Some(parent_job) = ctx.parent_job {
                update_flow_status_module_with_actions_success(ctx.db, parent_job, true).await?;
            }

            // An MCP tool runs inside the agent's job, whose result holds every call of the
            // turn and nothing tying one of them to this row: same job id for all of them,
            // no call id on the row. Kept here so the card shows this call — and the row
            // names that job, so retention sweeps it with every other row of the turn.
            let content = format!("Used {} tool", tool_call.function.name);
            let agent_job_id = ctx.job.id;
            add_tool_message_to_chat(
                ctx,
                Some(agent_job_id),
                &content,
                true,
                Some(MessageExtras {
                    tool_arguments: Some(tool_call.function.arguments.clone()),
                    tool_result: Some(result_str),
                    ..Default::default()
                }),
            )
            .await;
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

            if let Some(parent_job) = ctx.parent_job {
                update_flow_status_module_with_actions_success(ctx.db, parent_job, false).await?;
            }

            // Add tool message to conversation if chat_input_enabled. The row is worded from
            // the tool, like every other tool row, and the error it failed with is its result
            // — the one field a call that produced nothing else still has something to put in.
            let agent_job_id = ctx.job.id;
            let content = format!("Error executing {}", tool_name);
            add_tool_message_to_chat(
                ctx,
                Some(agent_job_id),
                &content,
                false,
                Some(MessageExtras {
                    tool_arguments: Some(tool_call.function.arguments.clone()),
                    tool_result: Some(error_msg.clone()),
                    ..Default::default()
                }),
            )
            .await;
        }
    }

    Ok(())
}

/// Execute a Windmill tool (script or flow)
async fn enqueue_windmill_tool(
    ctx: &ToolExecutionContext<'_>,
    tool_call: &OpenAIToolCall,
    tool: &Tool,
    actions: &mut Vec<AgentAction>,
    reserved: bool,
) -> Result<Uuid, Error> {
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
                None,
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
                delete_after_secs: None,
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
                delete_after_secs: None,
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
        job_payload.tag,
        job_payload.timeout,
        None,
        job_priority,
        job_perms.as_ref(),
        reserved,
        None,
        None,
        None,
    )
    .await?;

    let mut tx = tx;
    if reserved {
        // Running ownership reserves the first child; its normal tag allows zombie recovery.
        sqlx::query!(
            "UPDATE v2_job_queue SET worker = $1 WHERE id = $2",
            ctx.worker_name,
            uuid,
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!("UPDATE v2_job_runtime SET ping = now() WHERE id = $1", uuid)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(uuid)
}

fn spawn_local_tool(
    ctx: &ToolExecutionContext<'_>,
    tool_job: MiniPulledJob,
    reserved: bool,
) -> tokio::task::JoinHandle<Result<OccupancyMetrics, Error>> {
    let mut tool_job = Arc::new(tool_job);

    // Spawn handle_queued_job on separate task to prevent tokio stack overflow
    // Clone everything needed for the spawned task
    let db = ctx.db.clone();
    let conn_spawn = ctx.conn.clone();
    let hostname_spawn = ctx.hostname.to_string();
    let worker_name_spawn = ctx.worker_name.to_string();
    let worker_dir_spawn = ctx.worker_dir.to_string();
    let base_internal_url_spawn = ctx.base_internal_url.to_string();
    let job_completed_tx = ctx.job_completed_tx.clone();
    let mut occupancy_metrics_spawn = ctx.occupancy_metrics.clone();
    let mut killpill_rx_spawn = ctx.killpill_rx.resubscribe();

    // Spawn on separate tokio task with fresh stack
    let join_handle = tokio::task::spawn(async move {
        #[cfg(feature = "benchmark")]
        let mut bench_spawn = windmill_common::bench::BenchmarkIter::new();

        let result = async {
            if reserved {
                loop {
                    let queued = get_mini_pulled_job(&db, &tool_job.id)
                        .await?
                        .ok_or_else(|| Error::AlreadyCompleted("Tool job already completed".to_string()))?;
                    tool_job = Arc::new(queued);
                    sqlx::query!(
                        "UPDATE v2_job_runtime SET ping = now() WHERE id = $1",
                        tool_job.id,
                    )
                    .execute(&db)
                    .await?;
                    if try_admit_owned_job(&db, &tool_job).await? {
                        let started_at = sqlx::query_scalar!(
                            "UPDATE v2_job_queue SET started_at = now() WHERE id = $1 RETURNING started_at",
                            tool_job.id,
                        )
                        .fetch_optional(&db)
                        .await?
                        .flatten();
                        Arc::make_mut(&mut tool_job).started_at = started_at;
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
            let perms =
                windmill_common::auth::get_job_perms(&db, &tool_job.id, &tool_job.workspace_id).await?;
            let token = windmill_queue::create_token(&db, &tool_job, perms).await;
            let client_spawn = AuthedClient::new(
                base_internal_url_spawn.clone(),
                tool_job.workspace_id.clone(),
                token,
                None,
            );
            let job_dir = create_job_dir(&worker_dir_spawn, tool_job.id).await;

            handle_queued_job(
                tool_job.clone(),
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
                job_completed_tx,
                &mut occupancy_metrics_spawn,
                &mut killpill_rx_spawn,
                None,
                None,
                #[cfg(feature = "benchmark")]
                &mut bench_spawn,
            )
            .await
        }
        .await;

        match result {
            Err(err) => {
                let err_string = format!("{}: {}", err.name(), err);
                handle_non_flow_job_error(
                    &db,
                    &MiniCompletedJob::from(tool_job),
                    0,
                    None,
                    err_string,
                    windmill_common::worker::error_to_value(&err),
                    &worker_name_spawn,
                )
                .await?;
            }
            Ok(_) => {}
        }
        Ok(occupancy_metrics_spawn)
    });

    // Register abort handle so the task can be killed on force-cancel
    let abort_handle = join_handle.abort_handle();
    // unwrap safe: lock is only held briefly for push/drain, no panic possible inside
    ctx.tool_abort_handles.lock().unwrap().push(abort_handle);
    join_handle
}

async fn execute_windmill_tools(
    ctx: &mut ToolExecutionContext<'_>,
    batch: &[(&OpenAIToolCall, &Tool)],
    actions: &mut Vec<AgentAction>,
    messages: &mut Vec<OpenAIMessage>,
    final_events_str: &mut String,
) -> Result<(), Error> {
    let mut job_ids = Vec::with_capacity(batch.len());
    let mut local = None;
    for (index, (call, tool)) in batch.iter().enumerate() {
        if index > 0 {
            if let Some(processor) = ctx.stream_event_processor {
                processor
                    .send(
                        StreamingEvent::ToolExecution {
                            call_id: call.id.clone(),
                            function_name: call.function.name.clone(),
                        },
                        final_events_str,
                    )
                    .await?;
            }
        }
        job_ids.push(enqueue_windmill_tool(ctx, call, tool, actions, index == 0).await?);
        if index == 0 {
            let job = get_mini_pulled_job(ctx.db, &job_ids[0])
                .await?
                .ok_or_else(|| Error::internal_err("Reserved tool job not found".to_string()))?;
            local = Some(spawn_local_tool(ctx, job, true));
        }
    }

    let mut results: HashMap<Uuid, (bool, String)> = HashMap::new();
    let mut next_result = 0;
    let mut poll = tokio::time::interval(Duration::from_millis(100));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    while next_result < batch.len() || local.is_some() {
        tokio::select! {
            result = async { local.as_mut().unwrap().await }, if local.is_some() => {
                local = None;
                let metrics = result.map_err(|e| Error::internal_err(format!("Tool task failed: {e}")))??;
                ctx.occupancy_metrics.total_duration_of_running_jobs = metrics.total_duration_of_running_jobs;
                ctx.tool_abort_handles.lock().unwrap().retain(|handle| !handle.is_finished());
            }
            _ = poll.tick() => {}
        }

        let pending: Vec<Uuid> = job_ids[next_result..]
            .iter()
            .filter(|id| !results.contains_key(id))
            .copied()
            .collect();
        if !pending.is_empty() {
            let completed = sqlx::query!(
                "SELECT id, status = 'success' AS \"success!\", result AS \"result: Json<Box<RawValue>>\"
                FROM v2_job_completed WHERE workspace_id = $1 AND id = ANY($2)",
                ctx.job.workspace_id,
                &pending,
            ).fetch_all(ctx.db).await?;
            for completed in completed {
                let index = job_ids
                    .iter()
                    .position(|id| *id == completed.id)
                    .ok_or_else(|| Error::internal_err("Unexpected tool completion".to_string()))?;
                let (call, tool) = batch[index];
                let result = completed
                    .result
                    .map(|value| value.0)
                    .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null));
                let is_agent = tool.module.as_ref().is_some_and(|module| {
                    matches!(module.get_value(), Ok(FlowModuleValue::AIAgent { .. }))
                });
                let content = if is_agent && completed.success {
                    extract_ai_agent_output(&result).unwrap_or_else(|| result.get().to_string())
                } else {
                    result.get().to_string()
                };
                if let Some(processor) = ctx.stream_event_processor {
                    processor
                        .send(
                            StreamingEvent::ToolResult {
                                call_id: call.id.clone(),
                                function_name: call.function.name.clone(),
                                result: content.clone(),
                                success: completed.success,
                            },
                            final_events_str,
                        )
                        .await?;
                }
                results.insert(completed.id, (completed.success, content));
            }
        }

        // Transcript rows, model messages, and positional action statuses share call order.
        while next_result < batch.len() {
            let job_id = job_ids[next_result];
            let Some((success, content)) = results.remove(&job_id) else {
                break;
            };
            let (call, tool) = batch[next_result];
            let module = tool
                .module
                .as_ref()
                .ok_or_else(|| Error::internal_err("Windmill tool has no module".to_string()))?;
            messages.push(OpenAIMessage {
                role: "tool".to_string(),
                content: Some(OpenAIContent::Text(content.clone())),
                tool_call_id: Some(call.id.clone()),
                agent_action: Some(AgentAction::ToolCall {
                    job_id,
                    function_name: call.function.name.clone(),
                    module_id: module.id.clone(),
                }),
                ..Default::default()
            });
            if let Some(parent) = ctx.parent_job {
                update_flow_status_module_with_actions_success(ctx.db, parent, success).await?;
            }
            let (content, extras) = windmill_tool_row(call, success, &content);
            add_tool_message_to_chat(ctx, Some(job_id), &content, success, Some(extras)).await;
            next_result += 1;
        }

        if local.is_none() && next_result < batch.len() {
            let pending: Vec<Uuid> = job_ids[next_result..]
                .iter()
                .filter(|id| !results.contains_key(id))
                .copied()
                .collect();
            if !pending.is_empty() {
                local = claim_local_tool(ctx, &pending).await?;
            }
        }
    }
    Ok(())
}

async fn claim_local_tool(
    ctx: &ToolExecutionContext<'_>,
    pending: &[Uuid],
) -> Result<Option<tokio::task::JoinHandle<Result<OccupancyMetrics, Error>>>, Error> {
    let query = (String::new(), make_tool_job_pull_query(pending));
    #[cfg(feature = "benchmark")]
    let mut bench = windmill_common::bench::BenchmarkIter::new();
    let mut pulled = pull(
        ctx.db,
        false,
        ctx.worker_name,
        Some(&query),
        #[cfg(feature = "benchmark")]
        &mut bench,
    )
    .await?;
    if let Err(err) = pulled.maybe_apply_debouncing(ctx.db).await {
        pulled.error_while_preprocessing = Some(err.to_string());
    }
    match pulled.to_pulled_job() {
        Ok(job) => Ok(job.map(|job| spawn_local_tool(ctx, job.job, false))),
        Err(
            windmill_queue::PulledJobResultToJobErr::MissingConcurrencyKey(job)
            | windmill_queue::PulledJobResultToJobErr::ErrorWhilePreprocessing(job),
        ) => {
            ctx.job_completed_tx.send_job(job, true).await?;
            Ok(None)
        }
    }
}

/// Extract the `output` field of an `AIAgentResult` envelope, serialized back to JSON.
/// Returns `None` if the result is not a JSON object carrying an `output` field.
fn extract_ai_agent_output(result: &RawValue) -> Option<String> {
    serde_json::from_str::<HashMap<String, &RawValue>>(result.get())
        .ok()?
        .get("output")
        .map(|output| output.get().to_string())
}

/// A Windmill tool's conversation row: worded from the tool, carrying the model's call and
/// the exact text the model got back, the same text agent memory keeps for that tool
/// message, so a card needs no job fetch. The call is the model's arguments, not the job's
/// args: the step's input transforms add inputs the model never wrote.
fn windmill_tool_row(
    tool_call: &OpenAIToolCall,
    success: bool,
    sent_to_model: &str,
) -> (String, MessageExtras) {
    let content = if success {
        format!("Used {} tool", tool_call.function.name)
    } else {
        format!("Error executing {}", tool_call.function.name)
    };
    let extras = MessageExtras {
        tool_arguments: Some(tool_call.function.arguments.clone()),
        tool_result: Some(sent_to_model.to_string()),
        ..Default::default()
    };
    (content, extras)
}

/// Add tool message to conversation if chat is enabled
async fn add_tool_message_to_chat(
    ctx: &mut ToolExecutionContext<'_>,
    // The job this row belongs to: the tool's own where it has one, else the agent's, which
    // is the job it ran inside. Every row names one so that retention collects the whole
    // turn — `delete_jobs` removes messages by `job_id = ANY(..)` (there is no FK on the
    // column; `drop_v2_job_side_table_cascades` dropped it), and a row naming no job would
    // survive every purge and leave a conversation that can never become empty.
    tool_job_id: Option<Uuid>,
    content: &str,
    success: bool,
    // The model's call and what it got back; every tool row carries both.
    extras: Option<MessageExtras>,
) {
    if ctx.omit_output_from_conversation {
        return;
    }
    let extras = match ctx.reasoning.take() {
        Some(reasoning) => {
            Some(MessageExtras { reasoning: Some(reasoning), ..extras.unwrap_or_default() })
        }
        None => extras,
    };

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
            let effective_step_id = ctx
                .flow_step_id_override
                .or(ctx.job.flow_step_id.as_deref());
            let step_name = get_step_name_from_flow(ctx.summary.as_deref(), effective_step_id);

            // created_seq defines transcript order, so these writes must stay sequential.
            if let Err(e) = add_message_to_conversation(
                ctx.db,
                &memory_id,
                tool_job_id,
                content,
                MessageType::Tool,
                &step_name,
                success,
                extras.as_ref(),
            )
            .await
            {
                tracing::warn!(
                    "Failed to add tool message to conversation {}: {}",
                    memory_id,
                    e
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_ai_agent_output, windmill_tool_row};
    use serde_json::value::RawValue;
    use windmill_ai::ai_types::{OpenAIFunction, OpenAIToolCall};

    #[test]
    fn a_windmill_tool_row_carries_the_models_call_and_what_it_got_back() {
        let tool_call = OpenAIToolCall {
            id: "call_1".to_string(),
            function: OpenAIFunction {
                name: "get_price".to_string(),
                arguments: r#"{"item":"widget"}"#.to_string(),
            },
            r#type: "function".to_string(),
            extra_content: None,
        };

        let (content, extras) = windmill_tool_row(&tool_call, true, r#"{"price":42}"#);
        assert_eq!(content, "Used get_price tool");
        assert_eq!(
            extras.tool_arguments.as_deref(),
            Some(r#"{"item":"widget"}"#)
        );
        assert_eq!(extras.tool_result.as_deref(), Some(r#"{"price":42}"#));

        let (content, extras) =
            windmill_tool_row(&tool_call, false, "Error running tool: ExecutionErr: boom");
        assert_eq!(content, "Error executing get_price");
        assert_eq!(
            extras.tool_arguments.as_deref(),
            Some(r#"{"item":"widget"}"#)
        );
        assert_eq!(
            extras.tool_result.as_deref(),
            Some("Error running tool: ExecutionErr: boom")
        );
    }

    #[test]
    fn extracts_only_the_output_of_an_agent_result() {
        let envelope = RawValue::from_string(
            r#"{"output":{"answer":"42"},"messages":[{"role":"user","content":"hi"}],"wm_stream":"...","usage":{"total_tokens":10}}"#
                .to_string(),
        )
        .unwrap();
        assert_eq!(
            extract_ai_agent_output(&envelope).as_deref(),
            Some(r#"{"answer":"42"}"#)
        );

        let not_an_envelope = RawValue::from_string(r#"["a"]"#.to_string()).unwrap();
        assert_eq!(extract_ai_agent_output(&not_an_envelope), None);
    }
}
