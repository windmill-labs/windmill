pub use windmill_types::s3::*;

use std::collections::HashMap;

use quick_cache::sync::Cache;
use windmill_common::error::{self};

#[cfg(feature = "parquet")]
use aws_config::{default_provider::credentials::DefaultCredentialsChain, Region};
#[cfg(feature = "parquet")]
use aws_sdk_sts::config::ProvideCredentials;
#[cfg(feature = "parquet")]
use axum::async_trait;
#[cfg(feature = "parquet")]
use bytes::Bytes;
#[cfg(feature = "parquet")]
use chrono::{DateTime, Utc};
#[cfg(feature = "parquet")]
use datafusion::arrow::array::{RecordBatch, RecordBatchWriter};
#[cfg(feature = "parquet")]
use datafusion::arrow::error::ArrowError;
#[cfg(feature = "parquet")]
use datafusion::arrow::json::writer::JsonArray;
#[cfg(feature = "parquet")]
use datafusion::arrow::{csv, json};
#[cfg(feature = "parquet")]
use datafusion::parquet::arrow::ArrowWriter;
#[cfg(feature = "parquet")]
use futures::TryStreamExt;
#[cfg(feature = "parquet")]
use object_store::aws::AwsCredential;
#[cfg(feature = "parquet")]
use object_store::azure::MicrosoftAzureBuilder;
#[cfg(feature = "parquet")]
use object_store::gcp::GoogleCloudStorageBuilder;
#[cfg(feature = "parquet")]
use object_store::CredentialProvider;
#[cfg(feature = "parquet")]
use object_store::ObjectStore;
#[cfg(feature = "parquet")]
use object_store::{aws::AmazonS3Builder, ClientOptions};
#[cfg(feature = "parquet")]
use reqwest::header::HeaderMap;
#[cfg(feature = "parquet")]
use std::io::Write;
#[cfg(feature = "parquet")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "parquet")]
use tokio::sync::RwLock;
#[cfg(feature = "parquet")]
use tokio::task;
#[cfg(feature = "parquet")]
use windmill_common::error::to_anyhow;
#[cfg(feature = "parquet")]
use windmill_common::utils::rd_string;
#[cfg(all(feature = "parquet", feature = "private"))]
pub mod job_s3_helpers_ee;
#[cfg(feature = "parquet")]
pub mod job_s3_helpers_oss;

// Re-export object_store types so consumers don't need a direct object_store dep
#[cfg(feature = "parquet")]
pub mod object_store_reexports {
    pub use object_store::local::LocalFileSystem;
    pub use object_store::memory::InMemory;
    pub use object_store::path::Path;
    pub use object_store::{
        Attribute, Attributes, Error as ObjectStoreError, GetResult, ObjectStore,
        PutMultipartOpts, PutPayload, PutResult, Result as ObjectStoreResult, WriteMultipart,
    };
}

#[cfg(feature = "parquet")]
pub fn object_store_error_to_error(err: object_store::Error) -> error::Error {
    use object_store::Error::*;
    match err {
        Generic { store, source } => error::Error::Generic(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Generic {} error: {}", store, source),
        ),
        NotFound { path, source } => error::Error::NotFound(format!("{}: {}", path, source)),
        InvalidPath { source } => error::Error::BadRequest(format!("Invalid path: {}", source)),
        JoinError { source } => error::Error::InternalErr(format!("Join error: {}", source)),
        NotSupported { source } => {
            error::Error::BadRequest(format!("Operation not supported: {}", source))
        }
        AlreadyExists { path, source } => {
            error::Error::BadRequest(format!("Object at {} already exists: {}", path, source))
        }
        Precondition { path, source } => {
            error::Error::BadRequest(format!("Precondition failed at {}: {}", path, source))
        }
        NotModified { path, source } => {
            error::Error::ExecutionErr(format!("Not modified at {}: {}", path, source))
        }
        NotImplemented => error::Error::BadRequest("Operation not yet implemented.".to_string()),
        PermissionDenied { path, source } => {
            error::Error::PermissionDenied(format!("Permission denied at {}: {}", path, source))
        }
        Unauthenticated { path, source } => {
            error::Error::NotAuthorized(format!("Unauthenticated for {}: {}", path, source))
        }
        UnknownConfigurationKey { store, key } => error::Error::BadConfig(format!(
            "Invalid config key '{}' for store '{}'",
            key, store
        )),
        _ => error::Error::InternalErr(format!("Object store error: {}", err)),
    }
}

// --- Object store builder infrastructure (moved from windmill-common/src/s3_helpers.rs) ---

#[cfg(feature = "parquet")]
#[derive(Clone)]
pub struct ExpirableObjectStore {
    pub store: Arc<dyn ObjectStore>,
    pub refresh: Option<ObjectStoreRefresh>,
}

#[cfg(feature = "parquet")]
#[derive(Clone)]
pub struct ObjectStoreRefresh {
    refresh: Option<DateTime<Utc>>,
    settings: ObjectSettings,
}

#[cfg(feature = "parquet")]
impl ObjectStoreRefresh {
    pub fn new(settings: ObjectSettings, refresh: Option<DateTime<Utc>>) -> Self {
        Self { settings, refresh }
    }
    fn refresh_needed(&self) -> bool {
        if let Some(refresh) = self.refresh {
            if refresh < Utc::now() - chrono::Duration::minutes(1) {
                return true;
            }
        }
        return false;
    }

    async fn refresh(&self) -> Option<ExpirableObjectStore> {
        return build_object_store_from_settings(self.settings.clone(), None)
            .await
            .map_err(|e| {
                tracing::error!("Error building s3 client from settings: {:?}", e);
                e
            })
            .ok();
    }
}

#[cfg(feature = "parquet")]
impl From<Arc<dyn ObjectStore>> for ExpirableObjectStore {
    fn from(store: Arc<dyn ObjectStore>) -> Self {
        Self { store, refresh: None }
    }
}

#[cfg(feature = "parquet")]
lazy_static::lazy_static! {
    pub static ref OBJECT_STORE_SETTINGS: Arc<RwLock<Option<ExpirableObjectStore>>> = Arc::new(RwLock::new(None));
}

#[cfg(feature = "parquet")]
pub async fn get_object_store() -> Option<Arc<dyn ObjectStore>> {
    let settings = OBJECT_STORE_SETTINGS.read().await;
    if let Some(s) = settings.as_ref() {
        match &s.refresh {
            Some(refresh) => {
                if refresh.refresh_needed() {
                    let refresh = refresh.clone();
                    drop(settings);
                    let new_store = refresh.refresh().await;
                    if let Some(new_store) = new_store {
                        let mut s3_cache_settings = OBJECT_STORE_SETTINGS.write().await;
                        let arc = new_store.store.clone();
                        *s3_cache_settings = Some(new_store);
                        return Some(arc);
                    } else {
                        return None;
                    }
                } else {
                    return Some(s.store.clone());
                }
            }
            None => {
                return Some(s.store.clone());
            }
        }
    } else {
        return None;
    }
}

#[cfg(feature = "parquet")]
pub enum ObjectStoreReload {
    Later,
    Never,
}

#[cfg(feature = "parquet")]
pub async fn reload_object_store_setting(db: &windmill_common::DB) -> ObjectStoreReload {
    use windmill_common::{
        ee_oss::{get_license_plan, LicensePlan},
        global_settings::{load_value_from_global_settings, OBJECT_STORE_CONFIG_SETTING},
    };

    let s3_config = load_value_from_global_settings(db, OBJECT_STORE_CONFIG_SETTING).await;
    if let Err(e) = s3_config {
        tracing::error!("Error reloading s3 cache config: {:?}", e)
    } else {
        if let Some(v) = s3_config.unwrap() {
            if matches!(get_license_plan().await, LicensePlan::Pro) {
                tracing::error!("S3 cache is not available for pro plan");
                return ObjectStoreReload::Never;
            }
            let setting = serde_json::from_value::<ObjectSettings>(v);
            match setting {
                Ok(setting) => {
                    let is_oidc = matches!(setting, ObjectSettings::AwsOidc(_));
                    let s3_client = build_object_store_from_settings(setting, Some(db)).await;
                    match s3_client {
                        Ok(s3_client) => {
                            let mut s3_cache_settings = OBJECT_STORE_SETTINGS.write().await;
                            *s3_cache_settings = Some(s3_client);
                        }
                        Err(e) => {
                            if is_oidc {
                                tracing::error!("Error building s3 client from oidc settings. It may be due to the jwks endpoints not being up yet, it will be attempted again in 10s to leave time for the server to be ready: {:?}", e);
                                return ObjectStoreReload::Later;
                            } else {
                                tracing::error!("Error building s3 client from settings: {:?}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error parsing s3 cache config: {:?}", e)
                }
            }
        } else {
            let mut s3_cache_settings = OBJECT_STORE_SETTINGS.write().await;
            if std::env::var("S3_CACHE_BUCKET").is_ok() {
                if matches!(get_license_plan().await, LicensePlan::Pro) {
                    tracing::error!("S3 cache is not available for pro plan");
                    return ObjectStoreReload::Never;
                }
                *s3_cache_settings = build_s3_client_from_settings(S3Settings {
                    bucket: None,
                    region: None,
                    access_key: None,
                    secret_key: None,
                    endpoint: None,
                    store_logs: None,
                    path_style: None,
                    allow_http: None,
                    port: None,
                })
                .await
                .ok()
                .map(|x| ExpirableObjectStore::from(x))
            } else {
                *s3_cache_settings = None;
            }
        }
    }
    return ObjectStoreReload::Never;
}

pub fn render_endpoint(
    raw_endpoint: String,
    use_ssl: bool,
    port: Option<u16>,
    path_style: Option<bool>,
    bucket: String,
) -> String {
    let url_with_prefix =
        if raw_endpoint.starts_with("http://") || raw_endpoint.starts_with("https://") {
            raw_endpoint.clone()
        } else {
            let scheme = if use_ssl { "https" } else { "http" };
            format!(
                "{}://{}",
                scheme,
                if path_style.unwrap_or(true) {
                    raw_endpoint
                } else {
                    format!("{}.{}", bucket, raw_endpoint)
                }
            )
        };
    if port.is_some() {
        format!("{}:{}", url_with_prefix, port.unwrap())
    } else {
        url_with_prefix
    }
}

#[cfg(feature = "parquet")]
pub async fn build_object_store_client(
    resource_ref: &ObjectStoreResource,
) -> error::Result<Arc<dyn ObjectStore>> {
    match resource_ref {
        ObjectStoreResource::S3(s3_resource_ref) => build_s3_client(&s3_resource_ref).await,
        ObjectStoreResource::Azure(azure_blob_resource_ref) => {
            build_azure_blob_client(&azure_blob_resource_ref)
        }
        ObjectStoreResource::Gcs(gcs_resource_ref) => build_gcs_client(&gcs_resource_ref).await,
        ObjectStoreResource::Filesystem(fs) => build_filesystem_client(&fs.root_path),
    }
}

#[cfg(feature = "parquet")]
pub async fn attempt_fetch_bytes(
    client: Arc<dyn ObjectStore>,
    path: &str,
) -> error::Result<bytes::Bytes> {
    use object_store::path::Path;

    let object = client.get(&Path::from(path)).await;
    if let Err(e) = object {
        tracing::info!(
            "Failed to pull bytes from object store at path {path}. Error: {:?}",
            e
        );
        return Err(error::Error::ExecutionErr(format!(
            "Failed to pull bytes from object store: {path}"
        )));
    }

    let bytes = object.unwrap().bytes().await;
    if bytes.is_err() {
        tracing::info!(
            "Failed to read bytes from object store: {path}. Error: {:?}",
            bytes.err()
        );
        return Err(error::Error::ExecutionErr(format!(
            "Failed to read bytes from object store: {path}"
        )));
    }
    let bytes = bytes.unwrap();

    tracing::info!("{path} len: {}", bytes.len());

    if bytes.len() == 0 {
        tracing::info!("object {path} not found in bucket, bytes empty",);
        return Err(error::Error::ExecutionErr(format!(
            "object {path} does not exist in bucket"
        )));
    }

    return Ok(bytes);
}

#[cfg(feature = "parquet")]
pub async fn build_s3_client(s3_resource_ref: &S3Resource) -> error::Result<Arc<dyn ObjectStore>> {
    let static_creds = s3_resource_ref.access_key.as_ref().is_some_and(|x| x != "")
        || s3_resource_ref.secret_key.as_ref().is_some_and(|x| x != "");

    let credentials_provider = if !static_creds {
        Some(
            DefaultCredentialsChain::builder()
                .region(Region::new(s3_resource_ref.region.clone()))
                .build()
                .await,
        )
    } else {
        None
    };

    let s3_resource = s3_resource_ref.clone();
    let endpoint = render_endpoint(
        s3_resource.endpoint_with_region_fallback(None),
        s3_resource.use_ssl,
        s3_resource.port,
        s3_resource.path_style,
        s3_resource.bucket.clone(),
    );
    let mut store_builder = AmazonS3Builder::new()
        .with_client_options(
            ClientOptions::new()
                .with_timeout_disabled()
                .with_default_headers(HeaderMap::from_iter(vec![(
                    "Accept-Encoding".parse().unwrap(),
                    "".parse().unwrap(),
                )])),
        )
        .with_region(s3_resource.region)
        .with_bucket_name(s3_resource.bucket)
        .with_endpoint(endpoint);

    if let Some(credentials_provider) = credentials_provider {
        store_builder = store_builder.with_credentials(Arc::new(AwsCredentialAdapter {
            inner: credentials_provider,
        }));
    }

    if !s3_resource.use_ssl {
        store_builder = store_builder.with_allow_http(true)
    }

    if let Some(key) = s3_resource.access_key {
        if key != "" {
            store_builder = store_builder.with_access_key_id(key);
        }
    }

    if let Some(token) = s3_resource.token {
        if token != "" {
            store_builder = store_builder.with_token(token);
        }
    }
    if let Some(secret_key) = s3_resource.secret_key {
        if secret_key != "" {
            store_builder = store_builder.with_secret_access_key(secret_key);
        }
    }
    if !s3_resource.path_style.unwrap_or(true) {
        store_builder = store_builder.with_virtual_hosted_style_request(true);
    }

    let store = store_builder.build().map_err(|err| {
        tracing::error!("Error building object store client: {:?}", err);
        error::Error::internal_err(format!("Error building object store client: {:?}", err))
    })?;

    return Ok(Arc::new(store));
}

#[cfg(feature = "parquet")]
fn build_azure_blob_client(
    azure_blob_resource_ref: &AzureBlobResource,
) -> error::Result<Arc<dyn ObjectStore>> {
    let blob_resource = azure_blob_resource_ref.clone();

    let mut store_builder = MicrosoftAzureBuilder::new()
        .with_client_options(
            ClientOptions::new()
                .with_timeout_disabled()
                .with_default_headers(HeaderMap::from_iter(vec![(
                    "Accept-Encoding".parse().unwrap(),
                    "".parse().unwrap(),
                )])),
        )
        .with_account(blob_resource.account_name)
        .with_container_name(blob_resource.container_name);

    if let Some(federated_token_file) = blob_resource.federated_token_file {
        if federated_token_file != "" {
            store_builder = store_builder.with_federated_token_file(federated_token_file);
        }
    }
    if let Some(tenant_id) = blob_resource.tenant_id {
        if tenant_id != "" {
            store_builder = store_builder.with_tenant_id(tenant_id);
        }
    }
    if let Some(client_id) = blob_resource.client_id {
        if client_id != "" {
            store_builder = store_builder.with_client_id(client_id);
        }
    }
    if let Some(endpoint) = blob_resource.endpoint {
        if endpoint != "" {
            let endpoint = render_endpoint(
                endpoint,
                blob_resource.use_ssl.unwrap_or(false),
                None,
                None,
                "".to_string(),
            );
            store_builder = store_builder.with_endpoint(endpoint)
        }
    }

    if !blob_resource.use_ssl.unwrap_or(false) {
        store_builder = store_builder.with_allow_http(true)
    }

    if let Some(key) = blob_resource.access_key {
        if key != "" {
            store_builder = store_builder.with_access_key(key);
        }
    }

    let store = store_builder.build().map_err(|err| {
        tracing::error!("Error building object store client: {:?}", err);
        error::Error::internal_err(format!("Error building object store client: {:?}", err))
    })?;

    return Ok(Arc::new(store));
}

#[cfg(feature = "parquet")]
async fn build_gcs_client(gcs_resource_ref: &GcsResource) -> error::Result<Arc<dyn ObjectStore>> {
    let gcs_resource = gcs_resource_ref.clone();

    let mut store_builder = GoogleCloudStorageBuilder::new()
        .with_client_options(
            ClientOptions::new()
                .with_timeout_disabled()
                .with_default_headers(HeaderMap::from_iter(vec![(
                    "Accept-Encoding".parse().unwrap(),
                    "".parse().unwrap(),
                )])),
        )
        .with_bucket_name(gcs_resource.bucket);

    store_builder = store_builder.with_service_account_key(gcs_resource.service_account_key);

    let store = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| store_builder.build()))
        .map_err(|panic_info| {
            tracing::error!(
                "Panic while building GCS object store client: {:?}",
                panic_info
            );
            error::Error::internal_err(format!(
                "Panic while building GCS object store client: {:?}",
                panic_info
            ))
        })?
        .map_err(|err| {
            tracing::error!("Error building GCS object store client: {:?}", err);
            error::Error::internal_err(format!("Error building GCS object store client: {:?}", err))
        })?;

    return Ok(Arc::new(store));
}

#[cfg(feature = "parquet")]
pub fn build_filesystem_client(root_path: &str) -> error::Result<Arc<dyn ObjectStore>> {
    let store = object_store::local::LocalFileSystem::new_with_prefix(root_path).map_err(|e| {
        error::Error::internal_err(format!(
            "Error building filesystem object store: {:?}",
            e
        ))
    })?;
    Ok(Arc::new(store))
}

#[cfg(feature = "parquet")]
pub async fn build_object_store_from_settings(
    settings: ObjectSettings,
    init_private_key: Option<&windmill_common::DB>,
) -> error::Result<ExpirableObjectStore> {
    match settings {
        ObjectSettings::S3(s3_settings) => build_s3_client_from_settings(s3_settings)
            .await
            .map(|x| ExpirableObjectStore::from(x)),
        ObjectSettings::Azure(azure_settings) => {
            let azure_blob_resource = azure_settings;
            build_azure_blob_client(&azure_blob_resource).map(|x| ExpirableObjectStore::from(x))
        }
        ObjectSettings::AwsOidc(ref s3_aws_oidc_settings) => {
            let token_generator = crate::job_s3_helpers_oss::TokenGenerator::AsServerInstance();
            let res = crate::job_s3_helpers_oss::generate_s3_aws_oidc_resource(
                s3_aws_oidc_settings.clone(),
                token_generator,
                init_private_key,
            )
            .await?;

            build_object_store_client(&res)
                .await
                .map(|x| ExpirableObjectStore {
                    store: x,
                    refresh: Some(ObjectStoreRefresh::new(settings.clone(), res.expiration())),
                })
        }
        ObjectSettings::Gcs(gcs_settings) => {
            let gcs_resource = gcs_settings;
            build_gcs_client(&gcs_resource)
                .await
                .map(|x| ExpirableObjectStore::from(x))
        }
        ObjectSettings::Filesystem(fs) => {
            build_filesystem_client(&fs.root_path).map(|x| ExpirableObjectStore::from(x))
        }
    }
}

#[cfg(feature = "parquet")]
fn none_if_empty(s: Option<String>) -> Option<String> {
    if s.is_none() || s.as_ref().unwrap().is_empty() {
        None
    } else {
        s
    }
}

#[cfg(feature = "parquet")]
pub async fn build_s3_client_from_settings(
    settings: S3Settings,
) -> error::Result<Arc<dyn ObjectStore>> {
    let region = none_if_empty(settings.region)
        .unwrap_or_else(|| std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()));

    let s3_resource = S3Resource {
        endpoint: none_if_empty(settings.endpoint).unwrap_or_else(|| {
            std::env::var("S3_ENDPOINT").unwrap_or_else(|_| format!("s3.{region}.amazonaws.com"))
        }),
        bucket: settings.bucket.clone().unwrap_or_else(|| {
            std::env::var("S3_CACHE_BUCKET").unwrap_or_else(|_| "missingbucket".to_string())
        }),
        region,
        access_key: settings.access_key,
        secret_key: settings.secret_key,
        use_ssl: !settings.allow_http.unwrap_or(true),
        path_style: settings.path_style,
        port: settings.port,
        token: None,
        expiration: None,
    };

    build_s3_client(&s3_resource).await
}

#[cfg(feature = "parquet")]
#[derive(Debug)]
struct AwsCredentialAdapter {
    pub inner: DefaultCredentialsChain,
}

#[cfg(feature = "parquet")]
#[async_trait]
impl CredentialProvider for AwsCredentialAdapter {
    type Credential = AwsCredential;
    async fn get_credential(&self) -> object_store::Result<Arc<Self::Credential>> {
        let creds = self.inner.provide_credentials().await.map_err(|e| {
            tracing::error!("Error getting credentials: {:?}", e);
            object_store::Error::Generic { store: "AWS", source: Box::new(e) }
        })?;
        Ok(Arc::new(Self::Credential {
            key_id: creds.access_key_id().to_string(),
            secret_key: creds.secret_access_key().to_string(),
            token: creds.session_token().map(|s| s.to_string()),
        }))
    }
}

// --- End moved code ---

lazy_static::lazy_static! {
    static ref S3_BUCKET_RESTRICTIONS: Option<HashMap<String, Vec<String>>> = {
        parse_bucket_restrictions()
    };
}

fn parse_bucket_restrictions() -> Option<HashMap<String, Vec<String>>> {
    let env_var = std::env::var("S3_BUCKETS_WORKSPACE_RESTRICTIONS").ok()?;
    parse_bucket_restrictions_from_str(&env_var)
}

fn parse_bucket_restrictions_from_str(input: &str) -> Option<HashMap<String, Vec<String>>> {
    if input.trim().is_empty() {
        return None;
    }

    let mut restrictions = HashMap::new();

    for bucket_rule in input.split(';') {
        let bucket_rule = bucket_rule.trim();
        if bucket_rule.is_empty() {
            continue;
        }

        let parts: Vec<&str> = bucket_rule.splitn(2, ':').collect();
        if parts.len() != 2 {
            tracing::warn!(
                "Invalid bucket restriction format: '{}'. Expected 'bucket:workspace1,workspace2'",
                bucket_rule
            );
            continue;
        }

        let bucket_name = parts[0].trim().to_string();
        let workspaces: Vec<String> = parts[1]
            .split(',')
            .map(|w| w.trim().to_string())
            .filter(|w| !w.is_empty())
            .collect();

        if workspaces.is_empty() {
            tracing::warn!(
                "No workspaces specified for bucket '{}', skipping restriction",
                bucket_name
            );
            continue;
        }

        restrictions.insert(bucket_name, workspaces);
    }

    if restrictions.is_empty() {
        None
    } else {
        tracing::info!(
            "S3 bucket restrictions loaded for {} buckets",
            restrictions.len()
        );
        Some(restrictions)
    }
}

pub fn check_bucket_workspace_restriction(
    bucket_name: &str,
    workspace_id: &str,
) -> error::Result<()> {
    if let Some(ref restrictions) = *S3_BUCKET_RESTRICTIONS {
        if let Some(allowed_workspaces) = restrictions.get(bucket_name) {
            if !allowed_workspaces.contains(&workspace_id.to_string()) {
                return Err(error::Error::NotAuthorized(format!(
                    "Workspace '{}' is not authorized to access bucket '{}'",
                    workspace_id, bucket_name
                )));
            }
        }
    }
    Ok(())
}

pub const DEFAULT_STORAGE: &str = "_default_";

pub fn bundle(w_id: &str, hash: &str) -> String {
    format!("script_bundle/{}/{}", w_id, hash)
}

pub fn raw_app(w_id: &str, version: &i64) -> String {
    format!("raw_app/{}/{}", w_id, version)
}

pub async fn upload_artifact_to_store(
    path: &str,
    data: bytes::Bytes,
    standalone_dir: &str,
) -> error::Result<()> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    let object_store = get_object_store().await;
    #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
    let object_store: Option<()> = None;
    Ok(
        if &windmill_common::utils::MODE_AND_ADDONS.mode == &windmill_common::utils::Mode::Standalone
            && object_store.is_none()
        {
            let path = format!("{}/{}", standalone_dir, path);
            tracing::info!("Writing file to path {path}");

            let split_path = path.split("/").collect::<Vec<&str>>();
            std::fs::create_dir_all(split_path[..split_path.len() - 1].join("/"))?;

            windmill_common::worker::write_file_bytes(&path, &data)?;
        } else {
            #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
            {
                return Err(error::Error::ExecutionErr(
                    "codebase is an EE feature".to_string(),
                ));
            }

            #[cfg(all(feature = "enterprise", feature = "parquet"))]
            if let Some(os) = object_store {
                if let Err(e) = os
                    .put(&object_store::path::Path::from(path), data.into())
                    .await
                {
                    tracing::info!("Failed to put snapshot to s3 at {path}: {:?}", e);
                    return Err(error::Error::ExecutionErr(format!(
                        "Failed to put {path} to s3"
                    )));
                }
            } else {
                return Err(error::Error::BadConfig("Object store is required for snapshot script and is not configured for servers".to_string()));
            }
        },
    )
}

#[cfg(feature = "parquet")]
pub async fn get_etag_or_empty(
    object_store_resource: &ObjectStoreResource,
    s3_object: S3Object,
) -> Option<String> {
    let object_store_client = build_object_store_client(object_store_resource).await;
    if object_store_client.is_err() {
        return None;
    }

    let object_key = object_store::path::Path::from(s3_object.s3);

    return object_store_client
        .unwrap()
        .head(&object_key)
        .await
        .ok()
        .map(|meta| meta.e_tag)
        .flatten();
}

pub fn lfs_to_object_store_resource(
    lfs: &LargeFileStorage,
    resource_value: serde_json::Value,
) -> error::Result<ObjectStoreResource> {
    match lfs {
        LargeFileStorage::S3Storage(_) | LargeFileStorage::S3AwsOidc(_) => {
            let s3_resource: S3Resource = serde_json::from_value(resource_value).map_err(|e| {
                error::Error::internal_err(format!("Error parsing S3 resource: {e:?}"))
            })?;
            Ok(ObjectStoreResource::S3(s3_resource))
        }
        LargeFileStorage::AzureBlobStorage(_) | LargeFileStorage::AzureWorkloadIdentity(_) => {
            let azure_blob_resource: AzureBlobResource = serde_json::from_value(resource_value)
                .map_err(|e| {
                    error::Error::internal_err(format!("Error parsing Azure Blob resource: {e:?}"))
                })?;
            Ok(ObjectStoreResource::Azure(azure_blob_resource))
        }
        LargeFileStorage::GoogleCloudStorage(_) => {
            let gcs_resource: GcsResource =
                serde_json::from_value(resource_value).map_err(|e| {
                    error::Error::internal_err(format!("Error parsing GCS resource: {e:?}"))
                })?;
            Ok(ObjectStoreResource::Gcs(gcs_resource))
        }
        LargeFileStorage::FilesystemStorage(fs) => Ok(ObjectStoreResource::Filesystem(
            FilesystemSettings {
                root_path: fs.root_path.clone(),
            },
        )),
    }
}

pub fn format_duckdb_connection_settings(
    object_store_resource: ObjectStoreResource,
) -> error::Result<DuckdbConnectionSettingsResponse> {
    match object_store_resource {
        ObjectStoreResource::S3(s3_resource) => duckdb_connection_settings_internal(s3_resource),
        ObjectStoreResource::Azure(azure_resource) => {
            let connection_string = format!(
                "CREATE SECRET az_secret (TYPE AZURE, CONNECTION_STRING 'DefaultEndpointsProtocol=https;AccountName={};AccountKey={};EndpointSuffix=core.windows.net');",
                azure_resource.account_name,
                azure_resource.access_key.unwrap_or_default()
            );
            let response = DuckdbConnectionSettingsResponse {
                connection_settings_str: connection_string,
                azure_container_path: Some(format!("az://{}", azure_resource.container_name)),
                s3_bucket: None,
            };
            Ok(response)
        }
        ObjectStoreResource::Gcs(_) => {
            return Err(error::Error::BadRequest(
                "GCS is not supported in DuckDB".to_string(),
            ));
        }
        ObjectStoreResource::Filesystem(_) => {
            return Err(error::Error::BadRequest(
                "Filesystem is not supported in DuckDB".to_string(),
            ));
        }
    }
}

pub fn duckdb_connection_settings_internal(
    s3_resource: S3Resource,
) -> error::Result<DuckdbConnectionSettingsResponse> {
    let mut duckdb_settings: String = String::new();

    duckdb_settings.push_str("SET home_directory='./';\n");
    duckdb_settings.push_str("INSTALL 'httpfs';\n");
    if s3_resource.path_style.unwrap_or(true) {
        duckdb_settings.push_str("SET s3_url_style='path';\n");
    }
    duckdb_settings.push_str(format!("SET s3_region='{}';\n", s3_resource.region).as_str());
    duckdb_settings.push_str(
        format!(
            "SET s3_endpoint='{}';\n",
            s3_resource.endpoint_with_region_fallback(None)
        )
        .as_str(),
    );
    if !s3_resource.use_ssl {
        duckdb_settings.push_str("SET s3_use_ssl=0;\n");
    }
    if let Some(access_key_id) = s3_resource.access_key {
        duckdb_settings.push_str(format!("SET s3_access_key_id='{}';\n", access_key_id).as_str());
    }
    if let Some(secret_access_key) = s3_resource.secret_key {
        duckdb_settings
            .push_str(format!("SET s3_secret_access_key='{}';\n", secret_access_key).as_str());
    }

    let response = DuckdbConnectionSettingsResponse {
        connection_settings_str: duckdb_settings,
        azure_container_path: None,
        s3_bucket: Some(s3_resource.bucket),
    };
    return Ok(response);
}

#[cfg(feature = "parquet")]
enum RecordBatchWriterEnum {
    Parquet(ArrowWriter<ChannelWriter>),
    Csv(csv::Writer<ChannelWriter>),
    Json(json::Writer<ChannelWriter, JsonArray>),
}

#[cfg(feature = "parquet")]
impl RecordBatchWriter for RecordBatchWriterEnum {
    fn write(&mut self, batch: &RecordBatch) -> Result<(), ArrowError> {
        match self {
            RecordBatchWriterEnum::Parquet(w) => w.write(batch).map_err(|e| e.into()),
            RecordBatchWriterEnum::Csv(w) => w.write(batch),
            RecordBatchWriterEnum::Json(w) => w.write(batch),
        }
    }

    fn close(self) -> Result<(), ArrowError> {
        match self {
            RecordBatchWriterEnum::Parquet(w) => w.close().map_err(|e| e.into()).map(drop),
            RecordBatchWriterEnum::Csv(w) => w.close(),
            RecordBatchWriterEnum::Json(w) => w.close(),
        }
    }
}

#[cfg(feature = "parquet")]
struct ChannelWriter {
    sender: tokio::sync::mpsc::Sender<anyhow::Result<Bytes>>,
}

#[cfg(feature = "parquet")]
impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let data: Bytes = buf.to_vec().into();
        self.sender.blocking_send(Ok(data)).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Channel send error: {}", e),
            )
        })?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(not(feature = "parquet"))]
pub async fn convert_json_line_stream<E: Into<anyhow::Error>>(
    mut _stream: impl futures::TryStreamExt<Item = Result<serde_json::Value, E>> + Unpin,
    _output_format: S3ModeFormat,
) -> anyhow::Result<impl futures::TryStreamExt<Item = anyhow::Result<bytes::Bytes>>> {
    Ok(async_stream::stream! {
        yield Err(anyhow::anyhow!("Parquet feature is not enabled. Cannot convert JSON line stream."));
    })
}

#[cfg(feature = "parquet")]
pub async fn convert_json_line_stream<E: Into<anyhow::Error>>(
    mut stream: impl TryStreamExt<Item = Result<serde_json::Value, E>> + Unpin,
    output_format: S3ModeFormat,
) -> anyhow::Result<impl TryStreamExt<Item = anyhow::Result<bytes::Bytes>>> {
    const MAX_MPSC_SIZE: usize = 1000;

    use datafusion::{execution::context::SessionContext, prelude::NdJsonReadOptions};
    use futures::StreamExt;
    use std::path::PathBuf;
    use tokio::io::AsyncWriteExt;

    let mut path = PathBuf::from(std::env::temp_dir());
    path.push(format!("{}.json", rd_string(8)));
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;

    let mut file: tokio::fs::File = tokio::fs::File::create(&path).await.map_err(to_anyhow)?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                let b: bytes::Bytes = serde_json::to_string(&chunk)?.into();
                file.write_all(&b).await?;
                file.write_all(b"\n").await?;
            }
            Err(e) => {
                tokio::fs::remove_file(&path).await?;
                return Err(e.into());
            }
        }
    }

    file.flush().await?;
    file.sync_all().await?;
    drop(file);

    let ctx = SessionContext::new();
    ctx.register_json(
        "my_table",
        path_str,
        NdJsonReadOptions::default(),
    )
    .await
    .map_err(to_anyhow)?;

    let df = ctx.sql("SELECT * FROM my_table").await.map_err(to_anyhow)?;
    let schema = df.schema().clone().into();
    let mut datafusion_stream = df.execute_stream().await.map_err(to_anyhow)?;

    let (tx, rx) = tokio::sync::mpsc::channel(MAX_MPSC_SIZE);
    let writer: Arc<Mutex<Option<RecordBatchWriterEnum>>> =
        Arc::new(Mutex::new(Some(match output_format {
            S3ModeFormat::Parquet => RecordBatchWriterEnum::Parquet(
                ArrowWriter::try_new(ChannelWriter { sender: tx.clone() }, Arc::new(schema), None)
                    .map_err(to_anyhow)?,
            ),

            S3ModeFormat::Csv => {
                RecordBatchWriterEnum::Csv(csv::Writer::new(ChannelWriter { sender: tx.clone() }))
            }
            S3ModeFormat::Json => {
                RecordBatchWriterEnum::Json(json::Writer::<_, JsonArray>::new(ChannelWriter {
                    sender: tx.clone(),
                }))
            }
        })));

    task::spawn(async move {
        while let Some(batch_result) = datafusion_stream.next().await {
            let batch: RecordBatch = match batch_result {
                Ok(batch) => batch,
                Err(e) => {
                    tracing::error!("Error in datafusion stream: {:?}", &e);
                    match tx.send(Err(e.into())).await {
                        Ok(_) => {}
                        Err(e) => tracing::error!("Failed to write error to channel: {:?}", &e),
                    }
                    break;
                }
            };
            let writer = writer.clone();
            let write_result = task::spawn_blocking(move || {
                writer.lock().unwrap().as_mut().unwrap().write(&batch)
            })
            .await;
            match write_result {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    tracing::error!("Error writing batch: {:?}", &e);
                    match tx.send(Err(e.into())).await {
                        Ok(_) => {}
                        Err(e) => tracing::error!("Failed to write error to channel: {:?}", &e),
                    }
                }
                Err(e) => tracing::error!("Error in blocking task: {:?}", &e),
            };
        }
        task::spawn_blocking(move || {
            writer.lock().unwrap().take().unwrap().close()?;
            drop(writer);
            Ok::<_, anyhow::Error>(())
        })
        .await??;
        drop(ctx);
        tokio::fs::remove_file(&path).await?;
        Ok::<_, anyhow::Error>(())
    });

    Ok(tokio_stream::wrappers::ReceiverStream::new(rx))
}

lazy_static::lazy_static! {
    pub static ref S3_PROXY_LAST_ERRORS_CACHE: Cache<String, String> = Cache::new(4);
}

#[cfg(feature = "parquet")]
pub async fn get_logs_from_store(
    log_offset: i32,
    logs: &str,
    log_file_index: &Option<Vec<String>>,
) -> Option<impl futures::Stream<Item = Result<bytes::Bytes, object_store::Error>>> {
    if log_offset > 0 {
        if let Some(file_index) = log_file_index.clone() {
            if let Some(os) = get_object_store().await {
                let logs = logs.to_string();
                let stream = async_stream::stream! {
                    for file_p in file_index {
                        let file = os.get(&object_store::path::Path::from(file_p.clone())).await;
                        match file {
                            Ok(file) => {
                                if let Ok(bytes) = file.bytes().await {
                                    yield Ok(bytes::Bytes::from(bytes));
                                }
                            }
                            Err(e) => {
                                tracing::debug!("error getting file from store: {file_p}: {e}");
                            }
                        }
                    }
                    yield Ok(bytes::Bytes::from(logs))
                };
                return Some(stream);
            } else {
                tracing::debug!("object store client not present");
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- render_endpoint tests ---

    #[test]
    fn test_render_endpoint_ssl_prefix() {
        let result = render_endpoint(
            "s3.amazonaws.com".to_string(),
            true,
            None,
            Some(true),
            "mybucket".to_string(),
        );
        assert_eq!(result, "https://s3.amazonaws.com");
    }

    #[test]
    fn test_render_endpoint_non_ssl_prefix() {
        let result = render_endpoint(
            "minio.local".to_string(),
            false,
            None,
            Some(true),
            "mybucket".to_string(),
        );
        assert_eq!(result, "http://minio.local");
    }

    #[test]
    fn test_render_endpoint_with_port() {
        let result = render_endpoint(
            "minio.local".to_string(),
            false,
            Some(9000),
            Some(true),
            "mybucket".to_string(),
        );
        assert_eq!(result, "http://minio.local:9000");
    }

    #[test]
    fn test_render_endpoint_virtual_hosted_style() {
        let result = render_endpoint(
            "s3.amazonaws.com".to_string(),
            true,
            None,
            Some(false),
            "mybucket".to_string(),
        );
        assert_eq!(result, "https://mybucket.s3.amazonaws.com");
    }

    #[test]
    fn test_render_endpoint_passthrough_with_scheme() {
        let result = render_endpoint(
            "https://custom.endpoint.com".to_string(),
            false, // use_ssl is ignored when scheme already present
            None,
            Some(false), // path_style is also ignored
            "mybucket".to_string(),
        );
        assert_eq!(result, "https://custom.endpoint.com");
    }

    // --- lfs_to_object_store_resource tests ---

    #[test]
    fn test_lfs_to_object_store_resource_s3() {
        let lfs = LargeFileStorage::S3Storage(S3Storage {
            s3_resource_path: "u/admin/s3_resource".to_string(),
            public_resource: None,
            advanced_permissions: None,
        });
        let resource_json = serde_json::json!({
            "bucket": "my-bucket",
            "region": "us-east-1",
            "endPoint": "s3.amazonaws.com",
            "useSSL": true,
            "accessKey": "AKIA...",
            "secretKey": "secret"
        });
        let result = lfs_to_object_store_resource(&lfs, resource_json).unwrap();
        match result {
            ObjectStoreResource::S3(s3) => {
                assert_eq!(s3.bucket, "my-bucket");
                assert_eq!(s3.region, "us-east-1");
                assert_eq!(s3.endpoint, "s3.amazonaws.com");
                assert!(s3.use_ssl);
            }
            _ => panic!("Expected S3 resource"),
        }
    }

    #[test]
    fn test_lfs_to_object_store_resource_azure() {
        let lfs = LargeFileStorage::AzureBlobStorage(AzureBlobStorage {
            azure_blob_resource_path: "u/admin/azure_resource".to_string(),
            public_resource: None,
            advanced_permissions: None,
        });
        let resource_json = serde_json::json!({
            "accountName": "myaccount",
            "containerName": "mycontainer"
        });
        let result = lfs_to_object_store_resource(&lfs, resource_json).unwrap();
        match result {
            ObjectStoreResource::Azure(az) => {
                assert_eq!(az.account_name, "myaccount");
                assert_eq!(az.container_name, "mycontainer");
            }
            _ => panic!("Expected Azure resource"),
        }
    }

    #[test]
    fn test_lfs_to_object_store_resource_gcs() {
        let lfs = LargeFileStorage::GoogleCloudStorage(GoogleCloudStorage {
            gcs_resource_path: "u/admin/gcs_resource".to_string(),
            public_resource: None,
            advanced_permissions: None,
        });
        let resource_json = serde_json::json!({
            "bucket": "gcs-bucket",
            "serviceAccountKey": {"type": "service_account", "project_id": "test"}
        });
        let result = lfs_to_object_store_resource(&lfs, resource_json).unwrap();
        match result {
            ObjectStoreResource::Gcs(gcs) => {
                assert_eq!(gcs.bucket, "gcs-bucket");
            }
            _ => panic!("Expected GCS resource"),
        }
    }

    #[test]
    fn test_lfs_to_object_store_resource_filesystem() {
        let lfs = LargeFileStorage::FilesystemStorage(FilesystemStorage {
            root_path: "/tmp/mydata".to_string(),
            public_resource: None,
            advanced_permissions: None,
        });
        // resource_value is ignored for filesystem
        let result =
            lfs_to_object_store_resource(&lfs, serde_json::Value::Null).unwrap();
        match result {
            ObjectStoreResource::Filesystem(fs) => {
                assert_eq!(fs.root_path, "/tmp/mydata");
            }
            _ => panic!("Expected Filesystem resource"),
        }
    }

    // --- duckdb_connection_settings tests ---

    #[test]
    fn test_duckdb_connection_settings_s3_basic() {
        let s3 = S3Resource {
            bucket: "test-bucket".to_string(),
            region: "eu-west-1".to_string(),
            endpoint: "s3.eu-west-1.amazonaws.com".to_string(),
            use_ssl: true,
            access_key: Some("AKIA123".to_string()),
            secret_key: Some("secret456".to_string()),
            path_style: Some(true),
            token: None,
            expiration: None,
            port: None,
        };
        let result = duckdb_connection_settings_internal(s3).unwrap();
        assert!(result.connection_settings_str.contains("SET s3_region='eu-west-1'"));
        assert!(result.connection_settings_str.contains("SET s3_access_key_id='AKIA123'"));
        assert!(result.connection_settings_str.contains("SET s3_secret_access_key='secret456'"));
        assert!(result.connection_settings_str.contains("SET s3_url_style='path'"));
        assert!(!result.connection_settings_str.contains("SET s3_use_ssl=0"));
        assert_eq!(result.s3_bucket, Some("test-bucket".to_string()));
        assert!(result.azure_container_path.is_none());
    }

    #[test]
    fn test_duckdb_connection_settings_s3_no_ssl() {
        let s3 = S3Resource {
            bucket: "bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: "minio:9000".to_string(),
            use_ssl: false,
            access_key: None,
            secret_key: None,
            path_style: Some(false),
            token: None,
            expiration: None,
            port: None,
        };
        let result = duckdb_connection_settings_internal(s3).unwrap();
        assert!(result.connection_settings_str.contains("SET s3_use_ssl=0"));
        assert!(!result.connection_settings_str.contains("SET s3_url_style='path'"));
    }

    #[test]
    fn test_duckdb_connection_settings_azure() {
        let resource = ObjectStoreResource::Azure(AzureBlobResource {
            endpoint: None,
            use_ssl: None,
            account_name: "myaccount".to_string(),
            tenant_id: None,
            client_id: None,
            container_name: "mycontainer".to_string(),
            access_key: Some("base64key==".to_string()),
            federated_token_file: None,
        });
        let result = format_duckdb_connection_settings(resource).unwrap();
        assert!(result.connection_settings_str.contains("AccountName=myaccount"));
        assert!(result.connection_settings_str.contains("AccountKey=base64key=="));
        assert_eq!(
            result.azure_container_path,
            Some("az://mycontainer".to_string())
        );
        assert!(result.s3_bucket.is_none());
    }

    #[test]
    fn test_duckdb_connection_settings_gcs_unsupported() {
        let resource = ObjectStoreResource::Gcs(GcsResource {
            bucket: "bucket".to_string(),
            service_account_key: "{}".to_string(),
        });
        let result = format_duckdb_connection_settings(resource);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("GCS is not supported"));
    }

    #[test]
    fn test_duckdb_connection_settings_filesystem_unsupported() {
        let resource = ObjectStoreResource::Filesystem(FilesystemSettings {
            root_path: "/tmp/data".to_string(),
        });
        let result = format_duckdb_connection_settings(resource);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Filesystem is not supported"));
    }

    // --- object_store error mapping ---

    #[cfg(feature = "parquet")]
    #[test]
    fn test_object_store_error_mapping() {
        use windmill_common::error::Error;

        // NotFound
        let err = object_store::Error::NotFound {
            path: "test/path".into(),
            source: "missing".into(),
        };
        let mapped = object_store_error_to_error(err);
        assert!(matches!(mapped, Error::NotFound(_)));

        // PermissionDenied
        let err = object_store::Error::PermissionDenied {
            path: "secret".into(),
            source: "forbidden".into(),
        };
        let mapped = object_store_error_to_error(err);
        assert!(matches!(mapped, Error::PermissionDenied(_)));

        // InvalidPath
        let err = object_store::Error::InvalidPath {
            source: object_store::path::Error::EmptySegment { path: "".into() },
        };
        let mapped = object_store_error_to_error(err);
        assert!(matches!(mapped, Error::BadRequest(_)));

        // NotImplemented
        let mapped = object_store_error_to_error(object_store::Error::NotImplemented);
        assert!(matches!(mapped, Error::BadRequest(_)));

        // Unauthenticated
        let err = object_store::Error::Unauthenticated {
            path: "obj".into(),
            source: "no creds".into(),
        };
        let mapped = object_store_error_to_error(err);
        assert!(matches!(mapped, Error::NotAuthorized(_)));
    }

    // --- Filesystem-backed integration tests ---

    #[cfg(feature = "parquet")]
    #[test]
    fn test_build_filesystem_client() {
        let dir = tempfile::tempdir().unwrap();
        let result = build_filesystem_client(dir.path().to_str().unwrap());
        assert!(result.is_ok());
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_filesystem_put_get_roundtrip() {
        use object_store::{path::Path, ObjectStore, PutPayload};

        let dir = tempfile::tempdir().unwrap();
        let client = build_filesystem_client(dir.path().to_str().unwrap()).unwrap();
        let path = Path::from("test.txt");
        let data = bytes::Bytes::from("hello world");

        client
            .put(&path, PutPayload::from(data.clone()))
            .await
            .unwrap();
        let result = client.get(&path).await.unwrap().bytes().await.unwrap();
        assert_eq!(result, data);
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_filesystem_nested_paths() {
        use object_store::{path::Path, ObjectStore, PutPayload};

        let dir = tempfile::tempdir().unwrap();
        let client = build_filesystem_client(dir.path().to_str().unwrap()).unwrap();
        let path = Path::from("a/b/c.txt");
        let data = bytes::Bytes::from("nested content");

        client
            .put(&path, PutPayload::from(data.clone()))
            .await
            .unwrap();
        let result = client.get(&path).await.unwrap().bytes().await.unwrap();
        assert_eq!(result, data);
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_filesystem_overwrite() {
        use object_store::{path::Path, ObjectStore, PutPayload};

        let dir = tempfile::tempdir().unwrap();
        let client = build_filesystem_client(dir.path().to_str().unwrap()).unwrap();
        let path = Path::from("overwrite.txt");

        client
            .put(&path, PutPayload::from(bytes::Bytes::from("v1")))
            .await
            .unwrap();
        client
            .put(&path, PutPayload::from(bytes::Bytes::from("v2")))
            .await
            .unwrap();
        let result = client.get(&path).await.unwrap().bytes().await.unwrap();
        assert_eq!(result, bytes::Bytes::from("v2"));
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_attempt_fetch_bytes_success() {
        use object_store::{path::Path, ObjectStore, PutPayload};

        let dir = tempfile::tempdir().unwrap();
        let client = build_filesystem_client(dir.path().to_str().unwrap()).unwrap();
        let data = bytes::Bytes::from("fetch me");
        client
            .put(&Path::from("fetch.txt"), PutPayload::from(data.clone()))
            .await
            .unwrap();

        let result = attempt_fetch_bytes(client, "fetch.txt").await.unwrap();
        assert_eq!(result, data);
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_attempt_fetch_bytes_missing_key() {
        let dir = tempfile::tempdir().unwrap();
        let client = build_filesystem_client(dir.path().to_str().unwrap()).unwrap();

        let result = attempt_fetch_bytes(client, "nonexistent.txt").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            windmill_common::error::Error::ExecutionErr(_)
        ));
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_attempt_fetch_bytes_empty_object() {
        use object_store::{path::Path, ObjectStore, PutPayload};

        let dir = tempfile::tempdir().unwrap();
        let client = build_filesystem_client(dir.path().to_str().unwrap()).unwrap();
        client
            .put(
                &Path::from("empty.txt"),
                PutPayload::from(bytes::Bytes::new()),
            )
            .await
            .unwrap();

        let result = attempt_fetch_bytes(client, "empty.txt").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not exist in bucket"));
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_get_etag_or_empty() {
        use object_store::{path::Path, ObjectStore, PutPayload};

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let client = build_filesystem_client(root).unwrap();
        client
            .put(
                &Path::from("etag.txt"),
                PutPayload::from(bytes::Bytes::from("content")),
            )
            .await
            .unwrap();

        let resource = ObjectStoreResource::Filesystem(FilesystemSettings {
            root_path: root.to_string(),
        });
        let s3_obj = S3Object {
            s3: "etag.txt".to_string(),
            storage: None,
            filename: None,
            presigned: None,
        };

        let etag = get_etag_or_empty(&resource, s3_obj).await;
        // LocalFileSystem should return an etag based on file metadata
        assert!(etag.is_some());
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_get_etag_or_empty_missing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let resource = ObjectStoreResource::Filesystem(FilesystemSettings {
            root_path: root.to_string(),
        });
        let s3_obj = S3Object {
            s3: "nonexistent.txt".to_string(),
            storage: None,
            filename: None,
            presigned: None,
        };

        let etag = get_etag_or_empty(&resource, s3_obj).await;
        assert!(etag.is_none());
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_settings_to_client_end_to_end() {
        use object_store::{path::Path, ObjectStore, PutPayload};

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap().to_string();
        let settings = ObjectSettings::Filesystem(FilesystemSettings {
            root_path: root,
        });

        let expirable = build_object_store_from_settings(settings, None)
            .await
            .unwrap();
        let data = bytes::Bytes::from("end to end via settings");
        expirable
            .store
            .put(
                &Path::from("e2e.txt"),
                PutPayload::from(data.clone()),
            )
            .await
            .unwrap();
        let result = expirable
            .store
            .get(&Path::from("e2e.txt"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(result, data);
        assert!(expirable.refresh.is_none());
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_resource_to_client_end_to_end() {
        use object_store::{path::Path, ObjectStore, PutPayload};

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap().to_string();
        let resource = ObjectStoreResource::Filesystem(FilesystemSettings {
            root_path: root,
        });

        let client = build_object_store_client(&resource).await.unwrap();
        let data = bytes::Bytes::from("end to end via resource");
        client
            .put(&Path::from("e2e.txt"), PutPayload::from(data.clone()))
            .await
            .unwrap();
        let result = client
            .get(&Path::from("e2e.txt"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(result, data);
    }

    // --- bundle / raw_app path tests ---

    #[test]
    fn test_bundle_path_format() {
        assert_eq!(bundle("my_workspace", "abc123"), "script_bundle/my_workspace/abc123");
    }

    #[test]
    fn test_raw_app_path_format() {
        assert_eq!(raw_app("my_workspace", &42), "raw_app/my_workspace/42");
    }

    // --- parse_bucket_restrictions tests ---

    #[test]
    fn test_parse_bucket_restrictions_single_bucket() {
        let result =
            parse_bucket_restrictions_from_str("my-bucket:workspace1,workspace2").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("my-bucket").unwrap(),
            &vec!["workspace1".to_string(), "workspace2".to_string()]
        );
    }

    #[test]
    fn test_parse_bucket_restrictions_multiple_buckets() {
        let result = parse_bucket_restrictions_from_str(
            "bucket-a:ws1,ws2;bucket-b:ws3",
        )
        .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get("bucket-a").unwrap(),
            &vec!["ws1".to_string(), "ws2".to_string()]
        );
        assert_eq!(
            result.get("bucket-b").unwrap(),
            &vec!["ws3".to_string()]
        );
    }

    #[test]
    fn test_parse_bucket_restrictions_empty_string() {
        assert!(parse_bucket_restrictions_from_str("").is_none());
        assert!(parse_bucket_restrictions_from_str("   ").is_none());
    }

    #[test]
    fn test_parse_bucket_restrictions_invalid_format_skipped() {
        // "no-colon" is invalid, only "valid:ws1" should be parsed
        let result =
            parse_bucket_restrictions_from_str("no-colon;valid:ws1").unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("valid"));
    }

    #[test]
    fn test_parse_bucket_restrictions_trailing_semicolons() {
        let result =
            parse_bucket_restrictions_from_str(";bucket:ws1;;").unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("bucket"));
    }

    // --- build_filesystem_client error path ---

    #[cfg(feature = "parquet")]
    #[test]
    fn test_build_filesystem_client_nonexistent_path() {
        // LocalFileSystem::new_with_prefix fails when the directory doesn't exist
        let result = build_filesystem_client("/tmp/windmill_test_nonexistent_dir_12345_xyz");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Error building filesystem object store"));
    }

    // --- get_logs_from_store test ---

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_get_logs_from_store_with_filesystem() {
        use futures::StreamExt;
        use object_store::{path::Path, ObjectStore, PutPayload};

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap().to_string();
        let client = build_filesystem_client(&root).unwrap();

        // Write two log chunk files
        client
            .put(
                &Path::from("logs/chunk1.log"),
                PutPayload::from(bytes::Bytes::from("chunk1 content\n")),
            )
            .await
            .unwrap();
        client
            .put(
                &Path::from("logs/chunk2.log"),
                PutPayload::from(bytes::Bytes::from("chunk2 content\n")),
            )
            .await
            .unwrap();

        // Set the global OBJECT_STORE_SETTINGS to our filesystem store
        {
            let mut settings = OBJECT_STORE_SETTINGS.write().await;
            *settings = Some(ExpirableObjectStore::from(client));
        }

        let file_index = Some(vec![
            "logs/chunk1.log".to_string(),
            "logs/chunk2.log".to_string(),
        ]);
        let tail_logs = "tail logs here";

        // log_offset > 0, file_index Some, object store set → should return a stream
        let stream = get_logs_from_store(1, tail_logs, &file_index).await;
        assert!(stream.is_some());

        let chunks: Vec<bytes::Bytes> = stream
            .unwrap()
            .filter_map(|r| async { r.ok() })
            .collect()
            .await;
        assert_eq!(chunks.len(), 3); // chunk1 + chunk2 + tail_logs
        assert_eq!(chunks[0], bytes::Bytes::from("chunk1 content\n"));
        assert_eq!(chunks[1], bytes::Bytes::from("chunk2 content\n"));
        assert_eq!(chunks[2], bytes::Bytes::from("tail logs here"));

        // Clean up global state
        {
            let mut settings = OBJECT_STORE_SETTINGS.write().await;
            *settings = None;
        }
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_get_logs_from_store_zero_offset() {
        // log_offset == 0 → always returns None regardless of other params
        let file_index = Some(vec!["some/file.log".to_string()]);
        let result = get_logs_from_store(0, "logs", &file_index).await;
        assert!(result.is_none());
    }

    #[cfg(feature = "parquet")]
    #[tokio::test]
    async fn test_get_logs_from_store_no_file_index() {
        // file_index is None → returns None
        let result = get_logs_from_store(1, "logs", &None).await;
        assert!(result.is_none());
    }
}
