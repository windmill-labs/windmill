use crate::{error, DB};
use uuid::Uuid;

pub const STREAM_PREFIX: &str = "WM_STREAM: ";

pub fn extract_stream_from_logs(line: &str) -> Option<String> {
    if line.starts_with(STREAM_PREFIX) {
        // Extract the content after "WM_STREAM:" prefix
        let stream_content = line.strip_prefix(STREAM_PREFIX).unwrap_or("");
        if !stream_content.is_empty() {
            return Some(stream_content.to_string().replace("\\n", "\n"));
        }
    }
    None
}

pub async fn append_result_stream_db(
    db: &DB,
    workspace_id: &str,
    job_id: &Uuid,
    nstream: &str,
    offset: i32,
) -> error::Result<()> {
    if !nstream.is_empty() {
        sqlx::query!(
            r#"
            INSERT INTO job_result_stream_v2 (workspace_id, job_id, stream, idx)
            VALUES (
                $1, 
                $2,
                $3, 
                $4
            )
            ON CONFLICT (job_id, idx) DO UPDATE SET stream = job_result_stream_v2.stream || EXCLUDED.stream
            "#,
            workspace_id,
            job_id,
            nstream,
            offset
        )
        .execute(db)
        .await?;
    }
    Ok(())
}
