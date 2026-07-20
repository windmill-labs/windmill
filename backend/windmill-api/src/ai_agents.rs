//! Endpoints for reusable AI agents (the `ai_agent` resource type).
//!
//! Agents are run by pushing a single-module flow-preview job whose one step is an
//! `AIAgent` module. For a saved agent that step is *linked* (`agent: Some(path)`), so the
//! brain config + tools resolve at runtime from the resource via the same hybrid-linking path
//! the flow executor uses. The eval judge is itself run as an inline `AIAgent` step with a
//! structured `output_schema`, so the whole feature reuses the existing job machinery.

use std::collections::HashMap;
use std::time::Instant;

use axum::{extract::Path, routing::post, Extension, Json, Router};
use serde::Deserialize;
use serde_json::value::RawValue;

use windmill_ai::types::{
    AgentEvalCase, Assertion, AssertionResult, EvalCaseResult, EvalInput, JudgeResult,
};
use windmill_common::error::{self, Error};
use windmill_common::flows::{FlowModule, FlowModuleValue, FlowValue, InputTransform};
use windmill_common::jobs::JobPayload;
use windmill_common::worker::to_raw_value;
use windmill_queue::{push, PushArgs, PushIsolationLevel};

use crate::db::{ApiAuthed, DB};
use crate::jobs::run_wait_result_internal;
use crate::utils::check_scopes;
use windmill_common::db::UserDB;
use windmill_common::users::username_to_permissioned_as;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/run", post(run_agent))
        .route("/eval_case", post(eval_case))
}

#[derive(Deserialize)]
struct RunAgentRequest {
    /// Path of a saved `ai_agent` resource.
    agent: String,
    #[serde(default)]
    input: EvalInput,
}

#[derive(Deserialize)]
struct EvalCaseRequest {
    /// Path of a saved `ai_agent` resource.
    agent: String,
    case: AgentEvalCase,
    /// Optional judge provider (ai-provider shape, may contain nested `$res:`). Defaults to the
    /// agent's own provider when omitted.
    #[serde(default)]
    judge_provider: Option<Box<RawValue>>,
}

/// Run a saved agent once on a given input and return its raw output.
async fn run_agent(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(req): Json<RunAgentRequest>,
) -> error::Result<Json<Box<RawValue>>> {
    check_scopes(&authed, || format!("jobs:run"))?;
    let flow = build_linked_agent_flow(&req.agent, &req.input);
    let (output, success, _ms) = run_preview(&db, &user_db, &authed, &w_id, flow).await?;
    if !success {
        return Err(Error::ExecutionErr(format!(
            "agent run failed: {}",
            output.get()
        )));
    }
    Ok(Json(output))
}

/// Run a single eval case against a saved agent: execute, apply deterministic assertions, and
/// (if the case has a checklist) grade with an LLM judge.
async fn eval_case(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(req): Json<EvalCaseRequest>,
) -> error::Result<Json<EvalCaseResult>> {
    check_scopes(&authed, || format!("jobs:run"))?;
    let case = req.case;

    // 1. Run the agent under test.
    let started = Instant::now();
    let flow = build_linked_agent_flow(&req.agent, &case.input);
    let run = run_preview(&db, &user_db, &authed, &w_id, flow).await;
    let latency_ms = started.elapsed().as_millis() as u64;

    let (output, success) = match run {
        Ok((output, success, _)) => (output, success),
        Err(e) => {
            return Ok(Json(EvalCaseResult {
                case_id: case.id,
                passed: false,
                output: None,
                error: Some(e.to_string()),
                assertions: vec![],
                judge: None,
                latency_ms,
            }));
        }
    };

    if !success {
        return Ok(Json(EvalCaseResult {
            case_id: case.id,
            passed: false,
            output: Some(output.clone()),
            error: Some(format!("agent run failed: {}", output.get())),
            assertions: vec![],
            judge: None,
            latency_ms,
        }));
    }

    // 2. Deterministic assertions.
    let assertion_results: Vec<AssertionResult> = case
        .assertions
        .iter()
        .map(|a| evaluate_assertion(a, &output))
        .collect();
    let assertions_pass = assertion_results.iter().all(|r| r.passed);

    // 3. LLM judge (only when a checklist is provided).
    let judge = if case.judge_checklist.is_empty() {
        None
    } else {
        let provider = match req.judge_provider {
            Some(p) => p,
            None => agent_provider(&db, &w_id, &req.agent).await?,
        };
        match run_judge(
            &db,
            &user_db,
            &authed,
            &w_id,
            provider,
            &output,
            &case.judge_checklist,
        )
        .await
        {
            Ok(j) => Some(j),
            Err(e) => {
                Some(JudgeResult { score: 0, pass: false, summary: format!("judge failed: {e}") })
            }
        }
    };

    let passed = assertions_pass && judge.as_ref().map(|j| j.pass).unwrap_or(true);

    Ok(Json(EvalCaseResult {
        case_id: case.id,
        passed,
        output: Some(output),
        error: None,
        assertions: assertion_results,
        judge,
        latency_ms,
    }))
}

// ---------------------------------------------------------------------------
// Flow construction
// ---------------------------------------------------------------------------

fn static_transform<T: serde::Serialize>(value: &T) -> InputTransform {
    InputTransform::new_static_value(to_raw_value(value))
}

fn single_module_flow(value: FlowModuleValue) -> FlowValue {
    FlowValue {
        modules: vec![FlowModule {
            id: "a".to_string(),
            value: to_raw_value(&value),
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// A one-step flow whose AIAgent module links to the saved agent resource. Only the flow-local
/// inputs are set locally; the brain + tools resolve from the resource at runtime.
fn build_linked_agent_flow(agent_path: &str, input: &EvalInput) -> FlowValue {
    let mut input_transforms = HashMap::new();
    if let Some(msg) = &input.user_message {
        input_transforms.insert("user_message".to_string(), static_transform(msg));
    }
    if let Some(att) = &input.user_attachments {
        input_transforms.insert("user_attachments".to_string(), static_transform(att));
    }
    single_module_flow(FlowModuleValue::AIAgent {
        input_transforms,
        tools: vec![],
        tag: None,
        omit_output_from_conversation: false,
        agent: Some(agent_path.to_string()),
        tool_inputs: HashMap::new(),
    })
}

const JUDGE_SYSTEM_PROMPT: &str = "You are a strict evaluator. You are given the OUTPUT produced \
by an AI agent and a CHECKLIST of acceptance criteria. Judge whether the output satisfies every \
criterion. Respond ONLY via the structured output: `score` is an integer 0-100 reflecting overall \
quality against the checklist, `pass` is true only if all criteria are satisfied, and `summary` \
is one or two sentences explaining the verdict.";

/// Run the judge as an inline AIAgent step with a structured output schema.
async fn run_judge(
    db: &DB,
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    provider: Box<RawValue>,
    output: &RawValue,
    checklist: &[String],
) -> error::Result<JudgeResult> {
    let checklist_rendered = checklist
        .iter()
        .map(|c| format!("- {c}"))
        .collect::<Vec<_>>()
        .join("\n");
    let user_message = format!(
        "OUTPUT:\n{}\n\nCHECKLIST:\n{}",
        output.get(),
        checklist_rendered
    );

    let output_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "score": { "type": "integer", "description": "0-100 overall quality" },
            "pass": { "type": "boolean", "description": "true only if all criteria pass" },
            "summary": { "type": "string", "description": "short rationale" }
        },
        "required": ["score", "pass", "summary"]
    });

    let mut it = HashMap::new();
    it.insert(
        "provider".to_string(),
        InputTransform::new_static_value(provider),
    );
    it.insert(
        "system_prompt".to_string(),
        static_transform(&JUDGE_SYSTEM_PROMPT),
    );
    it.insert("user_message".to_string(), static_transform(&user_message));
    it.insert("output_type".to_string(), static_transform(&"text"));
    it.insert(
        "output_schema".to_string(),
        static_transform(&output_schema),
    );
    it.insert("max_iterations".to_string(), static_transform(&1));

    let flow = single_module_flow(FlowModuleValue::AIAgent {
        input_transforms: it,
        tools: vec![],
        tag: None,
        omit_output_from_conversation: false,
        agent: None,
        tool_inputs: HashMap::new(),
    });

    let (result, success, _) = run_preview(db, user_db, authed, w_id, flow).await?;
    if !success {
        return Err(Error::ExecutionErr(format!(
            "judge run failed: {}",
            result.get()
        )));
    }
    parse_judge_result(&result)
}

/// Lenient parse: tolerate float scores and out-of-range values from the model.
fn parse_judge_result(value: &RawValue) -> error::Result<JudgeResult> {
    let v: serde_json::Value = serde_json::from_str(value.get())
        .map_err(|e| Error::internal_err(format!("judge returned non-JSON output: {e}")))?;
    let score = v
        .get("score")
        .and_then(|s| s.as_f64())
        .map(|s| s.round().clamp(0.0, 100.0) as u8)
        .unwrap_or(0);
    let pass = v.get("pass").and_then(|p| p.as_bool()).unwrap_or(false);
    let summary = v
        .get("summary")
        .and_then(|s| s.as_str())
        .unwrap_or_default()
        .to_string();
    Ok(JudgeResult { score, pass, summary })
}

/// Read the saved agent's `provider` field to use as the default judge provider.
async fn agent_provider(db: &DB, w_id: &str, path: &str) -> error::Result<Box<RawValue>> {
    let value = sqlx::query_scalar!(
        "SELECT value FROM resource WHERE workspace_id = $1 AND path = $2 AND resource_type = 'ai_agent'",
        w_id,
        path
    )
    .fetch_optional(db)
    .await?
    .flatten()
    .ok_or_else(|| Error::NotFound(format!("ai_agent resource {path} not found")))?;

    value.get("provider").map(to_raw_value).ok_or_else(|| {
        Error::BadRequest(format!(
            "ai_agent resource {path} has no provider; pass judge_provider explicitly"
        ))
    })
}

// ---------------------------------------------------------------------------
// Job push + wait
// ---------------------------------------------------------------------------

/// Push a single-step flow preview and block until it completes. Returns (result, success, ms).
async fn run_preview(
    db: &DB,
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    flow: FlowValue,
) -> error::Result<(Box<RawValue>, bool, u64)> {
    let started = Instant::now();
    let args: HashMap<String, Box<RawValue>> = HashMap::new();
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into());

    let (uuid, tx) = push(
        db,
        tx,
        w_id,
        JobPayload::RawFlow {
            value: flow,
            path: Some("ai_agent/eval".to_string()),
            restarted_from: None,
        },
        PushArgs::from(&args),
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        None,
        None,
        None,
        None,
        Some(&authed.clone().into()),
        false,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    let (result, success) =
        run_wait_result_internal(db, uuid, w_id, None, false, &authed.username).await?;
    Ok((result, success, started.elapsed().as_millis() as u64))
}

// ---------------------------------------------------------------------------
// Deterministic assertions
// ---------------------------------------------------------------------------

/// Render an agent output as a plain string for text-based assertions. A JSON string output is
/// unwrapped; anything else uses its compact JSON text.
fn output_as_string(output: &RawValue) -> String {
    match serde_json::from_str::<serde_json::Value>(output.get()) {
        Ok(serde_json::Value::String(s)) => s,
        _ => output.get().to_string(),
    }
}

fn evaluate_assertion(assertion: &Assertion, output: &RawValue) -> AssertionResult {
    let (passed, detail) = match assertion {
        Assertion::Contains { value, case_sensitive } => {
            let (haystack, needle) = casefold(output_as_string(output), value, *case_sensitive);
            (haystack.contains(&needle), None)
        }
        Assertion::NotContains { value, case_sensitive } => {
            let (haystack, needle) = casefold(output_as_string(output), value, *case_sensitive);
            (!haystack.contains(&needle), None)
        }
        Assertion::Regex { pattern } => match regex::Regex::new(pattern) {
            Ok(re) => (re.is_match(&output_as_string(output)), None),
            Err(e) => (false, Some(format!("invalid regex: {e}"))),
        },
        Assertion::JsonPathEquals { path, value } => {
            match serde_json::from_str::<serde_json::Value>(output.get()) {
                Ok(json) => {
                    let found = json_path_get(&json, path);
                    (found == Some(value), found.map(|f| f.to_string()))
                }
                Err(e) => (false, Some(format!("output is not JSON: {e}"))),
            }
        }
        Assertion::OutputSchemaValid => {
            // v1: a basic structural check that the output is valid, non-null JSON. A full
            // schema validation against the agent's output_schema is a fast-follow.
            match serde_json::from_str::<serde_json::Value>(output.get()) {
                Ok(serde_json::Value::Null) | Err(_) => (false, None),
                Ok(_) => (true, None),
            }
        }
    };
    AssertionResult { assertion: assertion.clone(), passed, detail }
}

fn casefold(haystack: String, needle: &str, case_sensitive: bool) -> (String, String) {
    if case_sensitive {
        (haystack, needle.to_string())
    } else {
        (haystack.to_lowercase(), needle.to_lowercase())
    }
}

/// Navigate a dotted JSON path (e.g. `data.items.0.name`) into a value.
fn json_path_get<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = match current {
            serde_json::Value::Object(map) => map.get(segment)?,
            serde_json::Value::Array(arr) => arr.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn raw(s: &str) -> Box<RawValue> {
        RawValue::from_string(s.to_string()).unwrap()
    }

    #[test]
    fn output_as_string_unwraps_json_strings() {
        assert_eq!(output_as_string(&raw("\"hello\"")), "hello");
        assert_eq!(output_as_string(&raw("{\"a\":1}")), "{\"a\":1}");
        assert_eq!(output_as_string(&raw("42")), "42");
    }

    #[test]
    fn contains_assertion_respects_case_sensitivity() {
        let out = raw("\"Hello World\"");
        let sensitive = Assertion::Contains { value: "hello".into(), case_sensitive: true };
        assert!(!evaluate_assertion(&sensitive, &out).passed);
        let insensitive = Assertion::Contains { value: "hello".into(), case_sensitive: false };
        assert!(evaluate_assertion(&insensitive, &out).passed);
    }

    #[test]
    fn not_contains_and_regex_and_schema_assertions() {
        let out = raw("\"order 123 shipped\"");
        assert!(
            evaluate_assertion(
                &Assertion::NotContains { value: "refund".into(), case_sensitive: false },
                &out
            )
            .passed
        );
        assert!(
            evaluate_assertion(&Assertion::Regex { pattern: r"order \d+".into() }, &out).passed
        );
        assert!(!evaluate_assertion(&Assertion::Regex { pattern: "(".into() }, &out).passed);
        assert!(evaluate_assertion(&Assertion::OutputSchemaValid, &raw("{\"a\":1}")).passed);
        assert!(!evaluate_assertion(&Assertion::OutputSchemaValid, &raw("null")).passed);
    }

    #[test]
    fn json_path_navigates_objects_and_arrays() {
        let v = json!({"data": {"items": [{"name": "a"}, {"name": "b"}]}});
        assert_eq!(json_path_get(&v, "data.items.1.name"), Some(&json!("b")));
        assert_eq!(json_path_get(&v, "data.missing"), None);
        assert_eq!(json_path_get(&v, "data.items.9"), None);
    }

    #[test]
    fn json_path_equals_assertion() {
        let out = raw("{\"status\":\"ok\",\"count\":3}");
        assert!(
            evaluate_assertion(
                &Assertion::JsonPathEquals { path: "status".into(), value: json!("ok") },
                &out
            )
            .passed
        );
        assert!(
            !evaluate_assertion(
                &Assertion::JsonPathEquals { path: "count".into(), value: json!(5) },
                &out
            )
            .passed
        );
    }

    #[test]
    fn parse_judge_result_is_lenient() {
        // Float score is rounded and clamped; pass/summary parsed.
        let j = parse_judge_result(&raw(
            "{\"score\": 87.6, \"pass\": true, \"summary\": \"good\"}",
        ))
        .unwrap();
        assert_eq!(j.score, 88);
        assert!(j.pass);
        assert_eq!(j.summary, "good");
        // Missing fields default safely.
        let j2 = parse_judge_result(&raw("{\"score\": 200}")).unwrap();
        assert_eq!(j2.score, 100);
        assert!(!j2.pass);
    }
}
