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

lazy_static::lazy_static! {
    static ref TOOL_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
}

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

// Use the new unified agent runner from the ai module
use crate::ai::agent_runner::run_agent_unified;

#[async_recursion]
async fn run_agent(
    // connection
    db: &DB,
    conn: &Connection,

    // agent job and flow data
    job: &MiniPulledJob,
    parent_job: &uuid::Uuid,
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
    // Delegate to the new unified agent runner
    run_agent_unified(
        db,
        conn,
        job,
        parent_job,
        args,
        tools,
        client,
        occupancy_metrics,
        job_completed_tx,
        worker_dir,
        base_internal_url,
        worker_name,
        hostname,
        killpill_rx,
    )
    .await
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
