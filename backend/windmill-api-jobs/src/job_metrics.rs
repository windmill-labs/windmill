use windmill_common::DB;

use axum::{
    extract::Path,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;
use windmill_common::{
    error::{self, Error},
    job_metrics::{
        record_metric, register_metric_for_job, JobStatsRecord, MetricKind, MetricNumericValue,
    },
};

pub fn workspaced_service() -> Router {
    let cors = CorsLayer::new()
        .allow_methods([http::Method::GET, http::Method::POST])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
        .allow_origin(Any);

    Router::new()
        .route("/get/:id", post(get_job_metrics).layer(cors.clone()))
        .route(
            "/set_progress/:id",
            post(set_job_progress).layer(cors.clone()),
        )
        .route(
            "/get_progress/:id",
            get(get_job_progress).layer(cors.clone()),
        )
}

#[derive(Deserialize)]
struct JobStatsRequest {
    from_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    to_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    timeseries_max_datapoints: Option<u32>, // default to 100, any value lower than that will be ignored. Set to 0 to retrieve all
}

#[derive(Serialize)]
struct JobStatsResponse {
    metrics_metadata: Vec<MetricsMetadata>,
    scalar_metrics: Vec<ScalarMetric>,
    timeseries_metrics: Vec<TimeseriesMetric>,
}

#[derive(Serialize)]
pub struct MetricsMetadata {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Serialize)]
struct ScalarMetric {
    metric_id: String,
    value: f64,
}

#[derive(Serialize)]
struct TimeseriesMetric {
    metric_id: String,
    values: Vec<DataPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataPoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: f64,
}

async fn get_job_metrics(
    Extension(db): Extension<DB>,
    Path((w_id, job_id)): Path<(String, Uuid)>,
    Json(JobStatsRequest { from_timestamp, to_timestamp, timeseries_max_datapoints }): Json<
        JobStatsRequest,
    >,
) -> error::JsonResult<JobStatsResponse> {
    let records = sqlx::query_as::<_, JobStatsRecord>(
        "SELECT * FROM job_stats where workspace_id = $1 and job_id = $2",
    )
    .bind(w_id)
    .bind(job_id)
    .fetch_all(&db)
    .await?;

    let mut metrics_metadata: Vec<MetricsMetadata> = vec![];
    let mut scalar_metrics: Vec<ScalarMetric> = vec![];
    let mut timeseries_metrics: Vec<TimeseriesMetric> = vec![];

    for record in records {
        let metric_id = record.metric_id;
        match record.metric_kind {
            MetricKind::ScalarInt => {
                let value = record.scalar_int.unwrap_or_default() as f64;
                scalar_metrics.push(ScalarMetric { metric_id: metric_id.clone(), value });
            }
            MetricKind::ScalarFloat => {
                let value = record.scalar_float.unwrap_or_default() as f64;
                scalar_metrics.push(ScalarMetric { metric_id: metric_id.clone(), value });
            }
            MetricKind::TimeseriesInt => {
                if record.timestamps.clone().unwrap_or_default().len()
                    != record.timeseries_int.clone().unwrap_or_default().len()
                {
                    tracing::warn!("Timeseries metric {} has an invalid shape. It doesn't have one timestamp per measurement. (timestamps: {:?}, measurements: {:?})", metric_id, record.timestamps, record.timeseries_int)
                }
                let (timestamps, timeseries_int) = timeseries_sample(
                    from_timestamp,
                    to_timestamp,
                    timeseries_max_datapoints,
                    record.timestamps.unwrap_or_default(),
                    record.timeseries_int.unwrap_or_default(),
                );
                let mut values: Vec<DataPoint> = vec![];
                for (idx, value) in timeseries_int.iter().enumerate() {
                    values.push(DataPoint {
                        timestamp: timestamps[idx],
                        value: value.to_owned() as f64,
                    });
                }
                timeseries_metrics.push(TimeseriesMetric { metric_id: metric_id.clone(), values });
            }
            MetricKind::TimeseriesFloat => {
                if record.timestamps.clone().unwrap_or_default().len()
                    != record.timeseries_int.clone().unwrap_or_default().len()
                {
                    tracing::warn!("Timeseries metric {} has an invalid shape. It doesn't have one timestamp per measurement. (timestamps: {:?}, measurements: {:?})", metric_id, record.timestamps, record.timeseries_float)
                }
                let (timestamps, timeseries_float) = timeseries_sample(
                    from_timestamp,
                    to_timestamp,
                    timeseries_max_datapoints,
                    record.timestamps.unwrap_or_default(),
                    record.timeseries_float.unwrap_or_default(),
                );
                let mut values: Vec<DataPoint> = vec![];
                for (idx, value) in timeseries_float.iter().enumerate() {
                    values.push(DataPoint {
                        timestamp: timestamps[idx],
                        value: value.to_owned() as f64,
                    });
                }
                timeseries_metrics.push(TimeseriesMetric { metric_id: metric_id.clone(), values });
            }
        };
        metrics_metadata.push(MetricsMetadata { id: metric_id, name: record.metric_name });
    }

    let response = JobStatsResponse { metrics_metadata, scalar_metrics, timeseries_metrics };
    Ok(Json(response))
}
#[derive(Deserialize)]
struct JobProgressSetRequest {
    percent: i32,
    /// Optional parent flow id
    /// Used to modify flow status
    /// Specifically `progress` field in corresponding FlowStatusModule in `InProgress` state
    flow_job_id: Option<Uuid>,
}

async fn set_job_progress(
    Extension(db): Extension<DB>,
    Path((w_id, job_id)): Path<(String, Uuid)>,
    Json(JobProgressSetRequest { percent, flow_job_id }): Json<JobProgressSetRequest>,
) -> error::JsonResult<()> {
    // If flow_job_id exists, than we should modify flow_status of corresponding module
    // Individual jobs and flows are handled differently
    if let Some(flow_job_id) = flow_job_id {
        // TODO: Return error if trying to set completed job?
        sqlx::query!(
            "UPDATE v2_job_status
                SET flow_status = JSONB_SET(flow_status, ARRAY['modules', flow_status->>'step', 'progress'], $1)
                WHERE id = $2",
            serde_json::json!(percent.clamp(0, 99)),
            flow_job_id
        )
        .execute(&db)
        .await?;
    }

    let record_progress = || {
        record_metric(
            &db,
            w_id.clone(),
            job_id,
            "progress_perc".to_owned(),
            MetricNumericValue::Integer(percent),
        )
    };

    // Try to record
    if let Err(err) = record_progress().await {
        if matches!(err, Error::MetricNotFound(..)) {
            // Register
            // TODO: Reset progress after job is finished (in case it reruns same job)?
            _ = register_metric_for_job(
                &db,
                w_id.clone(),
                job_id,
                "progress_perc".to_string(),
                MetricKind::ScalarInt,
                Some("Job Execution Progress (%)".to_owned()),
            )
            .await?;
            // Retry recording progress
            record_progress().await.map_err(|err| {
                // If for some reason it still returns same error, this error will be converted to BadRequest and returned
                if let Error::MetricNotFound(body) = err {
                    Error::BadRequest(body)
                } else {
                    err
                }
            })?;
        } else {
            return Err(err);
        }
    };
    return Ok(Json(()));
}

async fn get_job_progress(
    Extension(db): Extension<DB>,
    Path((w_id, job_id)): Path<(String, Uuid)>,
) -> error::JsonResult<Option<i32>> {
    let progress: Option<Option<i32>> = sqlx::query_scalar!(
            "SELECT (scalar_int)::int FROM job_stats WHERE job_id = $1 AND workspace_id = $2 AND metric_id = 'progress_perc'",
            job_id, w_id)
            .fetch_optional(&db)
            .await?;

    let respond_value = if let Some(Some(progress)) = progress {
        Some(progress.clamp(0, 99))
    } else {
        None
    };

    Ok(Json(respond_value))
}

fn timeseries_sample<T: Copy>(
    from: Option<chrono::DateTime<chrono::Utc>>,
    to: Option<chrono::DateTime<chrono::Utc>>,
    _datapoints: Option<u32>,
    timestamps: Vec<chrono::DateTime<chrono::Utc>>,
    values: Vec<T>,
) -> (Vec<chrono::DateTime<chrono::Utc>>, Vec<T>) {
    if timestamps.len() != values.len() {
        tracing::warn!("Timeseries metric has an invalid shape. It doesn't have one timestamp per measurement. (timestamps: {:?}, measurements: {:?})", timestamps.len(), values.len());
        return (vec![], vec![]);
    }
    let mut filtered_timestamp: Vec<chrono::DateTime<chrono::Utc>> = vec![];
    let mut filtered_values: Vec<T> = vec![];
    for (idx, timestamp) in timestamps.iter().enumerate() {
        if *timestamp > from.unwrap_or(chrono::DateTime::<chrono::Utc>::MIN_UTC)
            && *timestamp < to.unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC)
        {
            filtered_timestamp.push(timestamps[idx]);
            filtered_values.push(values[idx]);
        }
    }
    // TODO: implement sampling
    return (filtered_timestamp, filtered_values);
}
