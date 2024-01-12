use std::{cmp, time::Duration};

use crate::{db::DB, resources::get_resource_value_interpolated_internal, users::Tokened};
use anyhow::Context;
use aws_sdk_s3::{
    presigning::PresigningConfig,
    primitives::ByteStream,
    types::{CompletedMultipartUpload, CompletedPart},
};
use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use hyper::http;
use itertools::Itertools;
use object_store::ClientConfigKey;
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
use tower_http::cors::{Any, CorsLayer};
use windmill_common::{
    db::UserDB,
    error,
    s3_helpers::{build_s3_client, render_endpoint, LargeFileStorage, S3Resource},
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
            "/delete_s3_file",
            delete(delete_s3_file).layer(cors.clone()),
        )
        .route("/move_s3_file", get(move_s3_file).layer(cors.clone()))
        .route(
            "/multipart_upload_s3_file",
            post(multipart_upload_s3_file).layer(cors.clone()),
        )
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
) -> error::JsonResult<DuckdbConnectionSettingsResponse> {
    let mut duckdb_settings: String = String::new();

    let s3_resource = query.s3_resource;
    duckdb_settings.push_str(format!("SET home_directory='./';\n").as_str()); // TODO: make this configurable maybe, or point to a temporary folder
    duckdb_settings.push_str(format!("INSTALL 'httpfs';\n").as_str());
    if s3_resource.path_style {
        duckdb_settings.push_str(format!("SET s3_url_style='path';\n").as_str());
    }
    duckdb_settings.push_str(format!("SET s3_region='{}';\n", s3_resource.region).as_str());
    duckdb_settings.push_str(format!("SET s3_endpoint='{}';\n", s3_resource.endpoint).as_str());
    if !s3_resource.use_ssl {
        duckdb_settings.push_str(format!("SET s3_use_ssl=0;\n").as_str()); // default is true for DuckDB
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
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Json(query): Json<DuckdbConnectionSettingsQueryV2>,
) -> error::JsonResult<DuckdbConnectionSettingsResponse> {
    let s3_resource_opt = match query.s3_resource_path {
        Some(s3_resource_path) => {
            get_s3_resource(
                &authed,
                &user_db,
                &db,
                &token,
                &w_id,
                s3_resource_path.as_str(),
            )
            .await?
        }
        None => get_workspace_s3_resource(&authed, &user_db, &db, &token, &w_id).await?,
    };
    let s3_resource = s3_resource_opt.ok_or(error::Error::NotFound(
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
) -> error::JsonResult<S3fsArgs> {
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
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Json(query): Json<PolarsConnectionSettingsQueryV2>,
) -> error::JsonResult<PolarsConnectionSettingsResponse> {
    let s3_resource_opt = match query.s3_resource_path {
        Some(s3_resource_path) => {
            get_s3_resource(
                &authed,
                &user_db,
                &db,
                &token,
                &w_id,
                s3_resource_path.as_str(),
            )
            .await?
        }
        None => get_workspace_s3_resource(&authed, &user_db, &db, &token, &w_id).await?,
    };
    let s3_resource = s3_resource_opt.ok_or(error::Error::NotFound(
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
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Json(query): Json<S3ResourceInfoQuery>,
) -> error::JsonResult<S3Resource> {
    let s3_resource_opt = match query.s3_resource_path {
        Some(s3_resource_path) => {
            get_s3_resource(
                &authed,
                &user_db,
                &db,
                &token,
                &w_id,
                s3_resource_path.as_str(),
            )
            .await?
        }
        None => get_workspace_s3_resource(&authed, &user_db, &db, &token, &w_id).await?,
    };
    let s3_resource = s3_resource_opt.ok_or(error::Error::NotFound(
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
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
) -> error::JsonResult<()> {
    let s3_resource_opt = get_workspace_s3_resource(&authed, &user_db, &db, &token, &w_id).await?;
    if s3_resource_opt.is_none() {
        return Err(error::Error::NotFound(
            "No datasets storage resource defined at the workspace level".to_string(),
        ));
    }

    let s3_resource = s3_resource_opt.ok_or(error::Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_s3_client(&s3_resource);
    s3_client
        .list_objects()
        .bucket(s3_resource.bucket)
        .max_keys(1)
        .send()
        .await
        .map_err(|err| {
            tracing::error!("Error testing connection to S3 bucket: {:?}", err);
            error::Error::InternalErr(err.to_string())
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
    windmill_large_files: Vec<WindmillLargeFile>,
    pub next_marker: Option<String>,
}

async fn list_stored_files(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<ListStoredFilesQuery>,
) -> error::JsonResult<ListStoredDatasetsResponse> {
    let s3_resource_opt = get_workspace_s3_resource(&authed, &user_db, &db, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.ok_or(error::Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_s3_client(&s3_resource);

    let s3_bucket = s3_resource.bucket;

    let list_object_query = s3_client
        .list_objects()
        .bucket(s3_bucket.clone())
        .max_keys(query.max_keys)
        .set_marker(query.marker);

    let bucket_objects = list_object_query
        .send()
        .await
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;

    let stored_datasets = bucket_objects
        .contents()
        .iter()
        .filter(|object| object.key().is_some())
        .map(|object| object.key())
        .map(Option::unwrap)
        .map(&str::to_string)
        .map(|object_key| WindmillLargeFile { s3: object_key.clone() })
        .collect::<Vec<WindmillLargeFile>>();

    #[cfg(not(feature = "enterprise"))]
    if stored_datasets.len() > 20 {
        return Err(error::Error::ExecutionErr(
            "The workspace s3 bucket contains more than 20 files. Consider upgrading to Windmill Enterprise Edition to continue to use this feature."
                .to_string(),
        ));
    }

    let next_marker = if bucket_objects.is_truncated().unwrap_or(false) {
        if bucket_objects.next_marker().is_some() {
            // some S3 providers returns the next marker for us. If that's the case just re-use it
            bucket_objects.next_marker().map(|v| v.to_owned())
        } else {
            // others, like AWS, doesn't and implicitly expect users to return the last key of the current page
            stored_datasets.last().map(|v| v.s3.clone())
        }
    } else {
        None
    };

    return Ok(Json(ListStoredDatasetsResponse {
        windmill_large_files: stored_datasets,
        next_marker: next_marker,
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
    pub file_size_in_bytes: Option<i64>,
    pub file_mime_type: Option<String>,

    // For CSV files, the separator needs to be specified
    pub csv_separator: Option<String>, // defaults to ','
    pub csv_has_header: Option<bool>,  // defaults to true

    // Specify the content length to be read. Both will be taken into account when reading files, except for:
    // - CSVs: only the length will be taken into account, a CSV file larger than this will be truncated.
    //   Note that truncated CSV files might not be valid CSV files anymore, and therefore the preview might fail
    // - Parquet files: Parquet files are lazy-loaded. Therefore none of those params will be taken into account
    pub read_bytes_from: i64,
    pub read_bytes_length: i64,
}

#[derive(Serialize)]
struct LoadFilePreviewResponse {
    pub content: Option<String>,
    pub content_type: WindmillContentType,
    pub msg: Option<String>,
    pub download_url: Option<String>,
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
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<LoadFileMetadataQuery>,
) -> error::JsonResult<LoadFileMetadataResponse> {
    let file_key = query.file_key.clone();
    let s3_resource_opt = get_workspace_s3_resource(&authed, &user_db, &db, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.ok_or(error::Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_s3_client(&s3_resource);

    let s3_bucket = s3_resource.bucket.clone();
    let s3_object_metadata = s3_client
        .head_object()
        .bucket(&s3_bucket)
        .key(&file_key)
        .send()
        .await
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;

    let response = LoadFileMetadataResponse {
        mime_type: s3_object_metadata.content_type().map(&str::to_string),
        size_in_bytes: s3_object_metadata.content_length(),
        last_modified: s3_object_metadata
            .last_modified()
            .map(|dt| chrono::DateTime::from_timestamp(dt.secs(), dt.subsec_nanos()))
            .flatten(),
        expires: s3_object_metadata
            .expires()
            .map(|dt| chrono::DateTime::from_timestamp(dt.secs(), dt.subsec_nanos()))
            .flatten(),
        version_id: s3_object_metadata.version_id().map(&str::to_string),
    };
    return Ok(Json(response));
}

async fn load_file_preview(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<LoadFilePreviewQuery>,
) -> error::JsonResult<LoadFilePreviewResponse> {
    // query validation
    if query.read_bytes_length > 8 * 1024 * 1024 {
        return Err(error::Error::BadRequest(
            "Cannot load file bigger than 8MB".to_string(),
        ));
    }

    let file_key = query.file_key.clone();
    let s3_resource_opt = get_workspace_s3_resource(&authed, &user_db, &db, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.ok_or(error::Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_s3_client(&s3_resource);

    let s3_bucket = s3_resource.bucket.clone();

    // if content length is provided in the request, use it, otherwise get it from s3
    let (s3_object_mime_type, s3_object_content_length) =
        if query.file_size_in_bytes.is_some() || query.file_mime_type.is_some() {
            (query.file_mime_type.clone(), query.file_size_in_bytes)
        } else {
            let s3_object_metadata = s3_client
                .head_object()
                .bucket(&s3_bucket)
                .key(&file_key)
                .send()
                .await
                .map_err(|err| error::Error::InternalErr(err.to_string()))?;
            (
                s3_object_metadata.content_type().map(|v| v.to_owned()),
                s3_object_metadata.content_length(),
            )
        };

    // URL expires 30 minutes after its generation
    let presigned_config = PresigningConfig::expires_in(Duration::from_secs(60 * 30))
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;
    let download_url = s3_client
        .get_object()
        .bucket(&s3_bucket)
        .key(&file_key)
        .presigned(presigned_config)
        .await
        .map_err(|err| error::Error::InternalErr(err.to_string()))?
        .uri()
        .to_string();

    let file_chunk_length = if s3_object_content_length.is_some() {
        cmp::min(
            query.read_bytes_length,
            s3_object_content_length.unwrap() - query.read_bytes_from,
        )
    } else {
        query.read_bytes_length
    };
    let content_type: WindmillContentType;
    let content_preview = match s3_object_mime_type.as_deref() {
        Some("text/csv") => {
            content_type = WindmillContentType::Csv;
            csv_file_preview_with_fallback(
                &s3_client,
                &s3_bucket,
                &file_key,
                file_chunk_length,
                query.csv_separator,
                query.csv_has_header,
            )
            .await
        }
        Some(mt)
            if mt.starts_with("text/")
                || mt == "application/json"
                || mt == "application/x-yaml" =>
        {
            content_type = WindmillContentType::RawText;
            read_s3_text_object_head(
                &s3_client,
                &s3_bucket,
                &file_key,
                query.read_bytes_from,
                file_chunk_length,
            )
            .await
        }
        mt_opt => {
            let lowercased_file_key = file_key.to_lowercase();
            // sometimes S3 doesn't infer the content type on upload. Guess it from the file extension
            if lowercased_file_key.ends_with(".parquet") {
                content_type = WindmillContentType::Parquet;
                read_s3_parquet_object_head(&s3_resource, &file_key).await
            } else if lowercased_file_key.ends_with(".csv") {
                content_type = WindmillContentType::Csv;
                csv_file_preview_with_fallback(
                    &s3_client,
                    &s3_bucket,
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
                    &s3_client,
                    &s3_bucket,
                    &file_key,
                    query.read_bytes_from,
                    file_chunk_length,
                )
                .await
            } else {
                content_type = WindmillContentType::Unknown;
                let msg = match mt_opt {
                    Some(mt) => {
                        format!("Preview is not available for content of type '{}'", mt).to_string()
                    }
                    None => "Preview is not available. Content type is unknown or not supported"
                        .to_string(),
                };
                Err(error::Error::ExecutionErr(msg))
            }
        }
    };

    let response: LoadFilePreviewResponse = match content_preview {
        Ok(content) => LoadFilePreviewResponse {
            content_type: content_type,
            content: Some(content),
            msg: None,
            download_url: Some(download_url),
        },

        Err(err) => LoadFilePreviewResponse {
            content_type: content_type,
            content: None,
            msg: Some(err.to_string()),
            download_url: Some(download_url),
        },
    };

    return Ok(Json(response));
}

#[derive(Deserialize)]
struct DeleteS3FileQuery {
    pub file_key: String,
}

async fn delete_s3_file(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<DeleteS3FileQuery>,
) -> error::JsonResult<()> {
    let file_key = query.file_key.clone();
    let s3_resource_opt = get_workspace_s3_resource(&authed, &user_db, &db, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.ok_or(error::Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_s3_client(&s3_resource);

    let s3_bucket = s3_resource.bucket.clone();
    s3_client
        .delete_object()
        .bucket(&s3_bucket)
        .key(&file_key)
        .send()
        .await
        .map_err(|err| {
            tracing::error!("{:?}", err);
            error::Error::InternalErr(err.to_string())
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
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<MoveS3FileQuery>,
) -> error::JsonResult<()> {
    let s3_resource_opt = get_workspace_s3_resource(&authed, &user_db, &db, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.ok_or(error::Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let s3_client = build_s3_client(&s3_resource);

    let s3_bucket = s3_resource.bucket.clone();
    let source_uri = format!("{}/{}", s3_bucket, query.src_file_key);
    s3_client
        .copy_object()
        .copy_source(&source_uri)
        .bucket(&s3_bucket)
        .key(&query.dest_file_key)
        .send()
        .await
        .map_err(|err| {
            tracing::error!("{:?}", err);
            error::Error::InternalErr(err.to_string())
        })?;

    s3_client
        .delete_object()
        .bucket(&s3_bucket)
        .key(&query.src_file_key)
        .send()
        .await
        .map_err(|err| {
            tracing::error!("{:?}", err);
            error::Error::InternalErr(err.to_string())
        })?;
    return Ok(Json(()));
}

#[derive(Deserialize)]
struct UploadFileQuery {
    pub file_key: String,
    pub part_content: Vec<u8>,
    pub upload_id: Option<String>, // should be None for the first call to initiate the upload

    pub parts: Vec<UploadFilePart>, // parts already uploaded, with their part_number and the tag associated

    pub is_final: bool,      // whether it's the final chunk
    pub cancel_upload: bool, // whether the upload should be cancelled. upload_id should be set. subsequent calls with this upload_id will fail

    pub s3_resource_path: Option<String>, // custom S3 resource to use for this upload. It None, the workspace S3 resource will be used
}

#[derive(Deserialize, Serialize, Clone)]
struct UploadFilePart {
    pub part_number: u16,
    pub tag: String,
}

#[derive(Serialize)]
struct UploadFileResponse {
    pub upload_id: String,
    pub parts: Vec<UploadFilePart>, // parts already uploaded, with their part_number and the tag associated
    pub is_done: bool, // whether the transfer is finished, either b/c it got cancelled or because the last chunk was uploaded
}

async fn multipart_upload_s3_file(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Json(query): Json<UploadFileQuery>,
) -> error::JsonResult<UploadFileResponse> {
    tracing::debug!(
        "Multi part file upload: part number: {} - is_final: {}",
        query.parts.len(),
        query.is_final
    );
    let file_key = query.file_key.clone();

    let s3_resource_opt = match query.s3_resource_path.clone() {
        Some(s3_resource_path) => {
            get_s3_resource(
                &authed,
                &user_db,
                &db,
                &token,
                &w_id,
                s3_resource_path.as_str(),
            )
            .await?
        }
        None => get_workspace_s3_resource(&authed, &user_db, &db, &token, &w_id).await?,
    };

    let s3_resource = s3_resource_opt.ok_or(error::Error::InternalErr(
        "No files storage resource defined at the workspace level".to_string(),
    ))?;
    let bucket = s3_resource.bucket.clone();
    let s3_client = build_s3_client(&s3_resource);

    if query.cancel_upload && query.upload_id.clone().is_some() {
        let upload_id = query.upload_id.unwrap();
        s3_client
            .abort_multipart_upload()
            .bucket(&bucket)
            .key(&file_key)
            .upload_id(&upload_id)
            .send()
            .await
            .map_err(|err| {
                tracing::error!("{:?}", err);
                error::Error::InternalErr(err.to_string())
            })?;
        return Ok(Json(UploadFileResponse {
            upload_id: upload_id,
            parts: vec![], // empty parts as the transfer has been cancelled
            is_done: true,
        }));
    }

    let (upload_id, part_number) = match query {
        UploadFileQuery { upload_id: Some(upload_id), ref parts, .. } if parts.len() > 0 => {
            (upload_id, parts.len() + 1)
        }
        UploadFileQuery { upload_id: None, ref parts, .. } if parts.len() == 0 => {
            let multipart_upload_res = s3_client
                .create_multipart_upload()
                .bucket(&bucket)
                .key(&file_key)
                .send()
                .await
                .map_err(|err| {
                    tracing::error!("{:?}", err);
                    error::Error::InternalErr(err.to_string())
                })?;
            let upload_id = multipart_upload_res
                .upload_id
                .context("Upload ID is missing in the response")?;
            (upload_id, 1)
        }
        _ => {
            return Err(error::Error::BadRequest(
                "parts should be empty when upload_id is not provided, as a new upload will be created."
                    .to_string(),
            ))
        }
    };

    let chunk_content = query.part_content.clone();
    let chunk_content_stream = ByteStream::from(chunk_content);
    let multipart_upload_res = s3_client
        .upload_part()
        .bucket(&bucket)
        .key(&file_key)
        .upload_id(&upload_id)
        .body(chunk_content_stream)
        .part_number(part_number as i32)
        .send()
        .await
        .map_err(|err| {
            tracing::error!("{:?}", err);
            error::Error::InternalErr(err.to_string())
        })?;

    let mut new_parts = query.parts.clone();
    new_parts.push(UploadFilePart {
        part_number: part_number as u16,
        tag: multipart_upload_res.e_tag.unwrap_or_default(),
    });

    if query.is_final {
        let completed_parts = new_parts
            .iter()
            .map(|part| {
                CompletedPart::builder()
                    .e_tag(&part.tag)
                    .part_number(part.part_number as i32)
                    .build()
            })
            .collect_vec();
        let _complete_multipart_upload_res = s3_client
            .complete_multipart_upload()
            .bucket(&s3_resource.bucket)
            .key(&query.file_key)
            .upload_id(&upload_id)
            .multipart_upload(
                CompletedMultipartUpload::builder()
                    .set_parts(Some(completed_parts))
                    .build(),
            )
            .send()
            .await
            .map_err(|err| {
                tracing::error!("{:?}", err);
                error::Error::InternalErr(err.to_string())
            })?;
    }

    return Ok(Json(UploadFileResponse {
        upload_id: upload_id,
        parts: new_parts,
        is_done: query.is_final,
    }));
}

#[derive(Deserialize)]
pub struct S3Object {
    pub s3: String,
}

async fn get_workspace_s3_resource<'c>(
    authed: &ApiAuthed,
    user_db: &UserDB,
    db: &DB,
    token: &str,
    w_id: &str,
) -> error::Result<Option<S3Resource>> {
    let mut tx = user_db.clone().begin(authed).await?;
    let raw_lfs_opt = sqlx::query_scalar!(
        "SELECT large_file_storage FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();
    tx.commit().await?;

    if raw_lfs_opt.is_none() {
        return Ok(None);
    }

    let large_file_storage = serde_json::from_value::<LargeFileStorage>(
        raw_lfs_opt.unwrap(),
    )
    .map_err(|err| {
        tracing::error!(
            "Value stored in large_file_storage column is invalid and could not be deserialized: {}",
            err
        );
        error::Error::InternalErr(
            "Could not deserialize LargeFileStorage value found in database".to_string(),
        )
    })?;
    let s3_lfs = match large_file_storage {
        LargeFileStorage::S3Storage(s3_lfs) => s3_lfs,
    };

    let stripped_resource_path = match s3_lfs.s3_resource_path.strip_prefix("$res:") {
        Some(stripped) => stripped,
        None => s3_lfs.s3_resource_path.as_str(),
    };
    return get_s3_resource(authed, user_db, db, token, w_id, stripped_resource_path).await;
}

async fn get_s3_resource<'c>(
    authed: &ApiAuthed,
    user_db: &UserDB,
    db: &DB,
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
        return Err(error::Error::NotFound("Resource not found".to_string()));
    }

    let s3_resource = serde_json::from_value::<S3Resource>(s3_resource_value_raw.unwrap())
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;
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

async fn read_s3_text_object_head(
    s3_client: &aws_sdk_s3::Client,
    s3_bucket: &str,
    file_key: &str,
    from_char: i64,
    length: i64,
) -> error::Result<String> {
    let s3_object = s3_client
        .get_object()
        .range(format!("bytes={}-{}", from_char, length).to_string())
        .bucket(s3_bucket)
        .key(file_key)
        .send()
        .await
        .map_err(|err| {
            tracing::warn!("Error fetching text file from S3: {:?}", err);
            error::Error::InternalErr(err.to_string())
        })?;

    let payload = s3_object
        .body
        .collect()
        .await
        .map_err(|err| {
            tracing::warn!(
                "Error reading raw text file {}. Error was: {:?}",
                file_key,
                err
            );
            error::Error::InternalErr("File encoding is not supported".to_string())
        })?
        .into_bytes()
        .to_vec();

    let file_header_str = String::from_utf8(payload).map_err(|err| {
        tracing::warn!(
            "Encoding of file {} unsupported. Error was: {:?}",
            file_key,
            err
        );
        error::Error::InternalErr("File encoding is not supported".to_string())
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
                return Err(error::Error::InternalErr(err.to_string()));
            }
            Ok(lzdf) => {
                let df = lzdf
                    .select(&[col("*")])
                    .limit(10) // for now read only first 10 lines
                    .collect()
                    .map_err(|err| error::Error::InternalErr(err.to_string()))?;
                return Ok(format!("{:?}", df).to_string());
            }
        }
    })
    .await
    .map_err(|err| error::Error::InternalErr(err.to_string()))?;

    return polars_df_result;
}

async fn csv_file_preview_with_fallback(
    s3_client: &aws_sdk_s3::Client,
    s3_bucket: &str,
    file_key: &str,
    length: i64,
    separator: Option<String>,
    has_header: Option<bool>,
) -> error::Result<String> {
    match read_s3_csv_object_head(
        &s3_client, &s3_bucket, &file_key, length, separator, has_header,
    )
    .await
    {
        Ok(csv_preview) => Ok(csv_preview),
        Err(_) => {
            // fallback to default text file preview is the CSV could not be parsed. It's a text file after all
            let raw_text =
                read_s3_text_object_head(&s3_client, &s3_bucket, &file_key, 0, length).await?;
            return Ok(raw_text);
        }
    }
}

async fn read_s3_csv_object_head(
    s3_client: &aws_sdk_s3::Client,
    s3_bucket: &str,
    file_key: &str,
    length: i64,
    separator: Option<String>,
    has_header: Option<bool>,
) -> error::Result<String> {
    let separator_final = match separator {
        Some(separator_char) if separator_char == "\\t" => Ok("\t".as_bytes()[0]),
        Some(separator_char) if separator_char.len() != 1 => Err(error::Error::BadRequest(
            "Separator must be a single character".to_string(),
        )),
        Some(separator_char) => Ok(separator_char.as_bytes()[0]),
        None => Ok(",".as_bytes()[0]), // polars uses the comma as default, doing the same here
    }?;

    let s3_object = s3_client
        .get_object()
        .bucket(s3_bucket)
        .range(format!("bytes=0-{}", length).to_string())
        .key(file_key)
        .send()
        .await
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;

    // TODO: polars does not seem to support lazy csv reader, unfortunately. We can implement it ourselves if needed
    // Right now it's fine b/c we limit the download from AWS to 32MB. We should recomment users to use parquet
    // for larger files
    let file_content_bytes = s3_object
        .body
        .collect()
        .await
        .map_err(|err| error::Error::InternalErr(err.to_string()))?
        .into_bytes();
    let cursor = std::io::Cursor::new(file_content_bytes);

    let csv_df = CsvReader::new(cursor)
        .with_n_rows(Some(10)) // for now read only first 10 lines
        .with_separator(separator_final)
        .has_header(has_header.unwrap_or(true))
        .finish()
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;

    return Ok(format!("{:?}", csv_df).to_string());
}
