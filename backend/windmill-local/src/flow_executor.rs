//! Flow executor for local mode
//!
//! This module implements flow execution using the real Windmill flow types
//! from windmill-common, but with libSQL as the backend.

use std::collections::HashMap;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use uuid::Uuid;

use windmill_common::flows::{
    Branch, FlowModule, FlowModuleValue, FlowValue, InputTransform,
};
use windmill_common::scripts::ScriptLang as WmScriptLang;

use crate::db::LocalDb;
use crate::error::{LocalError, Result};
use crate::executor::{execute_script, ExecutionResult};
use crate::jobs::ScriptLang;

/// Flow execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStatus {
    pub step: usize,
    pub modules: Vec<ModuleStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_module: Option<FailureModule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStatus {
    pub id: String,
    #[serde(rename = "type")]
    pub status_type: ModuleStatusType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterator: Option<IteratorStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_chosen: Option<BranchChosen>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branchall: Option<BranchAllStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ModuleStatusType {
    WaitingForPriorSteps,
    WaitingForEvents,
    WaitingForExecutor,
    InProgress,
    Success,
    Failure,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IteratorStatus {
    pub index: usize,
    pub itered: Vec<serde_json::Value>,
    pub args: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchChosen {
    #[serde(rename = "type")]
    pub branch_type: String,
    pub branch: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchAllStatus {
    pub branch: usize,
    pub len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureModule {
    pub id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStatus {
    pub fail_count: u32,
}

/// Context passed through flow execution
#[derive(Debug, Clone)]
pub struct FlowContext {
    pub flow_input: serde_json::Value,
    pub previous_result: serde_json::Value,
    pub results_by_id: HashMap<String, serde_json::Value>,
}

impl FlowContext {
    pub fn new(flow_input: serde_json::Value) -> Self {
        Self {
            flow_input: flow_input.clone(),
            previous_result: flow_input,
            results_by_id: HashMap::new(),
        }
    }

    /// Get the evaluation context for input transforms
    pub fn to_eval_context(&self) -> serde_json::Value {
        serde_json::json!({
            "flow_input": self.flow_input,
            "previous_result": self.previous_result,
            "results": self.results_by_id,
        })
    }
}

/// Execute a flow and return the result
pub async fn execute_flow(
    db: &LocalDb,
    flow_value: &FlowValue,
    flow_input: serde_json::Value,
) -> Result<(serde_json::Value, FlowStatus)> {
    let mut ctx = FlowContext::new(flow_input);
    let mut status = FlowStatus {
        step: 0,
        modules: Vec::new(),
        failure_module: None,
        retry: None,
    };

    // Execute modules sequentially
    for (idx, module) in flow_value.modules.iter().enumerate() {
        status.step = idx;

        let module_status = ModuleStatus {
            id: module.id.clone(),
            status_type: ModuleStatusType::InProgress,
            result: None,
            iterator: None,
            branch_chosen: None,
            branchall: None,
        };
        status.modules.push(module_status);

        tracing::info!("Executing flow module {}: {}", idx, module.id);

        match execute_module(db, module, &mut ctx).await {
            Ok(result) => {
                // Update context with result
                ctx.results_by_id.insert(module.id.clone(), result.clone());
                ctx.previous_result = result.clone();

                // Update status
                if let Some(ms) = status.modules.last_mut() {
                    ms.status_type = ModuleStatusType::Success;
                    ms.result = Some(result);
                }
            }
            Err(e) => {
                // Module failed
                tracing::error!("Flow module {} failed: {}", module.id, e);

                if let Some(ms) = status.modules.last_mut() {
                    ms.status_type = ModuleStatusType::Failure;
                    ms.result = Some(serde_json::json!({"error": e.to_string()}));
                }

                status.failure_module = Some(FailureModule {
                    id: module.id.clone(),
                    error: e.to_string(),
                });

                // Check if continue_on_error is set
                if !module.continue_on_error.unwrap_or(false) {
                    return Ok((serde_json::json!({"error": e.to_string()}), status));
                }
            }
        }
    }

    Ok((ctx.previous_result, status))
}

/// Execute a single flow module
#[async_recursion::async_recursion]
async fn execute_module(
    db: &LocalDb,
    module: &FlowModule,
    ctx: &mut FlowContext,
) -> Result<serde_json::Value> {
    // Check skip_if condition
    if let Some(skip_if) = &module.skip_if {
        let should_skip = evaluate_expr(&skip_if.expr, &ctx.to_eval_context())?;
        if should_skip.as_bool().unwrap_or(false) {
            tracing::info!("Skipping module {} due to skip_if condition", module.id);
            return Ok(ctx.previous_result.clone());
        }
    }

    // Parse the module value
    let module_value: FlowModuleValue = serde_json::from_str(module.value.get())
        .map_err(|e| LocalError::Execution(format!("Failed to parse module value: {}", e)))?;

    match module_value {
        FlowModuleValue::Identity => {
            Ok(ctx.previous_result.clone())
        }

        FlowModuleValue::RawScript { content, language, input_transforms, .. } => {
            let args = resolve_input_transforms(&input_transforms, ctx)?;
            let lang = convert_script_lang(&language);
            let result = execute_script(lang, &content, &args).await?;
            if result.success {
                Ok(result.result)
            } else {
                Err(LocalError::Execution(result.result.to_string()))
            }
        }

        FlowModuleValue::Script { path, input_transforms, .. } => {
            // In local mode, we don't have access to saved scripts
            Err(LocalError::Execution(format!(
                "Script references (path: {}) are not supported in local mode. Use rawscript instead.",
                path
            )))
        }

        FlowModuleValue::Flow { path, .. } => {
            // In local mode, we don't have access to saved flows
            Err(LocalError::Execution(format!(
                "Flow references (path: {}) are not supported in local mode. Use inline modules instead.",
                path
            )))
        }

        FlowModuleValue::ForloopFlow { iterator, modules, skip_failures, parallel, .. } => {
            execute_forloop(db, &iterator, &modules, skip_failures, parallel, ctx).await
        }

        FlowModuleValue::WhileloopFlow { modules, skip_failures, .. } => {
            execute_whileloop(db, &modules, skip_failures, ctx).await
        }

        FlowModuleValue::BranchOne { branches, default, .. } => {
            execute_branch_one(db, &branches, &default, ctx).await
        }

        FlowModuleValue::BranchAll { branches, parallel } => {
            execute_branch_all(db, &branches, parallel, ctx).await
        }

        FlowModuleValue::FlowScript { .. } => {
            Err(LocalError::Execution(
                "FlowScript (internal reference) is not supported in local mode".to_string()
            ))
        }

        FlowModuleValue::AIAgent { .. } => {
            Err(LocalError::Execution(
                "AIAgent is not supported in local mode".to_string()
            ))
        }
    }
}

/// Execute a for loop
#[async_recursion::async_recursion]
async fn execute_forloop(
    db: &LocalDb,
    iterator: &InputTransform,
    modules: &[FlowModule],
    skip_failures: bool,
    _parallel: bool,  // TODO: implement parallel execution
    ctx: &mut FlowContext,
) -> Result<serde_json::Value> {
    // Evaluate the iterator expression
    let iter_value = evaluate_input_transform(iterator, ctx)?;

    let items = match iter_value.as_array() {
        Some(arr) => arr.clone(),
        None => {
            return Err(LocalError::Execution(
                "For loop iterator must evaluate to an array".to_string()
            ));
        }
    };

    let mut results = Vec::new();

    for (idx, item) in items.iter().enumerate() {
        tracing::debug!("For loop iteration {} of {}", idx + 1, items.len());

        // Create iteration context
        let mut iter_ctx = FlowContext {
            flow_input: ctx.flow_input.clone(),
            previous_result: item.clone(),
            results_by_id: ctx.results_by_id.clone(),
        };

        // Add iter context
        iter_ctx.results_by_id.insert("iter".to_string(), serde_json::json!({
            "index": idx,
            "value": item,
        }));

        // Execute modules in sequence
        let mut iter_result = item.clone();
        let mut had_error = false;

        for module in modules {
            match execute_module(db, module, &mut iter_ctx).await {
                Ok(result) => {
                    iter_ctx.results_by_id.insert(module.id.clone(), result.clone());
                    iter_ctx.previous_result = result.clone();
                    iter_result = result;
                }
                Err(e) => {
                    had_error = true;
                    if skip_failures {
                        tracing::warn!("For loop iteration {} failed (skipping): {}", idx, e);
                        iter_result = serde_json::json!({"error": e.to_string()});
                    } else {
                        return Err(e);
                    }
                    break;
                }
            }
        }

        if !had_error || skip_failures {
            results.push(iter_result);
        }
    }

    Ok(serde_json::Value::Array(results))
}

/// Execute a while loop
#[async_recursion::async_recursion]
async fn execute_whileloop(
    db: &LocalDb,
    modules: &[FlowModule],
    skip_failures: bool,
    ctx: &mut FlowContext,
) -> Result<serde_json::Value> {
    const MAX_ITERATIONS: usize = 1000;
    let mut results = Vec::new();
    let mut iteration = 0;

    loop {
        if iteration >= MAX_ITERATIONS {
            return Err(LocalError::Execution(format!(
                "While loop exceeded maximum iterations ({})", MAX_ITERATIONS
            )));
        }

        // Execute modules
        let mut iter_result = ctx.previous_result.clone();
        let mut should_continue = true;

        for module in modules {
            match execute_module(db, module, ctx).await {
                Ok(result) => {
                    ctx.results_by_id.insert(module.id.clone(), result.clone());
                    ctx.previous_result = result.clone();
                    iter_result = result;
                }
                Err(e) => {
                    if skip_failures {
                        tracing::warn!("While loop iteration {} failed (skipping): {}", iteration, e);
                        iter_result = serde_json::json!({"error": e.to_string()});
                    } else {
                        return Err(e);
                    }
                    should_continue = false;
                    break;
                }
            }
        }

        results.push(iter_result);
        iteration += 1;

        // Check stop condition (result should be truthy to continue)
        if !should_continue {
            break;
        }

        // Check if the result indicates we should stop
        let continue_loop = match &ctx.previous_result {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::Null => false,
            serde_json::Value::Object(obj) => {
                // Check for a "continue" field
                obj.get("continue")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            }
            _ => false,
        };

        if !continue_loop {
            break;
        }
    }

    Ok(serde_json::Value::Array(results))
}

/// Execute branch-one (if/else)
#[async_recursion::async_recursion]
async fn execute_branch_one(
    db: &LocalDb,
    branches: &[Branch],
    default: &[FlowModule],
    ctx: &mut FlowContext,
) -> Result<serde_json::Value> {
    // Find the first matching branch
    for (idx, branch) in branches.iter().enumerate() {
        let condition = evaluate_expr(&branch.expr, &ctx.to_eval_context())?;
        if condition.as_bool().unwrap_or(false) {
            tracing::debug!("Branch {} matched", idx);
            return execute_branch_modules(db, &branch.modules, ctx).await;
        }
    }

    // No branch matched, execute default
    tracing::debug!("No branch matched, executing default");
    execute_branch_modules(db, default, ctx).await
}

/// Execute branch-all (parallel branches)
#[async_recursion::async_recursion]
async fn execute_branch_all(
    db: &LocalDb,
    branches: &[Branch],
    _parallel: bool,  // TODO: implement true parallel execution
    ctx: &mut FlowContext,
) -> Result<serde_json::Value> {
    let mut results = Vec::new();

    // Execute all branches (sequentially for now)
    for (idx, branch) in branches.iter().enumerate() {
        tracing::debug!("Executing branch {}", idx);

        let mut branch_ctx = ctx.clone();
        match execute_branch_modules(db, &branch.modules, &mut branch_ctx).await {
            Ok(result) => {
                results.push(result);
            }
            Err(e) => {
                if branch.skip_failure {
                    tracing::warn!("Branch {} failed (skipping): {}", idx, e);
                    results.push(serde_json::json!({"error": e.to_string()}));
                } else {
                    return Err(e);
                }
            }
        }
    }

    Ok(serde_json::Value::Array(results))
}

/// Execute a sequence of modules in a branch
#[async_recursion::async_recursion]
async fn execute_branch_modules(
    db: &LocalDb,
    modules: &[FlowModule],
    ctx: &mut FlowContext,
) -> Result<serde_json::Value> {
    let mut result = ctx.previous_result.clone();

    for module in modules {
        result = execute_module(db, module, ctx).await?;
        ctx.results_by_id.insert(module.id.clone(), result.clone());
        ctx.previous_result = result.clone();
    }

    Ok(result)
}

/// Resolve input transforms to concrete arguments
fn resolve_input_transforms(
    transforms: &HashMap<String, InputTransform>,
    ctx: &FlowContext,
) -> Result<serde_json::Value> {
    let mut args = serde_json::Map::new();

    for (key, transform) in transforms {
        let value = evaluate_input_transform(transform, ctx)?;
        args.insert(key.clone(), value);
    }

    Ok(serde_json::Value::Object(args))
}

/// Evaluate an input transform
fn evaluate_input_transform(
    transform: &InputTransform,
    ctx: &FlowContext,
) -> Result<serde_json::Value> {
    match transform {
        InputTransform::Static { value } => {
            serde_json::from_str(value.get())
                .map_err(|e| LocalError::Execution(format!("Invalid static value: {}", e)))
        }
        InputTransform::Javascript { expr } => {
            evaluate_expr(expr, &ctx.to_eval_context())
        }
        InputTransform::Ai => {
            Err(LocalError::Execution("AI input transforms are not supported in local mode".to_string()))
        }
    }
}

/// Evaluate a JavaScript expression
fn evaluate_expr(expr: &str, context: &serde_json::Value) -> Result<serde_json::Value> {
    let expr = expr.trim();

    // Handle comparison operators
    if let Some(result) = try_evaluate_comparison(expr, context) {
        return Ok(result);
    }

    // Handle simple variable references
    if let Some(val) = resolve_path(expr, context) {
        return Ok(val);
    }

    // Handle boolean literals
    if expr == "true" {
        return Ok(serde_json::Value::Bool(true));
    }
    if expr == "false" {
        return Ok(serde_json::Value::Bool(false));
    }

    // Handle numeric literals
    if let Ok(n) = expr.parse::<i64>() {
        return Ok(serde_json::json!(n));
    }
    if let Ok(n) = expr.parse::<f64>() {
        return Ok(serde_json::json!(n));
    }

    // Handle string literals
    if (expr.starts_with('"') && expr.ends_with('"')) ||
       (expr.starts_with('\'') && expr.ends_with('\'')) {
        return Ok(serde_json::json!(&expr[1..expr.len()-1]));
    }

    // For complex expressions, we'd need a full JS runtime
    tracing::warn!("Complex expression not evaluated: {}", expr);
    Ok(serde_json::json!(expr))
}

/// Try to resolve a path like "flow_input.x" or "results.a.b"
fn resolve_path(path: &str, context: &serde_json::Value) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    let mut current = context.get(parts[0])?;

    for part in &parts[1..] {
        current = current.get(*part)?;
    }

    Some(current.clone())
}

/// Try to evaluate a comparison expression
fn try_evaluate_comparison(expr: &str, context: &serde_json::Value) -> Option<serde_json::Value> {
    // Supported operators: >, <, >=, <=, ==, !=, ===, !==
    let operators = ["===", "!==", ">=", "<=", "==", "!=", ">", "<"];

    for op in operators {
        if let Some(pos) = expr.find(op) {
            let left = expr[..pos].trim();
            let right = expr[pos + op.len()..].trim();

            let left_val = evaluate_expr(left, context).ok()?;
            let right_val = evaluate_expr(right, context).ok()?;

            let result = match op {
                ">" => compare_values(&left_val, &right_val, |a, b| a > b),
                "<" => compare_values(&left_val, &right_val, |a, b| a < b),
                ">=" => compare_values(&left_val, &right_val, |a, b| a >= b),
                "<=" => compare_values(&left_val, &right_val, |a, b| a <= b),
                "==" | "===" => Some(left_val == right_val),
                "!=" | "!==" => Some(left_val != right_val),
                _ => None,
            };

            return result.map(serde_json::Value::Bool);
        }
    }

    // Try logical operators
    if let Some(pos) = expr.find("&&") {
        let left = expr[..pos].trim();
        let right = expr[pos + 2..].trim();
        let left_val = evaluate_expr(left, context).ok()?;
        let right_val = evaluate_expr(right, context).ok()?;
        return Some(serde_json::Value::Bool(
            is_truthy(&left_val) && is_truthy(&right_val)
        ));
    }

    if let Some(pos) = expr.find("||") {
        let left = expr[..pos].trim();
        let right = expr[pos + 2..].trim();
        let left_val = evaluate_expr(left, context).ok()?;
        let right_val = evaluate_expr(right, context).ok()?;
        return Some(serde_json::Value::Bool(
            is_truthy(&left_val) || is_truthy(&right_val)
        ));
    }

    None
}

/// Compare two JSON values numerically
fn compare_values<F>(left: &serde_json::Value, right: &serde_json::Value, cmp: F) -> Option<bool>
where
    F: Fn(f64, f64) -> bool,
{
    let left_num = value_to_number(left)?;
    let right_num = value_to_number(right)?;
    Some(cmp(left_num, right_num))
}

/// Convert a JSON value to a number
fn value_to_number(val: &serde_json::Value) -> Option<f64> {
    match val {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse().ok(),
        serde_json::Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Check if a value is truthy (JavaScript semantics)
fn is_truthy(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(_) => true,
    }
}

/// Convert windmill-common ScriptLang to local ScriptLang
fn convert_script_lang(lang: &WmScriptLang) -> ScriptLang {
    match lang {
        WmScriptLang::Deno => ScriptLang::Deno,
        WmScriptLang::Python3 => ScriptLang::Python3,
        WmScriptLang::Bash => ScriptLang::Bash,
        WmScriptLang::Go => ScriptLang::Go,
        WmScriptLang::Bun => ScriptLang::Bun,
        WmScriptLang::Nativets => ScriptLang::Deno,  // Fallback to Deno
        _ => ScriptLang::Bash,  // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_simple_expr() {
        let ctx = serde_json::json!({
            "flow_input": {"x": 10},
            "previous_result": 42,
            "results": {"a": "hello"},
        });

        assert_eq!(
            evaluate_expr("flow_input", &ctx).unwrap(),
            serde_json::json!({"x": 10})
        );
        assert_eq!(
            evaluate_expr("previous_result", &ctx).unwrap(),
            serde_json::json!(42)
        );
        assert_eq!(
            evaluate_expr("results.a", &ctx).unwrap(),
            serde_json::json!("hello")
        );
        assert_eq!(
            evaluate_expr("flow_input.x", &ctx).unwrap(),
            serde_json::json!(10)
        );
    }

    #[test]
    fn test_evaluate_comparison_expr() {
        let ctx = serde_json::json!({
            "flow_input": {"x": 10, "y": 5},
            "previous_result": 42,
        });

        assert_eq!(
            evaluate_expr("flow_input.x > 5", &ctx).unwrap(),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            evaluate_expr("flow_input.x < 5", &ctx).unwrap(),
            serde_json::Value::Bool(false)
        );
        assert_eq!(
            evaluate_expr("flow_input.x >= 10", &ctx).unwrap(),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            evaluate_expr("flow_input.y == 5", &ctx).unwrap(),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            evaluate_expr("previous_result > 40", &ctx).unwrap(),
            serde_json::Value::Bool(true)
        );
    }

    #[test]
    fn test_evaluate_logical_expr() {
        let ctx = serde_json::json!({
            "flow_input": {"a": true, "b": false},
        });

        // Simple boolean logic (complex expressions with mixed operators need parentheses support)
        assert_eq!(
            evaluate_expr("flow_input.a && flow_input.a", &ctx).unwrap(),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            evaluate_expr("flow_input.a && flow_input.b", &ctx).unwrap(),
            serde_json::Value::Bool(false)
        );
        assert_eq!(
            evaluate_expr("flow_input.b || flow_input.a", &ctx).unwrap(),
            serde_json::Value::Bool(true)
        );
    }
}
