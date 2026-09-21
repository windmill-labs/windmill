/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::utils::{content_plain, require_devops_role};
use axum::{body::Body, extract::Query, response::Response, routing::get, Extension, Json, Router};
use serde::Serialize;

use windmill_common::{
    error::{Error, JsonResult},
    utils::Pagination,
};

use crate::db::{ApiAuthed, DB};

pub fn global_service() -> Router {
    Router::new()
        .route("/list_files", get(list_files))
        .route("/get_log_file/{*path}", get(get_log_file))
}
use axum::extract::Path;

#[derive(Debug, serde::Deserialize)]
pub struct LogFileQuery {
    before: Option<chrono::DateTime<chrono::Utc>>,
    after: Option<chrono::DateTime<chrono::Utc>>,
    with_error: Option<bool>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct LogFile {
    pub hostname: String,
    pub mode: String,
    pub worker_group: Option<String>,
    pub log_ts: chrono::NaiveDateTime,
    pub file_path: String,
    pub ok_lines: Option<i64>,
    pub err_lines: Option<i64>,
    pub json_fmt: bool,
}
async fn list_files(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<LogFileQuery>,
) -> JsonResult<Vec<LogFile>> {
    require_devops_role(&db, &authed).await?;
    let (per_page, offset) = windmill_common::utils::paginate(pagination);

    let mut sqlb = sql_builder::SqlBuilder::select_from("log_file")
        .fields(&[
            "hostname",
            "mode::text",
            "worker_group",
            "log_ts",
            "file_path",
            "ok_lines",
            "err_lines",
            "json_fmt",
        ])
        .order_by("log_ts", true)
        .offset(offset)
        .limit(per_page)
        .clone();

    if let Some(dt) = &lq.before {
        sqlb.and_where_le(
            "log_ts",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }
    if let Some(dt) = &lq.after {
        sqlb.and_where_ge(
            "log_ts",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }

    if let Some(true) = lq.with_error {
        sqlb.and_where("err_lines > 0");
    }
    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let rows = sqlx::query_as::<_, LogFile>(&sql).fetch_all(&db).await?;
    Ok(Json(rows))
}

/// Rebuild one source log file from the columnar store.
///
/// Not the original bytes: the store holds a line's fields rather than its text,
/// so the JSON is re-serialized here and key order and whitespace are this
/// writer's. Everything a reader can see survives — the drawer this feeds
/// renders a prettified view of each line either way, and a line that was never
/// JSON comes back exactly as it was written.
#[cfg(all(feature = "tantivy", feature = "private"))]
async fn get_log_file_from_store(
    db: &DB,
    store: &windmill_indexer::service_logs_store_ee::Store,
    path: &str,
) -> windmill_common::error::Result<Response> {
    let (hostname, file_name) = path
        .split_once('/')
        .ok_or_else(|| Error::BadRequest("Invalid path".to_string()))?;

    // The store is partitioned by day and mode, neither of which the path
    // carries. `log_file` names both, and its primary key starts with hostname.
    let file = sqlx::query!(
        // `mode!` because the column is NOT NULL and only the cast makes sqlx
        // think otherwise; a silent default would look up a `mode=` partition
        // that matches nothing and read as a missing file.
        "SELECT mode::text AS \"mode!\", log_ts FROM log_file WHERE hostname = $1 AND file_path = $2 ORDER BY log_ts DESC LIMIT 1",
        hostname,
        file_name
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("File {path} not found")))?;

    // A row registered by this version carries the minute in the file's own name,
    // so the two agree and the second is redundant. One written before the
    // uploader derived `log_ts` from the name carries a wall clock instead, and
    // those outlive an upgrade by the retention period — which is also what makes
    // the `ORDER BY` above worth having. The name is authoritative, so both go.
    let mut known_ts = vec![chrono::DateTime::from_naive_utc_and_offset(
        file.log_ts,
        chrono::Utc,
    )];
    if let Some(named) = file_name.rsplit('.').next().and_then(|s| {
        chrono::NaiveDateTime::parse_from_str(s, windmill_common::tracing_init::LOG_TIMESTAMP_FMT)
            .ok()
    }) {
        known_ts.push(chrono::DateTime::from_naive_utc_and_offset(
            named,
            chrono::Utc,
        ));
    }

    let text = windmill_indexer::service_logs_store_ee::read_log_file(
        store, &file.mode, hostname, file_name, &known_ts,
    )
    .await
    .map_err(|e| Error::internal_err(format!("Error reading the service log store: {e}")))?
    .ok_or_else(|| Error::NotFound(format!("File {path} not found")))?;

    Ok(content_plain(Body::from(text)))
}

async fn get_log_file(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(path): Path<windmill_common::utils::StripPath>,
) -> windmill_common::error::Result<Response> {
    use windmill_common::tracing_init::TMP_WINDMILL_LOGS_SERVICE;

    require_devops_role(&db, &authed).await?;
    let path = path.to_path();
    if path.contains("..") {
        return Err(Error::BadRequest("Invalid path".to_string()));
    }
    #[cfg(feature = "parquet")]
    let s3_client = windmill_object_store::get_object_store().await;
    #[cfg(feature = "parquet")]
    if let Some(s3_client) = s3_client {
        use windmill_object_store::object_store_reexports::ObjectStoreError;

        // The raw file, for as long as it is there. It outlives its ingestion by
        // one indexer pass at most, so this covers the most recent minutes of a
        // host's logs byte for byte; everything older is rebuilt from the store.
        let object_path = format!("{}{}", windmill_common::tracing_init::LOGS_SERVICE, path);
        match s3_client
            .get(&windmill_object_store::object_store_reexports::Path::from(
                object_path,
            ))
            .await
        {
            Ok(file) => match file.bytes().await {
                Ok(bytes) => {
                    return Ok(content_plain(Body::from(bytes::Bytes::from(bytes))));
                }
                Err(e) => {
                    return Err(Error::internal_err(format!(
                        "Error pulling the bytes: {}",
                        e
                    )));
                }
            },
            Err(ObjectStoreError::NotFound { .. }) => {}
            Err(e) => {
                return Err(Error::internal_err(format!(
                    "Error fetching the file: {}",
                    e
                )));
            }
        }

        #[cfg(all(feature = "tantivy", feature = "private"))]
        return get_log_file_from_store(&db, &s3_client, &path).await;
        #[cfg(not(all(feature = "tantivy", feature = "private")))]
        return Err(Error::NotFound(format!("File {path} not found")));
    }
    let full_path = format!("{}{}", *TMP_WINDMILL_LOGS_SERVICE, path);
    // SECURITY (defense in depth): refuse to read through a symlink so a planted
    // symlink in the logs directory cannot be used to exfiltrate arbitrary files.
    // `symlink_metadata` returns the link's own metadata without following it.
    match tokio::fs::symlink_metadata(&full_path).await {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(Error::BadRequest("Invalid path".to_string()));
        }
        Ok(_) => {}
        Err(_) => return Err(Error::NotFound(format!("File {path} not found"))),
    }
    let file = tokio::fs::read(&full_path).await;
    if let Ok(bytes) = file {
        Ok(content_plain(Body::from(bytes::Bytes::from(bytes))))
    } else {
        Err(Error::NotFound(format!("File {path} not found")))
    }
}
