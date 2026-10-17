//! The built-in `request_approval` tool parks the agent instead of running anything. The agent
//! stores its conversation, releases its worker the way a workflow-as-code parent does
//! (`suspend_wac_parent`), and is pulled again when `suspend` reaches 0 (an answer, or a
//! rejection) or `suspend_until` passes (the request timing out). `resume_suspended` answers it
//! like a WAC approval, since its queue row is the one suspended.
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
    pub expires_at: DateTime<Utc>,
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
        expires_at,
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
    /// The tool message answering the parked call, and whether it counts as a success on the run
    /// page.
    Answered(OpenAIMessage, bool),
    /// Woken by an answer to an earlier request; the parked one still waits.
    StillPending,
}

/// Read the answer to the parked approval.
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
    let (result, success) = match answer {
        Some((_, mut value, approver, approved)) => {
            let mut answer = serde_json::json!({
                "approved": approved,
                "approver": approver.unwrap_or_else(|| "anonymous".to_string()),
            });
            if let Some(object) = value.0.as_object_mut() {
                object.remove("call_id");
            }
            let comment = value.0.get("comment").cloned().unwrap_or(value.0);
            let empty = comment.is_null() || comment.as_object().is_some_and(|o| o.is_empty());
            if !empty {
                answer["comment"] = comment;
            }
            (answer, approved)
        }
        None if Utc::now() < pending.expires_at => return Ok(Resumed::StillPending),
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
        .find(|a| matches!(a, AgentAction::Approval { call_id, .. } if *call_id == pending.call_id))
        .cloned();
    let message = OpenAIMessage {
        role: "tool".to_string(),
        content: Some(OpenAIContent::Text(result.to_string())),
        tool_call_id: Some(pending.call_id.clone()),
        agent_action: action,
        ..Default::default()
    };
    Ok(Resumed::Answered(message, success))
}
