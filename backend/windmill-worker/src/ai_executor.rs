use async_recursion::async_recursion;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
};
use tokio::time::Instant;
use uuid::Uuid;
use windmill_common::{
    auth::{get_job_perms, JobPerms},
    cache::FlowData,
    client::AuthedClient,
    db::Authed,
    error::{self, to_anyhow, Error},
    flows::{FlowModule, FlowModuleValue},
    jobs::{JobPayload, DB},
    scripts::{ScriptHash, ScriptLang},
    utils::{WarnAfterExt, HTTP_CLIENT},
    worker::{to_raw_value, to_raw_value_owned, Connection},
};
use windmill_parser::Typ;
use windmill_queue::{
    get_mini_pulled_job, push, CanceledBy, JobCompleted, MiniPulledJob, PushArgs,
    PushIsolationLevel, SameWorkerPayload,
};

use crate::{
    common::{build_args_map, build_args_values, error_to_value, OccupancyMetrics},
    create_job_dir, handle_all_job_kind_error,
    handle_child::run_future_with_polling_update_job_poller,
    handle_code_execution_job, handle_queued_job,
    result_processor::handle_non_flow_job_error,
    worker_flow::{get_path, raw_script_to_payload, script_to_payload},
    JobCompletedSender, SendResult, SendResultPayload,
};

#[derive(Deserialize, Serialize)]
struct OpenAIFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize, Serialize)]
struct OpenAIToolCall {
    id: String,
    function: OpenAIFunction,
    r#type: String,
}

#[derive(Deserialize)]
struct OpenAIMessage {
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIResource {
    api_key: String,
}

#[derive(Deserialize)]
struct AIAgentArgs {
    model: String,
    resource: OpenAIResource,
    system_prompt: String,
    user_message: String,
}

const MAX_AGENT_ITERATIONS: usize = 10;

fn typ_to_openai_typ(typ: &Typ) -> serde_json::Value {
    match typ {
        Typ::Str(_) => serde_json::json!("string"),
        Typ::Int => serde_json::json!("integer"),
        Typ::Float => serde_json::json!("number"),
        Typ::Bool => serde_json::json!("boolean"),
        Typ::List(typ) => serde_json::json!({
            "type": "array",
            "items": typ_to_openai_typ(typ)
        }),
        Typ::Bytes => serde_json::json!("string"),
        Typ::Datetime => serde_json::json!("string"),
        Typ::Resource(resource) => serde_json::json!("string"),
        Typ::Email => serde_json::json!("string"),
        Typ::Sql => serde_json::json!("string"),
        Typ::DynSelect(dyn_select) => serde_json::json!("string"),
        Typ::Object(typ) => {
            serde_json::json!({
                "type": "object",
                "properties": typ.props.as_ref().map(|props| props.iter().map(|prop| {
                    let name = prop.key.clone();
                    let typ = typ_to_openai_typ(&prop.typ);
                    (name, serde_json::json!({
                        "type": typ,
                    }))
                }).collect::<HashMap<String, serde_json::Value>>()).unwrap_or_default(),
                "required": typ.props.as_ref().map(|props| props.iter().map(|prop| prop.key.clone()).collect::<Vec<_>>()).unwrap_or_default(),
            })
        }
        Typ::OneOf(variants) => serde_json::json!({
            "type": "string",
            "enum": variants.iter().map(|variant| variant.label.clone()).collect::<Vec<String>>(),
        }),
        Typ::Unknown => serde_json::json!({ "type": "object" }),
    }
}

fn parse_raw_script_schema(
    content: &str,
    language: &ScriptLang,
) -> Result<serde_json::Value, Error> {
    let main_arg_signature = match language {
        ScriptLang::Bun | ScriptLang::Deno | ScriptLang::Bunnative | ScriptLang::Nativets => {
            windmill_parser_ts::parse_deno_signature(content, false, false, None)
        }
        #[cfg(feature = "python")]
        ScriptLang::Python3 => windmill_parser_py::parse_python_signature(content, None, false),
        _ => {
            return Err(Error::internal_err(format!(
                "Unsupported language: {:?}",
                language
            )));
        }
    };

    let main_arg_signature = main_arg_signature.map_err(|e| Error::internal_err(e.to_string()))?;

    let schema = serde_json::json!({
        "type": "object",
        "properties": main_arg_signature.args.iter().map(|arg| {
            let name = arg.name.clone();
            let typ = typ_to_openai_typ(&arg.typ);
            (name, serde_json::json!({
                "type": typ,
            }))
        }).collect::<HashMap<String, serde_json::Value>>(),
        "required": main_arg_signature.args.iter().map(|arg| arg.name.clone()).collect::<Vec<_>>(),
    });

    Ok(schema)
}

#[async_recursion] // we only need it because handle_queued_job could call this function again but in practice it won't because we only access raw script flow modules
async fn call_tool(
    // connection
    db: &DB,
    conn: &Connection,

    // agent job and flow step id
    agent_job: &MiniPulledJob,
    flow_step_id: &str,

    // tool
    tool: &FlowModule,
    tool_call: &OpenAIToolCall,

    // execution context
    client: &AuthedClient,
    occupancy_metrics: &mut OccupancyMetrics,
    base_internal_url: &str,
    worker_dir: &str,
    worker_name: &str,
    hostname: &str,
    job_completed_tx: &JobCompletedSender,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
) -> error::Result<Arc<Box<RawValue>>> {
    let tool_call_args =
        serde_json::from_str::<HashMap<String, Box<RawValue>>>(&tool_call.function.arguments)?;

    let job_payload = match tool.get_value()? {
        FlowModuleValue::Script { path: script_path, hash: script_hash, tag_override, .. } => {
            let payload = script_to_payload(
                script_hash,
                script_path,
                db,
                agent_job,
                tool,
                tag_override,
                tool.apply_preprocessor,
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
                    "{}/{}/tools/{}",
                    agent_job.runnable_path(),
                    flow_step_id,
                    tool.id
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
                tool,
                tag,
                false,
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

    let job_perms = get_job_perms(&mut *tx, &agent_job.id, &agent_job.workspace_id)
        .await?
        .map(|x| x.into());

    let (email, permissioned_as) = if let Some(on_behalf_of) = job_payload.on_behalf_of.as_ref() {
        (&on_behalf_of.email, on_behalf_of.permissioned_as.clone())
    } else {
        (
            &agent_job.permissioned_as_email,
            agent_job.permissioned_as.to_owned(),
        )
    };

    let job_priority = tool.priority.or(agent_job.priority);

    let tx = PushIsolationLevel::Transaction(tx);
    let (uuid, tx) = push(
        db,
        tx,
        &agent_job.workspace_id,
        job_payload.payload,
        PushArgs { args: &tool_call_args, extra: None },
        &agent_job.created_by,
        email,
        permissioned_as,
        Some(&format!("job-span-{}", agent_job.id)),
        None,
        agent_job.schedule_path(),
        Some(agent_job.id),
        None,
        None,
        false,
        false,
        None,
        agent_job.visible_to_owner,
        Some(agent_job.tag.clone()), // we reuse the same tag as the agent job because it's run on the same worker
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
        return Err(Error::internal_err("Tool job not found".to_string()));
    };

    let tool_job = Arc::new(tool_job);

    let job_dir = create_job_dir(&worker_dir, agent_job.id).await;

    let (inner_job_completed_tx, inner_job_completed_rx) = JobCompletedSender::new(&conn, 1);

    let inner_job_completed_rx = inner_job_completed_rx.expect(
        "inner_job_completed_tx should be set as agent jobs are not supported on agent workers",
    );

    if let Err(err) = handle_queued_job(
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
    )
    .await
    {
        let err_string = format!("{}: {}", err.name(), err.to_string());
        let err_json = error_to_value(&err);
        let _ =
            handle_non_flow_job_error(db, &tool_job, 0, None, err_string, err_json, worker_name)
                .await;
        Err(err)
    } else {
        let send_result = inner_job_completed_rx.bounded_rx.try_recv().ok();

        let result = if let Some(SendResult {
            result: SendResultPayload::JobCompleted(JobCompleted { result, .. }),
            ..
        }) = send_result.as_ref()
        {
            job_completed_tx
                .send(send_result.as_ref().unwrap().result.clone(), true)
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

        Ok(result.clone())
    }
}

async fn run_agent(
    // connection
    conn: &Connection,

    // agent job and flow data
    job: &MiniPulledJob,
    flow_step_id: &str,
    args: AIAgentArgs,
    tool_defs: Vec<serde_json::Value>,
    tools: Vec<FlowModule>,

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
    let db = match conn {
        Connection::Sql(db) => db,
        Connection::Http(_) => {
            return Err(Error::internal_err(
                "AI agent job is not supported on agent workers".to_string(),
            ));
        }
    };

    let mut messages = vec![
        serde_json::json!({
            "role": "system",
            "content": args.system_prompt
        }),
        serde_json::json!({
            "role": "user",
            "content": args.user_message
        }),
    ];

    let mut content = None;

    for i in 0..MAX_AGENT_ITERATIONS {
        let response = {
            let resp = HTTP_CLIENT
                .post(format!("https://api.openai.com/v1/chat/completions"))
                .bearer_auth(&args.resource.api_key)
                .json(&serde_json::json!({
                    "model": args.model,
                    "messages": messages,
                    "tools": tool_defs
                }))
                .send()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to call OpenAI API: {}", e)))?;

            match resp.error_for_status_ref() {
                Ok(_) => resp,
                Err(e) => {
                    let status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    tracing::error!(
                        "Non 200 response from OpenAI API: status: {}, body: {}",
                        status,
                        text
                    );
                    return Err(Error::internal_err(format!(
                        "Non 200 response from OpenAI API: {} - {}",
                        e, text
                    )));
                }
            }
        };

        let mut response = response.json::<OpenAIResponse>().await.map_err(|e| {
            Error::internal_err(format!("Failed to parse OpenAI API response: {}", e))
        })?;

        let first_choice = response
            .choices
            .pop()
            .ok_or_else(|| Error::internal_err("No response from OpenAI API"))?;

        content = first_choice.message.content;
        let tool_calls = first_choice.message.tool_calls.unwrap_or_default();

        if let Some(ref content) = content {
            messages.push(serde_json::json!({
                "role": "assistant",
                "content": content
            }));
        }

        if tool_calls.is_empty() {
            break;
        } else if i == MAX_AGENT_ITERATIONS - 1 {
            return Err(Error::internal_err(
                "AI agent reached max iterations, but there are still tool calls".to_string(),
            ));
        }

        messages.push(serde_json::json!({
            "role": "assistant",
            "tool_calls": tool_calls
        }));

        for tool_call in tool_calls {
            let tool = tools
                .iter()
                .find(|t| t.summary.as_ref().unwrap_or(&t.id) == &tool_call.function.name);
            if let Some(tool) = tool {
                match call_tool(
                    db,
                    conn,
                    job,
                    flow_step_id,
                    tool,
                    &tool_call,
                    client,
                    occupancy_metrics,
                    base_internal_url,
                    worker_dir,
                    worker_name,
                    hostname,
                    job_completed_tx,
                    killpill_rx,
                )
                .await
                {
                    Ok(result) => {
                        messages.push(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": tool_call.id,
                            "content": result.get(),
                        }));
                    }
                    Err(err) => {
                        let err_string = format!("{}: {}", err.name(), err.to_string());
                        messages.push(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": tool_call.id,
                            "content": format!("Error running tool: {}", err_string),
                        }));
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

    Ok(to_raw_value(&content))
}

pub async fn handle_ai_agent_job(
    // connection
    conn: &Connection,

    // agent job and flow data
    job: &MiniPulledJob,
    flow_data: Arc<FlowData>,

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

    let value = flow_data.value();

    let module = value.modules.iter().find(|m| m.id == *flow_step_id);

    let Some(module) = module else {
        return Err(Error::internal_err(
            "AI agent module not found in flow".to_string(),
        ));
    };

    let FlowModuleValue::AIAgent { input_transforms, tools } = module.get_value()? else {
        return Err(Error::internal_err(
            "AI agent module is not an AI agent".to_string(),
        ));
    };

    let tool_defs = tools
        .iter()
        .map(|t| {
            let module_value = t.get_value().unwrap();

            let schema = match &module_value {
                FlowModuleValue::Script { .. } => {
                    return Err(Error::internal_err(format!("Unsupported tool: {}", t.id)));
                }
                FlowModuleValue::RawScript { content, language, .. } => {
                    parse_raw_script_schema(&content, &language)?
                }
                _ => {
                    return Err(Error::internal_err(format!("Unsupported tool: {}", t.id)));
                }
            };

            Ok(serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.summary.as_ref().unwrap_or(&t.id),
                    "parameters": schema
                }
            }))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    tracing::info!("tool_defs: {:#?}", tool_defs);

    let mut inner_occupancy_metrics = occupancy_metrics.clone();

    let agent_fut = run_agent(
        conn,
        job,
        flow_step_id,
        args,
        tool_defs,
        tools,
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
