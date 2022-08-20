use std::collections::HashMap;

use crate::{
    db::DB,
    error::{self, Error},
    flows::{FlowModule, FlowModuleValue, FlowValue, InputTransform},
    jobs::{
        add_completed_job, add_completed_job_error, get_queued_job, postprocess_queued_job, push,
        script_path_to_payload, JobPayload, QueuedJob,
    },
    js_eval::{eval_timeout, EvalCreds},
    users::create_token_for_owner,
    worker,
};
use anyhow::Context;
use async_recursion::async_recursion;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
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
    pub index: usize,
    pub itered: Vec<Value>,
    pub args: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum FlowStatusModule {
    WaitingForPriorSteps,
    WaitingForEvent { event: String },
    WaitingForExecutor { job: Uuid },
    InProgress { job: Uuid, iterator: Option<Iterator>, forloop_jobs: Option<Vec<Uuid>> },
    Success { job: Uuid, forloop_jobs: Option<Vec<Uuid>> },
    Failure { job: Uuid, forloop_jobs: Option<Vec<Uuid>> },
}

impl FlowStatus {
    pub fn new(f: &FlowValue) -> Self {
        Self {
            step: 0,
            modules: vec![FlowStatusModule::WaitingForPriorSteps; f.modules.len()],
            failure_module: FlowStatusModule::WaitingForPriorSteps,
        }
    }
}

#[async_recursion]
#[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion(
    db: &DB,
    job: &QueuedJob,
    success: bool,
    result: serde_json::Value,
    metrics: &worker::Metrics,
) -> error::Result<()> {
    tracing::debug!("HANDLE FLOW: {job:?} {success} {result:?}");

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
    .await
    .map_err(|e| {
        Error::InternalErr(format!(
            "fetching flow status {flow} while reporting {success} {result:?}: {e}"
        ))
    })?
    .ok_or_else(|| Error::InternalErr(format!("requiring a previous status")))?;

    let old_status = serde_json::from_value::<FlowStatus>(old_status_json)
        .ok()
        .ok_or_else(|| {
            Error::InternalErr(format!("requiring status to be parsabled as FlowStatus"))
        })?;

    let skip_loop_failures = skip_loop_failures(flow, old_status.step, &mut tx)
        .await?
        .unwrap_or(false);

    let module = usize::try_from(old_status.step)
        .ok()
        .and_then(|i| old_status.modules.get(i))
        .unwrap_or(&old_status.failure_module);

    let (step_counter, new_status) = match module {
        FlowStatusModule::InProgress { iterator: Some(Iterator { index, itered, .. }), .. }
            if *index + 1 < itered.len() && (success || skip_loop_failures) =>
        {
            (old_status.step, module.clone())
        }
        _ => {
            let forloop_jobs = match module {
                FlowStatusModule::InProgress { forloop_jobs, .. } => forloop_jobs.clone(),
                _ => None,
            };
            if success || (forloop_jobs.is_some() && skip_loop_failures) {
                (
                    old_status.step + 1,
                    FlowStatusModule::Success { job: job.id, forloop_jobs },
                )
            } else {
                (
                    old_status.step,
                    FlowStatusModule::Failure { job: job.id, forloop_jobs },
                )
            }
        }
    };

    let is_last_step = usize::try_from(step_counter)
        .map(|i| !(..old_status.modules.len()).contains(&i))
        .unwrap_or(true);

    tracing::debug!(
        "old status: {:#?}\n{:#?}\n{is_last_step}",
        old_status,
        new_status
    );

    let (stop_early_expr, skip_if_stop_early) =
        sqlx::query_as::<_, (Option<String>, Option<bool>)>(
            "
            UPDATE queue
               SET flow_status = JSONB_SET(
                                 JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2),
                                                        ARRAY['step'], $3)
             WHERE id = $4
            RETURNING
                (raw_flow->'modules'->$1->>'stop_after_if_expr'),
                (raw_flow->'modules'->$1->>'skip_if_stopped')::bool
            ",
        )
        .bind(old_status.step)
        .bind(serde_json::json!(new_status))
        .bind(serde_json::json!(step_counter))
        .bind(flow)
        .fetch_one(&mut tx)
        .await
        .map_err(|e| Error::InternalErr(format!("retrieval of stop_early_expr from state: {e}")))?;

    tracing::debug!("UPDATE: {:?}", new_status);

    let flow_job = get_queued_job(flow, w_id, &mut tx)
        .await?
        .ok_or_else(|| Error::InternalErr(format!("requiring flow to be in the queue")))?;

    let stop_early = success
        && if let Some(expr) = stop_early_expr {
            compute_stop_early(expr, result.clone()).await?
        } else {
            false
        };

    let result = match new_status {
        FlowStatusModule::Success { forloop_jobs: Some(jobs), .. } => {
            let results = sqlx::query_as(
                "
                  SELECT result
                    FROM completed_job
                   WHERE id = ANY($1)
                     AND workspace_id = $2
                     AND success = true
                ORDER BY args->>'_index'
                    ",
            )
            .bind(jobs.as_slice())
            .bind(w_id)
            .fetch(&mut tx)
            .map_ok(|(v,)| v)
            .try_collect::<Vec<Value>>()
            .await?;
            json!(results)
        }
        _ => result.clone(),
    };

    let should_continue_flow = match success {
        _ if stop_early => false,
        true => !is_last_step,
        false if skip_loop_failures => !is_last_step,
        false if has_failure_module(flow, &mut tx).await? => true,
        false => false,
    };

    tx.commit().await?;

    let done = if !should_continue_flow {
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
                    metrics,
                )
                .await;
                true
            }
            Ok(_) => false,
        }
    };

    if done {
        postprocess_queued_job(
            flow_job.is_flow_step,
            flow_job.schedule_path.clone(),
            flow_job.script_path.clone(),
            &w_id,
            flow,
            db,
        )
        .await?;

        if flow_job.parent_job.is_some() {
            return Ok(update_flow_status_after_job_completion(
                db, &flow_job, success, result, metrics,
            )
            .await?);
        }
    }

    Ok(())
}

async fn skip_loop_failures<'c>(
    flow: Uuid,
    step: i32,
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
) -> Result<Option<bool>, Error> {
    sqlx::query_as(
        "
    SELECT (raw_flow->'modules'->$1->'value'->>'skip_failures')::bool
      FROM queue
     WHERE id = $2
        ",
    )
    .bind(step)
    .bind(flow)
    .fetch_one(tx)
    .await
    .map(|(v,)| v)
    .map_err(|e| Error::InternalErr(format!("error during retrieval of skip_loop_failures: {e}")))
}

async fn has_failure_module<'c>(
    flow: Uuid,
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
) -> Result<bool, Error> {
    sqlx::query_scalar(
        "
    SELECT raw_flow->'failure_module' != 'null'::jsonb
      FROM queue
     WHERE id = $1
        ",
    )
    .bind(flow)
    .fetch_one(tx)
    .await
    .map_err(|e| Error::InternalErr(format!("error during retrieval of has_failure_module: {e}")))
}

async fn compute_stop_early(expr: String, result: serde_json::Value) -> error::Result<bool> {
    match eval_timeout(expr, [("result".to_string(), result)].into(), None, vec![]).await? {
        serde_json::Value::Bool(true) => Ok(true),
        serde_json::Value::Bool(false) => Ok(false),
        a @ _ => Err(Error::ExecutionErr(format!(
            "Expected a boolean value, found: {a:?}"
        ))),
    }
}

pub fn init_flow_status(f: &FlowValue) -> FlowStatus {
    FlowStatus::new(f)
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
            SET flow_status = jsonb_set(jsonb_set(flow_status, '{{modules, {step}, job}}', $1), '{{modules, {step}, type}}', $2)
            WHERE id = $3 AND workspace_id = $4",
    ))
    .bind(json!(job_in_progress.to_string()))
    .bind(json!("InProgress"))
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
    .await
    .map_err(|e| Error::InternalErr(format!("fetching step flow status: {e}")))?
    .ok_or_else(|| Error::InternalErr(format!("not found step")))?;
    Ok(r)
}

#[instrument(level = "trace", skip_all)]
async fn transform_input(
    flow_args: &Option<serde_json::Value>,
    last_result: serde_json::Value,
    input_transforms: &HashMap<String, InputTransform>,
    workspace: &str,
    token: &str,
    steps: Vec<String>,
) -> anyhow::Result<Map<String, serde_json::Value>> {
    let mut mapped = serde_json::Map::new();

    for (key, val) in input_transforms.into_iter() {
        if let InputTransform::Static { value } = val {
            mapped.insert(key.to_string(), value.to_owned());
        }
    }

    for (key, val) in input_transforms.into_iter() {
        match val {
            InputTransform::Static { value: _ } => (),
            InputTransform::Javascript { expr } => {
                let previous_result = last_result.clone();
                let flow_input = flow_args.clone().unwrap_or_else(|| json!({}));
                let v = eval_timeout(
                    expr.to_string(),
                    vec![
                        ("params".to_string(), serde_json::json!(mapped)),
                        ("previous_result".to_string(), previous_result),
                        ("flow_input".to_string(), flow_input),
                    ],
                    Some(EvalCreds { workspace: workspace.to_string(), token: token.to_string() }),
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

    Ok(mapped)
}

#[instrument(level = "trace", skip_all)]
pub async fn handle_flow(
    flow_job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    last_result: serde_json::Value,
) -> anyhow::Result<()> {
    let value = flow_job
        .raw_flow
        .as_ref()
        .ok_or_else(|| Error::InternalErr(format!("requiring a raw flow value")))?
        .to_owned();
    let flow = serde_json::from_value::<FlowValue>(value.to_owned())?;

    if flow.modules.len() == 0 {
        Err(Error::BadRequest(format!(
            "A flow needs at least one module to run"
        )))?;
    }

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

#[async_recursion]
#[instrument(level = "trace", skip_all)]
async fn push_next_flow_job(
    flow_job: &QueuedJob,
    flow: FlowValue,
    schedule_path: Option<String>,
    db: &sqlx::Pool<sqlx::Postgres>,
    last_result: serde_json::Value,
) -> anyhow::Result<()> {
    let status: FlowStatus =
        serde_json::from_value::<FlowStatus>(flow_job.flow_status.clone().unwrap_or_default())
            .with_context(|| format!("parse flow status {}", flow_job.id))?;

    /* `mut` because reassigned on FlowStatusModule::Failure when failure_module is Some */

    let mut i = usize::try_from(status.step)
        .with_context(|| format!("invalid module index {}", status.step))?;

    let mut module: &FlowModule = flow
        .modules
        .get(i)
        .with_context(|| format!("no module at index {}", status.step))?;

    let mut status_module: FlowStatusModule = status
        .modules
        .get(i)
        .cloned()
        .with_context(|| format!("no status at index {}", status.step))?;

    tracing::debug!(
        "PUSH: module: {:#?}, status: {:#?}",
        module.value,
        status_module
    );

    if matches!(&status_module, FlowStatusModule::Success { .. }) {
        anyhow::bail!("no job for {status_module:?}")
    } else if matches!(&status_module, FlowStatusModule::Failure { .. }) {
        /* To run to the failure module, call push_next_flow_job with the current step on
         * FlowStatusModule::Failure.  This must update the step index to the end so that no
         * subsequent steps are run after the failure module. */
        i = flow.modules.len();
        module = flow
            .failure_module
            .as_ref()
            /* If this fails, it's a update_flow_status_in_progress shouldn't have called
             * handle_flow to get here. */
            .context("missing failure module")?;
        status_module = status.failure_module.clone();
    };

    /* Don't evaluate `module.input_transforms` after iteration has begun.  Instead, args are
     * carried through the Iterator by the InProgress variant. */

    #[rustfmt::skip]
    let compute_input_transform = !(   matches!(&module.value, FlowModuleValue::ForloopFlow { .. })
                                    && matches!(&status_module, FlowStatusModule::InProgress { .. }));

    let mut args = if compute_input_transform {
        let steps = status
            .modules
            .iter()
            .map(|x| match x {
                FlowStatusModule::Success { job, .. } => job.to_string(),
                _ => "invalid step status".to_string(),
            })
            .collect();

        let token = create_token_for_owner(
            &db,
            &flow_job.workspace_id,
            &flow_job.permissioned_as,
            "transform-input",
            10,
            &flow_job.created_by,
        )
        .await?;

        transform_input(
            &flow_job.args,
            last_result.clone(),
            &module.input_transforms,
            &flow_job.workspace_id,
            &token,
            steps,
        )
        .await?
    } else {
        Map::new()
    };

    /* forloop modules are expected set `iter: { value: Value, index: usize }` as job arguments */

    let next_loop_status: Option<NextLoopStatus> = if let FlowModuleValue::ForloopFlow {
        iterator,
        ..
    } = &module.value
    {
        match status_module {
            FlowStatusModule::WaitingForPriorSteps => {
                /* Iterator is an InputTransform, evaluate it into an array. */
                let itered = iterator
                    .clone()
                    .evaluate_with(|| vec![("result".to_string(), last_result.clone())])
                    .await?
                    .into_array()
                    .map_err(|not_array| {
                        Error::BadRequest(format!("Expected an array value, found: {not_array}"))
                    })?;

                let first = if let Some(first) = itered.first() {
                    first
                } else {
                    /* Nothing to iterate, complete immediately and bail. */
                    let next_step = i
                        .checked_add(1)
                        .filter(|i| (..flow.modules.len()).contains(i));

                    let new_job = sqlx::query_as::<_, QueuedJob>(
                        r#"
                    UPDATE queue
                       SET flow_status = JSONB_SET(
                                         JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2),
                                                                ARRAY['step'], $3)
                     WHERE id = $4
                 RETURNING *
                    "#,
                    )
                    .bind(status.step)
                    .bind(json!(FlowStatusModule::Success {
                        job: flow_job.id,
                        forloop_jobs: Some(vec![])
                    }))
                    .bind(json!(next_step.unwrap_or(i)))
                    .bind(flow_job.id)
                    .fetch_one(db)
                    .await?;

                    return if next_step.is_some() {
                        push_next_flow_job(&new_job, flow, schedule_path, db, json!([])).await
                    } else {
                        let success = true;
                        let skipped = false;
                        let logs = "Forloop completed without iteration".to_string();
                        let _uuid =
                            add_completed_job(db, &new_job, success, skipped, json!([]), logs)
                                .await?;
                        postprocess_queued_job(
                            false,
                            new_job.schedule_path,
                            new_job.script_path,
                            &new_job.workspace_id,
                            new_job.id,
                            db,
                        )
                        .await?;
                        Ok(())
                    };
                };

                args.insert("iter".to_string(), json!({ "index": 0, "value": first }));

                Some(NextLoopStatus { index: 0, itered, forloop_jobs: vec![] })
            }

            FlowStatusModule::InProgress {
                iterator: Some(Iterator { itered, index, args: iterator_args }),
                forloop_jobs: Some(forloop_jobs),
                ..
            } => {
                let (index, next) = index
                    .checked_add(1)
                    .and_then(|i| itered.get(i).map(|next| (i, next)))
                    /* we shouldn't get here because update_flow_status_after_job_completion
                     * should leave this state if there iteration is complete, but also it should
                     * be reasonable to just enter a completed state instead of failing, similar to
                     * iterating an empty list above */
                    .with_context(|| format!("could not iterate index {index} of {itered:?}"))?;

                args.extend(iterator_args);
                args.insert("iter".to_string(), json!({ "index": index, "value": next }));

                Some(NextLoopStatus { index, itered, forloop_jobs })
            }

            _ => Err(Error::BadRequest(format!(
                "Unrecognized module status for ForloopFlow {status_module:?}"
            )))?,
        }
    } else {
        None
    };

    if matches!(&module.value, FlowModuleValue::ForloopFlow { .. }) {
        if let Some(value) = &flow_job.args {
            value
                .as_object()
                .ok_or_else(|| {
                    Error::BadRequest(format!("Expected an object value, found: {value:?}"))
                })
                .map(|map| args.extend(map.clone()))?;
        }
    }

    /* Finally, push the job into the queue */

    let mut tx = db.begin().await?;

    let job_payload = match &module.value {
        FlowModuleValue::Script { path: script_path } => {
            script_path_to_payload(script_path, &mut tx, &flow_job.workspace_id).await?
        }
        FlowModuleValue::RawScript(raw_code) => {
            let mut raw_code = raw_code.clone();
            if raw_code.path.is_none() {
                raw_code.path = Some(format!("{}/{}", flow_job.script_path(), status.step));
            }
            JobPayload::Code(raw_code)
        }
        FlowModuleValue::ForloopFlow { value, .. } => JobPayload::RawFlow {
            value: (**value).clone(),
            path: Some(format!("{}/{}", flow_job.script_path(), status.step)),
        },
        a @ FlowModuleValue::Flow { .. } => {
            tracing::info!("Unrecognized module values {:?}", a);
            Err(Error::BadRequest(format!(
                "Unrecognized module values {:?}",
                a
            )))?
        }
    };

    let (uuid, mut tx) = push(
        tx,
        &flow_job.workspace_id,
        job_payload,
        Some(args.clone()),
        &flow_job.created_by,
        flow_job.permissioned_as.to_owned(),
        None,
        schedule_path,
        Some(flow_job.id),
        true,
    )
    .await?;

    let new_status =
        if let Some(NextLoopStatus { index, itered, mut forloop_jobs }) = next_loop_status {
            forloop_jobs.push(uuid);

            FlowStatusModule::InProgress {
                job: uuid,
                iterator: Some(Iterator { index, itered, args }),
                forloop_jobs: Some(forloop_jobs),
            }
        } else {
            FlowStatusModule::WaitingForExecutor { job: uuid }
        };

    sqlx::query(
        "
        UPDATE queue
           SET flow_status = JSONB_SET(
                             JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2),
                                                    ARRAY['step'], $3)
         WHERE id = $4
          ",
    )
    .bind(status.step)
    .bind(json!(new_status))
    .bind(json!(i))
    .bind(flow_job.id)
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    return Ok(());

    /// Some state about the current/last forloop FlowStatusModule used to initialized the next
    /// iteration's FlowStatusModule after pushing a job
    struct NextLoopStatus {
        index: usize,
        itered: Vec<Value>,
        forloop_jobs: Vec<Uuid>,
    }
}

impl InputTransform {
    async fn evaluate_with<F>(self, vars: F) -> anyhow::Result<Value>
    where
        F: FnOnce() -> Vec<(String, Value)>,
    {
        match self {
            InputTransform::Static { value } => Ok(value),
            InputTransform::Javascript { expr } => eval_timeout(expr, vars(), None, vec![]).await,
        }
    }
}

trait IntoArray: Sized {
    fn into_array(self) -> Result<Vec<Value>, Self>;
}

impl IntoArray for Value {
    fn into_array(self) -> Result<Vec<Value>, Self> {
        match self {
            Value::Array(array) => Ok(array),
            not_array => Err(not_array),
        }
    }
}
