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
        .route("/get_log_file/*path", get(get_log_file))
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
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<LogFileQuery>,
) -> JsonResult<Vec<LogFile>> {
    require_devops_role(&db, &email).await?;
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

async fn get_log_file(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(path): Path<windmill_common::utils::StripPath>,
) -> windmill_common::error::Result<Response> {
    use windmill_common::tracing_init::TMP_WINDMILL_LOGS_SERVICE;

    require_devops_role(&db, &email).await?;
    let path = path.to_path();
    #[cfg(feature = "parquet")]
    let s3_client = windmill_object_store::get_object_store().await;
    #[cfg(feature = "parquet")]
    if let Some(s3_client) = s3_client {
        let path = format!("{}{}", windmill_common::tracing_init::LOGS_SERVICE, path);
        let file = s3_client.get(&windmill_object_store::object_store_reexports::Path::from(path)).await;
        match file {
            Ok(file) => {
                let bytes = file.bytes().await;
                match bytes {
                    Ok(bytes) => {
                        return Ok(content_plain(Body::from(bytes::Bytes::from(bytes))));
                    }
                    Err(e) => {
                        return Err(Error::internal_err(format!(
                            "Error pulling the bytes: {}",
                            e
                        )));
                    }
                }
            }
            Err(e) => {
                return Err(Error::internal_err(format!(
                    "Error fetching the file: {}",
                    e
                )));
            }
        }
    }
    let file = tokio::fs::read(format!("{}{}", TMP_WINDMILL_LOGS_SERVICE, path)).await;
    if let Ok(bytes) = file {
        Ok(content_plain(Body::from(bytes::Bytes::from(bytes))))
    } else {
        Err(Error::NotFound(format!("File {path} not found")))
    }
}
