use uuid::Uuid;
use windmill_common::{
    error::{self, Error},
    utils::WarnAfterExt,
    DB,
};

#[derive(Debug, Copy, Clone)]
pub enum Step {
    Step(usize),
    PreprocessorStep,
    FailureStep,
}

impl Step {
    pub fn from_i32_and_len(step: i32, len: usize) -> Self {
        if step < 0 {
            Step::PreprocessorStep
        } else if (step as usize) < len {
            Step::Step(step as usize)
        } else {
            Step::FailureStep
        }
    }
}

pub async fn update_flow_status_in_progress(
    db: &DB,
    _w_id: &str,
    flow: Uuid,
    job_in_progress: Uuid,
) -> error::Result<Step> {
    let step = get_step_of_flow_status(db, flow).await?;
    match step {
        Step::Step(step) => {
            sqlx::query!(
                "UPDATE v2_job_status SET
                    flow_status = jsonb_set(
                        jsonb_set(flow_status, ARRAY['modules', $3::INTEGER::TEXT, 'job'], to_jsonb($1::UUID::TEXT)),
                        ARRAY['modules', $3::INTEGER::TEXT, 'type'],
                        to_jsonb('InProgress'::text)
                    )
                WHERE id = $2",
                job_in_progress,
                flow,
                step as i32
            )
            .execute(db)
            .await?;
        }
        Step::PreprocessorStep => {
            sqlx::query!(
                "UPDATE v2_job_status SET
                    flow_status = jsonb_set(
                        jsonb_set(flow_status, ARRAY['preprocessor_module', 'job'], to_jsonb($1::UUID::TEXT)),
                        ARRAY['preprocessor_module', 'type'],
                        to_jsonb('InProgress'::text)
                    )
                WHERE id = $2",
                job_in_progress,
                flow
            )
            .execute(db)
            .await?;
        }
        Step::FailureStep => {
            sqlx::query!(
                "UPDATE v2_job_status SET
                    flow_status = jsonb_set(
                        jsonb_set(flow_status, ARRAY['failure_module', 'job'], to_jsonb($1::UUID::TEXT)),
                        ARRAY['failure_module', 'type'],
                        to_jsonb('InProgress'::text)
                    )
                WHERE id = $2",
                job_in_progress,
                flow
            )
            .execute(db)
            .await?;
        }
    }

    Ok(step)
}

pub async fn update_workflow_as_code_status(
    db: &DB,
    id: &Uuid,
    parent_job: &Uuid,
) -> error::Result<()> {
    let _ = sqlx::query_scalar!(
        "UPDATE v2_job_status SET
            workflow_as_code_status = jsonb_set(
                jsonb_set(
                    COALESCE(workflow_as_code_status, '{}'::jsonb),
                    array[$1],
                    COALESCE(workflow_as_code_status->$1, '{}'::jsonb)
                ),
                array[$1, 'started_at'],
                to_jsonb(now()::text)
                )
            WHERE id = $2",
        id.to_string(),
        parent_job
    )
    .execute(db)
    .warn_after_seconds(10)
    .await
    .inspect_err(|e| {
        tracing::error!(
            "Could not update parent job `started_at` in workflow as code status: {}",
            e
        )
    });
    Ok(())
}

// TODO: merge as a CTE
#[tracing::instrument(level = "trace", skip_all)]
async fn get_step_of_flow_status(db: &DB, id: Uuid) -> error::Result<Step> {
    let r = sqlx::query!(
        "SELECT (flow_status->'step')::integer as step, jsonb_array_length(flow_status->'modules') as len
        FROM v2_job_status WHERE id = $1",
        id
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::internal_err(format!("fetching step flow status: {e:#}")))?;

    if let Some(step) = r.step {
        Ok(Step::from_i32_and_len(step, r.len.unwrap_or(0) as usize))
    } else {
        Err(Error::internal_err("step is null".to_string()))
    }
}
