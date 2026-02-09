use crate::error::{self};
#[cfg(feature = "parquet")]
use aws_sdk_sts::config::ProvideCredentials;
#[cfg(feature = "parquet")]
use axum::async_trait;
use chrono::{DateTime, Utc};
#[cfg(feature = "parquet")]
use object_store::aws::AwsCredential;
#[cfg(feature = "parquet")]
use object_store::azure::MicrosoftAzureBuilder;
#[cfg(feature = "parquet")]
use object_store::gcp::GoogleCloudStorageBuilder;
#[cfg(feature = "parquet")]
use object_store::ObjectStore;
#[cfg(feature = "parquet")]
use object_store::{aws::AmazonS3Builder, ClientOptions};
use quick_cache::sync::Cache;
#[cfg(feature = "parquet")]
use reqwest::header::HeaderMap;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
#[cfg(feature = "parquet")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "parquet")]
use tokio::sync::RwLock;

#[cfg(feature = "parquet")]
use crate::error::to_anyhow;
#[cfg(feature = "parquet")]
use crate::utils::rd_string;
#[cfg(feature = "parquet")]
use bytes::Bytes;
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
use std::io::Write;
#[cfg(feature = "parquet")]
use tokio::task;
#[cfg(feature = "parquet")]
use windmill_parser_sql::S3ModeFormat;

use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref S3_BUCKET_RESTRICTIONS: Option<HashMap<String, Vec<String>>> = {
        parse_bucket_restrictions()
    };
}

/// Parses the S3_BUCKETS_WORKSPACE_RESTRICTIONS environment variable
/// Format: bucket_a:workspace1,workspace2;bucket_b:workspace3,workspace4
fn parse_bucket_restrictions() -> Option<HashMap<String, Vec<String>>> {
    let env_var = std::env::var("S3_BUCKETS_WORKSPACE_RESTRICTIONS").ok()?;

    if env_var.trim().is_empty() {
        return None;
    }

    let mut restrictions = HashMap::new();

    for bucket_rule in env_var.split(';') {
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

/// Checks if a workspace is allowed to access a given bucket based on restrictions
/// Returns Ok(()) if access is allowed, Err if restricted
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

// #[cfg(feature = "parquet")]

// impl ExpirableObjectStore {
//     pub fn new(store: Arc<dyn ObjectStore>, expiration: Option<DateTime<Utc>>) -> Self {
//         Self { store, expiration }
//     }
// }

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
    //if the jwks endpoints are not up yet, we should retry later soon
    Later,
    Never,
}

#[cfg(feature = "parquet")]
pub async fn reload_object_store_setting(db: &crate::DB) -> ObjectStoreReload {
    use crate::{
        ee_oss::{get_license_plan, LicensePlan},
        global_settings::{load_value_from_global_settings, OBJECT_STORE_CONFIG_SETTING},
        s3_helpers::ObjectSettings,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum LargeFileStorage {
    S3Storage(S3Storage),
    AzureBlobStorage(AzureBlobStorage),
    S3AwsOidc(S3Storage),
    AzureWorkloadIdentity(AzureBlobStorage),
    GoogleCloudStorage(GoogleCloudStorage),
    // TODO: Add a filesystem type here in the future if needed
}

impl LargeFileStorage {
    pub fn get_s3_resource_path(&self) -> &str {
        match self {
            LargeFileStorage::S3Storage(s3_lfs) => &s3_lfs.s3_resource_path,
            LargeFileStorage::S3AwsOidc(s3_lfs) => &s3_lfs.s3_resource_path,
            LargeFileStorage::AzureBlobStorage(az_lfs) => &az_lfs.azure_blob_resource_path,
            LargeFileStorage::AzureWorkloadIdentity(az_lfs) => &az_lfs.azure_blob_resource_path,
            LargeFileStorage::GoogleCloudStorage(gcs_lfs) => &gcs_lfs.gcs_resource_path,
        }
    }
    pub fn is_public_resource(&self) -> bool {
        match self {
            LargeFileStorage::S3Storage(lfs) => lfs.public_resource,
            LargeFileStorage::S3AwsOidc(lfs) => lfs.public_resource,
            LargeFileStorage::AzureBlobStorage(lfs) => lfs.public_resource,
            LargeFileStorage::AzureWorkloadIdentity(lfs) => lfs.public_resource,
            LargeFileStorage::GoogleCloudStorage(glfs) => glfs.public_resource,
        }
        .unwrap_or(false)
    }
    pub fn get_advanced_permissions(&self) -> Option<&Vec<S3PermissionRule>> {
        match self {
            LargeFileStorage::S3Storage(lfs) => lfs.advanced_permissions.as_ref(),
            LargeFileStorage::S3AwsOidc(lfs) => lfs.advanced_permissions.as_ref(),
            LargeFileStorage::AzureBlobStorage(lfs) => lfs.advanced_permissions.as_ref(),
            LargeFileStorage::AzureWorkloadIdentity(lfs) => lfs.advanced_permissions.as_ref(),
            LargeFileStorage::GoogleCloudStorage(glfs) => glfs.advanced_permissions.as_ref(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct S3Storage {
    pub s3_resource_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_resource: Option<bool>,
    pub advanced_permissions: Option<Vec<S3PermissionRule>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AzureBlobStorage {
    pub azure_blob_resource_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_resource: Option<bool>,
    pub advanced_permissions: Option<Vec<S3PermissionRule>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoogleCloudStorage {
    pub gcs_resource_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_resource: Option<bool>,
    pub advanced_permissions: Option<Vec<S3PermissionRule>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct S3PermissionRule {
    pub pattern: String,
    pub allow: S3Permission, // read, write, delete, list
}
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct S3Permission: u8 {
        const READ   = 0b0001;
        const WRITE  = 0b0010;
        const DELETE = 0b0100;
        const LIST   = 0b1000;
    }
}

impl Serialize for S3Permission {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut perms = Vec::new();
        if self.contains(S3Permission::READ) {
            perms.push("read");
        }
        if self.contains(S3Permission::WRITE) {
            perms.push("write");
        }
        if self.contains(S3Permission::DELETE) {
            perms.push("delete");
        }
        if self.contains(S3Permission::LIST) {
            perms.push("list");
        }
        let perms = perms.join(",");
        perms.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for S3Permission {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PermVisitor;
        impl<'de> Visitor<'de> for PermVisitor {
            type Value = S3Permission;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("comma separated list of permissions: read, write, delete, list")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                let mut perms = S3Permission::empty();
                for value in v.split(',') {
                    perms |= match value {
                        "read" => S3Permission::READ,
                        "write" => S3Permission::WRITE,
                        "delete" => S3Permission::DELETE,
                        "list" => S3Permission::LIST,
                        _ => S3Permission::empty(), // ignore unknown permissions
                    };
                }
                Ok(perms)
            }
        }

        deserializer.deserialize_str(PermVisitor)
    }
}

#[derive(Clone, Debug)]
pub enum ObjectStoreResource {
    S3(S3Resource),
    Azure(AzureBlobResource),
    Gcs(GcsResource),
}

impl ObjectStoreResource {
    pub fn expiration(&self) -> Option<DateTime<Utc>> {
        match self {
            ObjectStoreResource::S3(s3_resource) => s3_resource.expiration,
            _ => None,
        }
    }
}

#[derive(Deserialize, Debug)]
pub enum StorageResourceType {
    S3,
    AzureBlob,
    S3AwsOidc,
    AzureWorkloadIdentity,
    GoogleCloudStorage,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct S3Resource {
    pub bucket: String,
    pub region: String,
    #[serde(rename = "endPoint")]
    pub endpoint: String,
    #[serde(rename = "useSSL")]
    pub use_ssl: bool,
    #[serde(rename = "accessKey")]
    pub access_key: Option<String>,
    #[serde(rename = "secretKey")]
    pub secret_key: Option<String>,
    #[serde(rename = "pathStyle")]
    pub path_style: Option<bool>,
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<DateTime<Utc>>,
    pub port: Option<u16>,
}

impl S3Resource {
    pub fn endpoint_with_region_fallback(&self, region_fallback: Option<String>) -> String {
        if self.endpoint.is_empty() {
            let final_region = if self.region.is_empty() {
                region_fallback.unwrap_or_else(|| "us-east-1".to_string())
            } else {
                self.region.clone()
            };
            format!("s3.{}.amazonaws.com", final_region)
        } else {
            self.endpoint.clone()
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AzureBlobResource {
    pub endpoint: Option<String>,
    #[serde(rename = "useSSL")]
    pub use_ssl: Option<bool>,
    #[serde(rename = "accountName")]
    pub account_name: String,
    #[serde(rename = "tenantId")]
    pub tenant_id: Option<String>,
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
    #[serde(rename = "containerName")]
    pub container_name: String,
    #[serde(rename = "accessKey")]
    pub access_key: Option<String>,
    #[serde(rename = "federatedTokenFile")]
    pub federated_token_file: Option<String>,
}

fn as_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let v: serde_json::Value = Deserialize::deserialize(deserializer)?;
    serde_json::to_string(&v).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize, Clone)]
pub struct GcsResource {
    pub bucket: String,
    #[serde(rename = "serviceAccountKey")]
    #[serde(deserialize_with = "as_string")]
    pub service_account_key: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Hash)]
pub struct S3AwsOidcResource {
    #[serde(rename = "bucket")]
    pub bucket: String,
    pub region: Option<String>,
    #[serde(rename = "roleArn")]
    pub role_arn: String,
    pub audience: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct S3Object {
    pub s3: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presigned: Option<String>,
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
    }
}

#[derive(PartialEq)]
pub enum BundleFormat {
    Esm,
    Cjs,
}

impl BundleFormat {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "esm" => Some(Self::Esm),
            "cjs" => Some(Self::Cjs),
            _ => None,
        }
    }
}

pub const DEFAULT_STORAGE: &str = "_default_";

pub async fn upload_artifact_to_store(
    path: &str,
    data: bytes::Bytes,
    standalone_dir: &str,
) -> error::Result<()> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    let object_store = crate::s3_helpers::get_object_store().await;
    #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
    let object_store: Option<()> = None;
    Ok(
        if &crate::utils::MODE_AND_ADDONS.mode == &crate::utils::Mode::Standalone
            && object_store.is_none()
        {
            let path = format!("{}/{}", standalone_dir, path);
            tracing::info!("Writing file to path {path}");

            let split_path = path.split("/").collect::<Vec<&str>>();
            std::fs::create_dir_all(split_path[..split_path.len() - 1].join("/"))?;

            crate::worker::write_file_bytes(&path, &data)?;
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
use aws_config::{default_provider::credentials::DefaultCredentialsChain, Region};
#[cfg(feature = "parquet")]
use object_store::CredentialProvider;

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
        ) // TODO: make it configurable maybe
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
        ) // TODO: make it configurable maybe
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

    // if private key is malformed, it will panic => https://github.com/apache/arrow-rs-object-store/issues/419
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

#[derive(Serialize, Deserialize)]
#[serde(tag = "typ", content = "value")]
pub enum ObjectStoreSettings {
    S3(S3Settings),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ObjectSettings {
    S3(S3Settings),
    Azure(AzureBlobResource),
    AwsOidc(S3AwsOidcResource),
    Gcs(GcsResource),
}

impl ObjectSettings {
    pub fn get_bucket(&self) -> Option<&String> {
        match self {
            ObjectSettings::S3(s3_settings) => s3_settings.bucket.as_ref(),
            ObjectSettings::Azure(azure_settings) => Some(&azure_settings.container_name),
            ObjectSettings::AwsOidc(s3_aws_oidc_settings) => Some(&s3_aws_oidc_settings.bucket),
            ObjectSettings::Gcs(gcs_settings) => Some(&gcs_settings.bucket),
        }
    }
}

#[cfg(feature = "parquet")]
pub async fn build_object_store_from_settings(
    settings: ObjectSettings,
    init_private_key: Option<&crate::DB>,
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
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct S3Settings {
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub endpoint: Option<String>,
    pub allow_http: Option<bool>, // default to true
    pub path_style: Option<bool>,
    pub store_logs: Option<bool>,
    pub port: Option<u16>,
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

pub fn bundle(w_id: &str, hash: &str) -> String {
    format!("script_bundle/{}/{}", w_id, hash)
}

pub fn raw_app(w_id: &str, version: &i64) -> String {
    format!("/home/rfiszel/raw_app/{}/{}", w_id, version)
}

// Originally used a Arc<Mutex<dyn RecordBatchWriter + Send>>
// But cannot call .close() on it because it moves the value and the object is not Sized
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
    _output_format: windmill_parser_sql::S3ModeFormat,
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

    // Write the stream to a temporary file
    let mut file: tokio::fs::File = tokio::fs::File::create(&path).await.map_err(to_anyhow)?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                // Convert the chunk to bytes and write it to the file
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
        NdJsonReadOptions { ..Default::default() },
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

    // This spawn is so that the data is sent in the background. Else the function would deadlock
    // when hitting the mpsc channel limit
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
            // Writer calls blocking_send which would crash if called from the async context
            let write_result = task::spawn_blocking(move || {
                // SAFETY: We await so the code is actually sequential, lock unwrap cannot panic
                // Second unwrap is ok because we initialized the option with Some
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

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DuckdbConnectionSettingsResponse {
    pub connection_settings_str: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure_container_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_bucket: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct DuckdbConnectionSettingsQueryV2 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_resource_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
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
    }
}

pub fn duckdb_connection_settings_internal(
    s3_resource: S3Resource,
) -> error::Result<DuckdbConnectionSettingsResponse> {
    let mut duckdb_settings: String = String::new();

    duckdb_settings.push_str("SET home_directory='./';\n"); // TODO: make this configurable maybe, or point to a temporary folder
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
        duckdb_settings.push_str("SET s3_use_ssl=0;\n"); // default is true for DuckDB
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

// DuckDB does not parse anything in case of S3 errors and just returns a generic error message.
// To display better error messages, we cache the errors in a Map<Token, ErrorMessage>
//
// We leverage the fact that workers have an internal server to insert the error message
// from the S3 Proxy, and read it directly in memory from the worker.
lazy_static::lazy_static! {
    pub static ref S3_PROXY_LAST_ERRORS_CACHE: Cache<String, String> = Cache::new(4);
}
