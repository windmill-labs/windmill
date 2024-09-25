use async_recursion::async_recursion;
use deno_ast::swc::parser::lexer::util::CharExt;
use futures::Future;
use itertools::Itertools;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use object_store::path::Path;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{json, Value};
use sqlx::types::Json;
use sqlx::{Pool, Postgres};
use tokio::process::Command;
use tokio::{fs::File, io::AsyncReadExt};
use windmill_common::jobs::ENTRYPOINT_OVERRIDE;
#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS;
#[cfg(feature = "parquet")]
use windmill_common::s3_helpers::{
    get_etag_or_empty, LargeFileStorage, ObjectStoreResource, S3Object,
};
use windmill_common::variables::{build_crypt_with_key_suffix, decrypt_value_with_mc};
use windmill_common::worker::{
    get_windmill_memory_usage, get_worker_memory_usage, to_raw_value, write_file, CLOUD_HOSTED,
    ROOT_CACHE_DIR, TMP_DIR, WORKER_CONFIG,
};
use windmill_common::{
    error::{self, Error},
    jobs::QueuedJob,
    variables::ContextualVariable,
};

use anyhow::{anyhow, Result};
use windmill_queue::{append_logs, CanceledBy};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::process::ExitStatusExt;

use std::process::ExitStatus;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    io, panic,
    time::Duration,
};

use uuid::Uuid;
use windmill_common::{variables, DB};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Child,
    sync::broadcast,
    time::{interval, Instant, MissedTickBehavior},
};

use futures::{future::FutureExt, stream, StreamExt};

use crate::{
    AuthedClient, AuthedClientBackgroundTask, JOB_DEFAULT_TIMEOUT, MAX_RESULT_SIZE,
    MAX_TIMEOUT_DURATION,
};

pub enum CompactLogs {
    NotEE,
    NoS3,
    S3,
}

async fn compact_logs(
    job_id: Uuid,
    w_id: &str,
    db: &DB,
    nlogs: String,
    total_size: Arc<AtomicU32>,
    compact_kind: CompactLogs,
    _worker_name: &str,
) -> error::Result<(String, String)> {
    let mut prev_logs = sqlx::query_scalar!(
        "SELECT logs FROM job_logs WHERE job_id = $1 AND workspace_id = $2",
        job_id,
        w_id
    )
    .fetch_optional(db)
    .await?
    .flatten()
    .unwrap_or_default();
    let size = prev_logs.char_indices().count() as i32;
    let nlogs_len = nlogs.char_indices().count();
    let to_keep_in_db = usize::max(
        usize::min(nlogs_len, 3000),
        nlogs_len % LARGE_LOG_THRESHOLD_SIZE,
    );
    let extra_split = to_keep_in_db < nlogs_len;
    let stored_in_storage_len = if extra_split {
        nlogs_len - to_keep_in_db
    } else {
        0
    };
    let extra_to_newline = nlogs
        .chars()
        .skip(stored_in_storage_len)
        .find_position(|x| x.is_line_break())
        .map(|(i, _)| i)
        .unwrap_or(to_keep_in_db);
    let stored_in_storage_to_newline = stored_in_storage_len + extra_to_newline;

    let (append_to_storage, stored_in_db) = if extra_split {
        if stored_in_storage_to_newline == nlogs.len() {
            (nlogs.as_ref(), "".to_string())
        } else {
            let split_idx = nlogs
                .char_indices()
                .nth(stored_in_storage_to_newline)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let (append_to_storage, stored_in_db) = nlogs.split_at(split_idx);
            // tracing::error!("{append_to_storage} ||||| {stored_in_db}");
            // tracing::error!(
            //     "{:?} {:?} {} {}",
            //     excess_prev_logs.lines().last(),
            //     current_logs.lines().next(),
            //     split_idx,
            //     excess_size_modulo
            // );
            (append_to_storage, stored_in_db.to_string())
        }
    } else {
        // tracing::error!("{:?}", nlogs.lines().last());
        ("", nlogs.to_string())
    };

    let new_size_with_excess = size + stored_in_storage_to_newline as i32;

    let new_size = total_size.fetch_add(
        new_size_with_excess as u32,
        std::sync::atomic::Ordering::SeqCst,
    ) + new_size_with_excess as u32;

    let path = format!(
        "logs/{job_id}/{}_{new_size}.txt",
        chrono::Utc::now().timestamp_millis()
    );

    let mut new_current_logs = match compact_kind {
        CompactLogs::NoS3 => format!("\n[windmill] No object storage set in instance settings. Previous logs have been saved to disk at {path}"),
        CompactLogs::S3 => format!("\n[windmill] Previous logs have been saved to object storage at {path}"),
        CompactLogs::NotEE => format!("\n[windmill] Previous logs have been saved to disk at {path}"),
    };
    new_current_logs.push_str(&stored_in_db);

    sqlx::query!(
        "UPDATE job_logs SET logs = $1, log_offset = $2, 
        log_file_index = array_append(coalesce(log_file_index, array[]::text[]), $3) 
        WHERE workspace_id = $4 AND job_id = $5",
        new_current_logs,
        new_size as i32,
        path,
        w_id,
        job_id
    )
    .execute(db)
    .await?;
    prev_logs.push_str(&append_to_storage);

    return Ok((prev_logs, path));
}

async fn default_disk_log_storage(
    job_id: Uuid,
    w_id: &str,
    db: &DB,
    nlogs: String,
    total_size: Arc<AtomicU32>,
    compact_kind: CompactLogs,
    worker_name: &str,
) {
    match compact_logs(
        job_id,
        &w_id,
        &db,
        nlogs,
        total_size,
        compact_kind,
        worker_name,
    )
    .await
    {
        Err(e) => tracing::error!("Could not compact logs for job {job_id}: {e:?}",),
        Ok((prev_logs, path)) => {
            let path = format!("{}/{}", TMP_DIR, path);
            let splitted = &path.split("/").collect_vec();
            tokio::fs::create_dir_all(splitted.into_iter().take(splitted.len() - 1).join("/"))
                .await
                .map_err(|e| {
                    tracing::error!("Could not create logs directory: {e:?}",);
                    e
                })
                .ok();
            let created = tokio::fs::File::create(&path).await;
            if let Err(e) = created {
                tracing::error!("Could not create logs file {path}: {e:?}",);
                return;
            }
            if let Err(e) = tokio::fs::write(&path, prev_logs).await {
                tracing::error!("Could not write to logs file {path}: {e:?}");
            } else {
                tracing::info!("Logs length of {job_id} has exceeded a threshold. Previous logs have been saved to disk at {path}");
            }
        }
    }
}

async fn append_job_logs(
    job_id: Uuid,
    w_id: String,
    logs: String,
    db: DB,
    must_compact_logs: bool,
    total_size: Arc<AtomicU32>,
    worker_name: String,
) -> () {
    if must_compact_logs {
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
            match compact_logs(
                job_id,
                &w_id,
                &db,
                logs,
                total_size,
                CompactLogs::S3,
                &worker_name,
            )
            .await
            {
                Err(e) => tracing::error!("Could not compact logs for job {job_id}: {e:?}",),
                Ok((prev_logs, path)) => {
                    tracing::info!("Logs length of {job_id} has exceeded a threshold. Previous logs have been saved to object storage at {path}");
                    let path2 = path.clone();
                    if let Err(e) = os
                        .put(&Path::from(path), prev_logs.to_string().into_bytes().into())
                        .await
                    {
                        tracing::error!("Could not save logs to s3: {e:?}");
                    }
                    tracing::info!("Logs of {job_id} saved to object storage at {path2}");
                }
            }
        } else {
            default_disk_log_storage(
                job_id,
                &w_id,
                &db,
                logs,
                total_size,
                CompactLogs::NoS3,
                &worker_name,
            )
            .await;
        }

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
    } else {
        append_logs(&job_id, w_id, logs, db).await;
    }
}

pub const LARGE_LOG_THRESHOLD_SIZE: usize = 9000;

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
