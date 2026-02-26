/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! QuickJS-based JavaScript expression evaluation for flow transformations.
//!
//! This crate provides fast, lightweight JS expression evaluation using QuickJS (rquickjs).
//! It is used by both windmill-worker (for flow expression eval) and windmill-api (for batch rerun).
//!
//! ## Performance Characteristics (release mode benchmarks)
//! - **Simple expressions**: ~238μs (QuickJS) vs ~3.05ms (deno_core) = **~13x faster**
//! - **Complex expressions**: ~192μs (QuickJS) vs ~3.09ms (deno_core) = **~16x faster**
//! - **Memory**: ~2.5% of V8's footprint

use std::collections::HashMap;
use std::sync::Arc;

pub const EVAL_TIMEOUT_MS: u64 = 20000;

use lazy_static::lazy_static;
use regex::Regex;
#[cfg(feature = "quickjs")]
use rquickjs::{
    async_with,
    prelude::{Async, Func, MutFn},
    AsyncContext, AsyncRuntime, CatchResultExt, FromJs, IntoJs, Object, Value,
};
use serde_json::value::RawValue;
use uuid::Uuid;

use windmill_common::client::AuthedClient;
use windmill_common::flow_status::JobResult;

// ── Public types ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IdContext {
    pub flow_job: Uuid,
    #[allow(dead_code)]
    pub steps_results: HashMap<String, JobResult>,
    pub previous_id: String,
}

// ── Constants ─────────────────────────────────────────────────────────

const FLOW_INPUT_PREFIX: &str = "flow_input";
const ENV_KEY_PREFIX: &str = "flow_env";
const DOT_PATTERN: &str = ".";
const START_BRACKET_PATTERN: &str = "[\"";
const END_BRACKET_PATTERN: &str = "\"]";

// ── Regex statics ─────────────────────────────────────────────────────

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

// ── Shared helper functions (quickjs-only) ──────────────────────────

#[cfg(feature = "quickjs")]
pub fn replace_with_await(expr: String, fn_name: &str) -> String {
    let sep = format!("{}(", fn_name);
    let mut split = expr.split(&sep);
    let mut s = split.next().unwrap_or("").to_string();
    for x in split {
        s.push_str(&format!("(await {}({}", fn_name, add_closing_bracket(x)))
    }
    s
}

#[cfg(feature = "quickjs")]
pub fn replace_with_await_result(expr: String) -> String {
    RE.replace_all(&expr, "(await $r)").to_string()
}

#[cfg(feature = "quickjs")]
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

pub fn try_exact_property_access(
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

pub async fn handle_full_regex(
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
            let res = authed_client
                .get_result_by_id::<Option<Box<RawValue>>>(
                    &by_id.flow_job.to_string(),
                    obj_key,
                    query,
                )
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

#[cfg(feature = "quickjs")]
use windmill_common::utils::unsafe_raw;

// ── QuickJS evaluation ───────────────────────────────────────────────

#[cfg(feature = "quickjs")]

/// Shared state for async operations within QuickJS
#[derive(Clone)]
struct AsyncOpState {
    client: AuthedClient,
}

#[cfg(feature = "quickjs")]
pub async fn eval_timeout_quickjs(
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
        "evaluating js eval (quickjs): {} with context {:?}",
        expr,
        transform_context
    );

    // Clone data for the blocking task
    let by_id_clone = by_id.cloned();
    let flow_input_clone = flow_input.clone();
    let flow_env_clone = flow_env.cloned();
    let authed_client_clone = authed_client.cloned();

    // Determine which context keys are actually used in the expression
    let p_ids = by_id.map(|x| {
        [
            format!("results.{}", x.previous_id),
            format!("results?.{}", x.previous_id),
            format!("results[\"{}\"]", x.previous_id),
            format!("results?.[\"{}\"]", x.previous_id),
        ]
    });

    let mut context_keys: Vec<String> = transform_context
        .keys()
        .filter(|x| expr.contains(&x.to_string()))
        .cloned()
        .collect();

    if (!context_keys.contains(&"previous_result".to_string())
        && p_ids.is_some()
        && p_ids.as_ref().unwrap().iter().any(|x| expr.contains(x)))
        || expr.contains("error")
    {
        context_keys.push("previous_result".to_string());
    }

    let has_flow_input = expr.contains("flow_input");
    if has_flow_input {
        context_keys.push("flow_input".to_string())
    }

    // Filter transform_context to only include used keys
    let filtered_context: HashMap<String, Arc<Box<RawValue>>> = transform_context
        .into_iter()
        .filter(|(k, _)| context_keys.contains(k))
        .collect();

    let expr_clone = expr.clone();

    // Run the QuickJS evaluation with a timeout.
    // Use the current runtime handle rather than creating an independent
    // current-thread runtime so that HTTP connections opened by the global
    // reqwest client (HTTP_CLIENT) are managed by the main runtime.
    // Creating a child runtime caused connection-pool dispatch tasks to be
    // dropped when the child runtime exited, poisoning pooled connections
    // and producing spurious "DispatchGone" / "runtime dropped the dispatch
    // task" errors on subsequent requests from the main runtime.
    let handle = tokio::runtime::Handle::current();
    tokio::time::timeout(
        std::time::Duration::from_millis(EVAL_TIMEOUT_MS),
        tokio::task::spawn_blocking(move || {
            handle.block_on(async move {
                eval_quickjs_inner(
                    &expr_clone,
                    filtered_context,
                    flow_input_clone,
                    flow_env_clone,
                    authed_client_clone,
                    by_id_clone,
                    ctx,
                    context_keys,
                )
                .await
            })
        }),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "The expression evaluation `{expr}` took too long to execute (>{EVAL_TIMEOUT_MS}ms)"
        )
    })??
}

#[cfg(feature = "quickjs")]
const QUICKJS_MEMORY_LIMIT: usize = 32 * 1024 * 1024;

#[cfg(feature = "quickjs")]
async fn eval_quickjs_inner(
    expr: &str,
    transform_context: HashMap<String, Arc<Box<RawValue>>>,
    flow_input: Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
    flow_env: Option<HashMap<String, Box<RawValue>>>,
    authed_client: Option<AuthedClient>,
    by_id: Option<IdContext>,
    extra_ctx: Option<Vec<(String, String)>>,
    context_keys: Vec<String>,
) -> anyhow::Result<Box<RawValue>> {
    let runtime = AsyncRuntime::new()?;
    runtime.set_memory_limit(QUICKJS_MEMORY_LIMIT).await;
    let context = AsyncContext::full(&runtime).await?;

    // Create shared state for async ops if we have a client
    let op_state = authed_client.map(|client| Arc::new(AsyncOpState { client }));

    let op_state_clone = op_state.clone();
    let by_id_clone = by_id.clone();

    // Transform expression to add await for variable/resource/results access
    let expr_with_funcs = ["variable", "resource"]
        .into_iter()
        .fold(expr.to_string(), replace_with_await);
    let transformed_expr = replace_with_await_result(expr_with_funcs);

    async_with!(context => |ctx| {
        let globals = ctx.globals();

        // Set up context variables
        for key in &context_keys {
            if key == "flow_input" {
                if let Some(ref fi) = flow_input {
                    let json_str = serde_json::to_string(fi.as_ref())?;
                    let val: serde_json::Value = serde_json::from_str(&json_str)?;
                    let js_val = json_to_js(&ctx, &val)?;
                    globals.set(key.as_str(), js_val)?;
                } else {
                    globals.set(key.as_str(), Value::new_null(ctx.clone()))?;
                }
            } else if let Some(raw_val) = transform_context.get(key) {
                let val: serde_json::Value = serde_json::from_str(raw_val.get())?;
                let js_val = json_to_js(&ctx, &val)?;
                globals.set(key.as_str(), js_val)?;
            }
        }

        // Set up flow_env if referenced
        if expr.contains("flow_env") {
            if let Some(ref fe) = flow_env {
                let obj = Object::new(ctx.clone())?;
                for (k, v) in fe {
                    let val: serde_json::Value = serde_json::from_str(v.get())?;
                    let js_val = json_to_js(&ctx, &val)?;
                    obj.set(k.as_str(), js_val)?;
                }
                globals.set("flow_env", obj)?;
            } else {
                globals.set("flow_env", Object::new(ctx.clone())?)?;
            }
        }

        // Set up additional context variables
        if let Some(ctx_vars) = extra_ctx {
            for (k, v) in ctx_vars {
                globals.set(k.as_str(), v.as_str())?;
            }
        }

        // Set up error extraction if needed
        if expr.contains("error") && context_keys.contains(&"previous_result".to_string()) {
            let error_setup = r#"
                let error = previous_result?.error;
                if (!error) {
                    if (Array.isArray(previous_result)) {
                        const errors = previous_result.filter(item => item && typeof item === 'object' && 'error' in item);
                        if (errors.length === 1) {
                            error = errors[0].error;
                        } else if (errors.length > 1) {
                            error = {
                                name: 'MultipleErrors',
                                message: errors.map(({ error: e }, i) => `[${e.step_id || i}] ${e.message || e.name}`).join('; '),
                                errors: previous_result
                            };
                        } else {
                            error = {
                                name: 'MultipleErrors',
                                message: "Could not parse errors",
                                errors: previous_result
                            };
                        }
                    } else {
                        if (previous_result) {
                            error = { name: 'UnknownError', message: 'Could not parse the error', error: previous_result };
                        } else {
                            error = { name: 'UnknownError', message: 'No error found' };
                        }
                    }
                }
            "#;
            ctx.eval::<(), _>(error_setup).catch(&ctx).map_err(quickjs_error_to_anyhow)?;
        }

        // Set up async functions if we have a client
        if let Some(ref state) = op_state_clone {
            setup_async_ops(&ctx, &globals, state.clone())?;
        } else {
            // Set up stub functions that throw errors
            setup_stub_functions(&ctx, &globals)?;
        }

        // Set up results proxy if we have by_id context
        if let Some(ref by_id) = by_id_clone {
            setup_results_proxy(&ctx, &globals, by_id, op_state_clone.clone())?;
        }

        // Determine if we need to add return statement.
        let code = if should_add_return_quickjs(&transformed_expr) {
            format!("(async function() {{ return {}; }})().then((x) => JSON.stringify(x ?? null))", transformed_expr)
        } else {
            format!("(async function() {{ {} }})().then((x) => JSON.stringify(x ?? null))", transformed_expr)
        };

        // Evaluate the expression (returns a Promise that resolves to a JSON string)
        let promise: rquickjs::Promise = ctx.eval(code).catch(&ctx).map_err(quickjs_error_to_anyhow)?;

        // Await the promise
        let result: Value = promise.into_future().await.catch(&ctx).map_err(quickjs_error_to_anyhow)?;

        let json_str = String::from_js(&ctx, result)
            .unwrap_or_else(|_| "null".to_string());

        Ok(unsafe_raw(json_str))
    })
    .await
}

#[cfg(feature = "quickjs")]
fn setup_async_ops<'js>(
    ctx: &rquickjs::Ctx<'js>,
    globals: &Object<'js>,
    state: Arc<AsyncOpState>,
) -> anyhow::Result<()> {
    const ERR_PREFIX: &str = "\x00__WINDMILL_ERR__\x00";

    let state_for_var = state.clone();
    globals.set(
        "__fetchVariable",
        Func::from(Async(MutFn::new(move |path: String| {
            let client = state_for_var.client.clone();
            async move {
                match client.get_variable_value(&path).await {
                    Ok(value) => value,
                    Err(e) => format!("{}{}", ERR_PREFIX, e),
                }
            }
        }))),
    )?;

    let state_for_res = state.clone();
    globals.set(
        "__fetchResource",
        Func::from(Async(MutFn::new(move |path: String| {
            let client = state_for_res.client.clone();
            async move {
                match client
                    .get_resource_value_interpolated::<serde_json::Value>(&path, None)
                    .await
                {
                    Ok(value) => {
                        serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string())
                    }
                    Err(e) => format!("{}{}", ERR_PREFIX, e),
                }
            }
        }))),
    )?;

    let wrapper_code = r#"
        const __ERR_PREFIX = '\x00__WINDMILL_ERR__\x00';

        async function variable(path) {
            const result = await __fetchVariable(path);
            if (typeof result === 'string' && result.startsWith(__ERR_PREFIX)) {
                throw new Error(result.substring(__ERR_PREFIX.length));
            }
            return result;
        }

        async function resource(path) {
            const result = await __fetchResource(path);
            if (typeof result === 'string' && result.startsWith(__ERR_PREFIX)) {
                throw new Error(result.substring(__ERR_PREFIX.length));
            }
            return JSON.parse(result);
        }
    "#;

    ctx.eval::<(), _>(wrapper_code)
        .catch(ctx)
        .map_err(quickjs_error_to_anyhow)?;

    Ok(())
}

#[cfg(feature = "quickjs")]
fn setup_stub_functions<'js>(
    ctx: &rquickjs::Ctx<'js>,
    _globals: &Object<'js>,
) -> anyhow::Result<()> {
    let setup_code = r#"
        function variable(path) {
            return Promise.reject(new Error(`variable() is not available without an authenticated client`));
        }

        function resource(path) {
            return Promise.reject(new Error(`resource() is not available without an authenticated client`));
        }
    "#;

    ctx.eval::<(), _>(setup_code)
        .catch(ctx)
        .map_err(quickjs_error_to_anyhow)?;

    Ok(())
}

#[cfg(feature = "quickjs")]
fn setup_results_proxy<'js>(
    ctx: &rquickjs::Ctx<'js>,
    globals: &Object<'js>,
    by_id: &IdContext,
    op_state: Option<Arc<AsyncOpState>>,
) -> anyhow::Result<()> {
    globals.set("__previous_id", by_id.previous_id.clone())?;

    if let Some(state) = op_state {
        let by_id_for_result = by_id.clone();
        globals.set(
            "__fetchResult",
            Func::from(Async(MutFn::new(move |step_id: String| {
                let client = state.client.clone();
                let by_id = by_id_for_result.clone();
                let step_id_clone = step_id.clone();

                let job_result = by_id.steps_results.get(&step_id).cloned();
                let flow_job_id = by_id.flow_job.to_string();

                async move {
                    const ERR_PREFIX: &str = "\x00__WINDMILL_ERR__\x00";

                    let result: Result<serde_json::Value, String> = match job_result {
                        Some(jr) => match jr {
                            JobResult::SingleJob(job_id) => client
                                .get_completed_job_result::<serde_json::Value>(
                                    &job_id.to_string(),
                                    None,
                                )
                                .await
                                .map_err(|e| {
                                    format!(
                                        "Failed to fetch result for step '{}': {}",
                                        step_id_clone, e
                                    )
                                }),
                            JobResult::ListJob(job_ids) => {
                                let futs = job_ids.iter().map(|job_id| {
                                    let client = client.clone();
                                    let job_id_str = job_id.to_string();
                                    async move {
                                        client
                                            .get_completed_job_result::<serde_json::Value>(
                                                &job_id_str,
                                                None,
                                            )
                                            .await
                                    }
                                });
                                let results: Vec<_> = futures::future::join_all(futs).await;
                                let collected: Result<Vec<_>, _> = results.into_iter().collect();
                                collected.map(serde_json::Value::Array).map_err(|e| {
                                    format!(
                                        "Failed to fetch results for step '{}': {}",
                                        step_id_clone, e
                                    )
                                })
                            }
                        },
                        None => Ok(client
                            .get_result_by_id::<serde_json::Value>(
                                &flow_job_id,
                                &step_id_clone,
                                None,
                            )
                            .await
                            .ok()
                            .unwrap_or(serde_json::Value::Null)),
                    };

                    match result {
                        Ok(value) => {
                            serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string())
                        }
                        Err(e) => format!("{}{}", ERR_PREFIX, e),
                    }
                }
            }))),
        )?;

        let wrapper_code = r#"
            const __RESULT_ERR_PREFIX = '\x00__WINDMILL_ERR__\x00';
            async function __getResult(stepId) {
                const result = await __fetchResult(stepId);
                if (typeof result === 'string' && result.startsWith(__RESULT_ERR_PREFIX)) {
                    throw new Error(result.substring(__RESULT_ERR_PREFIX.length));
                }
                return JSON.parse(result);
            }
        "#;
        ctx.eval::<(), _>(wrapper_code)
            .catch(ctx)
            .map_err(quickjs_error_to_anyhow)?;
    } else {
        let stub_code = r#"
            function __getResult(stepId) {
                return Promise.reject(new Error('Result fetching not available without authenticated client'));
            }
        "#;
        ctx.eval::<(), _>(stub_code)
            .catch(ctx)
            .map_err(quickjs_error_to_anyhow)?;
    }

    let proxy_setup = r#"
        const results = new Proxy({}, {
            get: function(target, name, receiver) {
                if (typeof name === 'symbol') {
                    return undefined;
                }
                if (name === __previous_id && typeof previous_result !== 'undefined') {
                    return Promise.resolve(previous_result);
                }
                return __getResult(name);
            }
        });
    "#;
    ctx.eval::<(), _>(proxy_setup)
        .catch(ctx)
        .map_err(quickjs_error_to_anyhow)?;

    Ok(())
}

#[cfg(feature = "quickjs")]
fn json_to_js<'js>(
    ctx: &rquickjs::Ctx<'js>,
    val: &serde_json::Value,
) -> rquickjs::Result<Value<'js>> {
    match val {
        serde_json::Value::Null => Ok(Value::new_null(ctx.clone())),
        serde_json::Value::Bool(b) => Ok(Value::new_bool(ctx.clone(), *b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    Ok(Value::new_int(ctx.clone(), i as i32))
                } else {
                    Ok(Value::new_float(ctx.clone(), i as f64))
                }
            } else if let Some(f) = n.as_f64() {
                Ok(Value::new_float(ctx.clone(), f))
            } else {
                Ok(Value::new_float(ctx.clone(), 0.0))
            }
        }
        serde_json::Value::String(s) => s.clone().into_js(ctx),
        serde_json::Value::Array(arr) => {
            let js_arr = rquickjs::Array::new(ctx.clone())?;
            for (i, item) in arr.iter().enumerate() {
                js_arr.set(i, json_to_js(ctx, item)?)?;
            }
            Ok(js_arr.into_value())
        }
        serde_json::Value::Object(obj) => {
            let js_obj = Object::new(ctx.clone())?;
            for (k, v) in obj {
                js_obj.set(k.as_str(), json_to_js(ctx, v)?)?;
            }
            Ok(js_obj.into_value())
        }
    }
}

#[cfg(feature = "quickjs")]
fn should_add_return_quickjs(expr: &str) -> bool {
    let trimmed = expr.trim();

    if trimmed.is_empty() {
        return true;
    }

    if trimmed.starts_with("return ") || trimmed.starts_with("return;") || trimmed == "return" {
        return false;
    }

    let statement_prefixes = [
        "const ",
        "let ",
        "var ",
        "if ",
        "if(",
        "for ",
        "for(",
        "while ",
        "while(",
        "switch ",
        "switch(",
        "try ",
        "try{",
        "throw ",
        "function ",
        "class ",
        "async ",
        "await ",
    ];

    for prefix in &statement_prefixes {
        if trimmed.starts_with(prefix) {
            return false;
        }
    }

    if contains_semicolon_outside_strings(trimmed) {
        return false;
    }

    true
}

#[cfg(feature = "quickjs")]
fn contains_semicolon_outside_strings(expr: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_template = false;
    let mut prev_char = '\0';

    for ch in expr.chars() {
        match ch {
            '\'' if prev_char != '\\' && !in_double_quote && !in_template => {
                in_single_quote = !in_single_quote;
            }
            '"' if prev_char != '\\' && !in_single_quote && !in_template => {
                in_double_quote = !in_double_quote;
            }
            '`' if prev_char != '\\' && !in_single_quote && !in_double_quote => {
                in_template = !in_template;
            }
            ';' if !in_single_quote && !in_double_quote && !in_template => {
                return true;
            }
            _ => {}
        }
        prev_char = ch;
    }

    false
}

#[cfg(feature = "quickjs")]
fn quickjs_error_to_anyhow(err: rquickjs::CaughtError<'_>) -> anyhow::Error {
    anyhow::anyhow!("QuickJS evaluation error: {}", err)
}

// ── eval_simple_js for windmill-api batch rerun ──────────────────────

#[cfg(feature = "quickjs")]
pub async fn eval_simple_js(
    expr: String,
    globals: HashMap<String, serde_json::Value>,
) -> anyhow::Result<Box<RawValue>> {
    let handle = tokio::runtime::Handle::current();
    tokio::time::timeout(
        std::time::Duration::from_millis(EVAL_TIMEOUT_MS),
        tokio::task::spawn_blocking(move || {
            handle.block_on(async move {
                let runtime = AsyncRuntime::new()?;
                runtime.set_memory_limit(QUICKJS_MEMORY_LIMIT).await;
                let context = AsyncContext::full(&runtime).await?;

                async_with!(context => |ctx| {
                    let js_globals = ctx.globals();

                    // Set up each named global
                    for (name, value) in &globals {
                        let js_val = json_to_js(&ctx, value)?;
                        js_globals.set(name.as_str(), js_val)?;
                    }

                    // Wrap expression to return JSON string
                    let code = format!("JSON.stringify(({}) ?? null)", expr);
                    let result: String = ctx.eval(code)
                        .catch(&ctx)
                        .map_err(quickjs_error_to_anyhow)?;

                    Ok(unsafe_raw(result))
                })
                .await
            })
        }),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!("The expression evaluation took too long to execute (>{EVAL_TIMEOUT_MS}ms)")
    })??
}

// ── Fallback stubs when quickjs is disabled ──────────────────────────

#[cfg(not(feature = "quickjs"))]
pub async fn eval_timeout_quickjs(
    _expr: String,
    _transform_context: HashMap<String, Arc<Box<RawValue>>>,
    _flow_input: Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
    _flow_env: Option<&HashMap<String, Box<RawValue>>>,
    _authed_client: Option<&windmill_common::client::AuthedClient>,
    _by_id: Option<&IdContext>,
    _ctx: Option<Vec<(String, String)>>,
) -> anyhow::Result<Box<RawValue>> {
    anyhow::bail!("JavaScript expression evaluation requires the `quickjs` feature. Enable it with: cargo build --features quickjs")
}

#[cfg(not(feature = "quickjs"))]
pub async fn eval_simple_js(
    _expr: String,
    _globals: HashMap<String, serde_json::Value>,
) -> anyhow::Result<Box<RawValue>> {
    anyhow::bail!("JavaScript expression evaluation requires the `quickjs` feature. Enable it with: cargo build --features quickjs")
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "quickjs"))]
mod tests {
    use super::*;
    use serde_json::json;
    use windmill_common::worker::to_raw_value;

    /// Helper: evaluate an expression with a transform context and assert against expected JSON.
    async fn assert_eval(
        expr: &str,
        ctx: HashMap<String, Arc<Box<RawValue>>>,
        expected: serde_json::Value,
    ) {
        assert_eval_full(expr, ctx, None, None, expected).await;
    }

    /// Helper: evaluate with full context (transform_context, flow_input, flow_env).
    async fn assert_eval_full(
        expr: &str,
        ctx: HashMap<String, Arc<Box<RawValue>>>,
        flow_input: Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
        flow_env: Option<&HashMap<String, Box<RawValue>>>,
        expected: serde_json::Value,
    ) {
        let result = eval_timeout_quickjs(
            expr.to_string(),
            ctx,
            flow_input,
            flow_env,
            None,
            None,
            None,
        )
        .await
        .unwrap_or_else(|e| panic!("eval_timeout_quickjs failed for '{}': {}", expr, e));

        let actual: serde_json::Value = serde_json::from_str(result.get()).unwrap_or_else(|e| {
            panic!(
                "Failed to parse result '{}' for '{}': {}",
                result.get(),
                expr,
                e
            )
        });

        assert_eq!(actual, expected, "Mismatch for expression '{}'", expr);
    }

    // =====================================================================
    // HELPER FUNCTION TESTS
    // =====================================================================

    #[test]
    fn test_should_add_return_quickjs() {
        assert!(should_add_return_quickjs("5"));
        assert!(should_add_return_quickjs("x + y"));
        assert!(should_add_return_quickjs("foo()"));
        assert!(should_add_return_quickjs("obj.method()"));
        assert!(should_add_return_quickjs("a > b ? 'yes' : 'no'"));
        assert!(should_add_return_quickjs("({ key: 'value' })"));

        assert!(!should_add_return_quickjs("return 5"));
        assert!(!should_add_return_quickjs("return x + y"));
        assert!(!should_add_return_quickjs("const x = 5"));
        assert!(!should_add_return_quickjs("let y = 10"));
        assert!(!should_add_return_quickjs("if (x > 5) { return x; }"));
        assert!(!should_add_return_quickjs("let x = 5; x + 1"));
        assert!(!should_add_return_quickjs("try { return 1; } catch(e) {}"));
    }

    #[test]
    fn test_contains_semicolon_outside_strings() {
        assert!(contains_semicolon_outside_strings("a; b"));
        assert!(contains_semicolon_outside_strings("let x = 5; x + 1"));

        assert!(!contains_semicolon_outside_strings("'a;b'"));
        assert!(!contains_semicolon_outside_strings("\"a;b\""));
        assert!(!contains_semicolon_outside_strings("`a;b`"));
        assert!(!contains_semicolon_outside_strings("x + y"));
    }

    #[test]
    fn test_replace_with_await() {
        assert_eq!(
            replace_with_await("variable('test')".to_string(), "variable"),
            "(await variable('test'))"
        );
        assert_eq!(
            replace_with_await("x + variable('a') + variable('b')".to_string(), "variable"),
            "x + (await variable('a')) + (await variable('b'))"
        );
        assert_eq!(
            replace_with_await("no_match".to_string(), "variable"),
            "no_match"
        );
    }

    #[test]
    fn test_replace_with_await_result() {
        assert_eq!(
            replace_with_await_result("results.step_a".to_string()),
            "(await results.step_a)"
        );
        assert_eq!(
            replace_with_await_result("results.a + results.b".to_string()),
            "(await results.a) + (await results.b)"
        );
        assert_eq!(
            replace_with_await_result("no_results_here".to_string()),
            "no_results_here"
        );
    }

    #[test]
    fn test_try_exact_property_access_flow_input_dot() {
        let mut fi = HashMap::new();
        fi.insert("name".to_string(), to_raw_value(&json!("hello")));
        let fi = mappable_rc::Marc::new(fi);

        let result = try_exact_property_access("flow_input.name", Some(&fi), None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().get(), "\"hello\"");
    }

    #[test]
    fn test_try_exact_property_access_flow_input_bracket() {
        let mut fi = HashMap::new();
        fi.insert("my_key".to_string(), to_raw_value(&json!(42)));
        let fi = mappable_rc::Marc::new(fi);

        let result = try_exact_property_access("flow_input[\"my_key\"]", Some(&fi), None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().get(), "42");
    }

    #[test]
    fn test_try_exact_property_access_flow_env() {
        let mut fe = HashMap::new();
        fe.insert("ENV".to_string(), to_raw_value(&json!("production")));

        let result = try_exact_property_access("flow_env.ENV", None, Some(&fe));
        assert!(result.is_some());
        assert_eq!(result.unwrap().get(), "\"production\"");
    }

    #[test]
    fn test_try_exact_property_access_missing_key() {
        let fi = mappable_rc::Marc::new(HashMap::new());
        let result = try_exact_property_access("flow_input.missing", Some(&fi), None);
        assert!(result.is_none());
    }

    #[test]
    fn test_try_exact_property_access_no_prefix() {
        let result = try_exact_property_access("some_var.key", None, None);
        assert!(result.is_none());
    }

    // =====================================================================
    // SIMPLE ARITHMETIC
    // =====================================================================

    #[tokio::test]
    async fn test_simple_arithmetic() {
        let mut env = HashMap::new();
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(5))));
        env.insert("y".to_string(), Arc::new(to_raw_value(&json!(3))));

        assert_eval("x + y", env.clone(), json!(8)).await;
        assert_eval("x - y", env.clone(), json!(2)).await;
        assert_eval("x * y", env.clone(), json!(15)).await;
        assert_eval("x % y", env.clone(), json!(2)).await;
        assert_eval("x ** 2", env.clone(), json!(25)).await;
    }

    // =====================================================================
    // OBJECT PROPERTY ACCESS
    // =====================================================================

    #[tokio::test]
    async fn test_object_property_access() {
        let mut env = HashMap::new();
        env.insert(
            "obj".to_string(),
            Arc::new(to_raw_value(&json!({
                "name": "test",
                "value": 42,
                "nested": {"deep": {"property": "found"}}
            }))),
        );

        assert_eval("obj.name", env.clone(), json!("test")).await;
        assert_eval("obj.value", env.clone(), json!(42)).await;
        assert_eval("obj.nested.deep.property", env.clone(), json!("found")).await;
        assert_eval("obj['name']", env.clone(), json!("test")).await;
    }

    // =====================================================================
    // ARRAY OPERATIONS
    // =====================================================================

    #[tokio::test]
    async fn test_array_operations() {
        let mut env = HashMap::new();
        env.insert(
            "arr".to_string(),
            Arc::new(to_raw_value(&json!([1, 2, 3, 4, 5]))),
        );

        assert_eval("arr.length", env.clone(), json!(5)).await;
        assert_eval("arr[0]", env.clone(), json!(1)).await;
        assert_eval("arr.map(x => x * 2)", env.clone(), json!([2, 4, 6, 8, 10])).await;
        assert_eval("arr.filter(x => x > 2)", env.clone(), json!([3, 4, 5])).await;
        assert_eval("arr.reduce((a, b) => a + b, 0)", env.clone(), json!(15)).await;
        assert_eval("arr.find(x => x > 3)", env.clone(), json!(4)).await;
        assert_eval("arr.some(x => x > 4)", env.clone(), json!(true)).await;
        assert_eval("arr.every(x => x > 0)", env.clone(), json!(true)).await;
    }

    // =====================================================================
    // STRING OPERATIONS
    // =====================================================================

    #[tokio::test]
    async fn test_string_operations() {
        let mut env = HashMap::new();
        env.insert(
            "s".to_string(),
            Arc::new(to_raw_value(&json!("Hello World"))),
        );

        assert_eval("s.toLowerCase()", env.clone(), json!("hello world")).await;
        assert_eval("s.toUpperCase()", env.clone(), json!("HELLO WORLD")).await;
        assert_eval("s.length", env.clone(), json!(11)).await;
        assert_eval("s.split(' ')", env.clone(), json!(["Hello", "World"])).await;
        assert_eval(
            "s.replace('World', 'QuickJS')",
            env.clone(),
            json!("Hello QuickJS"),
        )
        .await;
        assert_eval("s.includes('World')", env.clone(), json!(true)).await;
        assert_eval("s.startsWith('Hello')", env.clone(), json!(true)).await;
        assert_eval("s.trim()", env.clone(), json!("Hello World")).await;
    }

    // =====================================================================
    // TERNARY AND CONDITIONALS
    // =====================================================================

    #[tokio::test]
    async fn test_ternary_and_conditionals() {
        let mut env = HashMap::new();
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(10))));
        env.insert("y".to_string(), Arc::new(to_raw_value(&json!(5))));

        assert_eval("x > y ? 'bigger' : 'smaller'", env.clone(), json!("bigger")).await;
        assert_eval("x === 10 ? true : false", env.clone(), json!(true)).await;
        assert_eval("x > 5 && y < 10", env.clone(), json!(true)).await;
        assert_eval("x > 20 || y < 10", env.clone(), json!(true)).await;
        assert_eval("!false", env.clone(), json!(true)).await;
    }

    // =====================================================================
    // OBJECT CREATION
    // =====================================================================

    #[tokio::test]
    async fn test_object_creation() {
        let mut env = HashMap::new();
        env.insert("name".to_string(), Arc::new(to_raw_value(&json!("test"))));
        env.insert("value".to_string(), Arc::new(to_raw_value(&json!(42))));

        assert_eval("({ foo: 'bar' })", env.clone(), json!({"foo": "bar"})).await;
        assert_eval(
            "({ name, value })",
            env.clone(),
            json!({"name": "test", "value": 42}),
        )
        .await;
        assert_eval(
            "({ ...{ a: 1 }, b: 2 })",
            env.clone(),
            json!({"a": 1, "b": 2}),
        )
        .await;
    }

    // =====================================================================
    // NULL / UNDEFINED
    // =====================================================================

    #[tokio::test]
    async fn test_null_undefined() {
        assert_eval("null", HashMap::new(), json!(null)).await;
        assert_eval("undefined", HashMap::new(), json!(null)).await;

        let mut env = HashMap::new();
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(null))));
        assert_eval("x", env.clone(), json!(null)).await;
        assert_eval("x ?? 'default'", env.clone(), json!("default")).await;
    }

    // =====================================================================
    // FLOW INPUT
    // =====================================================================

    #[tokio::test]
    async fn test_flow_input() {
        let mut fi = HashMap::new();
        fi.insert("name".to_string(), to_raw_value(&json!("test_flow")));
        fi.insert("count".to_string(), to_raw_value(&json!(100)));
        fi.insert(
            "config".to_string(),
            to_raw_value(&json!({"enabled": true})),
        );

        let fi = Some(mappable_rc::Marc::new(fi));

        assert_eval_full(
            "flow_input.name",
            HashMap::new(),
            fi.clone(),
            None,
            json!("test_flow"),
        )
        .await;
        assert_eval_full(
            "flow_input.count",
            HashMap::new(),
            fi.clone(),
            None,
            json!(100),
        )
        .await;
        assert_eval_full(
            "flow_input.config.enabled",
            HashMap::new(),
            fi.clone(),
            None,
            json!(true),
        )
        .await;
    }

    // =====================================================================
    // TEMPLATE LITERALS
    // =====================================================================

    #[tokio::test]
    async fn test_template_literals() {
        let mut env = HashMap::new();
        env.insert("name".to_string(), Arc::new(to_raw_value(&json!("World"))));
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(5))));

        assert_eval("`Hello ${name}!`", env.clone(), json!("Hello World!")).await;
        assert_eval(
            "`The answer is ${x * 2}`",
            env.clone(),
            json!("The answer is 10"),
        )
        .await;
    }

    // =====================================================================
    // JSON OPERATIONS
    // =====================================================================

    #[tokio::test]
    async fn test_json_operations() {
        let mut env = HashMap::new();
        env.insert(
            "obj".to_string(),
            Arc::new(to_raw_value(&json!({"a": 1, "b": 2}))),
        );

        // JSON.stringify produces a string result
        assert_eval(
            "JSON.stringify(obj)",
            env.clone(),
            json!("{\"a\":1,\"b\":2}"),
        )
        .await;
        assert_eval("Object.keys(obj)", env.clone(), json!(["a", "b"])).await;
        assert_eval("Object.values(obj)", env.clone(), json!([1, 2])).await;
    }

    // =====================================================================
    // MATH OPERATIONS
    // =====================================================================

    #[tokio::test]
    async fn test_math_operations() {
        let env = HashMap::new();

        assert_eval("Math.max(1, 5, 3)", env.clone(), json!(5)).await;
        assert_eval("Math.min(1, 5, 3)", env.clone(), json!(1)).await;
        assert_eval("Math.abs(-5)", env.clone(), json!(5)).await;
        assert_eval("Math.floor(3.7)", env.clone(), json!(3)).await;
        assert_eval("Math.ceil(3.2)", env.clone(), json!(4)).await;
        assert_eval("Math.round(3.5)", env.clone(), json!(4)).await;
    }

    // =====================================================================
    // TYPE COERCION
    // =====================================================================

    #[tokio::test]
    async fn test_type_coercion() {
        let mut env = HashMap::new();
        env.insert("num".to_string(), Arc::new(to_raw_value(&json!(42))));
        env.insert("str".to_string(), Arc::new(to_raw_value(&json!("123"))));

        assert_eval("String(num)", env.clone(), json!("42")).await;
        assert_eval("Number(str)", env.clone(), json!(123)).await;
        assert_eval("Boolean(num)", env.clone(), json!(true)).await;
        assert_eval("parseInt('42px')", env.clone(), json!(42)).await;
        assert_eval("parseFloat('3.14')", env.clone(), json!(3.14)).await;
    }

    // =====================================================================
    // ARRAY SPREAD
    // =====================================================================

    #[tokio::test]
    async fn test_array_spread() {
        let mut env = HashMap::new();
        env.insert(
            "arr1".to_string(),
            Arc::new(to_raw_value(&json!([1, 2, 3]))),
        );
        env.insert(
            "arr2".to_string(),
            Arc::new(to_raw_value(&json!([4, 5, 6]))),
        );

        assert_eval("[...arr1, ...arr2]", env.clone(), json!([1, 2, 3, 4, 5, 6])).await;
        assert_eval("[0, ...arr1, 99]", env.clone(), json!([0, 1, 2, 3, 99])).await;
    }

    // =====================================================================
    // MULTILINE STATEMENTS
    // =====================================================================

    #[tokio::test]
    async fn test_multiline_statements() {
        let mut env = HashMap::new();
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(5))));

        assert_eval(
            r#"let y = x * 2;
            return y + 1"#,
            env.clone(),
            json!(11),
        )
        .await;

        assert_eval(
            r#"const result = x > 3 ? 'big' : 'small';
            return result"#,
            env.clone(),
            json!("big"),
        )
        .await;
    }

    // =====================================================================
    // OPTIONAL CHAINING & NULLISH COALESCING
    // =====================================================================

    #[tokio::test]
    async fn test_optional_chaining_nullish() {
        let mut env = HashMap::new();
        env.insert(
            "obj".to_string(),
            Arc::new(to_raw_value(&json!({"a": {"b": 1}}))),
        );
        env.insert("empty".to_string(), Arc::new(to_raw_value(&json!(null))));

        assert_eval("obj?.a?.b", env.clone(), json!(1)).await;
        assert_eval("obj?.a?.c", env.clone(), json!(null)).await;
        assert_eval("obj?.x?.y", env.clone(), json!(null)).await;
        assert_eval("empty?.foo", env.clone(), json!(null)).await;

        assert_eval("null ?? 'default'", env.clone(), json!("default")).await;
        assert_eval("undefined ?? 'default'", env.clone(), json!("default")).await;
        assert_eval("0 ?? 'default'", env.clone(), json!(0)).await;
        assert_eval("'' ?? 'default'", env.clone(), json!("")).await;
        assert_eval("false ?? 'default'", env.clone(), json!(false)).await;
    }

    // =====================================================================
    // DESTRUCTURING
    // =====================================================================

    #[tokio::test]
    async fn test_destructuring() {
        let mut env = HashMap::new();
        env.insert(
            "obj".to_string(),
            Arc::new(to_raw_value(&json!({"name": "test", "value": 42}))),
        );
        env.insert(
            "arr".to_string(),
            Arc::new(to_raw_value(&json!([1, 2, 3, 4, 5]))),
        );

        assert_eval(
            "const { name, value } = obj; return { name, value }",
            env.clone(),
            json!({"name": "test", "value": 42}),
        )
        .await;

        assert_eval(
            "const [first, second, ...rest] = arr; return { first, second, rest }",
            env.clone(),
            json!({"first": 1, "second": 2, "rest": [3, 4, 5]}),
        )
        .await;

        assert_eval(
            "const { missing = 'default' } = obj; return missing",
            env.clone(),
            json!("default"),
        )
        .await;
    }

    // =====================================================================
    // NUMBER EDGE CASES
    // =====================================================================

    #[tokio::test]
    async fn test_number_edge_cases() {
        let env = HashMap::new();

        assert_eval(
            "Number.MAX_SAFE_INTEGER",
            env.clone(),
            json!(9007199254740991_i64),
        )
        .await;
        assert_eval(
            "Number.MIN_SAFE_INTEGER",
            env.clone(),
            json!(-9007199254740991_i64),
        )
        .await;
        assert_eval("Number.isInteger(5)", env.clone(), json!(true)).await;
        assert_eval("Number.isInteger(5.5)", env.clone(), json!(false)).await;
        assert_eval("Number.isFinite(Infinity)", env.clone(), json!(false)).await;
        assert_eval("Number.isNaN(NaN)", env.clone(), json!(true)).await;
        assert_eval("isNaN(NaN)", env.clone(), json!(true)).await;
        assert_eval("isFinite(100)", env.clone(), json!(true)).await;
    }

    // =====================================================================
    // REGEX BASIC
    // =====================================================================

    #[tokio::test]
    async fn test_regex_basic() {
        let mut env = HashMap::new();
        env.insert(
            "str".to_string(),
            Arc::new(to_raw_value(&json!("hello world 123"))),
        );

        assert_eval("/hello/.test(str)", env.clone(), json!(true)).await;
        assert_eval("str.match(/\\d+/)?.[0]", env.clone(), json!("123")).await;
        assert_eval(
            "str.replace(/world/, 'universe')",
            env.clone(),
            json!("hello universe 123"),
        )
        .await;
        assert_eval(
            "str.split(/\\s+/)",
            env.clone(),
            json!(["hello", "world", "123"]),
        )
        .await;
        assert_eval("'aaa'.replace(/a/g, 'b')", env.clone(), json!("bbb")).await;
        assert_eval("/HELLO/i.test(str)", env.clone(), json!(true)).await;
    }

    // =====================================================================
    // DATE OPERATIONS
    // =====================================================================

    #[tokio::test]
    async fn test_date_basic() {
        let env = HashMap::new();

        assert_eval(
            "Date.parse('2024-01-15T00:00:00.000Z')",
            env.clone(),
            json!(1705276800000_i64),
        )
        .await;
        assert_eval(
            "new Date('2024-01-15T00:00:00.000Z').getUTCFullYear()",
            env.clone(),
            json!(2024),
        )
        .await;
        assert_eval(
            "new Date('2024-01-15T00:00:00.000Z').getUTCMonth()",
            env.clone(),
            json!(0),
        )
        .await;
        assert_eval(
            "new Date('2024-01-15T00:00:00.000Z').getUTCDate()",
            env.clone(),
            json!(15),
        )
        .await;
        assert_eval(
            "new Date('2024-01-15T00:00:00.000Z').toISOString()",
            env.clone(),
            json!("2024-01-15T00:00:00.000Z"),
        )
        .await;
    }

    #[tokio::test]
    async fn test_date_serialization() {
        let env = HashMap::new();

        // Direct Date → ISO string via toJSON
        assert_eval(
            "new Date('2024-01-15T12:30:00.000Z')",
            env.clone(),
            json!("2024-01-15T12:30:00.000Z"),
        )
        .await;

        // Date within an object
        assert_eval(
            "({ date: new Date('2024-01-15T00:00:00.000Z'), name: 'test' })",
            env.clone(),
            json!({"date": "2024-01-15T00:00:00.000Z", "name": "test"}),
        )
        .await;

        // Deeply nested Date
        assert_eval(
            "({ level1: { level2: { date: new Date('2024-01-15T00:00:00.000Z') } } })",
            env.clone(),
            json!({"level1": {"level2": {"date": "2024-01-15T00:00:00.000Z"}}}),
        )
        .await;
    }

    // =====================================================================
    // SPECIAL OBJECT SERIALIZATION
    // =====================================================================

    #[tokio::test]
    async fn test_special_object_serialization() {
        let env = HashMap::new();

        // RegExp, Map, Set all serialize to {}
        assert_eval("/test/gi", env.clone(), json!({})).await;
        assert_eval("new Map([['key', 'value']])", env.clone(), json!({})).await;
        assert_eval("new Set([1, 2, 3])", env.clone(), json!({})).await;
    }

    // =====================================================================
    // ARRAY ADVANCED
    // =====================================================================

    #[tokio::test]
    async fn test_array_advanced() {
        let mut env = HashMap::new();
        env.insert(
            "arr".to_string(),
            Arc::new(to_raw_value(&json!([3, 1, 4, 1, 5, 9, 2, 6]))),
        );
        env.insert(
            "nested".to_string(),
            Arc::new(to_raw_value(&json!([[1, 2], [3, 4], [5, 6]]))),
        );

        assert_eval(
            "[...arr].sort((a, b) => a - b)",
            env.clone(),
            json!([1, 1, 2, 3, 4, 5, 6, 9]),
        )
        .await;
        assert_eval(
            "[...arr].sort((a, b) => b - a)",
            env.clone(),
            json!([9, 6, 5, 4, 3, 2, 1, 1]),
        )
        .await;
        assert_eval("nested.flat()", env.clone(), json!([1, 2, 3, 4, 5, 6])).await;
        assert_eval(
            "nested.flatMap(x => x)",
            env.clone(),
            json!([1, 2, 3, 4, 5, 6]),
        )
        .await;
        assert_eval("arr.indexOf(5)", env.clone(), json!(4)).await;
        assert_eval("arr.indexOf(99)", env.clone(), json!(-1)).await;
        assert_eval("arr.includes(9)", env.clone(), json!(true)).await;
        assert_eval("arr.slice(2, 5)", env.clone(), json!([4, 1, 5])).await;
        assert_eval("arr.slice(-3)", env.clone(), json!([9, 2, 6])).await;
    }

    // =====================================================================
    // LOGICAL OPERATORS
    // =====================================================================

    #[tokio::test]
    async fn test_logical_operators() {
        let mut env = HashMap::new();
        env.insert("a".to_string(), Arc::new(to_raw_value(&json!(true))));
        env.insert("b".to_string(), Arc::new(to_raw_value(&json!(false))));
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(5))));

        assert_eval("a && 'yes'", env.clone(), json!("yes")).await;
        assert_eval("b && 'yes'", env.clone(), json!(false)).await;
        assert_eval("b || 'no'", env.clone(), json!("no")).await;
        assert_eval("a || 'no'", env.clone(), json!(true)).await;
        assert_eval("let y = null; y ??= 10; return y", env.clone(), json!(10)).await;
        assert_eval("let y = 5; y ??= 10; return y", env.clone(), json!(5)).await;
        assert_eval("(a && x > 3) || (b && x < 3)", env.clone(), json!(true)).await;
    }

    // =====================================================================
    // TYPEOF
    // =====================================================================

    #[tokio::test]
    async fn test_typeof() {
        let mut env = HashMap::new();
        env.insert("str".to_string(), Arc::new(to_raw_value(&json!("hello"))));
        env.insert("num".to_string(), Arc::new(to_raw_value(&json!(42))));
        env.insert("arr".to_string(), Arc::new(to_raw_value(&json!([1, 2, 3]))));
        env.insert("obj".to_string(), Arc::new(to_raw_value(&json!({"a": 1}))));
        env.insert("n".to_string(), Arc::new(to_raw_value(&json!(null))));

        assert_eval("typeof str", env.clone(), json!("string")).await;
        assert_eval("typeof num", env.clone(), json!("number")).await;
        assert_eval("typeof arr", env.clone(), json!("object")).await;
        assert_eval("typeof obj", env.clone(), json!("object")).await;
        assert_eval("typeof n", env.clone(), json!("object")).await;
        assert_eval("typeof undefined", env.clone(), json!("undefined")).await;
        assert_eval("Array.isArray(arr)", env.clone(), json!(true)).await;
        assert_eval("Array.isArray(obj)", env.clone(), json!(false)).await;
    }

    // =====================================================================
    // COMPLEX MULTILINE EXPRESSIONS
    // =====================================================================

    #[tokio::test]
    async fn test_multiline_complex_logic() {
        let mut env = HashMap::new();
        env.insert(
            "users".to_string(),
            Arc::new(to_raw_value(&json!([
                {"name": "Alice", "age": 30, "role": "admin"},
                {"name": "Bob", "age": 25, "role": "user"},
                {"name": "Charlie", "age": 35, "role": "admin"},
                {"name": "Diana", "age": 28, "role": "user"}
            ]))),
        );

        assert_eval(
            r#"
            const admins = users.filter(u => u.role === 'admin');
            const names = admins.map(u => u.name);
            return names.join(', ')
            "#,
            env.clone(),
            json!("Alice, Charlie"),
        )
        .await;

        assert_eval(
            r#"
            const totalAge = users.reduce((sum, u) => sum + u.age, 0);
            const avgAge = totalAge / users.length;
            return Math.round(avgAge)
            "#,
            env.clone(),
            json!(30),
        )
        .await;

        assert_eval(
            r#"
            const grouped = users.reduce((acc, u) => {
                if (!acc[u.role]) acc[u.role] = [];
                acc[u.role].push(u.name);
                return acc;
            }, {});
            return grouped
            "#,
            env.clone(),
            json!({"admin": ["Alice", "Charlie"], "user": ["Bob", "Diana"]}),
        )
        .await;
    }

    // =====================================================================
    // DATA TRANSFORMATION
    // =====================================================================

    #[tokio::test]
    async fn test_multiline_data_transformation() {
        let mut env = HashMap::new();
        env.insert(
            "data".to_string(),
            Arc::new(to_raw_value(&json!({
                "items": [
                    {"id": 1, "price": 100, "quantity": 2},
                    {"id": 2, "price": 50, "quantity": 5},
                    {"id": 3, "price": 75, "quantity": 3}
                ],
                "discount": 0.1
            }))),
        );

        assert_eval(
            r#"
            const subtotals = data.items.map(item => item.price * item.quantity);
            const total = subtotals.reduce((a, b) => a + b, 0);
            const discounted = total * (1 - data.discount);
            return { subtotals, total, discounted }
            "#,
            env.clone(),
            json!({"subtotals": [200, 250, 225], "total": 675, "discounted": 607.5}),
        )
        .await;
    }

    // =====================================================================
    // TRY-CATCH
    // =====================================================================

    #[tokio::test]
    async fn test_try_catch() {
        let env = HashMap::new();

        assert_eval(
            r#"
            try {
                return JSON.parse('{"valid": true}');
            } catch (e) {
                return { problem: e.message };
            }
            "#,
            env.clone(),
            json!({"valid": true}),
        )
        .await;

        assert_eval(
            r#"
            try {
                return JSON.parse('invalid json');
            } catch (e) {
                return { problem: 'parse_failed' };
            }
            "#,
            env.clone(),
            json!({"problem": "parse_failed"}),
        )
        .await;

        assert_eval(
            r#"
            let result = 'initial';
            try {
                result = 'try';
            } catch (e) {
                result = 'catch';
            } finally {
                result = result + '_finally';
            }
            return result
            "#,
            env.clone(),
            json!("try_finally"),
        )
        .await;
    }

    // =====================================================================
    // OBJECT ADVANCED
    // =====================================================================

    #[tokio::test]
    async fn test_object_advanced() {
        let mut env = HashMap::new();
        env.insert(
            "config".to_string(),
            Arc::new(to_raw_value(&json!({
                "server": {"host": "localhost", "port": 8080},
                "database": {"host": "db.local", "port": 5432},
                "features": ["auth", "logging", "cache"]
            }))),
        );

        assert_eval(
            "Object.assign({}, config.server, { secure: true })",
            env.clone(),
            json!({"host": "localhost", "port": 8080, "secure": true}),
        )
        .await;

        assert_eval(
            "({ ...config.server, port: 443, secure: true })",
            env.clone(),
            json!({"host": "localhost", "port": 443, "secure": true}),
        )
        .await;

        assert_eval(
            "JSON.parse(JSON.stringify(config))",
            env.clone(),
            json!({
                "server": {"host": "localhost", "port": 8080},
                "database": {"host": "db.local", "port": 5432},
                "features": ["auth", "logging", "cache"]
            }),
        )
        .await;

        assert_eval(
            r#"
            const key = 'dynamic';
            return { [key]: 'value', [`${key}_2`]: 'value2' }
            "#,
            env.clone(),
            json!({"dynamic": "value", "dynamic_2": "value2"}),
        )
        .await;
    }

    // =====================================================================
    // STRING ADVANCED
    // =====================================================================

    #[tokio::test]
    async fn test_string_advanced() {
        let mut env = HashMap::new();
        env.insert(
            "text".to_string(),
            Arc::new(to_raw_value(&json!("  Hello, World!  "))),
        );
        env.insert(
            "path".to_string(),
            Arc::new(to_raw_value(&json!("/api/v1/users/123/profile"))),
        );

        assert_eval("text.trim()", env.clone(), json!("Hello, World!")).await;
        assert_eval("text.trimStart()", env.clone(), json!("Hello, World!  ")).await;
        assert_eval("text.trimEnd()", env.clone(), json!("  Hello, World!")).await;
        assert_eval("'42'.padStart(5, '0')", env.clone(), json!("00042")).await;
        assert_eval("'42'.padEnd(5, '-')", env.clone(), json!("42---")).await;
        assert_eval("'ab'.repeat(3)", env.clone(), json!("ababab")).await;
        assert_eval(
            "path.split('/').filter(p => p.length > 0)",
            env.clone(),
            json!(["api", "v1", "users", "123", "profile"]),
        )
        .await;
    }

    // =====================================================================
    // ARRAY MANIPULATION
    // =====================================================================

    #[tokio::test]
    async fn test_array_manipulation() {
        let mut env = HashMap::new();
        env.insert(
            "items".to_string(),
            Arc::new(to_raw_value(&json!([
                {"id": 1, "name": "Apple", "category": "fruit"},
                {"id": 2, "name": "Carrot", "category": "vegetable"},
                {"id": 3, "name": "Banana", "category": "fruit"},
                {"id": 4, "name": "Broccoli", "category": "vegetable"}
            ]))),
        );

        assert_eval(
            "items.find(i => i.name === 'Banana')",
            env.clone(),
            json!({"id": 3, "name": "Banana", "category": "fruit"}),
        )
        .await;
        assert_eval(
            "items.findIndex(i => i.name === 'Banana')",
            env.clone(),
            json!(2),
        )
        .await;
        assert_eval(
            "items.filter(i => i.category === 'fruit').map(i => i.name).sort()",
            env.clone(),
            json!(["Apple", "Banana"]),
        )
        .await;
        assert_eval(
            "Array.from({length: 5}, (_, i) => i * 2)",
            env.clone(),
            json!([0, 2, 4, 6, 8]),
        )
        .await;
        assert_eval("Array(3).fill(0)", env.clone(), json!([0, 0, 0])).await;
        assert_eval(
            "[1, 2].concat([3, 4], [5, 6])",
            env.clone(),
            json!([1, 2, 3, 4, 5, 6]),
        )
        .await;
        assert_eval(
            "items.map(i => i.name).join(' | ')",
            env.clone(),
            json!("Apple | Carrot | Banana | Broccoli"),
        )
        .await;
    }

    // =====================================================================
    // SET AND MAP OPERATIONS
    // =====================================================================

    #[tokio::test]
    async fn test_set_operations() {
        let mut env = HashMap::new();
        env.insert(
            "arr".to_string(),
            Arc::new(to_raw_value(&json!([1, 2, 2, 3, 3, 3, 4]))),
        );

        assert_eval("[...new Set(arr)]", env.clone(), json!([1, 2, 3, 4])).await;
        assert_eval("new Set(arr).size", env.clone(), json!(4)).await;
        assert_eval("new Set(arr).has(3)", env.clone(), json!(true)).await;
        assert_eval("new Set(arr).has(99)", env.clone(), json!(false)).await;
    }

    #[tokio::test]
    async fn test_map_operations() {
        let env = HashMap::new();

        assert_eval(
            r#"
            const map = new Map([['a', 1], ['b', 2], ['c', 3]]);
            return Object.fromEntries(map)
            "#,
            env.clone(),
            json!({"a": 1, "b": 2, "c": 3}),
        )
        .await;

        assert_eval(
            r#"
            const map = new Map();
            map.set('key1', 'value1');
            map.set('key2', 'value2');
            return map.get('key1')
            "#,
            env.clone(),
            json!("value1"),
        )
        .await;

        assert_eval(
            r#"
            const map = new Map([['a', 1], ['b', 2]]);
            return map.size
            "#,
            env.clone(),
            json!(2),
        )
        .await;
    }

    // =====================================================================
    // COMPARISONS
    // =====================================================================

    #[tokio::test]
    async fn test_comparisons() {
        let env = HashMap::new();

        assert_eval("1 === 1", env.clone(), json!(true)).await;
        assert_eval("1 === '1'", env.clone(), json!(false)).await;
        assert_eval("null === undefined", env.clone(), json!(false)).await;
        assert_eval("null === null", env.clone(), json!(true)).await;
        assert_eval("1 == '1'", env.clone(), json!(true)).await;
        assert_eval("null == undefined", env.clone(), json!(true)).await;
        assert_eval("5 !== '5'", env.clone(), json!(true)).await;
        assert_eval("5 > 3", env.clone(), json!(true)).await;
        assert_eval("5 >= 5", env.clone(), json!(true)).await;
        assert_eval("3 < 5", env.clone(), json!(true)).await;
        assert_eval("5 <= 5", env.clone(), json!(true)).await;
        assert_eval("'apple' < 'banana'", env.clone(), json!(true)).await;
    }

    // =====================================================================
    // BITWISE OPERATIONS
    // =====================================================================

    #[tokio::test]
    async fn test_bitwise() {
        let env = HashMap::new();

        assert_eval("5 & 3", env.clone(), json!(1)).await;
        assert_eval("5 | 3", env.clone(), json!(7)).await;
        assert_eval("5 ^ 3", env.clone(), json!(6)).await;
        assert_eval("~5", env.clone(), json!(-6)).await;
        assert_eval("5 << 2", env.clone(), json!(20)).await;
        assert_eval("20 >> 2", env.clone(), json!(5)).await;
    }

    // =====================================================================
    // REAL-WORLD FLOW PATTERNS
    // =====================================================================

    #[tokio::test]
    async fn test_flow_patterns_api_response() {
        let mut env = HashMap::new();
        env.insert(
            "previous_result".to_string(),
            Arc::new(to_raw_value(&json!({
                "status": 200,
                "data": {
                    "users": [
                        {"id": 1, "email": "alice@example.com", "active": true},
                        {"id": 2, "email": "bob@example.com", "active": false},
                        {"id": 3, "email": "charlie@example.com", "active": true}
                    ],
                    "pagination": {"page": 1, "total": 50, "per_page": 10}
                }
            }))),
        );

        assert_eval(
            "previous_result.data.users.filter(u => u.active).map(u => u.email)",
            env.clone(),
            json!(["alice@example.com", "charlie@example.com"]),
        )
        .await;

        assert_eval(
            r#"
            const { page, total, per_page } = previous_result.data.pagination;
            return page * per_page < total
            "#,
            env.clone(),
            json!(true),
        )
        .await;
    }

    #[tokio::test]
    async fn test_flow_patterns_batch_processing() {
        let mut env = HashMap::new();
        env.insert(
            "jobs".to_string(),
            Arc::new(to_raw_value(&json!([
                {"id": 1, "status": "completed", "result": 100},
                {"id": 2, "status": "failed", "reason": "timeout"},
                {"id": 3, "status": "completed", "result": 200},
                {"id": 4, "status": "failed", "reason": "connection"},
                {"id": 5, "status": "completed", "result": 150}
            ]))),
        );

        assert_eval(
            r#"
            const completed = jobs.filter(j => j.status === 'completed');
            const failed = jobs.filter(j => j.status === 'failed');
            const totalResult = completed.reduce((sum, j) => sum + j.result, 0);
            return {
                totalJobs: jobs.length,
                completedCount: completed.length,
                failedCount: failed.length,
                successRate: completed.length / jobs.length,
                totalResult,
                failureReasons: failed.map(j => j.reason)
            }
            "#,
            env.clone(),
            json!({
                "totalJobs": 5,
                "completedCount": 3,
                "failedCount": 2,
                "successRate": 0.6,
                "totalResult": 450,
                "failureReasons": ["timeout", "connection"]
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_flow_patterns_config_merge() {
        let mut env = HashMap::new();
        env.insert(
            "defaults".to_string(),
            Arc::new(to_raw_value(&json!({
                "timeout": 5000,
                "retries": 3,
                "headers": {"Content-Type": "application/json"},
                "features": {"logging": true, "caching": false}
            }))),
        );
        env.insert(
            "overrides".to_string(),
            Arc::new(to_raw_value(&json!({
                "timeout": 10000,
                "headers": {"Authorization": "Bearer token"},
                "features": {"caching": true}
            }))),
        );

        assert_eval(
            r#"({
                ...defaults,
                ...overrides,
                headers: { ...defaults.headers, ...overrides.headers },
                features: { ...defaults.features, ...overrides.features }
            })"#,
            env.clone(),
            json!({
                "timeout": 10000,
                "retries": 3,
                "headers": {"Content-Type": "application/json", "Authorization": "Bearer token"},
                "features": {"logging": true, "caching": true}
            }),
        )
        .await;
    }

    // =====================================================================
    // FLOW SIMULATION: Multi-step flow with various step results
    // =====================================================================

    fn create_multi_step_flow_context() -> (
        HashMap<String, Arc<Box<RawValue>>>,
        Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
        HashMap<String, Box<RawValue>>,
    ) {
        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), Arc::new(to_raw_value(&json!(42))));
        ctx.insert(
            "b".to_string(),
            Arc::new(to_raw_value(&json!({
                "status": "success",
                "data": {
                    "users": [
                        {"id": 1, "name": "Alice", "active": true, "roles": ["admin", "user"]},
                        {"id": 2, "name": "Bob", "active": false, "roles": ["user"]},
                        {"id": 3, "name": "Charlie", "active": true, "roles": ["moderator", "user"]}
                    ],
                    "total": 3,
                    "metadata": {"page": 1, "hasMore": true}
                }
            }))),
        );
        ctx.insert(
            "c".to_string(),
            Arc::new(to_raw_value(&json!([10, 20, 30, 40, 50]))),
        );
        ctx.insert("d".to_string(), Arc::new(to_raw_value(&json!(null))));
        ctx.insert(
            "f".to_string(),
            Arc::new(to_raw_value(&json!({
                "level1": {"level2": {"level3": {"level4": {"value": "deeply_nested"}}}}
            }))),
        );
        ctx.insert(
            "previous_result".to_string(),
            Arc::new(to_raw_value(&json!([
                "string", 123, true, null, {"key": "value"}, [1, 2, 3]
            ]))),
        );

        let mut fi = HashMap::new();
        fi.insert("name".to_string(), to_raw_value(&json!("test_flow")));
        fi.insert("count".to_string(), to_raw_value(&json!(100)));
        fi.insert("enabled".to_string(), to_raw_value(&json!(true)));
        fi.insert(
            "config".to_string(),
            to_raw_value(&json!({
                "timeout": 30, "retries": 3, "options": ["fast", "secure"]
            })),
        );
        fi.insert(
            "items".to_string(),
            to_raw_value(&json!([
                {"id": 1, "value": "first"},
                {"id": 2, "value": "second"},
                {"id": 3, "value": "third"}
            ])),
        );

        let mut fe = HashMap::new();
        fe.insert("ENV".to_string(), to_raw_value(&json!("production")));
        fe.insert("DEBUG".to_string(), to_raw_value(&json!(false)));
        fe.insert("VERSION".to_string(), to_raw_value(&json!("1.2.3")));

        (ctx, Some(mappable_rc::Marc::new(fi)), fe)
    }

    #[tokio::test]
    async fn test_flow_step_references() {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        assert_eval_full("a", ctx.clone(), fi.clone(), Some(&fe), json!(42)).await;
        assert_eval_full(
            "b.status",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!("success"),
        )
        .await;
        assert_eval_full("b.data.total", ctx.clone(), fi.clone(), Some(&fe), json!(3)).await;
        assert_eval_full(
            "b.data.users[0].name",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!("Alice"),
        )
        .await;
        assert_eval_full("c[0]", ctx.clone(), fi.clone(), Some(&fe), json!(10)).await;
        assert_eval_full(
            "c.map(x => x * 2)",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!([20, 40, 60, 80, 100]),
        )
        .await;
        assert_eval_full(
            "f.level1.level2.level3.level4.value",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!("deeply_nested"),
        )
        .await;
    }

    #[tokio::test]
    async fn test_flow_input_nested_and_combined() {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        assert_eval_full(
            "flow_input.name",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!("test_flow"),
        )
        .await;
        assert_eval_full(
            "flow_input.config.timeout",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(30),
        )
        .await;
        assert_eval_full(
            "flow_input.config.options[0]",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!("fast"),
        )
        .await;
        assert_eval_full(
            "flow_input.items.length",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(3),
        )
        .await;
        assert_eval_full(
            "flow_input.items[0].id",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(1),
        )
        .await;
        assert_eval_full(
            "flow_input.items.map(i => i.value)",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(["first", "second", "third"]),
        )
        .await;

        // Combining flow_input with step results
        assert_eval_full(
            "flow_input.count + a",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(142),
        )
        .await;
        assert_eval_full(
            "flow_input.enabled ? b.data.users : []",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!([
                {"id": 1, "name": "Alice", "active": true, "roles": ["admin", "user"]},
                {"id": 2, "name": "Bob", "active": false, "roles": ["user"]},
                {"id": 3, "name": "Charlie", "active": true, "roles": ["moderator", "user"]}
            ]),
        )
        .await;
    }

    #[tokio::test]
    async fn test_flow_env_access() {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        assert_eval_full(
            "flow_env.ENV",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!("production"),
        )
        .await;
        assert_eval_full(
            "flow_env.DEBUG",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(false),
        )
        .await;
        assert_eval_full(
            "flow_env.VERSION",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!("1.2.3"),
        )
        .await;
    }

    #[tokio::test]
    async fn test_flow_branch_conditions() {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        assert_eval_full("a > 40", ctx.clone(), fi.clone(), Some(&fe), json!(true)).await;
        assert_eval_full(
            "b.status === 'success'",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(true),
        )
        .await;
        assert_eval_full(
            "flow_input.enabled",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(true),
        )
        .await;
        assert_eval_full(
            "a > 40 && b.status === 'success'",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(true),
        )
        .await;
        assert_eval_full(
            "b.data.users.length > 0",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(true),
        )
        .await;
        assert_eval_full(
            "b.data.users.some(u => u.active)",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(true),
        )
        .await;
        assert_eval_full(
            "c.includes(30)",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(true),
        )
        .await;
    }

    #[tokio::test]
    async fn test_flow_complex_data_extraction() {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        assert_eval_full(
            "b.data.users.filter(u => u.active).map(u => u.name)",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(["Alice", "Charlie"]),
        )
        .await;

        assert_eval_full(
            "b.data.users.filter(u => u.roles.includes('admin'))[0]?.name",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!("Alice"),
        )
        .await;

        assert_eval_full(
            "b.data.users.reduce((acc, u) => acc + (u.active ? 1 : 0), 0)",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(2),
        )
        .await;

        // Combining multiple step results
        assert_eval_full("a + c[0]", ctx.clone(), fi.clone(), Some(&fe), json!(52)).await;
        assert_eval_full(
            "a * b.data.total",
            ctx.clone(),
            fi.clone(),
            Some(&fe),
            json!(126),
        )
        .await;
    }

    #[tokio::test]
    async fn test_flow_forloop_inner_expressions() {
        let mut ctx = HashMap::new();
        ctx.insert(
            "previous_result".to_string(),
            Arc::new(to_raw_value(&json!({"value": 42, "index": 2}))),
        );

        let mut fi = HashMap::new();
        fi.insert(
            "iter".to_string(),
            to_raw_value(&json!({"index": 2, "value": {"id": 3, "name": "test_item"}})),
        );
        fi.insert("name".to_string(), to_raw_value(&json!("parent_flow")));
        let fi = Some(mappable_rc::Marc::new(fi));

        assert_eval_full(
            "flow_input.iter.index",
            ctx.clone(),
            fi.clone(),
            None,
            json!(2),
        )
        .await;
        assert_eval_full(
            "flow_input.iter.value.id",
            ctx.clone(),
            fi.clone(),
            None,
            json!(3),
        )
        .await;
        assert_eval_full(
            "flow_input.iter.value.name",
            ctx.clone(),
            fi.clone(),
            None,
            json!("test_item"),
        )
        .await;
        assert_eval_full(
            "`Item ${flow_input.iter.index} of ${flow_input.name}`",
            ctx.clone(),
            fi.clone(),
            None,
            json!("Item 2 of parent_flow"),
        )
        .await;
    }

    // =====================================================================
    // EDGE CASES
    // =====================================================================

    #[tokio::test]
    async fn test_edge_cases_empty_values() {
        let mut env = HashMap::new();
        env.insert("emptyArray".to_string(), Arc::new(to_raw_value(&json!([]))));
        env.insert(
            "emptyObject".to_string(),
            Arc::new(to_raw_value(&json!({}))),
        );
        env.insert(
            "emptyString".to_string(),
            Arc::new(to_raw_value(&json!(""))),
        );
        env.insert("zero".to_string(), Arc::new(to_raw_value(&json!(0))));

        assert_eval("emptyArray.length", env.clone(), json!(0)).await;
        assert_eval("emptyArray.map(x => x * 2)", env.clone(), json!([])).await;
        assert_eval(
            "emptyArray.reduce((a, b) => a + b, 100)",
            env.clone(),
            json!(100),
        )
        .await;
        assert_eval("Object.keys(emptyObject)", env.clone(), json!([])).await;
        assert_eval("emptyString.length", env.clone(), json!(0)).await;
        assert_eval("emptyString || 'default'", env.clone(), json!("default")).await;
        assert_eval("emptyString ?? 'default'", env.clone(), json!("")).await;
        assert_eval("zero || 'default'", env.clone(), json!("default")).await;
        assert_eval("zero ?? 'default'", env.clone(), json!(0)).await;
    }

    #[tokio::test]
    async fn test_edge_cases_deep_access() {
        let mut env = HashMap::new();
        env.insert(
            "deep".to_string(),
            Arc::new(to_raw_value(
                &json!({"a": {"b": {"c": {"d": {"e": "found!"}}}}}),
            )),
        );

        assert_eval("deep.a.b.c.d.e", env.clone(), json!("found!")).await;
        assert_eval("deep?.a?.b?.c?.d?.e", env.clone(), json!("found!")).await;
        assert_eval("deep?.a?.b?.x?.y?.z", env.clone(), json!(null)).await;
        assert_eval(
            "deep?.a?.b?.x?.y?.z ?? 'not found'",
            env.clone(),
            json!("not found"),
        )
        .await;
    }

    #[tokio::test]
    async fn test_edge_cases_special_characters() {
        let mut env = HashMap::new();
        env.insert(
            "data".to_string(),
            Arc::new(to_raw_value(&json!({
                "key-with-dash": "value1",
                "key.with.dots": "value2",
                "key with spaces": "value3"
            }))),
        );

        assert_eval("data['key-with-dash']", env.clone(), json!("value1")).await;
        assert_eval("data['key.with.dots']", env.clone(), json!("value2")).await;
        assert_eval("data['key with spaces']", env.clone(), json!("value3")).await;
    }

    #[tokio::test]
    async fn test_edge_cases_boolean_coercion() {
        let env = HashMap::new();

        assert_eval("Boolean(0)", env.clone(), json!(false)).await;
        assert_eval("Boolean('')", env.clone(), json!(false)).await;
        assert_eval("Boolean(null)", env.clone(), json!(false)).await;
        assert_eval("Boolean(undefined)", env.clone(), json!(false)).await;
        assert_eval("Boolean(NaN)", env.clone(), json!(false)).await;
        assert_eval("Boolean(1)", env.clone(), json!(true)).await;
        assert_eval("Boolean('hello')", env.clone(), json!(true)).await;
        assert_eval("Boolean([])", env.clone(), json!(true)).await;
        assert_eval("Boolean({})", env.clone(), json!(true)).await;
        assert_eval("!!0", env.clone(), json!(false)).await;
        assert_eval("!!1", env.clone(), json!(true)).await;
    }

    // =====================================================================
    // PROMISE RESOLUTION
    // =====================================================================

    #[tokio::test]
    async fn test_promise_resolve() {
        let env = HashMap::new();

        assert_eval("Promise.resolve(42)", env.clone(), json!(42)).await;
        assert_eval(
            "Promise.resolve({ key: 'value' })",
            env.clone(),
            json!({"key": "value"}),
        )
        .await;
        assert_eval(
            "Promise.all([Promise.resolve(1), Promise.resolve(2), Promise.resolve(3)])",
            env.clone(),
            json!([1, 2, 3]),
        )
        .await;
    }

    // =====================================================================
    // eval_simple_js TESTS
    // =====================================================================

    #[tokio::test]
    async fn test_eval_simple_js_basic() {
        let mut globals = HashMap::new();
        globals.insert("job".to_string(), json!({"id": "abc", "input": {"x": 42}}));

        let result = eval_simple_js("job.input.x".to_string(), globals)
            .await
            .unwrap();
        assert_eq!(result.get(), "42");
    }

    #[tokio::test]
    async fn test_eval_simple_js_string() {
        let mut globals = HashMap::new();
        globals.insert("name".to_string(), json!("World"));

        let result = eval_simple_js("`Hello ${name}!`".to_string(), globals)
            .await
            .unwrap();
        assert_eq!(result.get(), "\"Hello World!\"");
    }

    #[tokio::test]
    async fn test_eval_simple_js_array_transform() {
        let mut globals = HashMap::new();
        globals.insert("data".to_string(), json!([1, 2, 3, 4, 5]));

        let result = eval_simple_js(
            "data.filter(x => x > 2).map(x => x * 10)".to_string(),
            globals,
        )
        .await
        .unwrap();
        let actual: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(actual, json!([30, 40, 50]));
    }

    #[tokio::test]
    async fn test_eval_simple_js_null_handling() {
        let mut globals = HashMap::new();
        globals.insert("x".to_string(), json!(null));

        let result = eval_simple_js("x ?? 'default'".to_string(), globals)
            .await
            .unwrap();
        assert_eq!(result.get(), "\"default\"");
    }

    #[tokio::test]
    async fn test_eval_simple_js_multiple_globals() {
        let mut globals = HashMap::new();
        globals.insert("a".to_string(), json!(10));
        globals.insert("b".to_string(), json!(20));
        globals.insert("prefix".to_string(), json!("result"));

        let result = eval_simple_js(
            "({ label: `${prefix}_sum`, value: a + b })".to_string(),
            globals,
        )
        .await
        .unwrap();
        let actual: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(actual, json!({"label": "result_sum", "value": 30}));
    }

    #[tokio::test]
    async fn test_eval_simple_js_error_handling() {
        let globals = HashMap::new();

        // Invalid JS should return an error
        let result = eval_simple_js("this is not valid js @#$".to_string(), globals).await;
        assert!(result.is_err());
    }
}
