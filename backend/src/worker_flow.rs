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
    },
    Success {
        job: Uuid,
    },
    Failure {
        job: Uuid,
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
            job: _,
            iterator:
                Some(Iterator {
                    index,
                    itered,
                    args: _,
                }),
        } if (index.to_owned() as usize) < itered.len() - 1 => {
            (old_status.step, module_status.clone())
        }
        _ => {
            let new_status = if success {
                FlowStatusModule::Success { job: job.id }
            } else {
                FlowStatusModule::Failure { job: job.id }
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

    sqlx::query(&format!(
        "UPDATE queue
            SET 
                flow_status = jsonb_set(jsonb_set(flow_status, '{{modules, {}}}', $1), '{{\"step\"}}', $2)
            WHERE id = $3",
        old_status.step,
    ))
    .bind(serde_json::json!(new_status))
    .bind(serde_json::json!(step_counter))
    .bind(flow)
    .execute(&mut tx)
    .await?;

    tracing::info!("UPDATE: {:?}", new_status);

    let flow_job = get_queued_job(flow, w_id, &mut tx)
        .await?
        .ok_or_else(|| Error::InternalErr(format!("requiring flow to be in the queue")))?;
    tx.commit().await?;

    let done = if !success || last_step {
        add_completed_job(
            db,
            &flow_job,
            success,
            false,
            result.clone(),
            "Flow job completed".to_string(),
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
            Ok(StopEarly::Continue) => false,
            Ok(stop_early) => {
                let _ = add_completed_job(
                    db,
                    &flow_job,
                    true,
                    matches!(stop_early, StopEarly::Skip),
                    result.clone(),
                    "Flow job stopped early".to_string(),
                )
                .await;
                true
            }
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

#[derive(Debug)]
pub enum StopEarly {
    Skip,
    Success,
    Continue,
}
#[instrument(level = "trace", skip_all)]
async fn push_next_flow_job(
    flow_job: &QueuedJob,
    flow: FlowValue,
    schedule_path: Option<String>,
    db: &sqlx::Pool<sqlx::Postgres>,
    last_result: Option<Map<String, serde_json::Value>>,
) -> anyhow::Result<StopEarly> {
    let flow_status_json = flow_job.flow_status.as_ref().ok_or_else(|| {
        Error::InternalErr(format!("not found status for flow job {:?}", flow_job.id))
    })?;
    let status = serde_json::from_value::<FlowStatus>(flow_status_json.to_owned())?;
    let i = status.step as usize;

    if i > 0 {
        let prev_module = &flow.modules[i - 1];
        if let Some(expr) = prev_module.stop_after_if_expr.clone() {
            let result =
                serde_json::Value::Object(last_result.clone().unwrap_or_else(|| Map::new()));
            match eval_timeout(expr, [("result".to_string(), result)].into(), None, vec![]).await? {
                serde_json::Value::Bool(true) => {
                    if let Some(true) = prev_module.skip_if_stopped {
                        return Ok(StopEarly::Skip);
                    } else {
                        return Ok(StopEarly::Success);
                    }
                }
                a @ _ => Error::ExecutionErr(format!("Expected a boolean value, found: {a:?}")),
            };
        }
    }
    if flow.modules.len() > i {
        let module = &flow.modules[i];
        let mut tx = db.begin().await?;
        let job_payload = match &module.value {
            FlowModuleValue::Script {
                path: script_path,
                trigger_script: None,
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
                let (index_forloop, itered, args) = match &status.modules[i] {
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
                            "_values".to_string(),
                            InputTransform::Static {
                                value: json!(itered),
                            },
                        );
                        input_transform.insert(
                            "_value".to_string(),
                            InputTransform::Static {
                                value: json!(itered[0]),
                            },
                        );
                        match itered {
                            serde_json::Value::Array(arr) => (0 as u8, arr, None),
                            a @ _ => Err(Error::BadRequest(format!(
                                "Expected an array value, found: {:?}",
                                a
                            )))?,
                        }
                    }
                    FlowStatusModule::InProgress {
                        job: _,
                        iterator:
                            Some(Iterator {
                                index,
                                itered,
                                args,
                            }),
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
                        (nindex, itered.to_owned(), Some(args))
                    }
                    a @ _ => Err(Error::BadRequest(format!(
                        "Unrecognized module status for ForloopFlow {:?}",
                        a
                    )))?,
                };
                (args, Some((index_forloop, itered)))
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
                    FlowStatusModule::Success { job } => job.to_string(),
                    _ => "invalid step status".to_string(),
                })
                .collect();

            let transformed = transform_input(
                &flow_job.args,
                last_result,
                &input_transform,
                &flow_job.workspace_id,
                &token,
                steps,
            )
            .await?;

            match (&flow_job.args, &module.value) {
                (Some(Value::Object(m)), FlowModuleValue::ForloopFlow { .. }) => {
                    transformed.map(|x| {
                        let mut args = m.to_owned();
                        args.extend(x);
                        args
                    })
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

        let new_status = if let Some((index, itered)) = forloop_iterator {
            serde_json::json!(FlowStatusModule::InProgress {
                job: uuid,
                iterator: Some(Iterator {
                    index: index,
                    itered: itered,
                    args: args.unwrap_or_else(|| Map::new()),
                })
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
    Ok(StopEarly::Continue)
}

#[instrument(level = "trace", skip_all)]
pub async fn handle_flow(
    flow_job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    last_result: Option<Map<String, serde_json::Value>>,
) -> anyhow::Result<StopEarly> {
    let value = flow_job
        .raw_flow
        .as_ref()
        .ok_or_else(|| Error::InternalErr(format!("requiring a raw flow value")))?
        .to_owned();
    let flow = serde_json::from_value::<FlowValue>(value.to_owned())?;
    let stop_early = push_next_flow_job(
        flow_job,
        flow,
        flow_job.schedule_path.clone(),
        db,
        last_result,
    )
    .await?;
    tracing::debug!("{:?}", stop_early);
    Ok(stop_early)
}
