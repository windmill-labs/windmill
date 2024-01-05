use crate::{db::DB, error};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct JobStatsRecord {
    pub workspace_id: String,
    pub job_id: Uuid,
    pub metric_id: String,
    pub metric_name: Option<String>,
    pub metric_kind: MetricKind,
    pub scalar_int: Option<i32>,
    pub scalar_float: Option<f32>,
    pub timestamps: Option<Vec<chrono::DateTime<chrono::Utc>>>,
    pub timeseries_int: Option<Vec<i32>>,
    pub timeseries_float: Option<Vec<f32>>,
}

#[derive(sqlx::Type, Debug, PartialEq, Deserialize, Serialize)]
#[sqlx(type_name = "METRIC_KIND", rename_all = "snake_case")]
pub enum MetricKind {
    ScalarInt,
    ScalarFloat,
    TimeseriesInt,
    TimeseriesFloat,
}

pub enum MetricNumericValue {
    Integer(i32),
    Float(f32),
}

pub async fn register_metric_for_job(
    db: &DB,
    workspace_id: String,
    job_id: Uuid,
    metric_id: String,
    metric_kind: MetricKind,
    metric_name: Option<String>,
) -> error::Result<String> {
    let exists = sqlx::query_scalar!(
        "SELECT true FROM job_stats WHERE workspace_id = $1 AND job_id = $2 AND metric_id = $3",
        workspace_id,
        job_id,
        metric_id
    )
    .fetch_optional(db)
    .await?
    .flatten();
    if exists.unwrap_or(false) {
        return Err(error::Error::BadRequest(format!(
            "Metric {} is already registered for job {}",
            metric_id, job_id
        )));
    }

    let (scalar_int, scalar_float, timestamps, timeseries_int, timeseries_float) = match metric_kind
    {
        MetricKind::ScalarInt | MetricKind::ScalarFloat => {
            (None as Option<i32>, None as Option<f32>, None, None, None)
        }
        MetricKind::TimeseriesInt => (
            None,
            None,
            Some(&[] as &[chrono::DateTime<chrono::Utc>]),
            Some(&[] as &[i32]),
            None,
        ),
        MetricKind::TimeseriesFloat => (
            None,
            None,
            Some(&[] as &[chrono::DateTime<chrono::Utc>]),
            None,
            Some(&[] as &[f32]),
        ),
    };

    sqlx::query(
        "INSERT INTO job_stats (workspace_id, job_id, metric_id, metric_name, metric_kind, scalar_int, scalar_float, timestamps, timeseries_int, timeseries_float) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(workspace_id)
    .bind(job_id)
    .bind(&metric_id)
    .bind(metric_name)
    .bind(metric_kind)
    .bind(scalar_int)
    .bind(scalar_float)
    .bind(timestamps)
    .bind(timeseries_int)
    .bind(timeseries_float)
    .execute(db)
    .await?;

    Ok(metric_id)
}

pub async fn record_metric(
    db: &DB,
    workspace_id: String,
    job_id: Uuid,
    metric_id: String,
    value: MetricNumericValue,
) -> error::Result<()> {
    let metric_kind_opt: Option<MetricKind> = sqlx::query_scalar(
        "SELECT metric_kind FROM job_stats WHERE workspace_id = $1 AND job_id = $2 AND metric_id = $3",
    )
    .bind(&workspace_id)
    .bind(&job_id)
    .bind(&metric_id)
    .fetch_optional(db)
    .await?;

    if metric_kind_opt.is_none() {
        return Err(error::Error::BadRequest(format!(
            "Metric {} not yet registered for job {}.",
            metric_id, job_id
        )));
    }
    let metric_kind = metric_kind_opt.unwrap();

    let (value_int, value_float) = match value {
        MetricNumericValue::Integer(val) => {
            if metric_kind != MetricKind::TimeseriesInt && metric_kind != MetricKind::ScalarInt {
                return Err(error::Error::BadRequest(format!(
                    "Metric {} is not a timeseries int metric.",
                    metric_id
                )));
            }
            (val, 0 as f32)
        }
        MetricNumericValue::Float(val) => {
            if metric_kind != MetricKind::TimeseriesFloat && metric_kind != MetricKind::ScalarFloat
            {
                return Err(error::Error::BadRequest(format!(
                    "Metric {} is not a timeseries float metric.",
                    metric_id
                )));
            }
            (0 as i32, val)
        }
    };

    let timestamp = chrono::Utc::now();
    match metric_kind {
        MetricKind::ScalarInt => {
            sqlx::query!(
                "UPDATE job_stats SET scalar_int = $4 WHERE workspace_id = $1 AND job_id = $2 AND metric_id = $3",
                &workspace_id,
                &job_id,
                &metric_id,
                value_int,
            ).execute(db).await?;
        }
        MetricKind::ScalarFloat => {
            sqlx::query!(
                "UPDATE job_stats SET scalar_float = $4 WHERE workspace_id = $1 AND job_id = $2 AND metric_id = $3",
                &workspace_id,
                &job_id,
                &metric_id,
                value_float,
            ).execute(db).await?;
        }
        MetricKind::TimeseriesInt => {
            sqlx::query!(
                "UPDATE job_stats SET timestamps = timestamps || $4, timeseries_int = timeseries_int || $5 WHERE workspace_id = $1 AND job_id = $2 AND metric_id = $3",
                &workspace_id,
                &job_id,
                &metric_id,
                &[timestamp],
                &[value_int]
            ).execute(db).await?;
        }
        MetricKind::TimeseriesFloat => {
            sqlx::query!(
                "UPDATE job_stats SET timestamps = timestamps || $4, timeseries_float = timeseries_float || $5 WHERE workspace_id = $1 AND job_id = $2 AND metric_id = $3",
                &workspace_id,
                &job_id,
                &metric_id,
                &[timestamp],
                &[value_float]
            ).execute(db).await?;
        }
    }

    Ok(())
}
