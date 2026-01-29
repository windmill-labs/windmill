/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{collections::HashMap, sync::Arc};

use lazy_static::lazy_static;
use regex::Regex;
use serde_json::value::RawValue;
use sqlx::types::Json;
use uuid::Uuid;

use windmill_common::flow_status::JobResult;
use windmill_queue::CanceledBy;

use crate::common::{OccupancyMetrics, StreamNotifier};
use windmill_common::client::AuthedClient;

#[cfg(feature = "deno_core")]
use crate::handle_child::run_future_with_polling_update_job_poller;

#[derive(Debug, Clone)]
pub struct IdContext {
    pub flow_job: Uuid,
    #[allow(dead_code)]
    pub steps_results: HashMap<String, JobResult>,
    pub previous_id: String,
}

const FLOW_INPUT_PREFIX: &'static str = "flow_input";
const ENV_KEY_PREFIX: &'static str = "flow_env";
const DOT_PATTERN: &'static str = ".";
const START_BRACKET_PATTERN: &'static str = "[\"";
const END_BRACKET_PATTERN: &'static str = "\"]";

fn try_exact_property_access(
    expr: &str,
    flow_input: Option<&mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
    flow_env: Option<&HashMap<String, Box<RawValue>>>,
) -> Option<Box<RawValue>> {
    let obj = if expr.starts_with(FLOW_INPUT_PREFIX) {
        Some((
            FLOW_INPUT_PREFIX,
            flow_input.as_ref().map(|obj| obj.as_ref()),
        ))
    } else if expr.starts_with(ENV_KEY_PREFIX) {
        Some((ENV_KEY_PREFIX, flow_env))
    } else {
        None
    };

    if let Some((prefix, obj)) = obj {
        let access_pattern_pos = prefix.len();
        let suffix = &expr[access_pattern_pos..];
        let maybe_key_name = if suffix.starts_with(DOT_PATTERN) {
            let key_name_pos = DOT_PATTERN.len();
            Some(&suffix[key_name_pos..])
        } else if suffix.starts_with(START_BRACKET_PATTERN) {
            let key_name_pos = START_BRACKET_PATTERN.len();
            let suffix = &suffix[key_name_pos..];

            let flow_arg_name = suffix
                .ends_with(END_BRACKET_PATTERN)
                .then(|| {
                    let start_key_name_pos = access_pattern_pos + key_name_pos;
                    let end_key_name_pos = expr.len() - END_BRACKET_PATTERN.len();
                    &expr[start_key_name_pos..end_key_name_pos]
                })
                .filter(|s| s.len() > 0);
            flow_arg_name
        } else {
            None
        };

        if let Some(key_name) = maybe_key_name {
            if let Some(key_value) = obj.and_then(|obj| obj.get(key_name)) {
                return Some(key_value.clone());
            }
        }
    }
    None
}

async fn handle_full_regex(
    expr: &str,
    authed_client: &AuthedClient,
    by_id: &IdContext,
) -> Option<anyhow::Result<Box<RawValue>>> {
    if let Some(captures) = RE_FULL.captures(&expr) {
        let obj_name = captures.get(1).unwrap().as_str();
        let obj_key = captures.get(2).unwrap().as_str();
        let idx_o = captures.get(3).map(|y| y.as_str());
        let rest = captures.get(4).map(|y| y.as_str());
        let query = if let Some(idx) = idx_o {
            match rest {
                Some(rest) => Some(format!("{}{}", idx, rest)),
                None => Some(idx.to_string()),
            }
        } else {
            rest.map(|x| x.trim_start_matches('.').to_string())
        };

        let result = if obj_name == "results" {
            // Use .ok() to match deno_core op_get_id behavior: return null for non-existent steps
            // instead of throwing an error
            let res = authed_client
                .get_result_by_id::<Option<Box<RawValue>>>(&by_id.flow_job.to_string(), obj_key, query)
                .await
                .ok()
                .flatten();
            match res {
                Some(v) => Ok(v),
                None => serde_json::value::to_raw_value(&serde_json::Value::Null)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize null: {}", e)),
            }
        } else if obj_name == "flow_env" {
            authed_client
                .get_flow_env_by_flow_job_id(&by_id.flow_job.to_string(), obj_key, query)
                .await
        } else {
            unreachable!();
        };

        return Some(result);
    }

    return None;
}

pub async fn eval_timeout(
    expr: String,
    transform_context: HashMap<String, Arc<Box<RawValue>>>,
    flow_input: Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
    flow_env: Option<&HashMap<String, Box<RawValue>>>,
    authed_client: Option<&AuthedClient>,
    by_id: Option<&IdContext>,
    #[allow(unused_variables)] ctx: Option<Vec<(String, String)>>,
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

    if let Some(value) = try_exact_property_access(&expr, flow_input.as_ref(), flow_env) {
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
        if let Some(result) = handle_full_regex(&expr, authed_client, by_id).await {
            return result;
        }
    }

    // Flow expressions are always evaluated with QuickJS (faster startup, lower memory)
    crate::js_eval_quickjs::eval_timeout_quickjs(
        expr,
        transform_context,
        flow_input,
        flow_env,
        authed_client,
        by_id,
        ctx,
    )
    .await
}

pub fn replace_with_await(expr: String, fn_name: &str) -> String {
    let sep = format!("{}(", fn_name);
    let mut split = expr.split(&sep);
    let mut s = split.next().unwrap_or_else(|| "").to_string();
    for x in split {
        s.push_str(&format!("(await {}({}", fn_name, add_closing_bracket(x)))
    }
    s
}

lazy_static! {
    static ref RE: Regex = Regex::new(
        r#"(?m)(?P<r>(?:results|flow_env)(?:\?)?(?:(?:\.[a-zA-Z_0-9]+)|(?:\[\".*?\"\])))"#
    )
    .unwrap();
    static ref RE_FULL: Regex = Regex::new(
        r"(?m)^(results|flow_env)(?:\?)?\.([a-zA-Z_0-9]+)(?:\[(\d+)\])?((?:\.[a-zA-Z_0-9]+)+)?$"
    )
    .unwrap();
}

pub fn replace_with_await_result(expr: String) -> String {
    RE.replace_all(&expr, "(await $r)").to_string()
}

fn add_closing_bracket(s: &str) -> String {
    let mut s = s.to_string();
    let mut level = 1;
    let mut idx = 0;
    for c in s.chars() {
        match c {
            '(' => level += 1,
            ')' => level -= 1,
            _ => (),
        };
        if level == 0 {
            break;
        }
        idx += 1;
    }
    s.insert_str(idx, ")");
    s
}

/// Transpile TypeScript to JavaScript using deno_ast (requires deno_core feature)
#[cfg(feature = "deno_core")]
pub fn transpile_ts(expr: String) -> anyhow::Result<String> {
    windmill_nativets::transpile_ts(expr)
}

#[cfg(not(feature = "deno_core"))]
pub fn transpile_ts(_expr: String) -> anyhow::Result<String> {
    Ok("require deno".to_string())
}

use windmill_common::worker::Connection;

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

/// Execute a NativeTS script using deno_core/V8
///
/// This function wraps the windmill-nativets crate execution with
/// job polling, cancellation handling, and log/stream processing.
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
    _load_client: bool,
    occupation_metrics: &mut OccupancyMetrics,
    stream_notifier: Option<StreamNotifier>,
    has_stream: &mut bool,
) -> windmill_common::error::Result<Box<RawValue>> {
    // Call into windmill-nativets to set up the execution
    let (mut isolate_receiver, channels, result_handle, extra_logs) =
        windmill_nativets::eval_fetch_timeout(
            env_code,
            ts_expr,
            js_expr,
            args,
            script_entrypoint_override,
            job_id,
        )
        .await?;

    // Destructure channels to use in separate tasks
    let windmill_nativets::NativeTsChannels {
        mut logs_receiver,
        mut result_stream_receiver,
    } = channels;

    // Spawn task to handle logs
    let conn_ = conn.clone();
    let w_id_ = w_id.to_string();
    tokio::spawn(async move {
        while let Some(log) = logs_receiver.recv().await {
            windmill_queue::append_logs(&job_id, &w_id_, log, &conn_).await
        }
    });

    // Spawn task to handle result streams
    let conn_ = conn.clone();
    let w_id_ = w_id.to_string();
    let stream_notifier_clone = stream_notifier.clone();
    tokio::spawn(async move {
        use crate::job_logger::append_result_stream;
        let mut offset = -1;
        let mut is_stream = false;
        while let Some(stream) = result_stream_receiver.recv().await {
            if let Some(sn) = stream_notifier_clone.as_ref() {
                if !is_stream {
                    is_stream = true;
                    sn.update_flow_status_with_stream_job();
                }
            }
            offset += 1;
            if let Err(e) = append_result_stream(&conn_, &w_id_, &job_id, &stream, offset).await {
                tracing::error!("failed to append result stream: {e}");
            }
        }
    });

    // Log extra info (user agent, proxy)
    if !extra_logs.is_empty() {
        windmill_queue::append_logs(&job_id, w_id, extra_logs, conn).await;
    }

    // Run the execution with job polling and cancellation support
    let (res, new_has_stream) = run_future_with_polling_update_job_poller(
        job_id,
        job_timeout,
        conn,
        mem_peak,
        canceled_by,
        async { result_handle.await.map_err(windmill_common::error::to_anyhow)? },
        worker_name,
        w_id,
        &mut Some(occupation_metrics),
        Box::pin(futures::stream::once(async { 0 })),
    )
    .await
    .map_err(|e| {
        // Try to terminate the isolate if the job was cancelled/timed out
        if let Ok(isolate) = isolate_receiver.try_recv() {
            isolate.terminate_execution();
        }
        e
    })?;

    *has_stream = new_has_stream;
    *mem_peak = (res.get().len() / 1000) as i32;

    // Merge result stream if present
    if new_has_stream {
        Ok(res)
    } else {
        Ok(res)
    }
}

// Tests for eval_timeout using QuickJS are in js_eval_quickjs.rs
// Tests for eval_fetch_timeout using deno_core can be added here if needed
