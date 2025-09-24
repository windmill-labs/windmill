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
) -> error::Result<()> {
    if !nstream.is_empty() {
        sqlx::query!(
            r#"
            INSERT INTO job_result_stream_v2 (workspace_id, job_id, stream, idx)
            VALUES (
                $1, 
                $2,
                $3, 
                (
                    SELECT COALESCE(MAX(idx), -1) + 1 
                    FROM job_result_stream_v2 
                    WHERE job_id = $2
                )
            )
            "#,
            workspace_id,
            job_id,
            nstream,
        )
        .execute(db)
        .await?;
    }
    Ok(())
}
