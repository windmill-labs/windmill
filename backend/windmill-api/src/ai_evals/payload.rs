use super::*;

/// What every scorer is handed: the answer, and the calls the agent made to reach it.
///
/// Built from the job the run already stored, which is what lets a scorer added later score an
/// experiment that has already run.
#[derive(Serialize, Debug, Clone)]
pub struct EvalRunPayload {
    pub input: EvalCaseInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Box<RawValue>>,
    pub tool_calls: Vec<EvalToolCall>,
    /// The tools that were actually called, with the schema they were called against. A tool
    /// whose schema could not be resolved carries `null`, and a scorer validating arguments must
    /// treat that as unchecked rather than as a failure.
    pub tools: Vec<EvalToolDef>,
    pub metrics: EvalMetrics,
    pub status: String,
    pub job_id: Uuid,
}

#[derive(Serialize, Debug, Clone)]
pub struct EvalToolCall {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    /// Set when the result was too large to carry and was cut down.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct EvalToolDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Box<RawValue>>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EvalMetrics {
    pub steps: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    /// The provider's token counts, when it reported any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Box<RawValue>>,
}

/// A tool result large enough to swamp a judge's context is cut here. The scorer is told, so a
/// check reading a truncated result can say so instead of failing on the missing tail.
const MAX_TOOL_RESULT_BYTES: usize = 4 * 1024;

fn truncate_value(value: Box<RawValue>) -> (Box<RawValue>, bool) {
    if value.get().len() <= MAX_TOOL_RESULT_BYTES {
        return (value, false);
    }
    let text = value.get();
    let mut end = MAX_TOOL_RESULT_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    match serde_json::value::to_raw_value(&format!("{}… [truncated]", &text[..end])) {
        Ok(v) => (v, true),
        Err(_) => (value, false),
    }
}

/// Assemble the payload from a completed case job: the agent step's own result carries the answer
/// and the message list, and every message that made a tool call names the job that ran it.
async fn build_run_payload(
    db: &DB,
    w_id: &str,
    job_id: Uuid,
    agent_job: Uuid,
    input: EvalCaseInput,
    expected: Option<Box<RawValue>>,
    status: String,
    duration_ms: Option<i64>,
) -> Result<EvalRunPayload> {
    // A read that failed is not a run with no answer: handing the scorers an empty payload would
    // have them grade the absence of evidence and record that verdict permanently.
    let agent_result = agent_result(db, w_id, job_id).await?.map(|(r, _)| r);

    let parsed: Option<serde_json::Value> = agent_result
        .as_ref()
        .and_then(|r| serde_json::from_str(r.get()).ok());
    let output = parsed
        .as_ref()
        .and_then(|p| p.get("output"))
        .map(|o| serde_json::value::to_raw_value(o))
        .transpose()?;
    let usage = parsed
        .as_ref()
        .and_then(|p| p.get("usage"))
        .map(|u| serde_json::value::to_raw_value(u))
        .transpose()?;

    // Walk the messages in order: a tool call is an `agent_action` on the message that made it.
    let mut calls: Vec<(String, Option<Uuid>, Option<Box<RawValue>>)> = vec![];
    if let Some(messages) = parsed
        .as_ref()
        .and_then(|p| p.get("messages"))
        .and_then(|m| m.as_array())
    {
        for message in messages {
            let Some(action) = message.get("agent_action") else {
                continue;
            };
            match action.get("type").and_then(|t| t.as_str()) {
                Some("tool_call") => calls.push((
                    action
                        .get("function_name")
                        .and_then(|f| f.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    action
                        .get("job_id")
                        .and_then(|j| j.as_str())
                        .and_then(|j| Uuid::parse_str(j).ok()),
                    None,
                )),
                // An MCP call runs inside the agent rather than as a job, so its arguments are on
                // the action itself. Its result lives in a later `role: "tool"` message rather
                // than a child-job row, and is not surfaced to scorers yet.
                Some("mcp_tool_call") => calls.push((
                    action
                        .get("function_name")
                        .and_then(|f| f.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    None,
                    action
                        .get("arguments")
                        .map(|a| serde_json::value::to_raw_value(a))
                        .transpose()?,
                )),
                _ => {}
            }
        }
    }

    let call_job_ids: Vec<Uuid> = calls.iter().filter_map(|(_, id, _)| *id).collect();
    let mut jobs = std::collections::HashMap::new();
    if !call_job_ids.is_empty() {
        // Constrained to the agent step's own children rather than to the workspace: these ids
        // come out of a job result, so a caller who can run a flow can put any id there. A tool
        // call is pushed as a child of the agent that made it, which is what makes that the
        // boundary.
        let rows = sqlx::query!(
            "SELECT j.id, j.args AS \"args: sqlx::types::Json<Box<RawValue>>\",
                    c.result AS \"result: sqlx::types::Json<Box<RawValue>>\",
                    c.status::text AS status, c.duration_ms,
                    s.schema AS \"schema: sqlx::types::Json<Box<RawValue>>\"
             FROM v2_job j
             LEFT JOIN v2_job_completed c ON c.id = j.id
             LEFT JOIN script s ON s.workspace_id = j.workspace_id AND s.hash = j.runnable_id
             WHERE j.id = ANY($1) AND j.workspace_id = $2 AND j.parent_job = $3",
            &call_job_ids,
            w_id,
            agent_job
        )
        .fetch_all(db)
        .await?;
        for row in rows {
            jobs.insert(row.id, row);
        }
    }

    let mut tool_calls = Vec::with_capacity(calls.len());
    let mut tools: Vec<EvalToolDef> = vec![];
    for (name, call_job_id, inline_args) in calls {
        let row = call_job_id.and_then(|id| jobs.get(&id));
        let (result, truncated) = match row.and_then(|r| r.result.as_ref()) {
            Some(result) => {
                let (value, truncated) = truncate_value(result.0.clone());
                (Some(value), truncated)
            }
            None => (None, false),
        };
        let failed = row
            .and_then(|r| r.status.as_deref())
            .map(|s| s != "success")
            .unwrap_or(false);
        if !tools.iter().any(|t| t.name == name) {
            tools.push(EvalToolDef {
                name: name.clone(),
                schema: row.and_then(|r| r.schema.as_ref()).map(|s| s.0.clone()),
            });
        }
        // The already truncated result restated. `render_tool_calls` shows `error` and not
        // `result` for a failed call, so the judge's context carries the payload once and bounded;
        // `result` stays on the raw call for a script scorer.
        let error = failed
            .then(|| result.as_ref().map(|r| r.get().to_string()))
            .flatten();
        tool_calls.push(EvalToolCall {
            name,
            args: inline_args.or_else(|| row.and_then(|r| r.args.as_ref()).map(|a| a.0.clone())),
            result,
            error,
            duration_ms: row.map(|r| r.duration_ms),
            truncated,
        });
    }

    Ok(EvalRunPayload {
        metrics: EvalMetrics { steps: tool_calls.len(), duration_ms, usage },
        input,
        output,
        expected,
        tool_calls,
        tools,
        status,
        job_id,
    })
}

#[derive(Deserialize)]
pub struct RunPayloadQuery {
    /// The flow job that answered the case: an iteration of a run.
    pub job_id: Uuid,
}

/// What the scorers of one iteration are handed.
#[derive(Serialize)]
pub struct RunPayloadResponse {
    pub run: EvalRunPayload,
    /// The same run as a judge reads it. Rendered once per case rather than once per judge.
    pub rendered: String,
}

/// Assemble the payload for one answered case, for the step that feeds the scorers.
///
/// The case is read from the job's arguments rather than from the experiment, so this works for an
/// iteration whose row has not been filled in yet.
pub async fn run_payload(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<RunPayloadQuery>,
) -> JsonResult<RunPayloadResponse> {
    // `UserDB` enforces row permissions but not a token's scopes, so without this an
    // `ai_evals:read` token would read job arguments, results and tool calls that `jobs:read`
    // is what actually gates. Job tokens are unscoped, so the run flow's payload step passes.
    check_scopes(&authed, || "jobs:read".to_string())?;
    // Through `user_db`: the caller is a job token, and it reads what its runner can read.
    let mut tx = user_db.begin(&authed).await?;
    let args = sqlx::query_scalar!(
        "SELECT args AS \"args: sqlx::types::Json<Box<RawValue>>\" FROM v2_job
         WHERE id = $1 AND workspace_id = $2",
        query.job_id,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten()
    .ok_or_else(|| Error::NotFound(format!("Job {} not found", query.job_id)))?;
    tx.commit().await?;

    let args: serde_json::Value = serde_json::from_str(args.0.get())?;
    // An iteration carries its case; a run recorded one job per case carries the same input under
    // the stamp that job was pushed with.
    let case = args.get("iter").and_then(|i| i.get("value"));
    let input = case
        .and_then(|c| c.get("input"))
        .or_else(|| args.get("_eval_input"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let expected = case
        .and_then(|c| c.get("expected"))
        .or_else(|| args.get("expected"))
        .cloned();

    // The agent step's own status and duration, never the iteration's: the iteration goes on to
    // assemble this payload and run the scorers, so a scorer reading the iteration's duration
    // would be measuring itself.
    let agent_job = agent_step_job(&db, &w_id, query.job_id)
        .await?
        .unwrap_or(query.job_id);
    let completed = sqlx::query!(
        "SELECT status::text AS \"status!\", duration_ms FROM v2_job_completed
         WHERE id = $1 AND workspace_id = $2",
        agent_job,
        w_id
    )
    .fetch_optional(&db)
    .await?;

    let run = build_run_payload(
        &db,
        &w_id,
        query.job_id,
        agent_job,
        serde_json::from_value(input)?,
        expected
            .map(|e| serde_json::value::to_raw_value(&e))
            .transpose()?,
        completed
            .as_ref()
            .map(|c| c.status.clone())
            // The iteration asking is itself still running: its agent step is what finished.
            .unwrap_or_else(|| "success".to_string()),
        completed.as_ref().map(|c| c.duration_ms),
    )
    .await?;
    let rendered = render_run(&run);
    Ok(Json(RunPayloadResponse { run, rendered }))
}

/// The job of the agent step inside a run's flow, from the flow status of either a running or a
/// finished one.
async fn agent_step_job(db: &DB, w_id: &str, flow_job: Uuid) -> Result<Option<Uuid>> {
    let modules = sqlx::query_scalar!(
        "SELECT COALESCE(s.flow_status, c.flow_status) -> 'modules' AS modules
         FROM v2_job j
         LEFT JOIN v2_job_status s ON s.id = j.id
         LEFT JOIN v2_job_completed c ON c.id = j.id
         WHERE j.id = $1 AND j.workspace_id = $2",
        flow_job,
        w_id
    )
    .fetch_optional(db)
    .await?
    .flatten();
    Ok(modules
        .as_ref()
        .and_then(|m| m.as_array())
        .and_then(|modules| {
            modules
                .iter()
                .find(|m| m.get("id").and_then(|i| i.as_str()) == Some(AGENT_NODE_ID))
        })
        .and_then(|m| m.get("job"))
        .and_then(|j| j.as_str())
        .and_then(|j| Uuid::parse_str(j).ok()))
}

/// The system prompt a judge agent is created with. It is the agent's own, so editing a judge is
/// editing that resource — there is no second copy of the grading contract on the dataset.
pub const JUDGE_SYSTEM_PROMPT: &str = r#"You are grading one run of an AI agent.

Score how well the agent handled the request, from 0 to 1. Judge the whole trajectory, not only the
final answer. Penalise asking for information already in the request, calling a tool twice with the
same arguments, and tool errors left unrecovered.

Reply with JSON only, of the form {"score": <number between 0 and 1>, "reason": <one sentence>}."#;

fn render_json(value: Option<&RawValue>) -> String {
    value
        .map(|v| v.get().to_string())
        .unwrap_or_else(|| "(none)".to_string())
}

/// Tool calls as the judge reads them: numbered, in order, with arguments, result and duration.
fn render_tool_calls(calls: &[EvalToolCall]) -> String {
    if calls.is_empty() {
        return "(none)".to_string();
    }
    calls
        .iter()
        .enumerate()
        .map(|(index, call)| {
            let args = call.args.as_ref().map(|a| a.get()).unwrap_or("{}");
            let outcome = match (&call.error, &call.result) {
                (Some(error), _) => format!("error: {}", error),
                (None, Some(result)) => result.get().to_string(),
                (None, None) => "(no result)".to_string(),
            };
            let timing = call
                .duration_ms
                .map(|ms| format!(" ({}ms)", ms))
                .unwrap_or_default();
            format!(
                "{}. {}({}) -> {}{}",
                index + 1,
                call.name,
                args,
                outcome,
                timing
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// One run, as a judge is shown it.
fn render_run(run: &EvalRunPayload) -> String {
    format!(
        "Request: {}\nTool calls, in order:\n{}\nAnswer: {}\nExpected: {}",
        run.input.user_message.as_deref().unwrap_or("(none)"),
        render_tool_calls(&run.tool_calls),
        render_json(run.output.as_deref()),
        render_json(run.expected.as_deref()),
    )
}

/// Module id of a scorer inside a scoring job. `assign_scorer_ids` keeps ids to
/// `[A-Za-z0-9_]`, so this is a valid identifier.
pub(crate) fn scorer_module_id(scorer_id: &str) -> String {
    format!("s_{}", scorer_id)
}
