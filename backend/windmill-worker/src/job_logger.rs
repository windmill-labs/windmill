use regex::Regex;

pub use windmill_common::jobs::LARGE_LOG_THRESHOLD_SIZE;
use windmill_common::worker::{Connection, CLOUD_HOSTED};

use windmill_queue::append_logs;

use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use uuid::Uuid;

#[cfg(not(all(feature = "enterprise", feature = "parquet")))]
use crate::job_logger_ee::default_disk_log_storage;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use crate::job_logger_ee::s3_storage;

pub enum CompactLogs {
    #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
    NotEE,
    #[allow(dead_code)]
    NoS3,
    #[allow(dead_code)]
    S3,
}

pub async fn append_job_logs(
    job_id: Uuid,
    w_id: String,
    logs: String,
    conn: Connection,
    must_compact_logs: bool,
    total_size: Arc<AtomicU32>,
    worker_name: String,
) -> () {
    match conn {
        Connection::Sql(db) if must_compact_logs => {
            #[cfg(all(feature = "enterprise", feature = "parquet"))]
            s3_storage(job_id, &w_id, &db, logs, total_size, &worker_name).await;

            #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
            {
                default_disk_log_storage(
                    job_id,
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
