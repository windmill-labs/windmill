/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::utils::content_plain;
use axum::{
    body::Body,
    extract::Query,
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::NaiveDateTime;
use serde::Serialize;

use windmill_common::{
    error::{Error, JsonResult},
    utils::Pagination,
};

use crate::{
    db::{ApiAuthed, DB},
    utils::require_super_admin,
};

pub fn global_service() -> Router {
    Router::new()
        .route("/list_files", get(list_files))
        .route("/get_log_stream", post(get_log_stream))
}

#[derive(Debug, serde::Deserialize)]
pub struct LogFileQuery {
    before: Option<NaiveDateTime>,
    after: Option<NaiveDateTime>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct LogFile {
    pub hostname: String,
    pub mode: String,
    pub file_ts: String,
    pub file_path: String,
}
async fn list_files(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<LogFileQuery>,
) -> JsonResult<Vec<LogFile>> {
    require_super_admin(&db, &email).await?;
    let (per_page, offset) = windmill_common::utils::paginate(pagination);

    let mut sqlb = sql_builder::SqlBuilder::select_from("log_file")
        .field("*")
        .order_by("file_ts", true)
        .offset(offset)
        .limit(per_page)
        .clone();

    if let Some(dt) = &lq.before {
        sqlb.and_where_le(
            "file_ts",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }
    if let Some(dt) = &lq.after {
        sqlb.and_where_ge(
            "file_ts",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }
    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let rows = sqlx::query_as::<_, LogFile>(&sql).fetch_all(&db).await?;
    Ok(Json(rows))
}

#[cfg(feature = "parquet")]
async fn get_log_stream(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(lf): Json<Vec<String>>,
) -> windmill_common::error::Result<Response> {
    use windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS;

    require_super_admin(&db, &email).await?;
    let s3_client = OBJECT_STORE_CACHE_SETTINGS.read().await.clone();
    if let Some(s3_client) = s3_client {
        let stream = async_stream::stream! {
            for file_p in lf {
                let file_p_2 = file_p.clone();
                let file = s3_client.get(&object_store::path::Path::from(file_p)).await;
                if let Ok(file) = file {
                    if let Ok(bytes) = file.bytes().await {
                        yield Ok(bytes::Bytes::from(bytes)) as object_store::Result<bytes::Bytes>;
                    }
                } else {
                    tracing::debug!("error getting file from store: {file_p_2}: {}", file.err().unwrap());
                }
            }
        };
        Ok(content_plain(Body::from_stream(stream)))
    } else {
        Err(Error::InternalErr("object store not enabled".to_string()))
    }
}

#[cfg(not(feature = "parquet"))]
async fn get_log_stream(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(lf): Json<Vec<String>>,
) -> windmill_common::error::Result<Response> {
    require_super_admin(&db, &email).await?;
    let stream = async_stream::stream! {
        for file_p in lf {
            let file_p_2 = file_p.clone();
            let file = tokio::fs::read(file_p).await;
            if let Ok(file) = file {
                yield Ok(bytes::Bytes::from(file)) as anyhow::Result<bytes::Bytes>;
            } else {
                tracing::debug!("error getting file from store: {file_p_2}: {}", file.err().unwrap());
            }
        }
    };
    Ok(content_plain(Body::from_stream(stream)))
}
