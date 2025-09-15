use async_recursion::async_recursion;
use regex::Regex;
use serde_json::value::RawValue;
#[cfg(feature = "benchmark")]
use windmill_common::bench::BenchmarkIter;
use windmill_common::{
    cache,
    client::AuthedClient,
    db::DB,
    error::{self, Error},
    flows::FlowModuleValue,
    get_latest_hash_for_path,
    jobs::JobKind,
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    utils::{StripPath, HTTP_CLIENT},
    worker::{to_raw_value, Connection},
};
use windmill_queue::{CanceledBy, MiniPulledJob};

use crate::{
    ai::types::*,
    common::{build_args_map, OccupancyMetrics},
    handle_child::run_future_with_polling_update_job_poller,
    parse_sig_of_lang, JobCompletedSender,
};

use async_recursion::async_recursion;
use serde_json::value::RawValue;
use std::{collections::HashMap, sync::Arc};
use ulid;
use uuid::Uuid;
use windmill_common::{
    ai_providers::AIProvider,
    client::AuthedClient,
    db::DB,
    error::{self, to_anyhow, Error},
    flow_status::AgentAction,
    flows::Step,
    s3_helpers::S3Object,
    utils::HTTP_CLIENT,
    worker::{to_raw_value, Connection},
};
use windmill_queue::{
    flow_status::get_step_of_flow_status, get_mini_pulled_job, push, JobCompleted, MiniPulledJob,
    PushArgs, PushIsolationLevel,
};

use crate::{
    ai::{
        image_handler::upload_image_to_s3,
        query_builder::{create_query_builder, BuildRequestArgs, ParsedResponse},
        types::*,
    },
    common::{error_to_value, OccupancyMetrics},
    create_job_dir, handle_queued_job,
    result_processor::handle_non_flow_job_error,
    worker_flow::{raw_script_to_payload, script_to_payload},
    JobCompletedSender, SendResult, SendResultPayload,
};

lazy_static::lazy_static! {
    static ref TOOL_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
}

const MAX_AGENT_ITERATIONS: usize = 10;

fn parse_raw_script_schema(content: &str, language: &ScriptLang) -> Result<Box<RawValue>, Error> {
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

    let tools = futures::future::try_join_all(tools.into_iter().map(|mut t| {
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
                            let hash = get_latest_hash_for_path(db, &job.workspace_id, path, true)
                                .await?
                                .0;
                            // update module definition to use a fixed hash so all tool calls match the same schema
                            t.value = to_raw_value(&FlowModuleValue::Script {
                                hash: Some(hash),
                                path: path.clone(),
                                tag_override: tag_override.clone(),
                                input_transforms: input_transforms.clone(),
                                is_trigger: *is_trigger,
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
                module: t,
            })
        }
    }))
    .await?;

    let mut inner_occupancy_metrics = occupancy_metrics.clone();

    let agent_fut = run_agent(
        db,
        conn,
        job,
        parent_job,
        &args,
        &tools,
        client,
        &mut inner_occupancy_metrics,
        job_completed_tx,
        worker_dir,
        base_internal_url,
        worker_name,
        hostname,
        killpill_rx,
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

    Ok(result)
}

/// Find a unique tool name to avoid collisions with user-provided tools
fn find_unique_tool_name(base_name: &str, existing_tools: Option<&[ToolDef]>) -> String {
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

async fn update_flow_status_module_with_actions(
    db: &DB,
    parent_job: &Uuid,
    actions: &[AgentAction],
) -> Result<(), Error> {
    let step = get_step_of_flow_status(db, parent_job.to_owned()).await?;
    match step {
        Step::Step(step) => {
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

async fn update_flow_status_module_with_actions_success(
    db: &DB,
    parent_job: &Uuid,
    action_success: bool,
) -> Result<(), Error> {
    let step = get_step_of_flow_status(db, parent_job.to_owned()).await?;
    match step {
        Step::Step(step) => {
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

/// Check if the provider is Anthropic (either direct or through OpenRouter)
fn is_anthropic_provider(provider: &ProviderWithResource) -> bool {
    let provider_is_anthropic = provider.kind.is_anthropic();
    let is_openrouter_anthropic =
        provider.kind == AIProvider::OpenRouter && provider.model.starts_with("anthropic/");
    provider_is_anthropic || is_openrouter_anthropic
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

    // job execution context
    client: &AuthedClient,
    occupancy_metrics: &mut OccupancyMetrics,
    job_completed_tx: &JobCompletedSender,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
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

    // Create user message with optional images
    let user_content = if let Some(images) = &args.images {
        let mut parts = vec![ContentPart::Text { text: args.user_message.clone() }];
        for image_wrapper in images.iter() {
            let image = &image_wrapper.s3_object;
            if !image.s3.is_empty() {
                parts.push(ContentPart::S3Object { s3_object: image.clone() });
            }
        }
        if parts.len() == 1 {
            OpenAIContent::Text(args.user_message.clone())
        } else {
            OpenAIContent::Parts(parts)
        }
    } else {
        OpenAIContent::Text(args.user_message.clone())
    };

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

    // Main agent loop
    for i in 0..MAX_AGENT_ITERATIONS {
        if used_structured_output_tool {
            break;
        }

        // For image output without tools, just generate the image
        if output_type == &OutputType::Image && tool_defs.is_none() {
            // Extract S3Objects from ImageWrappers
            let images_vec: Vec<S3Object> = args
                .images
                .as_ref()
                .map(|imgs| {
                    imgs.iter()
                        .map(|wrapper| wrapper.s3_object.clone())
                        .collect()
                })
                .unwrap_or_default();
            let images_slice = if images_vec.is_empty() {
                None
            } else {
                Some(images_vec.as_slice())
            };

            let build_args = BuildRequestArgs {
                messages: &messages,
                tools: None,
                model: args.provider.get_model(),
                temperature: args.temperature,
                max_tokens: args.max_completion_tokens,
                output_schema: args.output_schema.as_ref(),
                output_type,
                system_prompt: args.system_prompt.as_deref(),
                user_message: &args.user_message,
                images: images_slice,
                api_key,
                base_url: &base_url,
            };

            let request_body = query_builder
                .build_request(&build_args, client, &job.workspace_id)
                .await?;

            let endpoint =
                query_builder.get_endpoint(&base_url, args.provider.get_model(), output_type);
            let auth_headers = query_builder.get_auth_headers(api_key, output_type);

            let mut request = HTTP_CLIENT
                .post(&endpoint)
                .timeout(std::time::Duration::from_secs(120))
                .header("Content-Type", "application/json");

            // Apply authentication headers
            for (header_name, header_value) in auth_headers {
                request = request.header(header_name, header_value);
            }

            let resp = request
                .body(request_body)
                .send()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to call API: {}", e)))?;

            match resp.error_for_status_ref() {
                Ok(_) => {
                    let parsed = query_builder.parse_response(resp).await?;

                    match parsed {
                        ParsedResponse::Image { base64_data } => {
                            // Upload to S3
                            let s3_object = upload_image_to_s3(&base64_data, job, client).await?;
                            return Ok(to_raw_value(&s3_object));
                        }
                        ParsedResponse::Text { .. } => {
                            return Err(Error::internal_err(
                                "Expected image response but got text".to_string(),
                            ));
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

        // For text output or image output with tools
        // Extract S3Objects from ImageWrappers
        let images_vec: Vec<S3Object> = args
            .images
            .as_ref()
            .map(|imgs| {
                imgs.iter()
                    .map(|wrapper| wrapper.s3_object.clone())
                    .collect()
            })
            .unwrap_or_default();
        let images_slice = if images_vec.is_empty() {
            None
        } else {
            Some(images_vec.as_slice())
        };

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
            images: images_slice,
            api_key,
            base_url: &base_url,
        };

        let request_body = query_builder
            .build_request(&build_args, client, &job.workspace_id)
            .await?;

        let endpoint =
            query_builder.get_endpoint(&base_url, args.provider.get_model(), output_type);
        let auth_headers = query_builder.get_auth_headers(api_key, output_type);

        let mut request = HTTP_CLIENT
            .post(&endpoint)
            .timeout(std::time::Duration::from_secs(120))
            .header("Content-Type", "application/json");

        // Apply authentication headers
        for (header_name, header_value) in auth_headers {
            request = request.header(header_name, header_value);
        }

        let resp = request
            .body(request_body)
            .send()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to call API: {}", e)))?;

        match resp.error_for_status_ref() {
            Ok(_) => {
                let parsed = query_builder.parse_response(resp).await?;

                match parsed {
                    ParsedResponse::Text { content: response_content, tool_calls } => {
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

                        // Handle tool calls (keeping existing tool execution logic)
                        for tool_call in tool_calls.iter() {
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
                                    content: Some(OpenAIContent::Text(
                                        tool_call.function.arguments.clone(),
                                    )),
                                    agent_action: Some(AgentAction::Message {}),
                                    ..Default::default()
                                });
                                content =
                                    Some(OpenAIContent::Text(tool_call.function.arguments.clone()));
                                break;
                            }

                            // Execute regular tool
                            let tool = tools
                                .iter()
                                .find(|t| t.def.function.name == tool_call.function.name);
                            if let Some(tool) = tool {
                                let job_id = ulid::Ulid::new().into();
                                actions.push(AgentAction::ToolCall {
                                    job_id,
                                    function_name: tool_call.function.name.clone(),
                                    module_id: tool.module.id.clone(),
                                });

                                update_flow_status_module_with_actions(db, parent_job, &actions)
                                    .await?;

                                let tool_call_args =
                                    serde_json::from_str::<HashMap<String, Box<RawValue>>>(
                                        &tool_call.function.arguments,
                                    )?;

                                let job_payload = match tool.module.get_value()? {
                                    FlowModuleValue::Script {
                                        path: script_path,
                                        hash: script_hash,
                                        tag_override,
                                        ..
                                    } => {
                                        let payload = script_to_payload(
                                            script_hash,
                                            script_path,
                                            db,
                                            job,
                                            &tool.module,
                                            tag_override,
                                            tool.module.apply_preprocessor,
                                        )
                                        .await?;
                                        payload
                                    }
                                    FlowModuleValue::RawScript {
                                        path,
                                        content,
                                        language,
                                        lock,
                                        tag,
                                        custom_concurrency_key,
                                        concurrent_limit,
                                        concurrency_time_window_s,
                                        ..
                                    } => {
                                        let path = path.unwrap_or_else(|| {
                                            format!(
                                                "{}/tools/{}",
                                                job.runnable_path(),
                                                tool.module.id
                                            )
                                        });

                                        let payload = raw_script_to_payload(
                                            path,
                                            content,
                                            language,
                                            lock,
                                            custom_concurrency_key,
                                            concurrent_limit,
                                            concurrency_time_window_s,
                                            &tool.module,
                                            tag,
                                            tool.module.delete_after_use.unwrap_or(false),
                                        );
                                        payload
                                    }
                                    _ => {
                                        return Err(Error::internal_err(format!(
                                            "Unsupported tool: {}",
                                            tool_call.function.name
                                        )));
                                    }
                                };

                                let mut tx = db.begin().await?;

                                let job_perms = windmill_common::auth::get_job_perms(
                                    &mut *tx,
                                    &job.id,
                                    &job.workspace_id,
                                )
                                .await?
                                .map(|x| x.into());

                                let (email, permissioned_as) =
                                    if let Some(on_behalf_of) = job_payload.on_behalf_of.as_ref() {
                                        (&on_behalf_of.email, on_behalf_of.permissioned_as.clone())
                                    } else {
                                        (&job.permissioned_as_email, job.permissioned_as.to_owned())
                                    };

                                let job_priority = tool.module.priority.or(job.priority);

                                let tx = PushIsolationLevel::Transaction(tx);
                                let (uuid, tx) = push(
                                    db,
                                    tx,
                                    &job.workspace_id,
                                    job_payload.payload,
                                    PushArgs { args: &tool_call_args, extra: None },
                                    &job.created_by,
                                    email,
                                    permissioned_as,
                                    Some(&format!("job-span-{}", job.id)),
                                    None,
                                    job.schedule_path(),
                                    Some(job.id),
                                    None,
                                    None,
                                    Some(job_id),
                                    false,
                                    false,
                                    None,
                                    job.visible_to_owner,
                                    Some(job.tag.clone()),
                                    job_payload.timeout,
                                    None,
                                    job_priority,
                                    job_perms.as_ref(),
                                    true,
                                )
                                .await?;

                                tx.commit().await?;

                                let tool_job = get_mini_pulled_job(db, &uuid).await?;

                                let Some(tool_job) = tool_job else {
                                    return Err(Error::internal_err(
                                        "Tool job not found".to_string(),
                                    ));
                                };

                                let tool_job = Arc::new(tool_job);

                                let job_dir = create_job_dir(&worker_dir, job.id).await;

                                let (inner_job_completed_tx, inner_job_completed_rx) =
                                    JobCompletedSender::new(&conn, 1);

                                let inner_job_completed_rx = inner_job_completed_rx.expect(
                                     "inner_job_completed_tx should be set as agent jobs are not supported on agent workers",
                                 );

                                #[cfg(feature = "benchmark")]
                                let mut bench = windmill_common::bench::BenchmarkIter::new();

                                match handle_queued_job(
                                    tool_job.clone(),
                                    None,
                                    None,
                                    None,
                                    None,
                                    conn,
                                    client,
                                    hostname,
                                    worker_name,
                                    worker_dir,
                                    &job_dir,
                                    None,
                                    base_internal_url,
                                    inner_job_completed_tx,
                                    occupancy_metrics,
                                    killpill_rx,
                                    None,
                                    #[cfg(feature = "benchmark")]
                                    &mut bench,
                                )
                                .await
                                {
                                    Err(err) => {
                                        let err_string =
                                            format!("{}: {}", err.name(), err.to_string());
                                        let err_json = error_to_value(&err);
                                        let _ = handle_non_flow_job_error(
                                            db,
                                            &tool_job,
                                            0,
                                            None,
                                            err_string.clone(),
                                            err_json,
                                            worker_name,
                                        )
                                        .await;
                                        messages.push(OpenAIMessage {
                                            role: "tool".to_string(),
                                            content: Some(OpenAIContent::Text(format!(
                                                "Error running tool: {}",
                                                err_string
                                            ))),
                                            tool_call_id: Some(tool_call.id.clone()),
                                            agent_action: Some(AgentAction::ToolCall {
                                                job_id,
                                                function_name: tool_call.function.name.clone(),
                                                module_id: tool.module.id.clone(),
                                            }),
                                            ..Default::default()
                                        });
                                        update_flow_status_module_with_actions_success(
                                            db, parent_job, false,
                                        )
                                        .await?;
                                    }
                                    Ok(success) => {
                                        let send_result =
                                            inner_job_completed_rx.bounded_rx.try_recv().ok();

                                        let result = if let Some(SendResult {
                                            result:
                                                SendResultPayload::JobCompleted(JobCompleted {
                                                    result,
                                                    ..
                                                }),
                                            ..
                                        }) = send_result.as_ref()
                                        {
                                            job_completed_tx
                                                .send(
                                                    send_result.as_ref().unwrap().result.clone(),
                                                    true,
                                                )
                                                .await
                                                .map_err(to_anyhow)?;
                                            result
                                        } else {
                                            if let Some(send_result) = send_result {
                                                job_completed_tx
                                                    .send(send_result.result, true)
                                                    .await
                                                    .map_err(to_anyhow)?;
                                            }
                                            return Err(Error::internal_err(
                                                "Tool job completed but no result".to_string(),
                                            ));
                                        };

                                        messages.push(OpenAIMessage {
                                            role: "tool".to_string(),
                                            content: Some(OpenAIContent::Text(
                                                result.get().to_string(),
                                            )),
                                            tool_call_id: Some(tool_call.id.clone()),
                                            agent_action: Some(AgentAction::ToolCall {
                                                job_id,
                                                function_name: tool_call.function.name.clone(),
                                                module_id: tool.module.id.clone(),
                                            }),
                                            ..Default::default()
                                        });
                                        update_flow_status_module_with_actions_success(
                                            db, parent_job, success,
                                        )
                                        .await?;
                                    }
                                }
                            } else {
                                return Err(Error::internal_err(format!(
                                    "Tool not found: {}",
                                    tool_call.function.name
                                )));
                            }
                        }
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

    Ok(to_raw_value(&AIAgentResult {
        output: output_value,
        messages: final_messages,
    }))
}
