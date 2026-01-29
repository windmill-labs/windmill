/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! QuickJS-based JavaScript expression evaluation for flow transformations.
//!
//! This module provides an alternative to deno_core for evaluating arbitrary JavaScript
//! expressions in flow transformations. QuickJS offers significantly faster startup times
//! (~200μs vs ~3ms for V8), making it ideal for evaluating many small expressions.
//!
//! ## Performance Characteristics (release mode benchmarks)
//! - **Simple expressions**: ~238μs (QuickJS) vs ~3.05ms (deno_core) = **~13x faster**
//! - **Complex expressions**: ~192μs (QuickJS) vs ~3.09ms (deno_core) = **~16x faster**
//! - **Memory**: ~2.5% of V8's footprint
//!
//! For flow expression evaluation where startup time dominates, QuickJS is significantly
//! faster overall despite being slower for long-running CPU-intensive code.
//!
//! ## Async Operations
//! This implementation uses true async Rust callbacks (similar to deno_core's ops) for
//! `variable()`, `resource()`, and `results.xxx` access. The async functions use
//! rquickjs's `Async<MutFn<...>>` wrapper which returns JavaScript Promises that are
//! resolved when the Rust async operations complete. No pre-fetching is required.

use std::collections::HashMap;
use std::sync::Arc;

use rquickjs::{
    async_with,
    prelude::{Async, Func, MutFn},
    AsyncContext, AsyncRuntime, CatchResultExt, FromJs, IntoJs, Object, Value,
};
use serde_json::value::RawValue;

use windmill_common::client::AuthedClient;
use windmill_common::flow_status::JobResult;

use crate::js_eval::{replace_with_await, replace_with_await_result, IdContext};

/// Shared state for async operations within QuickJS
#[derive(Clone)]
struct AsyncOpState {
    client: AuthedClient,
}

/// Evaluates a JavaScript expression using QuickJS runtime.
///
/// This function provides the same interface as `eval_timeout` but uses QuickJS
/// instead of deno_core/V8 for potentially faster startup times.
///
/// Unlike deno_core, this uses true async Rust callbacks for `variable()`,
/// `resource()`, and `results.xxx` access - no pre-fetching required.
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

    if !context_keys.contains(&"previous_result".to_string())
        && (p_ids.is_some() && p_ids.as_ref().unwrap().iter().any(|x| expr.contains(x)))
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

    // Run the QuickJS evaluation with a timeout
    tokio::time::timeout(
        std::time::Duration::from_millis(10000),
        tokio::task::spawn_blocking(move || {
            // Create a new tokio runtime for async operations within the blocking context
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;

            rt.block_on(async move {
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
            "The expression evaluation `{expr}` took too long to execute (>10000ms)"
        )
    })??
}

/// Memory limit for QuickJS runtime (32MB).
/// This is much smaller than deno_core's 128MB limit since flow expressions
/// should be lightweight transformations, not memory-intensive operations.
const QUICKJS_MEMORY_LIMIT: usize = 32 * 1024 * 1024;

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

        // Determine if we need to add return statement
        let code = if should_add_return_quickjs(&transformed_expr) {
            format!("(async function() {{ return {}; }})()", transformed_expr)
        } else {
            format!("(async function() {{ {} }})()", transformed_expr)
        };

        // Evaluate the expression (returns a Promise)
        let promise: rquickjs::Promise = ctx.eval(code).catch(&ctx).map_err(quickjs_error_to_anyhow)?;

        // Await the promise
        let result: Value = promise.into_future().await.catch(&ctx).map_err(quickjs_error_to_anyhow)?;

        // Convert result to JSON
        let json_result = js_to_json(&ctx, &result)?;
        let json_str = serde_json::to_string(&json_result)?;

        Ok(windmill_common::worker::to_raw_value(&serde_json::from_str::<serde_json::Value>(&json_str)?))
    })
    .await
}

/// Set up async variable() and resource() functions using true Rust async callbacks.
///
/// This uses rquickjs's `Async<MutFn<...>>` wrapper to create JavaScript functions that
/// return Promises. The Promises are resolved by spawned Rust async operations.
fn setup_async_ops<'js>(
    ctx: &rquickjs::Ctx<'js>,
    globals: &Object<'js>,
    state: Arc<AsyncOpState>,
) -> anyhow::Result<()> {
    // Error prefix - must match the JavaScript side
    const ERR_PREFIX: &str = "\x00__WINDMILL_ERR__\x00";

    // Create variable() function with true async Rust callback
    // Returns a JSON string that JavaScript will parse
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

    // Create resource() function - returns JSON string
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
                    Ok(value) => serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()),
                    Err(e) => format!("{}{}", ERR_PREFIX, e),
                }
            }
        }))),
    )?;

    // Create JavaScript wrappers that parse the JSON results
    // We use a unique prefix that's extremely unlikely to appear in real data
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

/// Set up stub functions that throw errors when no client is available
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

/// Set up the `results` Proxy object with dynamic access to step results.
///
/// Uses async Rust callbacks to fetch results on-demand when accessed.
fn setup_results_proxy<'js>(
    ctx: &rquickjs::Ctx<'js>,
    globals: &Object<'js>,
    by_id: &IdContext,
    op_state: Option<Arc<AsyncOpState>>,
) -> anyhow::Result<()> {
    // Store previous_id for the shortcut optimization
    globals.set("__previous_id", by_id.previous_id.clone())?;

    // Create async __getResult function that fetches step results via Rust
    if let Some(state) = op_state {
        let by_id_for_result = by_id.clone();
        globals.set(
            "__fetchResult",
            Func::from(Async(MutFn::new(move |step_id: String| {
                let client = state.client.clone();
                let by_id = by_id_for_result.clone();
                let step_id_clone = step_id.clone();

                // Look up the job ID(s) for this step from the local cache
                let job_result = by_id.steps_results.get(&step_id).cloned();
                let flow_job_id = by_id.flow_job.to_string();

                async move {
                    const ERR_PREFIX: &str = "\x00__WINDMILL_ERR__\x00";

                    let result: Result<serde_json::Value, String> = match job_result {
                        Some(jr) => {
                            // Found in local cache, fetch result by job ID
                            match jr {
                                JobResult::SingleJob(job_id) => {
                                    client
                                        .get_completed_job_result::<serde_json::Value>(&job_id.to_string(), None)
                                        .await
                                        .map_err(|e| format!("Failed to fetch result for step '{}': {}", step_id_clone, e))
                                }
                                JobResult::ListJob(job_ids) => {
                                    let futs = job_ids.iter().map(|job_id| {
                                        let client = client.clone();
                                        let job_id_str = job_id.to_string();
                                        async move {
                                            client
                                                .get_completed_job_result::<serde_json::Value>(&job_id_str, None)
                                                .await
                                        }
                                    });
                                    let results: Vec<_> = futures::future::join_all(futs).await;
                                    let collected: Result<Vec<_>, _> = results.into_iter().collect();
                                    collected
                                        .map(serde_json::Value::Array)
                                        .map_err(|e| format!("Failed to fetch results for step '{}': {}", step_id_clone, e))
                                }
                            }
                        }
                        None => {
                            // Not in local cache, fallback to querying by flow_job_id and step_id
                            // This happens for branch modules that need to access parent flow step results
                            // Use .ok() to match deno_core behavior: return null for non-existent steps
                            // instead of throwing an error
                            Ok(client
                                .get_result_by_id::<serde_json::Value>(&flow_job_id, &step_id_clone, None)
                                .await
                                .ok()  // Swallow errors, convert to Option
                                .unwrap_or(serde_json::Value::Null))  // None -> null
                        }
                    };

                    match result {
                        Ok(value) => serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()),
                        Err(e) => format!("{}{}", ERR_PREFIX, e),
                    }
                }
            }))),
        )?;

        // Create JavaScript wrapper that parses the JSON result
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
        // No client - stub function that rejects
        let stub_code = r#"
            function __getResult(stepId) {
                return Promise.reject(new Error('Result fetching not available without authenticated client'));
            }
        "#;
        ctx.eval::<(), _>(stub_code)
            .catch(ctx)
            .map_err(quickjs_error_to_anyhow)?;
    }

    // Create the results proxy that calls __getResult for on-demand fetching
    // Matches deno_core behavior: always try to fetch, let backend handle unknown step IDs
    let proxy_setup = r#"
        const results = new Proxy({}, {
            get: function(target, name, receiver) {
                // Handle symbol properties (like Symbol.toStringTag)
                if (typeof name === 'symbol') {
                    return undefined;
                }

                // Check if it's the previous_id and previous_result exists
                if (name === __previous_id && typeof previous_result !== 'undefined') {
                    return Promise.resolve(previous_result);
                }

                // Always try to fetch - let Rust/backend handle unknown step IDs
                // This matches deno_core behavior
                return __getResult(name);
            }
        });
    "#;
    ctx.eval::<(), _>(proxy_setup)
        .catch(ctx)
        .map_err(quickjs_error_to_anyhow)?;

    Ok(())
}

/// Convert a serde_json::Value to a QuickJS Value
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

/// Convert a QuickJS Value to a serde_json::Value
fn js_to_json<'js>(
    ctx: &rquickjs::Ctx<'js>,
    val: &Value<'js>,
) -> anyhow::Result<serde_json::Value> {
    if val.is_null() || val.is_undefined() {
        return Ok(serde_json::Value::Null);
    }

    if let Some(b) = val.as_bool() {
        return Ok(serde_json::Value::Bool(b));
    }

    if let Some(i) = val.as_int() {
        return Ok(serde_json::Value::Number(i.into()));
    }

    if let Some(f) = val.as_float() {
        // Check if this float represents an exact integer
        // This preserves integer formatting for values like timestamps
        if f.fract() == 0.0 && f.abs() <= (i64::MAX as f64) {
            let i = f as i64;
            // Verify the conversion is exact (for very large numbers)
            if (i as f64) == f {
                return Ok(serde_json::Value::Number(i.into()));
            }
        }
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Ok(serde_json::Value::Number(n));
        } else {
            return Ok(serde_json::Value::Null);
        }
    }

    if let Ok(s) = String::from_js(ctx, val.clone()) {
        return Ok(serde_json::Value::String(s));
    }

    if let Ok(arr) = rquickjs::Array::from_js(ctx, val.clone()) {
        let mut json_arr = Vec::new();
        for i in 0..arr.len() {
            if let Ok(item) = arr.get::<Value>(i) {
                json_arr.push(js_to_json(ctx, &item)?);
            }
        }
        return Ok(serde_json::Value::Array(json_arr));
    }

    if let Ok(obj) = Object::from_js(ctx, val.clone()) {
        let mut json_obj = serde_json::Map::new();
        for res in obj.props::<String, Value>() {
            if let Ok((k, v)) = res {
                json_obj.insert(k, js_to_json(ctx, &v)?);
            }
        }
        return Ok(serde_json::Value::Object(json_obj));
    }

    // Fallback
    Ok(serde_json::Value::String("[object]".to_string()))
}

/// Determines if we should prepend "return" to the expression
fn should_add_return_quickjs(expr: &str) -> bool {
    let trimmed = expr.trim();

    if trimmed.is_empty() {
        return true;
    }

    if trimmed.starts_with("return ") || trimmed.starts_with("return;") || trimmed == "return" {
        return false;
    }

    let statement_prefixes = [
        "const ", "let ", "var ", "if ", "if(", "for ", "for(", "while ", "while(", "switch ",
        "switch(", "try ", "try{", "throw ", "function ", "class ", "async ", "await ",
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

fn quickjs_error_to_anyhow(err: rquickjs::CaughtError<'_>) -> anyhow::Error {
    anyhow::anyhow!("QuickJS evaluation error: {}", err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use windmill_common::worker::to_raw_value;

    #[tokio::test]
    async fn test_eval_quickjs_simple() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(5))));
        env.insert("y".to_string(), Arc::new(to_raw_value(&json!(3))));

        let result =
            eval_timeout_quickjs("x + y".to_string(), env, None, None, None, None, None).await?;

        assert_eq!(result.get(), "8");
        Ok(())
    }

    #[tokio::test]
    async fn test_eval_quickjs_object_access() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "params".to_string(),
            Arc::new(to_raw_value(&json!({"test": 42, "nested": {"value": 100}}))),
        );

        let result = eval_timeout_quickjs(
            "params.test".to_string(),
            env.clone(),
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

        assert_eq!(result.get(), "42");

        let result2 = eval_timeout_quickjs(
            "params.nested.value".to_string(),
            env,
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

        assert_eq!(result2.get(), "100");
        Ok(())
    }

    #[tokio::test]
    async fn test_eval_quickjs_array() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "arr".to_string(),
            Arc::new(to_raw_value(&json!([1, 2, 3, 4, 5]))),
        );

        let result = eval_timeout_quickjs(
            "arr.map(x => x * 2)".to_string(),
            env,
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

        assert_eq!(result.get(), "[2,4,6,8,10]");
        Ok(())
    }

    #[tokio::test]
    async fn test_eval_quickjs_flow_input() -> anyhow::Result<()> {
        let mut flow_input = HashMap::new();
        flow_input.insert("name".to_string(), to_raw_value(&json!("test")));
        flow_input.insert("count".to_string(), to_raw_value(&json!(10)));

        let result = eval_timeout_quickjs(
            "flow_input.name".to_string(),
            HashMap::new(),
            Some(mappable_rc::Marc::new(flow_input)),
            None,
            None,
            None,
            None,
        )
        .await?;

        assert_eq!(result.get(), "\"test\"");
        Ok(())
    }

    #[test]
    fn test_should_add_return_quickjs() {
        assert!(should_add_return_quickjs("5"));
        assert!(should_add_return_quickjs("x + y"));
        assert!(should_add_return_quickjs("foo()"));

        assert!(!should_add_return_quickjs("return 5"));
        assert!(!should_add_return_quickjs("return x + y"));

        assert!(!should_add_return_quickjs("const x = 5"));
        assert!(!should_add_return_quickjs("let y = 10"));
        assert!(!should_add_return_quickjs("if (x > 5) { return x; }"));

        assert!(!should_add_return_quickjs("let x = 5; x + 1"));
    }
}
