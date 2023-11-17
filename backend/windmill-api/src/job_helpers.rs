use std::cmp;

use crate::{resources::transform_json_value, users::Tokened, workspaces::LargeFileStorage};
use aws_sdk_s3::config::{Credentials, Region};
use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use hyper::http;
use object_store::ClientConfigKey;
use polars::{
    io::{
        cloud::{AmazonS3ConfigKey, CloudOptions},
        SerReader,
    },
    lazy::{
        dsl::col,
        frame::{LazyCsvReader, LazyFrame, ScanArgsParquet},
    },
    prelude::CsvReader,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use windmill_common::{db::UserDB, error};

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
            "/polars_connection_settings",
            post(polars_connection_settings).layer(cors.clone()),
        )
        .route("/test_connection", get(test_connection).layer(cors.clone()))
        .route(
            "/list_stored_datasets",
            get(list_stored_datasets).layer(cors.clone()),
        )
        .route(
            "/load_file_preview",
            get(load_file_preview).layer(cors.clone()),
        )
}

#[derive(Debug, Deserialize, Clone)]
struct S3Resource {
    #[serde(rename = "bucket")]
    bucket: String,
    region: String,
    #[serde(rename = "endPoint")]
    endpoint: String,
    #[serde(rename = "useSSL")]
    use_ssl: bool,
    #[serde(rename = "accessKey")]
    access_key: Option<String>,
    #[serde(rename = "secretKey")]
    secret_key: Option<String>,
    #[serde(rename = "pathStyle")]
    path_style: bool,
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
struct PolarsConnectionSettingsQuery {
    s3_resource: S3Resource,
}

#[derive(Serialize)]
struct PolarsConnectionSettingsResponse {
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
struct PolarsConnectionSettings {
    pub region_name: String,
}

async fn polars_connection_settings(
    Path(_w_id): Path<String>,
    Json(query): Json<PolarsConnectionSettingsQuery>,
) -> error::JsonResult<PolarsConnectionSettingsResponse> {
    let s3_resource = query.s3_resource;

    let response = PolarsConnectionSettingsResponse {
        endpoint_url: s3_resource.endpoint,
        key: s3_resource.access_key,
        secret: s3_resource.secret_key,
        use_ssl: s3_resource.use_ssl,
        cache_regions: false,
        client_kwargs: PolarsConnectionSettings { region_name: s3_resource.region },
    };

    return Ok(Json(response));
}

#[derive(Serialize)]
struct ListStoredDatasetsResponse {
    windmill_large_files: Vec<WindmillLargeFile>,
}

#[derive(Serialize, Clone)]
struct WindmillLargeFile {
    s3: String,
}

async fn test_connection(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
) -> error::JsonResult<()> {
    let s3_resource_opt = get_workspace_s3_resource(&authed, &user_db, &token, &w_id).await?;
    if s3_resource_opt.is_none() {
        return Err(error::Error::NotFound(
            "No datasets storage resource defined at the workspace level".to_string(),
        ));
    }

    let s3_resource = s3_resource_opt.unwrap();
    let s3_client = build_s3_client(&s3_resource);
    s3_client
        .list_objects()
        .bucket(s3_resource.bucket)
        .max_keys(1)
        .send()
        .await
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;
    return Ok(Json(()));
}

async fn list_stored_datasets(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
) -> error::JsonResult<ListStoredDatasetsResponse> {
    let s3_resource_opt = get_workspace_s3_resource(&authed, &user_db, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.unwrap();
    let s3_client = build_s3_client(&s3_resource);

    let mut stored_datasets = Vec::<WindmillLargeFile>::new();
    let s3_bucket = s3_resource.bucket;
    loop {
        let mut list_object_query = s3_client
            .list_objects()
            .bucket(s3_bucket.clone())
            .max_keys(32);

        if let Some(last_dataset_name) = stored_datasets.last() {
            // if stored_datasets already has some elements, it means we're looping through pages
            // according to AWS SDK docs, we can set the marker to the last returned dataset
            list_object_query = list_object_query.set_marker(Some(last_dataset_name.clone().s3))
        }

        let bucket_objects = list_object_query
            .send()
            .await
            .map_err(|err| error::Error::InternalErr(err.to_string()))?;

        let page_datasets = bucket_objects
            .contents()
            .iter()
            .filter(|object| object.key().is_some())
            .map(|object| object.key())
            .map(Option::unwrap)
            .map(&str::to_string)
            .map(|object_key| WindmillLargeFile { s3: object_key.clone() })
            .collect::<Vec<WindmillLargeFile>>();

        stored_datasets.extend(page_datasets.clone());

        if !bucket_objects.is_truncated() {
            break;
        }
    }

    return Ok(Json(ListStoredDatasetsResponse {
        windmill_large_files: stored_datasets,
    }));
}

#[derive(Deserialize)]
pub struct LoadFilePreviewQuery {
    pub file_key: String,
    pub from: Option<usize>,
    pub length: Option<usize>,
    pub separator: Option<String>,
}

#[derive(Serialize)]
pub struct LoadFilePreviewResponse {
    pub mime_type: Option<String>,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    pub expires: Option<chrono::DateTime<chrono::Utc>>,
    pub version_id: Option<String>,
    pub content_preview: Option<String>,
}

async fn load_file_preview(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Tokened { token }: Tokened,
    Path(w_id): Path<String>,
    Query(query): Query<LoadFilePreviewQuery>,
) -> error::JsonResult<LoadFilePreviewResponse> {
    let file_key = query.file_key.clone();
    let s3_resource_opt = get_workspace_s3_resource(&authed, &user_db, &token, &w_id).await?;

    let s3_resource = s3_resource_opt.unwrap();
    let s3_client = build_s3_client(&s3_resource);

    let s3_bucket = s3_resource.bucket.clone();
    let s3_object = s3_client
        .get_object()
        .bucket(&s3_bucket)
        .key(&file_key)
        .range("bytes=0-0".to_string()) // get only the metadata here, download will happen later depending of the type of the file
        .send()
        .await
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;

    let object_content_type = s3_object.content_type();
    let content_preview = match object_content_type {
        Some("application/json") | Some("application/x-yaml") => Some(
            read_s3_text_object_head(
                &s3_client,
                &s3_bucket,
                &file_key,
                query.from.unwrap_or(0),
                query.length.unwrap_or(256),
            )
            .await?,
        ),
        Some("text/csv") => {
            Some(read_s3_csv_object_head(&s3_client, &s3_bucket, &file_key, query.separator).await?)
        }
        Some("application/octet-stream") => {
            if file_key.to_lowercase().ends_with(".parquet") {
                Some(read_s3_parquet_object_head(&s3_resource, &file_key).await?)
            } else {
                None
            }
        }
        _ => None,
    };

    let response = LoadFilePreviewResponse {
        mime_type: s3_object.content_type().map(&str::to_string),
        last_modified: s3_object
            .last_modified()
            .map(|dt| chrono::DateTime::from_timestamp(dt.secs(), dt.subsec_nanos()))
            .flatten(),
        expires: s3_object
            .expires()
            .map(|dt| chrono::DateTime::from_timestamp(dt.secs(), dt.subsec_nanos()))
            .flatten(),
        version_id: s3_object.version_id().map(&str::to_string),
        content_preview: content_preview,
    };

    return Ok(Json(response));
}

async fn get_workspace_s3_resource<'c>(
    authed: &ApiAuthed,
    user_db: &UserDB,
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

    let resource_path_json_value = serde_json::to_value(s3_lfs.s3_resource_path)
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;
    let interpolated_value = transform_json_value(
        authed,
        user_db,
        &w_id,
        resource_path_json_value,
        &Option::None,
        token,
    )
    .await?;

    let s3_resource = serde_json::from_value::<S3Resource>(interpolated_value)
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;
    return Ok(Some(s3_resource));
}

fn build_s3_client(s3_resource_ref: &S3Resource) -> aws_sdk_s3::Client {
    let s3_resource = s3_resource_ref.clone();

    let endpoint = render_endpoint(&s3_resource);
    let mut s3_config_builder = aws_sdk_s3::Config::builder()
        .endpoint_url(endpoint)
        .region(Region::new(s3_resource.region));
    if s3_resource.access_key.is_some() {
        s3_config_builder = s3_config_builder.credentials_provider(Credentials::new(
            s3_resource.access_key.unwrap_or_default(),
            s3_resource.secret_key.unwrap_or_default(),
            None,
            None,
            "s3_storage",
        ));
    }
    if s3_resource.path_style {
        s3_config_builder = s3_config_builder.force_path_style(true);
    }
    let s3_config = s3_config_builder.build();
    return aws_sdk_s3::Client::from_conf(s3_config);
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

fn render_endpoint(s3_resource: &S3Resource) -> String {
    if s3_resource.endpoint.starts_with("http://") || s3_resource.endpoint.starts_with("https://") {
        s3_resource.endpoint.clone()
    } else if s3_resource.use_ssl {
        format!("https://{}", s3_resource.endpoint)
    } else {
        format!("http://{}", s3_resource.endpoint)
    }
}

async fn read_s3_text_object_head(
    s3_client: &aws_sdk_s3::Client,
    s3_bucket: &str,
    file_key: &str,
    from_char: usize,
    length: usize,
) -> error::Result<String> {
    let mut payload: Vec<u8> = vec![];

    let mut start = from_char;
    let chunk_size = 128;
    let mut end = start + cmp::min(chunk_size, length);
    let mut number_of_chars_read = 0;
    loop {
        let s3_object = s3_client
            .get_object()
            .range(format!("bytes={}-{}", start, end).to_string())
            .bucket(s3_bucket)
            .key(file_key)
            .send()
            .await
            .map_err(|err| error::Error::InternalErr(err.to_string()))?;
        start = end + 1;
        end = end + cmp::min(chunk_size, length - number_of_chars_read);

        let chunk_bytes = s3_object
            .body
            .collect()
            .await
            .unwrap()
            .into_bytes()
            .to_vec();
        number_of_chars_read += chunk_bytes.len();
        payload.extend(chunk_bytes);
        if number_of_chars_read >= length {
            break;
        }
    }

    let file_header_str = String::from_utf8(payload).unwrap();
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

async fn read_s3_csv_object_head(
    s3_client: &aws_sdk_s3::Client,
    s3_bucket: &str,
    file_key: &str,
    separator: Option<String>,
) -> error::Result<String> {
    let separator_final = if let Some(separator_char) = separator {
        if separator_char.len() != 1 {
            return Err(error::Error::BadRequest(
                "Separator must be a single character".to_string(),
            ));
        }
        separator_char.as_bytes()[0]
    } else {
        ",".as_bytes()[0]
    };

    let s3_object = s3_client
        .get_object()
        .bucket(s3_bucket)
        .range("bytes=0-33554432".to_string()) // safeguard - do not load CSV files larger than 32MB
        .key(file_key)
        .send()
        .await
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;

    // TODO: polars does not seem to support lazy csv reader, unfortunately. We can implement it ourselves if needed
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
        .finish()
        .map_err(|err| error::Error::InternalErr(err.to_string()))?;

    return Ok(format!("{:?}", csv_df).to_string());
}
