//! Job types and operations for local mode

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::LocalDb;
use crate::error::{LocalError, Result};

/// Job kind (mirrors windmill-common JobKind)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobKind {
    Script,
    Preview,
    Flow,
    FlowPreview,
    Dependencies,
    FlowDependencies,
    ScriptHub,
    Identity,
    Http,
    Graphql,
    Postgresql,
    Noop,
    AppDependencies,
    DeploymentCallback,
    SingleScriptFlow,
    FlowScript,
    FlowNode,
    AppScript,
}

impl JobKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobKind::Script => "script",
            JobKind::Preview => "preview",
            JobKind::Flow => "flow",
            JobKind::FlowPreview => "flowpreview",
            JobKind::Dependencies => "dependencies",
            JobKind::FlowDependencies => "flowdependencies",
            JobKind::ScriptHub => "script_hub",
            JobKind::Identity => "identity",
            JobKind::Http => "http",
            JobKind::Graphql => "graphql",
            JobKind::Postgresql => "postgresql",
            JobKind::Noop => "noop",
            JobKind::AppDependencies => "appdependencies",
            JobKind::DeploymentCallback => "deploymentcallback",
            JobKind::SingleScriptFlow => "singlescriptflow",
            JobKind::FlowScript => "flowscript",
            JobKind::FlowNode => "flownode",
            JobKind::AppScript => "appscript",
        }
    }
}

/// Script language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLang {
    Python3,
    Deno,
    Go,
    Bash,
    Postgresql,
    Nativets,
    Bun,
    Mysql,
    Bigquery,
    Snowflake,
    Graphql,
    Powershell,
    Mssql,
    Php,
    Bunnative,
    Rust,
    Ansible,
    Csharp,
    Oracledb,
    Nu,
    Java,
    Duckdb,
}

impl ScriptLang {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptLang::Python3 => "python3",
            ScriptLang::Deno => "deno",
            ScriptLang::Go => "go",
            ScriptLang::Bash => "bash",
            ScriptLang::Postgresql => "postgresql",
            ScriptLang::Nativets => "nativets",
            ScriptLang::Bun => "bun",
            ScriptLang::Mysql => "mysql",
            ScriptLang::Bigquery => "bigquery",
            ScriptLang::Snowflake => "snowflake",
            ScriptLang::Graphql => "graphql",
            ScriptLang::Powershell => "powershell",
            ScriptLang::Mssql => "mssql",
            ScriptLang::Php => "php",
            ScriptLang::Bunnative => "bunnative",
            ScriptLang::Rust => "rust",
            ScriptLang::Ansible => "ansible",
            ScriptLang::Csharp => "csharp",
            ScriptLang::Oracledb => "oracledb",
            ScriptLang::Nu => "nu",
            ScriptLang::Java => "java",
            ScriptLang::Duckdb => "duckdb",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "python3" => Some(ScriptLang::Python3),
            "deno" => Some(ScriptLang::Deno),
            "go" => Some(ScriptLang::Go),
            "bash" => Some(ScriptLang::Bash),
            "postgresql" => Some(ScriptLang::Postgresql),
            "nativets" => Some(ScriptLang::Nativets),
            "bun" => Some(ScriptLang::Bun),
            "mysql" => Some(ScriptLang::Mysql),
            "bigquery" => Some(ScriptLang::Bigquery),
            "snowflake" => Some(ScriptLang::Snowflake),
            "graphql" => Some(ScriptLang::Graphql),
            "powershell" => Some(ScriptLang::Powershell),
            "mssql" => Some(ScriptLang::Mssql),
            "php" => Some(ScriptLang::Php),
            "bunnative" => Some(ScriptLang::Bunnative),
            "rust" => Some(ScriptLang::Rust),
            "ansible" => Some(ScriptLang::Ansible),
            "csharp" => Some(ScriptLang::Csharp),
            "oracledb" => Some(ScriptLang::Oracledb),
            "nu" => Some(ScriptLang::Nu),
            "java" => Some(ScriptLang::Java),
            "duckdb" => Some(ScriptLang::Duckdb),
            _ => None,
        }
    }
}

/// Job status for completed jobs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Success,
    Failure,
    Canceled,
    Skipped,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Success => "success",
            JobStatus::Failure => "failure",
            JobStatus::Canceled => "canceled",
            JobStatus::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "success" => Some(JobStatus::Success),
            "failure" => Some(JobStatus::Failure),
            "canceled" => Some(JobStatus::Canceled),
            "skipped" => Some(JobStatus::Skipped),
            _ => None,
        }
    }
}

/// Preview job request (simplified from windmill-api Preview struct)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewRequest {
    pub content: String,
    pub language: ScriptLang,
    #[serde(default)]
    pub args: serde_json::Value,
    pub lock: Option<String>,
    pub tag: Option<String>,
}

/// Flow preview request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPreviewRequest {
    pub value: serde_json::Value,  // FlowValue as JSON
    #[serde(default)]
    pub args: serde_json::Value,
    pub tag: Option<String>,
}

/// A queued job (combines v2_job and v2_job_queue)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedJob {
    pub id: Uuid,
    pub workspace_id: String,
    pub kind: JobKind,
    pub script_lang: Option<ScriptLang>,
    pub raw_code: Option<String>,
    pub raw_lock: Option<String>,
    pub raw_flow: Option<serde_json::Value>,
    pub args: serde_json::Value,
    pub tag: String,
    pub created_at: DateTime<Utc>,
    pub scheduled_for: DateTime<Utc>,
    pub running: bool,
    pub parent_job: Option<Uuid>,
    pub root_job: Option<Uuid>,
    pub flow_step_id: Option<String>,
}

/// A completed job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedJob {
    pub id: Uuid,
    pub workspace_id: String,
    pub status: JobStatus,
    pub result: serde_json::Value,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: Option<i64>,
}

/// Push a preview job to the queue
pub async fn push_preview(db: &LocalDb, req: PreviewRequest) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let args_json = serde_json::to_string(&req.args)?;
    let tag = req.tag.as_deref().unwrap_or("deno");

    // Insert into v2_job
    db.execute(
        r#"
        INSERT INTO v2_job (id, kind, script_lang, raw_code, raw_lock, args, tag, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        libsql::params![
            id.to_string(),
            JobKind::Preview.as_str(),
            req.language.as_str(),
            req.content,
            req.lock,
            args_json,
            tag,
            now.clone(),
        ],
    )
    .await?;

    // Insert into v2_job_queue
    db.execute(
        r#"
        INSERT INTO v2_job_queue (id, tag, created_at, scheduled_for, running)
        VALUES (?1, ?2, ?3, ?4, 0)
        "#,
        libsql::params![id.to_string(), tag, now.clone(), now],
    )
    .await?;

    // Insert into v2_job_runtime (for heartbeat tracking)
    db.execute(
        "INSERT INTO v2_job_runtime (id) VALUES (?1)",
        libsql::params![id.to_string()],
    )
    .await?;

    // Insert into job_perms (simplified)
    db.execute(
        "INSERT INTO job_perms (job_id) VALUES (?1)",
        libsql::params![id.to_string()],
    )
    .await?;

    tracing::info!("Pushed preview job: {}", id);
    Ok(id)
}

/// Push a flow preview job to the queue
pub async fn push_flow_preview(db: &LocalDb, req: FlowPreviewRequest) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let args_json = serde_json::to_string(&req.args)?;
    let flow_json = serde_json::to_string(&req.value)?;
    let tag = req.tag.as_deref().unwrap_or("flow");

    // Insert into v2_job
    db.execute(
        r#"
        INSERT INTO v2_job (id, kind, raw_flow, args, tag, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        libsql::params![
            id.to_string(),
            JobKind::FlowPreview.as_str(),
            flow_json,
            args_json,
            tag,
            now.clone(),
        ],
    )
    .await?;

    // Insert into v2_job_queue
    db.execute(
        r#"
        INSERT INTO v2_job_queue (id, tag, created_at, scheduled_for, running)
        VALUES (?1, ?2, ?3, ?4, 0)
        "#,
        libsql::params![id.to_string(), tag, now.clone(), now],
    )
    .await?;

    // Insert into v2_job_runtime
    db.execute(
        "INSERT INTO v2_job_runtime (id) VALUES (?1)",
        libsql::params![id.to_string()],
    )
    .await?;

    // Insert into job_perms
    db.execute(
        "INSERT INTO job_perms (job_id) VALUES (?1)",
        libsql::params![id.to_string()],
    )
    .await?;

    // Insert initial flow status
    db.execute(
        "INSERT INTO v2_job_status (id, flow_status) VALUES (?1, '{}')",
        libsql::params![id.to_string()],
    )
    .await?;

    tracing::info!("Pushed flow preview job: {}", id);
    Ok(id)
}

/// Get a completed job result (for polling)
pub async fn get_completed_job(db: &LocalDb, id: Uuid) -> Result<Option<CompletedJob>> {
    let mut rows = db
        .query(
            r#"
            SELECT id, workspace_id, status, result, started_at, completed_at, duration_ms
            FROM v2_job_completed
            WHERE id = ?1
            "#,
            libsql::params![id.to_string()],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        let status_str: String = row.get(2)?;
        let status = JobStatus::from_str(&status_str)
            .ok_or_else(|| LocalError::InvalidJobState(status_str))?;

        let result_str: Option<String> = row.get(3)?;
        let result: serde_json::Value = result_str
            .map(|s| serde_json::from_str(&s))
            .transpose()?
            .unwrap_or(serde_json::Value::Null);

        let started_at_str: Option<String> = row.get(4)?;
        let started_at = started_at_str
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()
            .ok()
            .flatten();

        let completed_at_str: String = row.get(5)?;
        let completed_at = DateTime::parse_from_rfc3339(&completed_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let duration_ms: Option<i64> = row.get(6)?;

        Ok(Some(CompletedJob {
            id,
            workspace_id: row.get(1)?,
            status,
            result,
            started_at,
            completed_at,
            duration_ms,
        }))
    } else {
        Ok(None)
    }
}

/// Mark a job as completed with result
pub async fn complete_job(
    db: &LocalDb,
    id: Uuid,
    status: JobStatus,
    result: serde_json::Value,
    started_at: DateTime<Utc>,
) -> Result<()> {
    let now = Utc::now();
    let duration_ms = (now - started_at).num_milliseconds();
    let result_json = serde_json::to_string(&result)?;

    db.execute(
        r#"
        INSERT INTO v2_job_completed (id, workspace_id, status, result, started_at, completed_at, duration_ms)
        SELECT ?1, workspace_id, ?2, ?3, ?4, ?5, ?6
        FROM v2_job WHERE id = ?1
        "#,
        libsql::params![
            id.to_string(),
            status.as_str(),
            result_json,
            started_at.to_rfc3339(),
            now.to_rfc3339(),
            duration_ms,
        ],
    )
    .await?;

    // Remove from queue
    db.execute(
        "DELETE FROM v2_job_queue WHERE id = ?1",
        libsql::params![id.to_string()],
    )
    .await?;

    tracing::info!("Completed job {}: {:?}", id, status);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_push_preview() {
        let db = LocalDb::in_memory().await.unwrap();

        let req = PreviewRequest {
            content: "export function main() { return 42; }".to_string(),
            language: ScriptLang::Deno,
            args: serde_json::json!({}),
            lock: None,
            tag: None,
        };

        let id = push_preview(&db, req).await.unwrap();

        // Verify job exists in queue
        let mut rows = db
            .query(
                "SELECT running FROM v2_job_queue WHERE id = ?1",
                libsql::params![id.to_string()],
            )
            .await
            .unwrap();

        let row = rows.next().await.unwrap().unwrap();
        let running: i64 = row.get(0).unwrap();
        assert_eq!(running, 0);
    }

    #[tokio::test]
    async fn test_complete_job() {
        let db = LocalDb::in_memory().await.unwrap();

        let req = PreviewRequest {
            content: "export function main() { return 42; }".to_string(),
            language: ScriptLang::Deno,
            args: serde_json::json!({}),
            lock: None,
            tag: None,
        };

        let id = push_preview(&db, req).await.unwrap();
        let started_at = Utc::now();

        complete_job(
            &db,
            id,
            JobStatus::Success,
            serde_json::json!(42),
            started_at,
        )
        .await
        .unwrap();

        // Verify job is in completed
        let completed = get_completed_job(&db, id).await.unwrap().unwrap();
        assert_eq!(completed.status, JobStatus::Success);
        assert_eq!(completed.result, serde_json::json!(42));
    }
}
