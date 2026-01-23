//! Queue operations for local mode
//!
//! Since local mode uses a single worker, we don't need the complex
//! `FOR UPDATE SKIP LOCKED` mechanism. Instead, we use simple atomic
//! operations with the database lock.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::LocalDb;
use crate::error::{LocalError, Result};
use crate::jobs::{JobKind, QueuedJob, ScriptLang};

/// Pull the next job from the queue
///
/// This is simplified from the PostgreSQL version since we have a single worker
/// and use the connection mutex for coordination.
pub async fn pull_job(db: &LocalDb) -> Result<Option<QueuedJob>> {
    let now = Utc::now().to_rfc3339();

    // Get the next job (ordered by priority, then scheduled_for)
    // We use a transaction-like approach: SELECT then UPDATE
    let mut rows = db
        .query(
            r#"
            SELECT q.id, j.workspace_id, j.kind, j.script_lang, j.raw_code, j.raw_lock,
                   j.raw_flow, j.args, q.tag, j.created_at, q.scheduled_for,
                   j.parent_job, j.root_job, j.flow_step_id
            FROM v2_job_queue q
            JOIN v2_job j ON q.id = j.id
            WHERE q.running = 0 AND q.scheduled_for <= ?1
            ORDER BY q.priority DESC, q.scheduled_for ASC
            LIMIT 1
            "#,
            libsql::params![now],
        )
        .await?;

    let Some(row) = rows.next().await? else {
        return Ok(None);
    };

    let id_str: String = row.get(0)?;
    let id = Uuid::parse_str(&id_str).map_err(|e| LocalError::InvalidJobState(e.to_string()))?;

    // Mark as running
    let started_at = Utc::now().to_rfc3339();
    db.execute(
        "UPDATE v2_job_queue SET running = 1, started_at = ?2 WHERE id = ?1",
        libsql::params![id_str.clone(), started_at],
    )
    .await?;

    // Parse the job fields
    let kind_str: String = row.get(2)?;
    let kind = match kind_str.as_str() {
        "preview" => JobKind::Preview,
        "flowpreview" => JobKind::FlowPreview,
        "script" => JobKind::Script,
        "flow" => JobKind::Flow,
        "flowscript" => JobKind::FlowScript,
        "flownode" => JobKind::FlowNode,
        _ => JobKind::Preview, // Default
    };

    let lang_str: Option<String> = row.get(3)?;
    let script_lang = lang_str.and_then(|s| ScriptLang::from_str(&s));

    let raw_code: Option<String> = row.get(4)?;
    let raw_lock: Option<String> = row.get(5)?;

    let raw_flow_str: Option<String> = row.get(6)?;
    let raw_flow = raw_flow_str
        .map(|s| serde_json::from_str(&s))
        .transpose()?;

    let args_str: Option<String> = row.get(7)?;
    let args: serde_json::Value = args_str
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    let tag: String = row.get(8)?;

    let created_at_str: String = row.get(9)?;
    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let scheduled_for_str: String = row.get(10)?;
    let scheduled_for = DateTime::parse_from_rfc3339(&scheduled_for_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let parent_job_str: Option<String> = row.get(11)?;
    let parent_job = parent_job_str.and_then(|s| Uuid::parse_str(&s).ok());

    let root_job_str: Option<String> = row.get(12)?;
    let root_job = root_job_str.and_then(|s| Uuid::parse_str(&s).ok());

    let flow_step_id: Option<String> = row.get(13)?;

    Ok(Some(QueuedJob {
        id,
        workspace_id: row.get(1)?,
        kind,
        script_lang,
        raw_code,
        raw_lock,
        raw_flow,
        args,
        tag,
        created_at,
        scheduled_for,
        running: true,
        parent_job,
        root_job,
        flow_step_id,
    }))
}

/// Get queue statistics
pub async fn queue_stats(db: &LocalDb) -> Result<QueueStats> {
    let mut rows = db
        .query(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN running = 1 THEN 1 ELSE 0 END) as running,
                SUM(CASE WHEN running = 0 THEN 1 ELSE 0 END) as pending
            FROM v2_job_queue
            "#,
            (),
        )
        .await?;

    let row = rows.next().await?.ok_or(LocalError::QueueEmpty)?;

    Ok(QueueStats {
        total: row.get::<i64>(0)? as u64,
        running: row.get::<i64>(1).unwrap_or(0) as u64,
        pending: row.get::<i64>(2).unwrap_or(0) as u64,
    })
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub total: u64,
    pub running: u64,
    pub pending: u64,
}

/// Update job heartbeat (ping)
pub async fn ping_job(db: &LocalDb, id: Uuid) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    db.execute(
        "UPDATE v2_job_runtime SET ping = ?2 WHERE id = ?1",
        libsql::params![id.to_string(), now],
    )
    .await?;
    Ok(())
}

/// Update flow status for a running flow job
pub async fn update_flow_status(
    db: &LocalDb,
    id: Uuid,
    flow_status: &serde_json::Value,
) -> Result<()> {
    let status_json = serde_json::to_string(flow_status)?;
    db.execute(
        "UPDATE v2_job_status SET flow_status = ?2 WHERE id = ?1",
        libsql::params![id.to_string(), status_json],
    )
    .await?;
    Ok(())
}

/// Push a child job for flow execution
pub async fn push_flow_child_job(
    db: &LocalDb,
    parent_id: Uuid,
    root_id: Uuid,
    step_id: &str,
    kind: JobKind,
    script_lang: Option<ScriptLang>,
    raw_code: Option<&str>,
    args: &serde_json::Value,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let args_json = serde_json::to_string(args)?;

    // Insert into v2_job
    db.execute(
        r#"
        INSERT INTO v2_job (id, kind, script_lang, raw_code, args, parent_job, root_job, flow_step_id, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        libsql::params![
            id.to_string(),
            kind.as_str(),
            script_lang.map(|l| l.as_str()),
            raw_code,
            args_json,
            parent_id.to_string(),
            root_id.to_string(),
            step_id,
            now.clone(),
        ],
    )
    .await?;

    // Insert into v2_job_queue
    db.execute(
        r#"
        INSERT INTO v2_job_queue (id, tag, created_at, scheduled_for, running)
        VALUES (?1, 'flow', ?2, ?3, 0)
        "#,
        libsql::params![id.to_string(), now.clone(), now],
    )
    .await?;

    // Insert into v2_job_runtime
    db.execute(
        "INSERT INTO v2_job_runtime (id) VALUES (?1)",
        libsql::params![id.to_string()],
    )
    .await?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::{push_preview, PreviewRequest};

    #[tokio::test]
    async fn test_pull_job() {
        let db = LocalDb::in_memory().await.unwrap();

        // Push a job
        let req = PreviewRequest {
            content: "export function main() { return 42; }".to_string(),
            language: ScriptLang::Deno,
            args: serde_json::json!({}),
            lock: None,
            tag: None,
        };
        let pushed_id = push_preview(&db, req).await.unwrap();

        // Pull it
        let job = pull_job(&db).await.unwrap().unwrap();
        assert_eq!(job.id, pushed_id);
        assert!(job.running);
        assert_eq!(job.kind, JobKind::Preview);

        // Queue should now be empty (job is running)
        let job2 = pull_job(&db).await.unwrap();
        assert!(job2.is_none());
    }

    #[tokio::test]
    async fn test_queue_stats() {
        let db = LocalDb::in_memory().await.unwrap();

        // Initially empty
        let stats = queue_stats(&db).await.unwrap();
        assert_eq!(stats.total, 0);

        // Push two jobs
        let req = PreviewRequest {
            content: "test".to_string(),
            language: ScriptLang::Deno,
            args: serde_json::json!({}),
            lock: None,
            tag: None,
        };
        push_preview(&db, req.clone()).await.unwrap();
        push_preview(&db, req).await.unwrap();

        let stats = queue_stats(&db).await.unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.pending, 2);
        assert_eq!(stats.running, 0);

        // Pull one
        pull_job(&db).await.unwrap();

        let stats = queue_stats(&db).await.unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.running, 1);
    }
}
