use sqlx::types::Json;
use windmill_common::{error::Error, jobs::QueuedJob};
use windmill_parser_sql::parse_db_resource;
use windmill_queue::CanceledBy;
use serde_json::value::RawValue;

use crate::{common::{build_args_map, OccupancyMetrics}, AuthedClientBackgroundTask};



pub async fn do_oracledb(
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    query: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    column_order: &mut Option<Vec<String>>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> windmill_common::error::Result<Box<RawValue>> {
    let args = build_args_map(job, client, db).await?.map(Json);
    let job_args = if args.is_some() {
        args.as_ref()
    } else {
        job.args.as_ref()
    };

    let inline_db_res_path = parse_db_resource(&query);

    let db_arg = if let Some(inline_db_res_path) = inline_db_res_path {
        let val = client
            .get_authed()
            .await
            .get_resource_value_interpolated::<serde_json::Value>(
                &inline_db_res_path,
                Some(job.id.to_string()),
            )
            .await?;

        let as_raw = serde_json::from_value(val).map_err(|e| {
            Error::InternalErr(format!("Error while parsing inline resource: {e:#}"))
        })?;

        Some(as_raw)
    } else {
        job_args.and_then(|x| x.get("database").cloned())
    };

    todo!();
}
