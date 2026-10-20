//! A call of a tool that requires approval, or of the built-in question tool, parks the agent
//! instead of running. The agent stores its conversation, releases its worker the way a
//! workflow-as-code parent does (`suspend_wac_parent`), and is pulled again when `suspend` reaches
//! 0 (an answer, or a rejection) or `suspend_until` passes (the request timing out).
//! `resume_suspended` answers it like a WAC approval, since its queue row is the one suspended.
//!
//! The checkpoint row outlives the segment that reads it, so a job restarted after its worker died
//! resumes from it again. It is deleted by the monitor once the job leaves the queue.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use windmill_ai::{
    ai_types::OpenAIToolCall,
    types::{OpenAIContent, OpenAIMessage, TokenUsage, Tool, ToolDef, ToolDefFunction},
};
use windmill_common::{
    db::DB,
    error::{self, Error},
    flow_status::{AgentAction, ApprovalCall},
    flows::QuestionToolValue,
    worker::{to_raw_value, Connection},
};
use windmill_queue::MiniPulledJob;

use crate::ai::compaction::HistoryCheckpoint;
use crate::ai::utils::find_unique_tool_name;
use crate::wac_executor::{end_wac_segment, suspend_wac_parent, WacPark};

/// The name a run enables the question tool by, reserved like `WEBSEARCH_ENABLED_NAME`: the
/// model-facing name may be renamed away from a flow module tool that already holds it.
pub(crate) const QUESTION_ENABLED_NAME: &str = "__wm_question";

/// Matches a suspended flow step's default.
const DEFAULT_TIMEOUT_SECS: u32 = 1800;

pub(crate) fn question_timeout(value: &QuestionToolValue) -> u32 {
    value.timeout.unwrap_or(DEFAULT_TIMEOUT_SECS).max(1)
}

pub(crate) fn question_tool(timeout_secs: u32, existing: &[ToolDef]) -> Tool {
    let parameters = serde_json::json!({
        "type": "object",
        "properties": {
            "question": {
                "type": "string",
                "description": "The question, as the person answering will read it."
            }
        },
        "required": ["question"],
    });
    Tool {
        def: ToolDef {
            r#type: "function".to_string(),
            function: ToolDefFunction {
                name: find_unique_tool_name("ask_question", Some(existing)),
                description: Some(
                    "Ask a person a question you cannot answer yourself, such as a choice only \
                     they can make or information you do not have. This run stops until someone \
                     answers from the flow's run page or chat, or the question times out, then \
                     resumes with the answer as this tool's result."
                        .to_string(),
                ),
                parameters: to_raw_value(&parameters),
            },
        },
        module: None,
        mcp_source: None,
        requires_approval: false,
        question_timeout_secs: Some(timeout_secs),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum PendingRequest {
    /// Calls of tools that require approval, run once approved.
    Approval {
        calls: Vec<OpenAIToolCall>,
    },
    Question {
        function_name: String,
    },
}

/// What the agent waits on, read back to answer it when it is pulled again.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct PendingPause {
    /// The question's call, or an approval's first call: the answer must name it.
    pub call_id: String,
    pub expires_at: DateTime<Utc>,
    pub request: PendingRequest,
    #[serde(skip)]
    pub action: Option<AgentAction>,
}

fn expires_at(timeout_secs: u32) -> DateTime<Utc> {
    Utc::now() + chrono::Duration::seconds(timeout_secs as i64)
}

/// The model's arguments turned into a park, or the error the model is answered with instead.
pub(crate) fn start_question(
    timeout_secs: u32,
    tool_call: &OpenAIToolCall,
) -> Result<PendingPause, String> {
    let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
        .map_err(|e| format!("Invalid arguments: {e}"))?;
    let question = args
        .get("question")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("`question` is required")?
        .to_string();
    let expires_at = expires_at(timeout_secs);
    Ok(PendingPause {
        call_id: tool_call.id.clone(),
        expires_at,
        request: PendingRequest::Question { function_name: tool_call.function.name.clone() },
        action: Some(AgentAction::Question { call_id: tool_call.id.clone(), question, expires_at }),
    })
}

/// One request for all of a turn's calls that require approval: they are answered together.
pub(crate) fn start_approval(calls: Vec<&OpenAIToolCall>) -> PendingPause {
    let call_id = calls[0].id.clone();
    let expires_at = expires_at(DEFAULT_TIMEOUT_SECS);
    let shown = calls
        .iter()
        .map(|call| ApprovalCall {
            function_name: call.function.name.clone(),
            arguments: serde_json::from_str(&call.function.arguments)
                .unwrap_or_else(|_| serde_json::Value::String(call.function.arguments.clone())),
        })
        .collect();
    PendingPause {
        action: Some(AgentAction::Approval { call_id: call_id.clone(), calls: shown, expires_at }),
        call_id,
        expires_at,
        request: PendingRequest::Approval { calls: calls.into_iter().cloned().collect() },
    }
}

/// Everything `run_agent` holds across iterations that the next segment needs.
#[derive(Serialize, Deserialize)]
pub(crate) struct AgentCheckpoint {
    pub history: HistoryCheckpoint,
    pub actions: Vec<AgentAction>,
    pub content: Option<OpenAIContent>,
    pub usage: Option<TokenUsage>,
    pub events: String,
    pub reasoning: String,
    pub next_iteration: usize,
    pub pending: PendingPause,
    /// `resume_job` rows never leave while the agent is queued, so each answer must skip the
    /// ones earlier parks already read.
    #[serde(default)]
    pub consumed_resume_rows: Vec<Uuid>,
}

pub(crate) async fn load_checkpoint(
    db: &DB,
    job_id: &Uuid,
) -> error::Result<Option<AgentCheckpoint>> {
    let raw: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>> =
        sqlx::query_scalar("SELECT checkpoint FROM ai_agent_checkpoint WHERE job_id = $1")
            .bind(job_id)
            .fetch_optional(db)
            .await?;
    raw.map(|r| {
        serde_json::from_str(r.0.get())
            .map_err(|e| Error::internal_err(format!("Invalid AI agent checkpoint: {e}")))
    })
    .transpose()
}

pub(crate) enum ParkOutcome {
    Parked,
    /// A cancel reached the queue row first, so nothing was written.
    Cancelled,
}

/// Park the agent until its approval is answered or expires.
pub(crate) async fn park(
    db: &DB,
    conn: &Connection,
    job: &MiniPulledJob,
    checkpoint: &AgentCheckpoint,
) -> error::Result<ParkOutcome> {
    let remaining_ms = (checkpoint.pending.expires_at - Utc::now()).num_milliseconds();
    let suspend_secs = (remaining_ms as f64 / 1000.0).max(1.0);
    let checkpoint = serde_json::to_value(checkpoint).map_err(|e| {
        Error::internal_err(format!("Failed to serialize AI agent checkpoint: {e}"))
    })?;
    let mut tx = db.begin().await?;
    // One event: an answer, or a rejection, which zeroes `suspend` outright.
    let segment_ms =
        match suspend_wac_parent(&mut tx, &job.id, &job.workspace_id, 1, suspend_secs).await? {
            WacPark::Parked(ms) => ms,
            WacPark::Cancelled(_) => return Ok(ParkOutcome::Cancelled),
        };
    sqlx::query(
        "INSERT INTO ai_agent_checkpoint (job_id, workspace_id, checkpoint) VALUES ($1, $2, $3)
         ON CONFLICT (job_id) DO UPDATE SET checkpoint = EXCLUDED.checkpoint",
    )
    .bind(&job.id)
    .bind(&job.workspace_id)
    .bind(&checkpoint)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    end_wac_segment(conn, job, segment_ms);
    Ok(ParkOutcome::Parked)
}

pub(crate) enum Resumed {
    /// The answer, and whether it counts as a success on the run page.
    Answered(Answer, bool),
    /// Woken by an answer to an earlier request; the parked one still waits.
    StillPending,
}

pub(crate) enum Answer {
    /// The tool message answering the question.
    Question(OpenAIMessage),
    /// The approved calls run.
    Run,
    /// Each call is refused with this message.
    Refuse(String),
}

/// Read the answer to the parked request.
///
/// `suspend` is zeroed in the same transaction as the read, so `resume_suspended` refuses any
/// answer sent while the agent runs instead of storing a row the next request would take. An answer
/// that names another request (the page sends the call id it shows) is read and ignored: a stale
/// page must not answer a request its user never saw.
pub(crate) async fn answer_pause(
    db: &DB,
    job_id: &Uuid,
    pending: &PendingPause,
    actions: &[AgentAction],
    consumed_resume_rows: &mut Vec<Uuid>,
) -> error::Result<Resumed> {
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE v2_job_queue SET suspend = 0 WHERE id = $1")
        .bind(job_id)
        .execute(&mut *tx)
        .await?;
    let rows: Vec<(
        Uuid,
        sqlx::types::Json<serde_json::Value>,
        Option<String>,
        bool,
    )> = sqlx::query_as(
        "SELECT id, value, approver, approved FROM resume_job
             WHERE job = $1 AND id <> ALL($2) ORDER BY created_at ASC",
    )
    .bind(job_id)
    .bind(&*consumed_resume_rows)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    consumed_resume_rows.extend(rows.iter().map(|(id, ..)| *id));

    let answer = rows.into_iter().find(|(_, value, ..)| {
        value
            .0
            .get("call_id")
            .and_then(|v| v.as_str())
            .map_or(true, |call_id| call_id == pending.call_id)
    });
    let Some((_, value, approver, approved)) = answer else {
        if Utc::now() < pending.expires_at {
            return Ok(Resumed::StillPending);
        }
        let answer = match pending.request {
            PendingRequest::Approval { .. } => Answer::Refuse(
                "Not run: nobody approved this call before the request timed out.".to_string(),
            ),
            PendingRequest::Question { .. } => Answer::Question(question_message(
                pending,
                actions,
                serde_json::json!({
                    "timed_out": true,
                    "message": "Nobody answered before the question timed out.",
                }),
            )),
        };
        return Ok(Resumed::Answered(answer, false));
    };
    let approver = approver.unwrap_or_else(|| "anonymous".to_string());
    let text = |key: &str| {
        value
            .0
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let answer = match (&pending.request, approved) {
        (PendingRequest::Approval { .. }, true) => Answer::Run,
        (PendingRequest::Approval { .. }, false) => Answer::Refuse(match text("comment") {
            Some(comment) => {
                format!("Not run: {approver} rejected this call. Their comment: {comment}")
            }
            None => format!("Not run: {approver} rejected this call."),
        }),
        (PendingRequest::Question { .. }, true) => Answer::Question(question_message(
            pending,
            actions,
            serde_json::json!({ "answer": text("answer").unwrap_or_default(), "answered_by": approver }),
        )),
        (PendingRequest::Question { .. }, false) => Answer::Question(question_message(
            pending,
            actions,
            serde_json::json!({ "declined": true, "declined_by": approver }),
        )),
    };
    Ok(Resumed::Answered(answer, approved))
}

fn question_message(
    pending: &PendingPause,
    actions: &[AgentAction],
    result: serde_json::Value,
) -> OpenAIMessage {
    let action = actions
        .iter()
        .rev()
        .find(|a| matches!(a, AgentAction::Question { call_id, .. } if *call_id == pending.call_id))
        .cloned();
    OpenAIMessage {
        role: "tool".to_string(),
        content: Some(OpenAIContent::Text(result.to_string())),
        tool_call_id: Some(pending.call_id.clone()),
        agent_action: action,
        ..Default::default()
    }
}
