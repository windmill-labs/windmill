use std::io::{self};
use std::ops::Range;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{cmp, future};

use crate::{db::DB, resources::get_resource_value_interpolated_internal, users::Tokened};
use axum::body::StreamBody;
use axum::extract::{BodyStream, DefaultBodyLimit};
use axum::headers::HeaderMap;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use futures::{StreamExt, TryStreamExt};
use hyper::http;
use object_store::{ClientConfigKey, ObjectStore};
use polars::{
    io::{
        cloud::{AmazonS3ConfigKey, CloudOptions},
        SerReader,
    },
    lazy::{
        dsl::col,
        frame::{LazyFrame, ScanArgsParquet},
    },
    prelude::CsvReader,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use tokio::io::{copy, AsyncWriteExt};
use tokio_util::io::StreamReader;
use tower_http::cors::{Any, CorsLayer};
use windmill_common::error::{Error, JsonResult};
use windmill_common::worker::to_raw_value;
use windmill_common::{
    db::UserDB,
    error,
    s3_helpers::{build_object_store_client, render_endpoint, LargeFileStorage, S3Resource},
};

use crate::db::ApiAuthed;

pub fn workspaced_service() -> Router {
    let cors = CorsLayer::new()
        .allow_methods([http::Method::GET, http::Method::POST])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
        .allow_origin(Any);

    Router::new()
        .route(
            "/duckdb_connection_settings",
            post(duckdb_connection_settings).layer(cors.clone()),
        )
        .route(
            "/v2/duckdb_connection_settings",
            post(duckdb_connection_settings_v2).layer(cors.clone()),
        )
        .route(
            "/polars_connection_settings",
            post(polars_connection_settings).layer(cors.clone()),
        )
        .route(
            "/v2/polars_connection_settings",
            post(polars_connection_settings_v2).layer(cors.clone()),
        )
        .route(
            "/v2/s3_resource_info",
            post(s3_resource_info).layer(cors.clone()),
        )
        .route("/test_connection", get(test_connection).layer(cors.clone()))
        .route(
            "/list_stored_files",
            get(list_stored_files).layer(cors.clone()),
        )
        .route(
            "/load_file_metadata",
            get(load_file_metadata).layer(cors.clone()),
        )
        .route(
            "/load_file_preview",
            get(load_file_preview).layer(cors.clone()),
        )
        .route(
            "/load_parquet_preview/*path",
            get(load_parquet_preview).layer(cors.clone()),
        )
        .route(
            "/delete_s3_file",
            delete(delete_s3_file).layer(cors.clone()),
        )
        .route("/move_s3_file", get(move_s3_file).layer(cors.clone()))
        .route(
            "/multipart_upload_s3_file",
            post(multipart_upload_s3_file).layer(cors.clone()),
        )
        .route(
            "/download_s3_file",
            get(download_s3_file).layer(cors.clone()),
        )
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // necessary for multipart upload
}

#[derive(Deserialize)]
struct DuckdbConnectionSettingsQuery {
    s3_resource: S3Resource,
}
#[derive(Serialize)]
struct DuckdbConnectionSettingsResponse {
    connection_settings_str: String,
}

async fn duckdb_connection_settings(
    Path(_w_id): Path<String>,
    Json(query): Json<DuckdbConnectionSettingsQuery>,
) -> JsonResult<DuckdbConnectionSettingsResponse> {
    let mut duckdb_settings: String = String::new();

    let s3_resource = query.s3_resource;
    duckdb_settings.push_str("SET home_directory='./';\n"); // TODO: make this configurable maybe, or point to a temporary folder
    duckdb_settings.push_str("INSTALL 'httpfs';\n");
    if s3_resource.path_style {
        duckdb_settings.push_str("SET s3_url_style='path';\n");
    }
    duckdb_settings.push_str(format!("SET s3_region='{}';\n", s3_resource.region).as_str());
    duckdb_settings.push_str(format!("SET s3_endpoint='{}';\n", s3_resource.endpoint).as_str());
    if !s3_resource.use_ssl {
        duckdb_settings.push_str("SET s3_use_ssl=0;\n"); // default is true for DuckDB
    }
    if let Some(access_key_id) = s3_resource.access_key {
        duckdb_settings.push_str(format!("SET s3_access_key_id='{}';\n", access_key_id).as_str());
    }
    if let Some(secret_access_key) = s3_resource.secret_key {
        duckdb_settings
            .push_str(format!("SET s3_secret_access_key='{}';\n", secret_access_key).as_str());
    }

    let response = DuckdbConnectionSettingsResponse { connection_settings_str: duckdb_settings };
    return Ok(Json(response));
}

#[derive(Deserialize)]
struct DuckdbConnectionSettingsQueryV2 {
    s3_resource_path: Option<String>,
}

async fn duckdb_connection_settings_v2(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Json(query): Json<DuckdbConnectionSettingsQueryV2>,
) -> JsonResult<DuckdbConnectionSettingsResponse> {
    let s3_resource_opt = match query.s3_resource_path {
        Some(s3_resource_path) => {
            get_s3_resource(
                &authed,
                &db,
                Some(user_db),
                &token,
                &w_id,
                s3_resource_path.as_str(),
            )
            .await?
        }
        None => {
            let (_, s3_resource_opt) =
                get_workspace_s3_resource(&authed, &db, Some(user_db), &token, &w_id).await?;
            s3_resource_opt
        }
    };
    let s3_resource = s3_resource_opt.ok_or(Error::NotFound(
        "No datasets storage resource defined at the workspace level".to_string(),
    ))?;
    return duckdb_connection_settings(
        Path(w_id),
        Json(DuckdbConnectionSettingsQuery { s3_resource }),
    )
    .await;
}

#[derive(Deserialize)]
struct PolarsConnectionSettingsQuery {
    s3_resource: S3Resource,
}

#[derive(Serialize)]
struct PolarsConnectionSettings {
    pub region_name: String,
}

async fn polars_connection_settings(
    Path(_w_id): Path<String>,
    Json(query): Json<PolarsConnectionSettingsQuery>,
) -> JsonResult<S3fsArgs> {
    let s3_resource = query.s3_resource;

    let response = S3fsArgs {
        endpoint_url: render_endpoint(&s3_resource),
        key: s3_resource.access_key,
        secret: s3_resource.secret_key,
        use_ssl: s3_resource.use_ssl,
        cache_regions: false,
        client_kwargs: PolarsConnectionSettings { region_name: s3_resource.region },
    };
    return Ok(Json(response));
}

#[derive(Deserialize)]
struct PolarsConnectionSettingsQueryV2 {
    s3_resource_path: Option<String>,
}

#[derive(Serialize)]
struct PolarsConnectionSettingsResponse {
    s3fs_args: S3fsArgs,
    storage_options: PolarsStorageOptions,
}

#[derive(Serialize)]
struct S3fsArgs {
    endpoint_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret: Option<String>,
    use_ssl: bool,
    cache_regions: bool,
    client_kwargs: PolarsConnectionSettings,
}

#[derive(Serialize)]
struct PolarsStorageOptions {
    aws_endpoint_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    aws_access_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aws_secret_access_key: Option<String>,
    aws_region: String,
    aws_allow_http: String,
}

async fn polars_connection_settings_v2(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Json(query): Json<PolarsConnectionSettingsQueryV2>,
) -> JsonResult<PolarsConnectionSettingsResponse> {
    let s3_resource_opt = match query.s3_resource_path {
        Some(s3_resource_path) => {
            get_s3_resource(
                &authed,
                &db,
                Some(user_db),
                &token,
                &w_id,
                s3_resource_path.as_str(),
            )
            .await?
        }
        None => {
            let (_, s3_resource_opt) =
                get_workspace_s3_resource(&authed, &db, Some(user_db), &token, &w_id).await?;
            s3_resource_opt
        }
    };
    let s3_resource = s3_resource_opt.ok_or(Error::NotFound(
        "No datasets storage resource defined at the workspace level".to_string(),
    ))?;
    let s3fs = polars_connection_settings(
        Path(w_id),
        Json(PolarsConnectionSettingsQuery { s3_resource: s3_resource.clone() }),
    )
    .await?
    .0;
    let response = PolarsConnectionSettingsResponse {
        s3fs_args: s3fs,
        storage_options: PolarsStorageOptions {
            aws_endpoint_url: render_endpoint(&s3_resource),
            aws_access_key_id: s3_resource.access_key,
            aws_secret_access_key: s3_resource.secret_key,
            aws_region: s3_resource.region,
            aws_allow_http: (!s3_resource.use_ssl).to_string(),
        },
    };
    return Ok(Json(response));
}

#[derive(Deserialize)]
struct S3ResourceInfoQuery {
    s3_resource_path: Option<String>,
}

async fn s3_resource_info(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Json(query): Json<S3ResourceInfoQuery>,
) -> JsonResult<S3Resource> {
    let s3_resource_opt = match query.s3_resource_path {
        Some(s3_resource_path) => {
            get_s3_resource(
                &authed,
                &db,
                Some(user_db),
                &token,
                &w_id,
                s3_resource_path.as_str(),
            )
            .await?
        }
        None => {
            let (_, s3_resource_opt) =
                get_workspace_s3_resource(&authed, &db, Some(user_db), &token, &w_id).await?;
            s3_resource_opt
        }
    };
    let s3_resource = s3_resource_opt.ok_or(Error::NotFound(
        "No datasets storage resource defined at the workspace level".to_string(),
    ))?;
    return Ok(Json(s3_resource));
}

#[derive(Serialize, Deserialize, Clone)]
struct WindmillLargeFile {
    s3: String,
}

async fn test_connection(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
) -> JsonResult<()> {
    let (_, s3_resource_opt) = get_workspace_s3_resource(&authed, &db, None, &token, &w_id).await?;
    if s3_resource_opt.is_none() {
        return Err(Error::NotFound(
            "No datasets storage resource defined at the workspace level".to_string(),
        ));
    }

    let s3_resource = s3_resource_opt.ok_or(Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_object_store_client(&s3_resource)?;
    s3_client
        .list(None)
        .next()
        .await
        .transpose()
        .map_err(|err| {
            tracing::error!("Error testing connection to S3 bucket: {:?}", err);
            Error::InternalErr(err.to_string())
        })?;
    return Ok(Json(()));
}

#[derive(Deserialize)]
struct ListStoredFilesQuery {
    pub max_keys: i32,
    pub marker: Option<String>,
}

#[derive(Serialize)]
struct ListStoredDatasetsResponse {
    pub restricted_access: Option<bool>,
    windmill_large_files: Vec<WindmillLargeFile>,
    pub next_marker: Option<String>,
}

async fn list_stored_files(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<ListStoredFilesQuery>,
) -> JsonResult<ListStoredDatasetsResponse> {
    let (public_resource, s3_resource_opt) =
        get_workspace_s3_resource(&authed, &db, Some(user_db), &token, &w_id).await?;
    if !public_resource.unwrap_or(false) && s3_resource_opt.is_none() {
        return Ok(Json(ListStoredDatasetsResponse {
            windmill_large_files: vec![],
            next_marker: None,
            restricted_access: Some(true),
        }));
    }

    let s3_resource = s3_resource_opt.ok_or(Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_object_store_client(&s3_resource)?;

    let object_stream = s3_client.list(None);

    let query_marker_int = query
        .marker
        .or(Some("0".to_string()))
        .map(|marker_str| marker_str.parse::<usize>().ok())
        .flatten()
        .unwrap_or(0);
    let object_stream_skipped = object_stream.skip(query_marker_int);

    let stored_datasets = object_stream_skipped
        .take((query.max_keys + 1) as usize) // taking +1 here to check if there's more in the bucket and return the next_marker if needed
        .filter(|obj| future::ready(obj.is_ok()))
        .map(|obj| obj.ok().unwrap())
        .map(|obj| WindmillLargeFile { s3: obj.location.to_string() })
        .collect::<Vec<WindmillLargeFile>>()
        .await;

    let next_marker = if stored_datasets.len() > query.max_keys as usize {
        Some((query_marker_int + query.max_keys as usize).to_string())
    } else {
        None
    };

    // TODO: ideally do it on the size hint, if reliable
    #[cfg(not(feature = "enterprise"))]
    if stored_datasets.len() > 20 {
        return Err(Error::ExecutionErr(
            "The workspace s3 bucket contains more than 20 files. Consider upgrading to Windmill Enterprise Edition to continue to use this feature."
                .to_string(),
        ));
    }

    return Ok(Json(ListStoredDatasetsResponse {
        windmill_large_files: stored_datasets,
        next_marker,
        restricted_access: Some(false),
    }));
}

#[derive(Deserialize)]
struct LoadFileMetadataQuery {
    pub file_key: String,
}

#[derive(Serialize)]
struct LoadFileMetadataResponse {
    pub mime_type: Option<String>,
    pub size_in_bytes: Option<i64>,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    pub expires: Option<chrono::DateTime<chrono::Utc>>,
    pub version_id: Option<String>,
}

#[derive(Deserialize)]
struct LoadFilePreviewQuery {
    pub file_key: String,

    // The two options below are requested from s3 with an additional query is not set
    pub file_size_in_bytes: Option<usize>,
    pub file_mime_type: Option<String>,

    // For CSV files, the separator needs to be specified
    pub csv_separator: Option<String>, // defaults to ','
    pub csv_has_header: Option<bool>,  // defaults to true

    // Specify the content length to be read. Both will be taken into account when reading files, except for:
    // - CSVs: only the length will be taken into account, a CSV file larger than this will be truncated.
    //   Note that truncated CSV files might not be valid CSV files anymore, and therefore the preview might fail
    // - Parquet files: Parquet files are lazy-loaded. Therefore none of those params will be taken into account
    pub read_bytes_from: usize,
    pub read_bytes_length: usize,
}

#[derive(Serialize)]
struct LoadFilePreviewResponse {
    pub content: Option<String>,
    pub content_type: WindmillContentType,
    pub msg: Option<String>,
}

#[derive(Serialize)]
enum WindmillContentType {
    RawText,
    Csv,
    Parquet,
    Unknown,
}

async fn load_file_metadata(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<LoadFileMetadataQuery>,
) -> JsonResult<LoadFileMetadataResponse> {
    let file_key = query.file_key.clone();
    let (_, s3_resource_opt) = get_workspace_s3_resource(&authed, &db, None, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.ok_or(Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_object_store_client(&s3_resource)?;

    let path = object_store::path::Path::from(file_key.as_str());
    let s3_object_metadata = s3_client
        .head(&path)
        .await
        .map_err(|err| Error::InternalErr(err.to_string()))?;

    let response = LoadFileMetadataResponse {
        mime_type: None,
        size_in_bytes: Some(s3_object_metadata.size as i64),
        last_modified: Some(s3_object_metadata.last_modified.to_utc()),
        expires: None,
        version_id: s3_object_metadata.version,
    };
    return Ok(Json(response));
}

#[derive(Deserialize)]
struct LoadParquetQuery {
    limit: Option<u32>,
    offset: Option<i64>,
    sort_col: Option<String>,
    sort_desc: Option<bool>,
    search_col: Option<String>,
    search_term: Option<String>,
}

async fn load_parquet_preview(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path((w_id, file_key)): Path<(String, String)>,
    Query(query): Query<LoadParquetQuery>,
) -> JsonResult<Box<RawValue>> {
    let (_, s3_resource_opt) = get_workspace_s3_resource(&authed, &db, None, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.ok_or(Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;

    return read_s3_parquet_chunk(
        &s3_resource,
        &file_key,
        query.limit,
        query.offset,
        query
            .sort_col
            .map(|v| (v.to_string(), query.sort_desc.unwrap_or(false))),
        query
            .search_col
            .map(|v| (v.to_string(), query.search_term.unwrap_or_default())),
    )
    .await
    .map(Json);
}

async fn load_file_preview(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<LoadFilePreviewQuery>,
) -> JsonResult<LoadFilePreviewResponse> {
    // query validation
    if query.read_bytes_length > 8 * 1024 * 1024 {
        return Err(Error::BadRequest(
            "Cannot load file bigger than 8MB".to_string(),
        ));
    }

    let file_key = query.file_key.clone();
    let (_, s3_resource_opt) = get_workspace_s3_resource(&authed, &db, None, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.ok_or(Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_object_store_client(&s3_resource)?;

    // if content length is provided in the request, use it, otherwise get it from s3
    let s3_object_content_length =
        if query.file_size_in_bytes.is_some() || query.file_mime_type.is_some() {
            query.file_size_in_bytes
        } else {
            let path = object_store::path::Path::from(file_key.clone());
            let s3_object_metadata = s3_client
                .head(&path)
                .await
                .map_err(|err| Error::InternalErr(err.to_string()))?;
            Some(s3_object_metadata.size)
        };

    let file_chunk_length = if s3_object_content_length.is_some() {
        cmp::min(
            query.read_bytes_length,
            s3_object_content_length.unwrap() - query.read_bytes_from,
        )
    } else {
        query.read_bytes_length
    };
    let content_type: WindmillContentType;

    let lowercased_file_key = file_key.clone().to_lowercase();
    let content_preview =
        if lowercased_file_key.ends_with(".csv") || lowercased_file_key.ends_with("tsv") {
            content_type = WindmillContentType::Csv;
            csv_file_preview_with_fallback(
                s3_client,
                &file_key,
                file_chunk_length,
                query.csv_separator,
                query.csv_has_header,
            )
            .await
        } else if lowercased_file_key.ends_with(".json")
            || lowercased_file_key.ends_with(".yaml")
            || lowercased_file_key.ends_with(".yml")
            || lowercased_file_key.ends_with(".xml")
            || lowercased_file_key.ends_with(".txt")
            || lowercased_file_key.ends_with(".log")
        {
            content_type = WindmillContentType::RawText;
            read_s3_text_object_head(
                s3_client,
                &file_key,
                query.read_bytes_from,
                file_chunk_length,
            )
            .await
        } else if lowercased_file_key.ends_with(".parquet") {
            content_type = WindmillContentType::Parquet;
            read_s3_parquet_object_head(&s3_resource, &file_key).await
        } else {
            content_type = WindmillContentType::Unknown;
            Err(Error::ExecutionErr(
                "Preview is not available. Content type is unknown or not supported".to_string(),
            ))
        };
    let response: LoadFilePreviewResponse = match content_preview {
        Ok(content) => LoadFilePreviewResponse { content_type, content: Some(content), msg: None },

        Err(err) => {
            LoadFilePreviewResponse { content_type, content: None, msg: Some(err.to_string()) }
        }
    };
    return Ok(Json(response));
}

#[derive(Deserialize)]
struct DeleteS3FileQuery {
    pub file_key: String,
}

async fn delete_s3_file(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<DeleteS3FileQuery>,
) -> JsonResult<()> {
    let file_key = query.file_key.clone();
    let (_, s3_resource_opt) = get_workspace_s3_resource(&authed, &db, None, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.ok_or(Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_object_store_client(&s3_resource)?;

    let path = object_store::path::Path::from(file_key.as_str());

    s3_client.delete(&path).await.map_err(|err| {
        tracing::error!("{:?}", err);
        Error::InternalErr(err.to_string())
    })?;
    return Ok(Json(()));
}

#[derive(Deserialize)]
struct MoveS3FileQuery {
    pub src_file_key: String,
    pub dest_file_key: String,
}

async fn move_s3_file(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<MoveS3FileQuery>,
) -> JsonResult<()> {
    let (_, s3_resource_opt) = get_workspace_s3_resource(&authed, &db, None, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.ok_or(Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_object_store_client(&s3_resource)?;

    let source_path = object_store::path::Path::from(query.src_file_key);
    let dest_path = object_store::path::Path::from(query.dest_file_key);

    s3_client
        .copy(&source_path, &dest_path)
        .await
        .map_err(|err| {
            tracing::error!("{:?}", err);
            Error::InternalErr(err.to_string())
        })?;
    return Ok(Json(()));
}

#[derive(Deserialize)]
struct DownloadFileQuery {
    pub file_key: String,
    pub s3_resource_path: Option<String>,
}

async fn download_s3_file(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<DownloadFileQuery>,
) -> error::Result<Response> {
    let s3_resource_opt = match query.s3_resource_path.clone() {
        Some(s3_resource_path) => {
            get_s3_resource(
                &authed,
                &db,
                Some(user_db),
                &token,
                &w_id,
                s3_resource_path.as_str(),
            )
            .await?
        }
        None => {
            let (_, s3_resource_opt) =
                get_workspace_s3_resource(&authed, &db, None, &token, &w_id).await?;
            s3_resource_opt
        }
    };

    let s3_resource = s3_resource_opt.ok_or(Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_object_store_client(&s3_resource)?;

    let path = object_store::path::Path::from(query.file_key);
    let s3_object = s3_client.get(&path).await.map_err(|err| {
        tracing::warn!("Error fetching text file from S3: {:?}", err);
        Error::InternalErr(err.to_string())
    })?;
    let body_stream = StreamBody::new(s3_object.into_stream());
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/octet-stream".parse().unwrap());
    return Ok((StatusCode::OK, headers, body_stream).into_response());
}

#[derive(Deserialize)]
struct UploadFileQuery {
    pub file_key: Option<String>, // if none, the file will be placed in windmill_uploads/ with a random name.
    pub file_extension: Option<String>, // preferred extension for the file in case a random name has to be generated
    pub s3_resource_path: Option<String>, // custom S3 resource to use for this upload. It None, the workspace S3 resource will be used
}

#[derive(Deserialize, Serialize, Clone)]
struct UploadFilePart {
    pub part_number: u16,
    pub tag: String,
}

#[derive(Serialize)]
struct UploadFileResponse {
    pub file_key: String,
}

use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use pin_project::pin_project;
use tokio::io::{AsyncRead, ReadBuf};
use tokio::time::{interval, Interval};

#[pin_project]
pub struct ProgressReadAdapter<R: AsyncRead> {
    #[pin]
    inner: R,
    interval: Interval,
    interval_bytes: usize,
}

impl<R: AsyncRead> ProgressReadAdapter<R> {
    pub fn new(inner: R) -> Self {
        Self { inner, interval: interval(Duration::from_millis(100)), interval_bytes: 0 }
    }
}

impl<R: AsyncRead> AsyncRead for ProgressReadAdapter<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        let this = self.project();

        let before = buf.filled().len();
        let result = this.inner.poll_read(cx, buf);
        let after = buf.filled().len();

        *this.interval_bytes += after - before;
        match this.interval.poll_tick(cx) {
            Poll::Pending => {}
            Poll::Ready(_) => {
                tracing::info!("read {} bytes for s3 upload", *this.interval_bytes * 10);
                *this.interval_bytes = 0;
            }
        };

        result
    }
}

async fn multipart_upload_s3_file(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<UploadFileQuery>,
    body: BodyStream,
) -> JsonResult<UploadFileResponse> {
    let file_key = match query.file_key.clone() {
        Some(fk) => fk,
        None => {
            // for now, we place all files into `windmill_uploads` folder with a random name
            // TODO: make the folder configurable via the workspace settings
            format!(
                "windmill_uploads/upload_{}_{}.{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis(),
                rand::random::<u16>(),
                query.file_extension.clone().unwrap_or("file".to_string())
            )
            .to_string()
        }
    };

    let s3_resource_opt = match query.s3_resource_path.clone() {
        Some(s3_resource_path) => {
            get_s3_resource(
                &authed,
                &db,
                Some(user_db),
                &token,
                &w_id,
                s3_resource_path.as_str(),
            )
            .await?
        }
        None => {
            let (_, s3_resource_opt) =
                get_workspace_s3_resource(&authed, &db, None, &token, &w_id).await?;
            s3_resource_opt
        }
    };

    let s3_resource = s3_resource_opt.ok_or(Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_object_store_client(&s3_resource)?;

    let body_with_io_error = body.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
    let body_reader = StreamReader::new(body_with_io_error);
    futures::pin_mut!(body_reader);

    let path = object_store::path::Path::from(file_key.clone());
    let (multipart_id, mut parts_writer) = s3_client.put_multipart(&path).await.map_err(|err| {
        tracing::error!("Error initializing multipart upload: {:?}", err);
        Error::InternalErr(err.to_string())
    })?;
    let mut progressed_body_reader = ProgressReadAdapter::new(&mut body_reader);

    copy(&mut progressed_body_reader, &mut parts_writer)
        .await
        .map_err(|err| {
            let _ = s3_client.abort_multipart(&path, &multipart_id);
            tracing::error!("Error forwarding stream to object writer: {:?}", err);
            Error::InternalErr(err.to_string())
        })?;

    parts_writer.flush().await.map_err(|err| {
        let _ = s3_client.abort_multipart(&path, &multipart_id);
        tracing::error!("Error flushing multipart writer: {:?}", err);
        Error::InternalErr(err.to_string())
    })?;
    parts_writer.shutdown().await.map_err(|err| {
        let _ = s3_client.abort_multipart(&path, &multipart_id);
        tracing::error!("Error closing multipart writer: {:?}", err);
        Error::InternalErr(err.to_string())
    })?;
    return Ok(Json(UploadFileResponse { file_key }));
}

#[derive(Deserialize)]
pub struct S3Object {
    pub s3: String,
}

async fn get_workspace_s3_resource<'c>(
    authed: &ApiAuthed,
    db: &DB,
    user_db: Option<UserDB>,
    token: &str,
    w_id: &str,
) -> error::Result<(Option<bool>, Option<S3Resource>)> {
    let raw_lfs_opt = sqlx::query_scalar!(
        "SELECT large_file_storage FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(db)
    .await?
    .flatten();

    if raw_lfs_opt.is_none() {
        return Ok((None, None));
    }

    let large_file_storage = serde_json::from_value::<LargeFileStorage>(
        raw_lfs_opt.unwrap(),
    )
    .map_err(|err| {
        tracing::error!(
            "Value stored in large_file_storage column is invalid and could not be deserialized: {}",
            err
        );
        Error::InternalErr(
            "Could not deserialize LargeFileStorage value found in database".to_string(),
        )
    })?;
    let s3_lfs = match large_file_storage {
        LargeFileStorage::S3Storage(s3_lfs) => s3_lfs,
    };

    // if the resource is declared public, we replace user_db with None such that the resource info will be
    // retrieved using `db` (and ACLs will be bypassed)
    let effective_user_db = if user_db.is_some() && s3_lfs.public_resource.unwrap_or(false) {
        None
    } else {
        user_db
    };

    let stripped_resource_path = match s3_lfs.s3_resource_path.strip_prefix("$res:") {
        Some(stripped) => stripped,
        None => s3_lfs.s3_resource_path.as_str(),
    };
    let s3_resource = match get_s3_resource(
        authed,
        db,
        effective_user_db,
        token,
        w_id,
        stripped_resource_path,
    )
    .await
    {
        Ok(s3_resource) => Ok(s3_resource),
        Err(Error::NotAuthorized(_)) if !s3_lfs.public_resource.unwrap_or(false) => Ok(None),
        Err(err) => Err(err),
    };
    return s3_resource.map(|res| (s3_lfs.public_resource, res));
}

async fn get_s3_resource<'c>(
    authed: &ApiAuthed,
    db: &DB,
    user_db: Option<UserDB>,
    token: &str,
    w_id: &str,
    s3_resource_path: &str,
) -> error::Result<Option<S3Resource>> {
    let s3_resource_value_raw = get_resource_value_interpolated_internal(
        authed,
        user_db,
        db,
        w_id,
        s3_resource_path,
        None,
        token,
    )
    .await?;

    if s3_resource_value_raw.is_none() {
        return Err(Error::NotFound("Resource not found".to_string()));
    }

    let s3_resource = serde_json::from_value::<S3Resource>(s3_resource_value_raw.unwrap())
        .map_err(|err| Error::InternalErr(err.to_string()))?;
    return Ok(Some(s3_resource));
}

fn build_polars_s3_config(s3_resource_ref: &S3Resource) -> CloudOptions {
    let s3_resource = s3_resource_ref.to_owned();
    let mut s3_configs: Vec<(AmazonS3ConfigKey, String)> = vec![
        (AmazonS3ConfigKey::Region, s3_resource.region),
        (AmazonS3ConfigKey::Bucket, s3_resource.bucket),
        (
            AmazonS3ConfigKey::Endpoint,
            render_endpoint(s3_resource_ref),
        ),
        (
            AmazonS3ConfigKey::Client(ClientConfigKey::AllowHttp),
            (!s3_resource.use_ssl).to_string(),
        ),
        (
            AmazonS3ConfigKey::VirtualHostedStyleRequest,
            (!s3_resource.path_style).to_string(),
        ),
    ];
    if let Some(access_key) = s3_resource.access_key {
        s3_configs.push((AmazonS3ConfigKey::AccessKeyId, access_key));
    }
    if let Some(secret_key) = s3_resource.secret_key {
        s3_configs.push((AmazonS3ConfigKey::SecretAccessKey, secret_key));
    }
    return CloudOptions::default().with_aws(s3_configs);
}

async fn read_object_chunk(
    s3_client: Arc<dyn ObjectStore>,
    file_key: &str,
    from_byte: usize,
    length: usize,
) -> error::Result<Vec<u8>> {
    let path = object_store::path::Path::from(file_key);
    let s3_object = s3_client
        .get_range(&path, Range { start: from_byte, end: from_byte + length })
        .await
        .map_err(|err| {
            tracing::warn!("Error fetching text file from S3: {:?}", err);
            Error::InternalErr(err.to_string())
        })?;

    return Ok(s3_object.to_vec());
}

async fn read_s3_text_object_head(
    s3_client: Arc<dyn ObjectStore>,
    file_key: &str,
    from_byte: usize,
    length: usize,
) -> error::Result<String> {
    let payload = read_object_chunk(s3_client, file_key, from_byte, length).await?;
    let file_header_str = String::from_utf8(payload).map_err(|err| {
        tracing::warn!(
            "Encoding of file {} unsupported. Error was: {:?}",
            file_key,
            err
        );
        Error::InternalErr("File encoding is not supported".to_string())
    })?;
    return Ok(file_header_str);
}

async fn read_s3_parquet_object_head(
    s3_resource_ref: &S3Resource,
    file_key: &str,
) -> error::Result<String> {
    let s3_cloud_config = build_polars_s3_config(s3_resource_ref);

    let args: ScanArgsParquet = ScanArgsParquet {
        n_rows: Some(1),
        cache: false,
        parallel: polars::io::parquet::ParallelStrategy::Auto,
        rechunk: false,
        row_count: None,
        low_memory: false,
        use_statistics: false,
        hive_partitioning: false,
        cloud_options: Some(s3_cloud_config),
    };

    let file_key_clone = file_key.to_string();
    let s3_bucket_clone = s3_resource_ref.bucket.to_string();
    let polars_df_result = tokio::task::spawn_blocking(move || {
        let s3_file_key = format!("s3://{}/{}", s3_bucket_clone, file_key_clone);
        let lzdf_result = LazyFrame::scan_parquet(s3_file_key, args);
        match lzdf_result {
            Err(err) => {
                tracing::warn!("Error fetching parquet file from S3: {:?}", err);
                return Err(Error::InternalErr(err.to_string()));
            }
            Ok(lzdf) => {
                let df = lzdf
                    .select(&[col("*")])
                    .limit(10) // for now read only first 10 lines
                    .collect()
                    .map_err(|err| Error::InternalErr(err.to_string()))?;
                return Ok(format!("{:?}", df).to_string());
            }
        }
    })
    .await
    .map_err(|err| Error::InternalErr(err.to_string()))?;

    return polars_df_result;
}

async fn read_s3_parquet_chunk(
    s3_resource_ref: &S3Resource,
    file_key: &str,
    limit: Option<u32>,
    offset: Option<i64>,
    sort: Option<(String, bool)>,
    search: Option<(String, String)>,
) -> error::Result<Box<RawValue>> {
    let s3_cloud_config = build_polars_s3_config(s3_resource_ref);

    let args: ScanArgsParquet = ScanArgsParquet {
        n_rows: None,
        cache: false,
        parallel: polars::io::parquet::ParallelStrategy::Auto,
        rechunk: false,
        row_count: None,
        low_memory: false,
        use_statistics: false,
        hive_partitioning: false,
        cloud_options: Some(s3_cloud_config),
    };

    let file_key_clone = file_key.to_string();
    let s3_bucket_clone = s3_resource_ref.bucket.to_string();
    return tokio::task::spawn_blocking(move || {
        let s3_file_key = format!("s3://{}/{}", s3_bucket_clone, file_key_clone);
        let lzdf_result = LazyFrame::scan_parquet(s3_file_key, args);
        match lzdf_result {
            Err(err) => {
                tracing::warn!("Error fetching parquet file from S3: {:?}", err);
                return Err(Error::InternalErr(err.to_string()));
            }
            Ok(lzdf) => {
                let df = lzdf
                    .select(&[col("*")])
                    .slice(offset.unwrap_or(0), limit.unwrap_or(100));

                let df = if let Some(sort) = sort {
                    df.sort(
                        &sort.0,
                        SortOptions {
                            descending: sort.1,
                            nulls_last: false,
                            multithreaded: false,
                            maintain_order: false,
                        },
                    )
                } else {
                    df
                };
                use polars::prelude::*;
                let df = if let Some(search) = search {
                    df.filter(col(&search.0).str().contains_literal(lit(search.1.clone())))
                } else {
                    df
                };
                let df = df
                    .collect()
                    .map_err(|err| Error::InternalErr(err.to_string()))?;
                return Ok(to_raw_value(&df));
            }
        }
    })
    .await
    .map_err(|err| Error::InternalErr(err.to_string()))?;
}
async fn csv_file_preview_with_fallback(
    s3_client: Arc<dyn ObjectStore>,
    file_key: &str,
    length: usize,
    separator: Option<String>,
    has_header: Option<bool>,
) -> error::Result<String> {
    match read_s3_csv_object_head(s3_client.clone(), &file_key, length, separator, has_header).await
    {
        Ok(csv_preview) => Ok(csv_preview),
        Err(_) => {
            // fallback to default text file preview is the CSV could not be parsed. It's a text file after all
            let raw_text =
                read_s3_text_object_head(s3_client.clone(), &file_key, 0, length).await?;
            return Ok(raw_text);
        }
    }
}

async fn read_s3_csv_object_head(
    s3_client: Arc<dyn ObjectStore>,
    file_key: &str,
    length: usize,
    separator: Option<String>,
    has_header: Option<bool>,
) -> error::Result<String> {
    let separator_final = match separator {
        Some(separator_char) if separator_char == "\\t" => Ok("\t".as_bytes()[0]),
        Some(separator_char) if separator_char.len() != 1 => Err(Error::BadRequest(
            "Separator must be a single character".to_string(),
        )),
        Some(separator_char) => Ok(separator_char.as_bytes()[0]),
        None => Ok(",".as_bytes()[0]), // polars uses the comma as default, doing the same here
    }?;

    let path = object_store::path::Path::from(file_key);
    let s3_object = s3_client
        .get(&path)
        .await
        .map_err(|err| Error::InternalErr(err.to_string()))?
        .into_stream();

    // TODO: polars does not seem to support lazy csv reader, unfortunately. We can implement it ourselves if needed
    // Right now it's fine b/c we limit the download from AWS to 32MB. We should recommend users to use parquet
    // for larger files
    let file_content_bytes = s3_object
        .take(length as usize)
        .filter(|obj| future::ready(obj.is_ok()))
        .map(|obj| obj.unwrap())
        .collect::<Vec<_>>()
        .await;

    let cursor = std::io::Cursor::new(file_content_bytes.concat());

    let csv_df = CsvReader::new(cursor)
        .with_n_rows(Some(10)) // for now read only first 10 lines
        .with_separator(separator_final)
        .has_header(has_header.unwrap_or(true))
        .finish()
        .map_err(|err| Error::InternalErr(err.to_string()))?;

    return Ok(format!("{:?}", csv_df).to_string());
}
