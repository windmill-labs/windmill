//! The built-in `request_approval` tool parks the agent instead of running anything. The agent
//! stores its conversation, releases its worker the way a workflow-as-code parent does
//! (`suspend_wac_parent`), and is pulled again when `suspend` reaches 0 (an answer, or a
//! rejection) or `suspend_until` passes (the request timing out). `resume_suspended` answers it
//! like a WAC approval, since its queue row is the one suspended.
//!
//! The checkpoint row outlives the segment that reads it, so a job restarted after its worker died
//! resumes from it again. It is deleted by the monitor once the job leaves the queue.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use windmill_ai::{
    ai_types::OpenAIToolCall,
    types::{OpenAIContent, OpenAIMessage, TokenUsage, Tool, ToolDef, ToolDefFunction},
};
use windmill_common::{
    db::DB,
    error::{self, Error},
    flow_status::AgentAction,
    flows::ApprovalToolValue,
    worker::{to_raw_value, Connection},
};
use windmill_queue::MiniPulledJob;

use crate::ai::compaction::HistoryCheckpoint;
use crate::ai::utils::find_unique_tool_name;
use crate::wac_executor::{end_wac_segment, suspend_wac_parent, WacPark};

/// The name a run enables the approval tool by, reserved like `WEBSEARCH_ENABLED_NAME`: the
/// model-facing name may be renamed away from a flow module tool that already holds it.
pub(crate) const APPROVAL_ENABLED_NAME: &str = "__wm_approval";

/// Matches a suspended flow step's default.
const DEFAULT_APPROVAL_TIMEOUT_SECS: u32 = 1800;

pub(crate) fn approval_timeout(value: &ApprovalToolValue) -> u32 {
    value
        .timeout
        .unwrap_or(DEFAULT_APPROVAL_TIMEOUT_SECS)
        .max(1)
}

pub(crate) fn approval_tool(timeout_secs: u32, existing: &[ToolDef]) -> Tool {
    let parameters = serde_json::json!({
        "type": "object",
        "properties": {
            "message": {
                "type": "string",
                "description": "What you want approved and why, as the approver will read it."
            }
        },
        "required": ["message"],
    });
    Tool {
        def: ToolDef {
            r#type: "function".to_string(),
            function: ToolDefFunction {
                name: find_unique_tool_name("request_approval", Some(existing)),
                description: Some(
                    "Ask a person to approve or reject something before you do it. This run stops \
                     until someone answers from the flow's run page or the request times out, then \
                     resumes with the answer as this tool's result: whether it was approved, by \
                     whom, and any comment. Do not call it alongside other tools you depend on in \
                     the same turn."
                        .to_string(),
                ),
                parameters: to_raw_value(&parameters),
            },
        },
        module: None,
        mcp_source: None,
        approval_timeout_secs: Some(timeout_secs),
    }
}

/// The approval the agent waits on, read back to answer the call when it is pulled again.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct PendingPause {
    pub call_id: String,
    pub function_name: String,
    #[serde(skip)]
    pub suspend_secs: f64,
    #[serde(skip)]
    pub action: Option<AgentAction>,
}

/// The model's arguments turned into a park, or the error the model is answered with instead.
pub(crate) fn start_pause(
    timeout_secs: u32,
    tool_call: &OpenAIToolCall,
) -> Result<PendingPause, String> {
    let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
        .map_err(|e| format!("Invalid arguments: {e}"))?;
    let message = args
        .get("message")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("`message` is required")?
        .to_string();
    let expires_at = Utc::now() + chrono::Duration::seconds(timeout_secs as i64);
    Ok(PendingPause {
        call_id: tool_call.id.clone(),
        function_name: tool_call.function.name.clone(),
        suspend_secs: timeout_secs as f64,
        action: Some(AgentAction::Approval { call_id: tool_call.id.clone(), message, expires_at }),
    })
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

/// Park the agent until its pause resolves. The error returned on success is the one the worker
/// treats as "not completed".
pub(crate) async fn park(
    db: &DB,
    conn: &Connection,
    job: &MiniPulledJob,
    checkpoint: &AgentCheckpoint,
) -> Error {
    match park_inner(db, job, checkpoint).await {
        Ok(Some(segment_ms)) => {
            end_wac_segment(conn, job, segment_ms);
            Error::WacSuspended(format!(
                "AI agent {} parked on {}",
                job.id, checkpoint.pending.function_name
            ))
        }
        Ok(None) => Error::ExecutionErr("Job cancelled".to_string()),
        Err(e) => e,
    }
}

async fn park_inner(
    db: &DB,
    job: &MiniPulledJob,
    checkpoint: &AgentCheckpoint,
) -> error::Result<Option<Option<i64>>> {
    let suspend_secs = checkpoint.pending.suspend_secs;
    let checkpoint = serde_json::to_value(checkpoint).map_err(|e| {
        Error::internal_err(format!("Failed to serialize AI agent checkpoint: {e}"))
    })?;
    let mut tx = db.begin().await?;
    // One event: an answer, or a rejection, which zeroes `suspend` outright.
    let segment_ms =
        match suspend_wac_parent(&mut tx, &job.id, &job.workspace_id, 1, suspend_secs).await? {
            WacPark::Parked(ms) => ms,
            WacPark::Cancelled(_) => return Ok(None),
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
    Ok(Some(segment_ms))
}

/// The tool message answering the parked call, and whether it counts as a success on the run page.
pub(crate) async fn answer_pause(
    db: &DB,
    job_id: &Uuid,
    pending: &PendingPause,
    actions: &[AgentAction],
    consumed_resume_rows: &mut Vec<Uuid>,
) -> error::Result<(OpenAIMessage, bool)> {
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
    .fetch_all(db)
    .await?;
    // Every row counts as read, an answer sent while the agent slept included: left unread it
    // would answer the next approval request before anyone saw it.
    consumed_resume_rows.extend(rows.iter().map(|(id, ..)| *id));

    let (result, success) = match rows.into_iter().next() {
        Some((_, value, approver, approved)) => {
            let mut answer = serde_json::json!({
                "approved": approved,
                "approver": approver.unwrap_or_else(|| "anonymous".to_string()),
            });
            let comment = value.0.get("comment").cloned().unwrap_or(value.0);
            let empty = comment.is_null() || comment.as_object().is_some_and(|o| o.is_empty());
            if !empty {
                answer["comment"] = comment;
            }
            (answer, approved)
        }
        None => (
            serde_json::json!({
                "approved": false,
                "timed_out": true,
                "message": "Nobody answered before the request timed out.",
            }),
            false,
        ),
    };

    let action = actions
        .iter()
        .rev()
        .find(|a| match a {
            AgentAction::Approval { call_id, .. } => *call_id == pending.call_id,
            _ => false,
        })
        .cloned();
    let message = OpenAIMessage {
        role: "tool".to_string(),
        content: Some(OpenAIContent::Text(result.to_string())),
        tool_call_id: Some(pending.call_id.clone()),
        agent_action: action,
        ..Default::default()
    };
    Ok((message, success))
}
