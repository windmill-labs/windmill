use std::collections::HashMap;

use crate::flow::{FlowModuleValue, FlowValue, InputTransform};
use crate::jobs::{
    add_completed_job, add_completed_job_error, get_queued_job, postprocess_queued_job, push,
    script_path_to_payload, JobPayload,
};
use crate::js_eval::{eval_timeout, EvalCreds};
use crate::users::create_token_for_owner;
use crate::{
    db::DB,
    error::{self, Error},
    jobs::QueuedJob,
};
use async_recursion::async_recursion;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use tracing::instrument;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct FlowStatus {
    pub step: i32,
    pub modules: Vec<FlowStatusModule>,
    pub failure_module: FlowStatusModule,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Iterator {
    pub index: u8,
    pub itered: Vec<Value>,
    pub args: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum FlowStatusModule {
    WaitingForPriorSteps,
    WaitingForEvent {
        event: String,
    },
    WaitingForExecutor {
        job: Uuid,
    },
    InProgress {
        job: Uuid,
        iterator: Option<Iterator>,
        forloop_jobs: Option<Vec<Uuid>>,
    },
    Success {
        job: Uuid,
        forloop_jobs: Option<Vec<Uuid>>,
    },
    Failure {
        job: Uuid,
        forloop_jobs: Option<Vec<Uuid>>,
    },
}

#[async_recursion]
#[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion(
    db: &DB,
    job: &QueuedJob,
    success: bool,
    result: Option<Map<String, Value>>,
) -> error::Result<()> {
    tracing::info!("HANDLE FLOW: {job:?} {success} {result:?}");

    let mut tx = db.begin().await?;

    let w_id = &job.workspace_id;

    let flow = job
        .parent_job
        .ok_or_else(|| Error::InternalErr(format!("expected parent job")))?;

    let old_status_json = sqlx::query_scalar!(
        "SELECT flow_status FROM queue WHERE id = $1 AND workspace_id = $2",
        flow,
        w_id
    )
    .fetch_one(&mut tx)
    .await?
    .ok_or_else(|| Error::InternalErr(format!("requiring a previous status")))?;

    let old_status = serde_json::from_value::<FlowStatus>(old_status_json)
        .ok()
        .ok_or_else(|| {
            Error::InternalErr(format!("requiring status to be parsabled as FlowStatus"))
        })?;

    let (step_counter, new_status) = match &old_status.modules[old_status.step as usize] {
        module_status @ FlowStatusModule::InProgress {
            iterator: Some(Iterator { index, itered, .. }),
            ..
        } if (index.to_owned() as usize) < itered.len() - 1 && success => {
            (old_status.step, module_status.clone())
        }
        module_status @ _ => {
            let forloop_jobs = match module_status {
                FlowStatusModule::InProgress {
                    forloop_jobs: Some(jobs),
                    ..
                } => Some(jobs.clone()),
                _ => None,
            };
            let new_status = if success {
                FlowStatusModule::Success {
                    job: job.id,
                    forloop_jobs,
                }
            } else {
                FlowStatusModule::Failure {
                    job: job.id,
                    forloop_jobs,
                }
            };
            (old_status.step + 1, new_status)
        }
    };

    let last_step = step_counter as usize == old_status.modules.len();

    tracing::debug!(
        "old status: {:#?}\n{:#?}\n{last_step}",
        old_status,
        new_status
    );

    let prev_step = old_status.step;
    let (stop_early_expr, skip_if_stop_early) = sqlx::query_as::<_, (Option<String>, Option<bool>)>(&format!(
        "UPDATE queue
            SET 
                flow_status = jsonb_set(jsonb_set(flow_status, '{{modules, {prev_step}}}', $1), '{{\"step\"}}', $2)
            WHERE id = $3
            RETURNING 
                (raw_flow->'modules'->{prev_step}->>'stop_after_if_expr'), 
                (raw_flow->'modules'->{prev_step}->>'skip_if_stopped')::bool",
    ))
    .bind(serde_json::json!(new_status))
    .bind(serde_json::json!(step_counter))
    .bind(flow)
    .fetch_one(&mut tx)
    .await?;

    tracing::info!("UPDATE: {:?}", new_status);

    let flow_job = get_queued_job(flow, w_id, &mut tx)
        .await?
        .ok_or_else(|| Error::InternalErr(format!("requiring flow to be in the queue")))?;
    tx.commit().await?;

    let stop_early = success
        && if let Some(expr) = stop_early_expr {
            compute_stop_early(expr, result.clone()).await?
        } else {
            false
        };

    let done = if !success || last_step || stop_early {
        let result = match new_status {
            FlowStatusModule::Success {
                forloop_jobs: Some(jobs),
                ..
            } => {
                let mut results = Vec::new();
                for job in jobs {
                    let result = sqlx::query_scalar!(
                        "SELECT result FROM completed_job WHERE id = $1 AND workspace_id = $2",
                        job,
                        w_id,
                    )
                    .fetch_optional(db)
                    .await?
                    .flatten();
                    results.push(result.clone());
                }
                let mut results_map = serde_json::Map::new();
                results_map.insert("res1".to_string(), serde_json::json!(results));
                Some(results_map)
            }
            _ => result.clone(),
        };

        let logs = if stop_early {
            "Flow job stopped early".to_string()
        } else {
            "Flow job completed".to_string()
        };
        tracing::debug!("{skip_if_stop_early:?}");
        add_completed_job(
            db,
            &flow_job,
            success,
            stop_early && skip_if_stop_early.unwrap_or(false),
            result.clone(),
            logs,
        )
        .await?;
        true
    } else {
        match handle_flow(&flow_job, db, result.clone()).await {
            Err(err) => {
                let _ = add_completed_job_error(
                    db,
                    &flow_job,
                    "Unexpected error during flow chaining:\n".to_string(),
                    err,
                )
                .await;
                true
            }
            Ok(_) => false,
        }
    };

    if done {
        postprocess_queued_job(
            flow_job.schedule_path.clone(),
            flow_job.script_path.clone(),
            &w_id,
            flow,
            db,
        )
        .await?;

        if flow_job.parent_job.is_some() {
            return Ok(
                update_flow_status_after_job_completion(db, &flow_job, success, result).await?,
            );
        }
    }

    Ok(())
}

async fn compute_stop_early(
    expr: String,
    result: Option<Map<String, Value>>,
) -> error::Result<bool> {
    let result = serde_json::Value::Object(result.clone().unwrap_or_else(|| Map::new()));
    match eval_timeout(expr, [("result".to_string(), result)].into(), None, vec![]).await? {
        serde_json::Value::Bool(true) => Ok(true),
        serde_json::Value::Bool(false) => Ok(false),
        a @ _ => Err(Error::ExecutionErr(format!(
            "Expected a boolean value, found: {a:?}"
        ))),
    }
}

pub fn init_flow_status(f: &FlowValue) -> FlowStatus {
    FlowStatus {
        step: 0,
        modules: (0..f.modules.len())
            .map(|_| FlowStatusModule::WaitingForPriorSteps)
            .collect(),
        failure_module: FlowStatusModule::WaitingForPriorSteps,
    }
}

pub async fn update_flow_status_in_progress(
    db: &DB,
    w_id: &str,
    flow: Uuid,
    job_in_progress: Uuid,
) -> error::Result<()> {
    let step = get_step_of_flow_status(db, flow).await?;
    sqlx::query(&format!(
        "UPDATE queue
            SET flow_status = jsonb_set(flow_status, '{{modules, {}}}', $1)
            WHERE id = $2 AND workspace_id = $3",
        step
    ))
    .bind(serde_json::json!(FlowStatusModule::InProgress {
        job: job_in_progress,
        iterator: None,
        forloop_jobs: None,
    }))
    .bind(flow)
    .bind(w_id)
    .execute(db)
    .await?;
    Ok(())
}

#[instrument(level = "trace", skip_all)]
pub async fn get_step_of_flow_status(db: &DB, id: Uuid) -> error::Result<i32> {
    let r = sqlx::query_scalar!(
        "SELECT (flow_status->'step')::integer FROM queue WHERE id = $1",
        id
    )
    .fetch_one(db)
    .await?
    .ok_or_else(|| Error::InternalErr(format!("not found step")))?;
    Ok(r)
}

#[instrument(level = "trace", skip_all)]
async fn transform_input(
    flow_args: &Option<serde_json::Value>,
    last_result: Option<Map<String, serde_json::Value>>,
    input_transform: &HashMap<String, InputTransform>,
    workspace: &str,
    token: &str,
    steps: Vec<String>,
) -> anyhow::Result<Option<Map<String, serde_json::Value>>> {
    let mut mapped = serde_json::Map::new();

    for (key, val) in input_transform.into_iter() {
        match val {
            InputTransform::Static { value } => {
                mapped.insert(key.to_string(), value.to_owned());
                ()
            }
            _ => (),
        };
    }

    for (key, val) in input_transform.into_iter() {
        match val {
            InputTransform::Static { value: _ } => (),
            InputTransform::Javascript { expr } => {
                let previous_result =
                    serde_json::Value::Object(last_result.clone().unwrap_or_else(|| Map::new()));
                let flow_input = flow_args.clone().unwrap_or_else(|| json!({}));
                let v = eval_timeout(
                    expr.to_string(),
                    vec![
                        ("params".to_string(), serde_json::json!(mapped)),
                        ("previous_result".to_string(), previous_result),
                        ("flow_input".to_string(), flow_input),
                    ],
                    Some(EvalCreds {
                        workspace: workspace.to_string(),
                        token: token.to_string(),
                    }),
                    steps.clone(),
                )
                .await
                .map_err(|e| {
                    Error::ExecutionErr(format!(
                        "Error during isolated evaluation of expression `{expr}`:\n{e}"
                    ))
                })?;
                mapped.insert(key.to_string(), v);
                ()
            }
        }
    }

    Ok(Some(mapped))
}

#[instrument(level = "trace", skip_all)]
pub async fn handle_flow(
    flow_job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    last_result: Option<Map<String, serde_json::Value>>,
) -> anyhow::Result<()> {
    let value = flow_job
        .raw_flow
        .as_ref()
        .ok_or_else(|| Error::InternalErr(format!("requiring a raw flow value")))?
        .to_owned();
    let flow = serde_json::from_value::<FlowValue>(value.to_owned())?;
    push_next_flow_job(
        flow_job,
        flow,
        flow_job.schedule_path.clone(),
        db,
        last_result,
    )
    .await?;
    Ok(())
}

#[instrument(level = "trace", skip_all)]
async fn push_next_flow_job(
    flow_job: &QueuedJob,
    flow: FlowValue,
    schedule_path: Option<String>,
    db: &sqlx::Pool<sqlx::Postgres>,
    last_result: Option<Map<String, serde_json::Value>>,
) -> anyhow::Result<()> {
    let flow_status_json = flow_job.flow_status.as_ref().ok_or_else(|| {
        Error::InternalErr(format!("not found status for flow job {:?}", flow_job.id))
    })?;
    let status = serde_json::from_value::<FlowStatus>(flow_status_json.to_owned())?;
    let i = status.step as usize;

    if flow.modules.len() > i {
        let module = &flow.modules[i];
        let mut tx = db.begin().await?;
        let job_payload = match &module.value {
            FlowModuleValue::Script {
                path: script_path
            } => script_path_to_payload(script_path, &mut tx, &flow_job.workspace_id).await?,
            FlowModuleValue::RawScript(raw_code) => {
                let mut raw_code = raw_code.clone();
                if raw_code.path.is_none() {
                    raw_code.path = Some(format!(
                        "{}/{i}",
                        flow_job
                            .script_path
                            .as_ref()
                            .unwrap_or(&"NO_FLOW_PATH".to_owned())
                    ));
                }
                JobPayload::Code(raw_code)
            }
            FlowModuleValue::ForloopFlow { iterator: _, value } => JobPayload::RawFlow {
                value: *(*value).to_owned(),
                path: Some(format!(
                    "{}/{i}",
                    flow_job
                        .script_path
                        .as_ref()
                        .unwrap_or(&"NO_FLOW_PATH".to_owned())
                )),
            },
            a @ _ => {
                tracing::info!("Unrecognized module values {:?}", a);
                Err(Error::BadRequest(format!(
                    "Unrecognized module values {:?}",
                    a
                )))?
            }
        };

        let token = create_token_for_owner(
            &db,
            &flow_job.workspace_id,
            &flow_job.permissioned_as,
            crate::users::NewToken {
                label: Some("transform-input".to_string()),
                expiration: Some(chrono::Utc::now() + chrono::Duration::seconds(10)),
            },
            &flow_job.created_by,
        )
        .await?;
        let mut input_transform = module.input_transform.clone();

        tracing::debug!(
            "PUSH: module: {:#?}, status: {:#?}",
            module.value,
            status.modules[i]
        );
        let (forloop_args, forloop_iterator) = match &module.value {
            FlowModuleValue::ForloopFlow { iterator, .. } => {
                let (index_forloop, itered, args, forloop_jobs) = match &status.modules[i] {
                    FlowStatusModule::WaitingForPriorSteps { .. } => {
                        let itered = match iterator {
                            InputTransform::Static { value } => value.clone(),
                            InputTransform::Javascript { expr } => {
                                let result = serde_json::Value::Object(
                                    last_result.clone().unwrap_or_else(|| Map::new()),
                                );
                                eval_timeout(
                                    expr.to_string(),
                                    [("result".to_string(), result)].into(),
                                    None,
                                    vec![],
                                )
                                .await?
                            }
                        };
                        input_transform.insert(
                            "_index".to_string(),
                            InputTransform::Static {
                                value: serde_json::Value::Number(serde_json::Number::from(0)),
                            },
                        );
                        input_transform.insert(
                            "_value".to_string(),
                            InputTransform::Static {
                                value: itered[0].clone(),
                            },
                        );

                        match itered {
                            serde_json::Value::Array(arr) => (0 as u8, arr, None, vec![]),
                            a @ _ => Err(Error::BadRequest(format!(
                                "Expected an array value, found: {:?}",
                                a
                            )))?,
                        }
                    }
                    FlowStatusModule::InProgress {
                        iterator:
                            Some(Iterator {
                                index,
                                itered,
                                args,
                            }),
                        forloop_jobs,
                        ..
                    } => {
                        let mut args = args.clone();
                        let nindex = index.to_owned() + 1;
                        args.insert(
                            "_index".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(nindex.to_owned())),
                        );
                        args.insert(
                            "_value".to_string(),
                            itered[nindex.to_owned() as usize].clone(),
                        );
                        (
                            nindex,
                            itered.to_owned(),
                            Some(args),
                            forloop_jobs.to_owned().unwrap_or_else(Vec::new),
                        )
                    }
                    a @ _ => Err(Error::BadRequest(format!(
                        "Unrecognized module status for ForloopFlow {:?}",
                        a
                    )))?,
                };
                (args, Some((index_forloop, itered, forloop_jobs)))
            }
            _ => (None, None),
        };

        let args = if forloop_args.is_some() {
            forloop_args.map(|x| x.to_owned())
        } else {
            let steps = status
                .modules
                .into_iter()
                .map(|x| match x {
                    FlowStatusModule::Success {
                        job,
                        forloop_jobs: _,
                    } => job.to_string(),
                    _ => "invalid step status".to_string(),
                })
                .collect();

            let transformed = transform_input(
                &flow_job.args,
                last_result.clone(),
                &input_transform,
                &flow_job.workspace_id,
                &token,
                steps,
            )
            .await?;

            match (&flow_job.args, &module.value) {
                (Some(Value::Object(m)), FlowModuleValue::ForloopFlow { .. }) => {
                    let mut args = transformed.unwrap_or_else(Map::new);
                    args.extend(m.to_owned());
                    args.extend(last_result.unwrap());
                    Some(args)
                }
                _ => transformed,
            }
        };

        let (uuid, mut tx) = push(
            tx,
            &flow_job.workspace_id,
            job_payload,
            args.clone(),
            &flow_job.created_by,
            flow_job.permissioned_as.to_owned(),
            None,
            schedule_path,
            Some(flow_job.id),
            true,
        )
        .await?;

        let new_status = if let Some((index, itered, mut forloop_jobs)) = forloop_iterator {
            forloop_jobs.push(uuid.to_owned());
            serde_json::json!(FlowStatusModule::InProgress {
                job: uuid,
                iterator: Some(Iterator {
                    index: index,
                    itered: itered,
                    args: args.unwrap_or_else(|| Map::new()),
                }),
                forloop_jobs: Some(forloop_jobs),
            })
        } else {
            serde_json::json!(FlowStatusModule::WaitingForExecutor { job: uuid })
        };

        sqlx::query(&format!(
            "UPDATE queue
            SET 
                flow_status = jsonb_set(flow_status, '{{modules, {}}}', $1)
            WHERE id = $2",
            i
        ))
        .bind(new_status)
        .bind(flow_job.id)
        .execute(&mut tx)
        .await?;
        tx.commit().await?;
    }
    Ok(())
}
