use async_recursion::async_recursion;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{collections::HashMap, sync::Arc};
#[cfg(feature = "benchmark")]
use windmill_common::bench::BenchmarkIter;
use windmill_common::{
    auth::get_job_perms,
    cache::{self, FlowData},
    client::AuthedClient,
    error::{self, to_anyhow, Error},
    flow_status::AgentAction,
    flows::{FlowModule, FlowModuleValue},
    get_latest_hash_for_path,
    jobs::DB,
    scripts::{get_full_hub_script_by_path, ScriptLang},
    utils::{StripPath, HTTP_CLIENT},
    worker::{to_raw_value, Connection},
};
use windmill_parser::Typ;
use windmill_queue::{
    flow_status::{get_step_of_flow_status, Step},
    get_mini_pulled_job, push, CanceledBy, JobCompleted, MiniPulledJob, PushArgs,
    PushIsolationLevel,
};

use crate::{
    common::{build_args_map, error_to_value, OccupancyMetrics},
    create_job_dir,
    handle_child::run_future_with_polling_update_job_poller,
    handle_queued_job, parse_sig_of_lang,
    result_processor::handle_non_flow_job_error,
    worker_flow::{raw_script_to_payload, script_to_payload},
    JobCompletedSender, SendResult, SendResultPayload,
};

const MAX_AGENT_ITERATIONS: usize = 10;

lazy_static::lazy_static! {
    static ref TOOL_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
}

#[derive(Deserialize, Serialize, Clone)]
struct OpenAIFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize, Serialize, Clone)]
struct OpenAIToolCall {
    id: String,
    function: OpenAIFunction,
    r#type: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
struct OpenAIMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing)]
    agent_action: Option<AgentAction>,
}

/// same as OpenAIMessage but with agent_action field included in the serialization
#[derive(Serialize)]
struct Message<'a> {
    #[serde(flatten)]
    message: &'a OpenAIMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_action: Option<&'a AgentAction>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Serialize)]
struct OpenAIRequest<'a> {
    model: &'a str,
    messages: &'a Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a Vec<&'a ToolDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
}

#[derive(Serialize, Clone)]
struct ToolDefFunction {
    name: String,
    parameters: Box<RawValue>,
}

#[derive(Serialize, Clone)]
struct ToolDef {
    r#type: String,
    function: ToolDefFunction,
}

struct Tool {
    module: FlowModule,
    def: ToolDef,
}

#[derive(Deserialize, Debug)]
struct AIAgentArgs {
    provider: Provider,
    system_prompt: String,
    user_message: String,
    temperature: Option<f32>,
    max_completion_tokens: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct ProviderResource {
    #[serde(alias = "apiKey")]
    api_key: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "kind")]
enum Provider {
    OpenAI { resource: ProviderResource, model: String },
    Anthropic { resource: ProviderResource, model: String },
}

impl Provider {
    fn get_api_key(&self) -> &str {
        match self {
            Provider::OpenAI { resource, .. } => &resource.api_key,
            Provider::Anthropic { resource, .. } => &resource.api_key,
        }
    }

    fn get_model(&self) -> &str {
        match self {
            Provider::OpenAI { model, .. } => model,
            Provider::Anthropic { model, .. } => model,
        }
    }

    fn get_base_url(&self) -> &str {
        match self {
            Provider::OpenAI { .. } => "https://api.openai.com/v1",
            Provider::Anthropic { .. } => "https://api.anthropic.com/v1",
        }
    }
}

#[derive(Serialize)]
struct AIAgentResult<'a> {
    output: String,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize, Default, Clone, Debug)]
struct OpenAPISchema {
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Box<OpenAPISchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<HashMap<String, Box<OpenAPISchema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "oneOf")]
    one_of: Option<Vec<Box<OpenAPISchema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#enum: Option<Vec<String>>,
}

impl OpenAPISchema {
    fn from_str(typ: &str) -> Self {
        OpenAPISchema { r#type: typ.to_string(), ..Default::default() }
    }

    fn from_str_with_enum(typ: &str, enu: &Option<Vec<String>>) -> Self {
        OpenAPISchema { r#type: typ.to_string(), r#enum: enu.clone(), ..Default::default() }
    }

    fn datetime() -> Self {
        Self {
            r#type: "string".to_string(),
            format: Some("date-time".to_string()),
            ..Default::default()
        }
    }

    fn from_typ(typ: &Typ) -> Self {
        match typ {
            Typ::Str(enu) => Self::from_str_with_enum("string", enu),
            Typ::Int => Self::from_str("integer"),
            Typ::Float => Self::from_str("number"),
            Typ::Bool => Self::from_str("boolean"),
            Typ::Bytes => Self::from_str("string"),
            Typ::Datetime => Self::datetime(),
            Typ::Resource(_) => Self::from_str("string"),
            Typ::Email => Self::from_str("string"),
            Typ::Sql => Self::from_str("string"),
            Typ::DynSelect(_) => Self::from_str("string"),
            Typ::List(typ) => OpenAPISchema {
                r#type: "array".to_string(),
                items: Some(Box::new(Self::from_typ(typ))),
                ..Default::default()
            },
            Typ::Object(typ) => OpenAPISchema {
                r#type: "object".to_string(),
                items: None,
                properties: typ.props.as_ref().map(|props| {
                    props
                        .iter()
                        .map(|prop| (prop.key.clone(), Box::new(Self::from_typ(&prop.typ))))
                        .collect()
                }),
                required: typ
                    .props
                    .as_ref()
                    .map(|props| props.iter().map(|prop| prop.key.clone()).collect()),
                ..Default::default()
            },
            Typ::OneOf(variants) => OpenAPISchema {
                r#type: "object".to_string(),
                one_of: Some(
                    variants
                        .iter()
                        .map(|variant| {
                            let schema = OpenAPISchema {
                                r#type: "object".to_string(),
                                properties: Some(
                                    variant
                                        .properties
                                        .iter()
                                        .map(|prop| {
                                            (
                                                prop.key.clone(),
                                                Box::new(
                                                    if prop.key == "label" || prop.key == "kind" {
                                                        Self::from_str_with_enum(
                                                            "string",
                                                            &Some(vec![variant.label.clone()]),
                                                        )
                                                    } else {
                                                        Self::from_typ(&prop.typ)
                                                    },
                                                ),
                                            )
                                        })
                                        .collect(),
                                ),
                                required: Some(
                                    variant
                                        .properties
                                        .iter()
                                        .map(|prop| prop.key.clone())
                                        .collect(),
                                ),
                                ..Default::default()
                            };
                            Box::new(schema)
                        })
                        .collect(),
                ),
                ..Default::default()
            },
            Typ::Unknown => Self::from_str("object"),
        }
    }
}

async fn update_flow_status_module_with_actions(
    db: &DB,
    parent_job: &uuid::Uuid,
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
                        array['modules', $3::INTEGER::TEXT, 'agent_actions'],
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

fn parse_raw_script_schema(content: &str, language: &ScriptLang) -> Result<Box<RawValue>, Error> {
    let main_arg_signature = parse_sig_of_lang(content, Some(&language), None)?.unwrap(); // safe to unwrap as langauge is some

    let schema = OpenAPISchema {
        r#type: "object".to_string(),
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

#[async_recursion] // we only need it because handle_queued_job could call this function again but in practice it won't because we only accept workspace/raw script flow modules
async fn call_tool(
    // connection
    db: &DB,
    conn: &Connection,

    // agent job and flow step id
    agent_job: &MiniPulledJob,

    // tool
    tool_module: &FlowModule,
    tool_call: &OpenAIToolCall,
    job_id: uuid::Uuid,

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

    let job_payload = match tool_module.get_value()? {
        FlowModuleValue::Script { path: script_path, hash: script_hash, tag_override, .. } => {
            let payload = script_to_payload(
                script_hash,
                script_path,
                db,
                agent_job,
                tool_module,
                tag_override,
                tool_module.apply_preprocessor,
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
                format!("{}/tools/{}", agent_job.runnable_path(), tool_module.id)
            });

            let payload = raw_script_to_payload(
                path,
                content,
                language,
                lock,
                custom_concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                tool_module,
                tag,
                tool_module.delete_after_use.unwrap_or(false),
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

    let job_priority = tool_module.priority.or(agent_job.priority);

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
        Some(job_id),
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

    #[cfg(feature = "benchmark")]
    let mut bench = BenchmarkIter::new();

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
        #[cfg(feature = "benchmark")]
        &mut bench,
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
    db: &DB,
    conn: &Connection,

    // agent job and flow data
    job: &MiniPulledJob,
    parent_job: &uuid::Uuid,
    args: AIAgentArgs,
    tools: Vec<Tool>,

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
    let mut messages = vec![
        OpenAIMessage {
            role: "system".to_string(),
            content: Some(args.system_prompt),
            ..Default::default()
        },
        OpenAIMessage {
            role: "user".to_string(),
            content: Some(args.user_message),
            ..Default::default()
        },
    ];

    let mut actions = vec![];

    let mut content = None;

    let base_url = args.provider.get_base_url();
    let api_key = args.provider.get_api_key();

    let tool_defs = if tools.is_empty() {
        None
    } else {
        Some(tools.iter().map(|t| &t.def).collect())
    };

    for i in 0..MAX_AGENT_ITERATIONS {
        let response = {
            let resp = HTTP_CLIENT
                .post(format!("{}/chat/completions", base_url))
                .bearer_auth(api_key)
                .json(&OpenAIRequest {
                    model: args.provider.get_model(),
                    messages: &messages,
                    tools: tool_defs.as_ref(),
                    temperature: args.temperature,
                    max_completion_tokens: args.max_completion_tokens,
                })
                .send()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to call API: {}", e)))?;

            match resp.error_for_status_ref() {
                Ok(_) => resp,
                Err(e) => {
                    let status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    tracing::error!(
                        "Non 200 response from API: status: {}, body: {}",
                        status,
                        text
                    );
                    return Err(Error::internal_err(format!(
                        "Non 200 response from API: {} - {}",
                        e, text
                    )));
                }
            }
        };

        let mut response = response
            .json::<OpenAIResponse>()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to parse API response: {}", e)))?;

        let first_choice = response
            .choices
            .pop()
            .ok_or_else(|| Error::internal_err("No response from API"))?;

        content = first_choice.message.content;
        let tool_calls = first_choice.message.tool_calls.unwrap_or_default();

        if let Some(ref content) = content {
            actions.push(AgentAction::Message {});
            messages.push(OpenAIMessage {
                role: "assistant".to_string(),
                content: Some(content.clone()),
                agent_action: Some(AgentAction::Message {}),
                ..Default::default()
            });

            update_flow_status_module_with_actions(db, parent_job, &actions).await?;
        }

        if tool_calls.is_empty() {
            break;
        } else if i == MAX_AGENT_ITERATIONS - 1 {
            return Err(Error::internal_err(
                "AI agent reached max iterations, but there are still tool calls".to_string(),
            ));
        }

        messages.push(OpenAIMessage {
            role: "assistant".to_string(),
            tool_calls: Some(tool_calls.clone()),
            ..Default::default()
        });

        for tool_call in tool_calls.iter() {
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

                update_flow_status_module_with_actions(db, parent_job, &actions).await?;

                match call_tool(
                    db,
                    conn,
                    job,
                    &tool.module,
                    &tool_call,
                    job_id,
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
                        messages.push(OpenAIMessage {
                            role: "tool".to_string(),
                            content: Some(result.get().to_string()),
                            tool_call_id: Some(tool_call.id.clone()),
                            agent_action: Some(AgentAction::ToolCall {
                                job_id,
                                function_name: tool_call.function.name.clone(),
                                module_id: tool.module.id.clone(),
                            }),
                            ..Default::default()
                        });
                    }
                    Err(err) => {
                        let err_string = format!("{}: {}", err.name(), err.to_string());
                        messages.push(OpenAIMessage {
                            role: "tool".to_string(),
                            content: Some(format!("Error running tool: {}", err_string)),
                            tool_call_id: Some(tool_call.id.clone()),
                            agent_action: Some(AgentAction::ToolCall {
                                job_id,
                                function_name: tool_call.function.name.clone(),
                                module_id: tool.module.id.clone(),
                            }),
                            ..Default::default()
                        });
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

    let final_messages: Vec<Message> = messages
        .iter()
        .map(|m| Message { message: m, agent_action: m.agent_action.as_ref() })
        .collect();

    Ok(to_raw_value(&AIAgentResult {
        output: content.unwrap_or_default().clone(),
        messages: final_messages,
    }))
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
    let db = match conn {
        Connection::Sql(db) => db,
        Connection::Http(_) => {
            return Err(Error::internal_err(
                "AI agent job is not supported on agent workers".to_string(),
            ));
        }
    };

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
                            let hash = get_latest_hash_for_path(db, &job.workspace_id, path)
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
        args,
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
