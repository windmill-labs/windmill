use regex::Regex;

pub use windmill_common::jobs::LARGE_LOG_THRESHOLD_SIZE;
use windmill_common::result_stream::append_result_stream_db;
use windmill_common::utils::WarnAfterExt;
use windmill_common::worker::{Connection, CLOUD_HOSTED};

use windmill_common::{error, DB};
use windmill_queue::append_logs;

use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use uuid::Uuid;

#[cfg(not(all(feature = "enterprise", feature = "parquet")))]
use crate::job_logger_oss::default_disk_log_storage;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use crate::job_logger_oss::s3_storage;

pub enum CompactLogs {
    #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
    NotEE,
    #[allow(dead_code)]
    NoS3,
    #[allow(dead_code)]
    S3,
}

pub async fn append_job_logs(
    job_id: &Uuid,
    w_id: &str,
    logs: &str,
    conn: &Connection,
    must_compact_logs: bool,
    total_size: Arc<AtomicU32>,
    worker_name: &str,
) -> () {
    match conn {
        Connection::Sql(db) if must_compact_logs => {
            #[cfg(all(feature = "enterprise", feature = "parquet"))]
            s3_storage(&job_id, &w_id, &db, logs, total_size, worker_name).await;

            #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
            {
                default_disk_log_storage(
                    &job_id,
                    &w_id,
                    &db,
                    logs,
                    total_size,
                    CompactLogs::NotEE,
                    &worker_name,
                )
                .await;
            }
        }
        _ => {
            append_logs(&job_id, w_id, logs, &conn).await;
        }
    }
}

pub async fn append_result_stream(
    conn: &Connection,
    workspace_id: &str,
    job_id: &Uuid,
    nstream: &str,
) -> error::Result<()> {
    match conn {
        Connection::Sql(db) => {
            append_result_stream_db(db, workspace_id, job_id, nstream).await?;
        }
        Connection::Http(client) => {
            if let Err(e) = client
                .post::<_, String>(
                    &format!("/api/w/{}/agent_workers/push_logs/{}", workspace_id, job_id),
                    None,
                    &nstream,
                )
                .await
            {
                tracing::error!(%job_id, %e, "error sending result stream for  job {job_id}: {e}");
            };
        }
    }
    Ok(())
}

pub async fn append_logs_with_compaction(
    job_id: &Uuid,
    w_id: &str,
    logs: &str,
    db: &DB,
    worker_name: &str,
) {
    let log_length = sqlx::query_scalar!(
        "INSERT INTO job_logs (logs, job_id, workspace_id) VALUES ($1, $2, $3) ON CONFLICT (job_id) DO UPDATE SET logs = concat(job_logs.logs, $1::text) RETURNING length(logs)",
        logs,
        job_id,
        &w_id,
    )
    .fetch_one(db)
    .warn_after_seconds(1)
    .await;
    match log_length {
        Ok(length) => {
            let len = length.unwrap_or(0);
            let conn: Connection = db.into();
            if len > LARGE_LOG_THRESHOLD_SIZE as i32 {
                append_job_logs(
                    &job_id,
                    w_id,
                    "",
                    &conn,
                    true,
                    Arc::new(AtomicU32::new(len as u32)),
                    worker_name,
                )
                .await;
            }
        }
        Err(err) => {
            tracing::error!(%job_id, %err, "error updating logs for job {job_id}: {err}");
        }
    }
}

lazy_static::lazy_static! {
    static ref RE_00: Regex = Regex::new('\u{00}'.to_string().as_str()).unwrap();
    pub static ref NO_LOGS_AT_ALL: bool = std::env::var("NO_LOGS_AT_ALL").ok().is_some_and(|x| x == "1" || x == "true");
}
// as a detail, `BufReader::lines()` removes \n and \r\n from the strings it yields,
// so this pushes \n to thd destination string in each call
pub fn append_with_limit(dst: &mut String, src: &str, limit: &mut usize) {
    if *NO_LOGS_AT_ALL {
        return;
    }

    let src_str;
    let src = {
        src_str = RE_00.replace_all(src, "");
        src_str.as_ref()
    };
    if !*CLOUD_HOSTED {
        dst.push('\n');
        dst.push_str(&src);
        return;
    } else {
        if *limit > 0 {
            dst.push('\n');
        }
        *limit -= 1;
    }

    let src_len = src.chars().count();
    if src_len <= *limit {
        dst.push_str(&src);
        *limit -= src_len;
    } else {
        let byte_pos = src
            .char_indices()
            .skip(*limit)
            .next()
            .map(|(byte_pos, _)| byte_pos)
            .unwrap_or(0);
        dst.push_str(&src[0..byte_pos]);
        *limit = 0;
    }
}
