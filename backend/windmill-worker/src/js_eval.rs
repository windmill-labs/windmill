/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{collections::HashMap, sync::Arc};

use serde_json::value::RawValue;
use sqlx::types::Json;
use uuid::Uuid;

use windmill_common::client::AuthedClient;
use windmill_common::worker::Connection;
use windmill_queue::CanceledBy;

use crate::common::{OccupancyMetrics, StreamNotifier};

#[cfg(feature = "deno_core")]
use crate::handle_child::run_future_with_polling_update_job_poller;

// Re-export IdContext from windmill-jseval for backward compatibility
pub use windmill_jseval::IdContext;

pub async fn eval_timeout(
    expr: String,
    transform_context: HashMap<String, Arc<Box<RawValue>>>,
    flow_input: Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
    flow_env: Option<&HashMap<String, Box<RawValue>>>,
    authed_client: Option<&AuthedClient>,
    by_id: Option<&IdContext>,
    ctx: Option<Vec<(String, String)>>,
) -> anyhow::Result<Box<RawValue>> {
    let expr = expr.trim().to_string();

    tracing::debug!(
        "evaluating js eval: {} with context {:?}",
        expr,
        transform_context
    );

    if let Some(value) = transform_context.get(&expr) {
        return Ok(value.as_ref().to_owned());
    }

    if let Some(value) =
        windmill_jseval::try_exact_property_access(&expr, flow_input.as_ref(), flow_env)
    {
        return Ok(value);
    }

    let p_ids = by_id.map(|x| {
        [
            format!("results.{}", x.previous_id),
            format!("results?.{}", x.previous_id),
            format!("results[\"{}\"]", x.previous_id),
            format!("results?.[\"{}\"]", x.previous_id),
        ]
    });

    if p_ids.is_some()
        && transform_context.contains_key("previous_result")
        && p_ids.as_ref().unwrap().iter().any(|x| x == &expr)
    {
        return Ok(transform_context
            .get("previous_result")
            .unwrap()
            .as_ref()
            .clone());
    }

    if let (Some(by_id), Some(authed_client)) = (by_id, authed_client) {
        if let Some(result) = windmill_jseval::handle_full_regex(&expr, authed_client, by_id).await
        {
            return result;
        }
    }

    let mut attempts = 0;
    loop {
        let result = windmill_jseval::eval_timeout_quickjs(
            expr.clone(),
            transform_context.clone(),
            flow_input.clone(),
            flow_env,
            authed_client,
            by_id,
            ctx.clone(),
        )
        .await;

        match result {
            Ok(v) => return Ok(v),
            Err(e) if attempts < 2 && e.to_string().contains("took too long") => {
                attempts += 1;
                tracing::warn!(
                    "js eval timed out (attempt {}/3), retrying in 5s: {}",
                    attempts,
                    expr
                );
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(feature = "deno_core")]
pub fn transpile_ts(expr: String) -> anyhow::Result<String> {
    windmill_runtime_nativets::transpile_ts(expr)
}

#[cfg(not(feature = "deno_core"))]
pub fn transpile_ts(_expr: String) -> anyhow::Result<String> {
    Ok("require deno".to_string())
}

#[cfg(not(feature = "deno_core"))]
pub async fn eval_fetch_timeout(
    _env_code: String,
    _ts_expr: String,
    _js_expr: String,
    _args: Option<&Json<HashMap<String, Box<RawValue>>>>,
    _script_entrypoint_override: Option<String>,
    _job_id: Uuid,
    _job_timeout: Option<i32>,
    _conn: &Connection,
    _mem_peak: &mut i32,
    _canceled_by: &mut Option<CanceledBy>,
    _worker_name: &str,
    _w_id: &str,
    _load_client: bool,
    _occupation_metrics: &mut OccupancyMetrics,
    _stream_notifier: Option<StreamNotifier>,
    _has_stream: &mut bool,
) -> anyhow::Result<Box<RawValue>> {
    use serde_json::value::to_raw_value;
    Ok(to_raw_value("require deno_core").unwrap())
}

#[cfg(feature = "deno_core")]
pub async fn eval_fetch_timeout(
    env_code: String,
    ts_expr: String,
    js_expr: String,
    args: Option<&Json<HashMap<String, Box<RawValue>>>>,
    script_entrypoint_override: Option<String>,
    job_id: Uuid,
    job_timeout: Option<i32>,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    w_id: &str,
    load_client: bool,
    occupation_metrics: &mut OccupancyMetrics,
    stream_notifier: Option<StreamNotifier>,
    has_stream: &mut bool,
) -> windmill_common::error::Result<Box<RawValue>> {
    let otel_initialized = {
        #[cfg(all(feature = "private", feature = "enterprise"))]
        {
            crate::DENO_OTEL_INITIALIZED.load(std::sync::atomic::Ordering::SeqCst)
        }
        #[cfg(not(all(feature = "private", feature = "enterprise")))]
        {
            false
        }
    };

    let stream_notifier_update: Option<Arc<dyn Fn() + Send + Sync + 'static>> = stream_notifier
        .map(|sn| {
            Arc::new(move || {
                sn.update_flow_status_with_stream_job();
            }) as Arc<dyn Fn() + Send + Sync + 'static>
        });

    let result_f = windmill_runtime_nativets::eval_fetch_timeout(
        env_code,
        ts_expr,
        js_expr,
        args,
        script_entrypoint_override,
        job_id,
        conn,
        w_id,
        load_client,
        otel_initialized,
        stream_notifier_update,
    );

    let (res, new_has_stream) = run_future_with_polling_update_job_poller(
        job_id,
        job_timeout,
        conn,
        mem_peak,
        canceled_by,
        result_f,
        worker_name,
        w_id,
        &mut Some(occupation_metrics),
        Box::pin(futures::stream::once(async { 0 })),
    )
    .await?;
    *has_stream = new_has_stream;
    *mem_peak = (res.get().len() / 1000) as i32;
    Ok(res)
}
