#[cfg(feature = "parquet")]
use crate::error;
#[cfg(feature = "parquet")]
use aws_sdk_sts::config::ProvideCredentials;
#[cfg(feature = "parquet")]
use axum::async_trait;
#[cfg(feature = "parquet")]
use object_store::aws::AwsCredential;
#[cfg(feature = "parquet")]
use object_store::azure::MicrosoftAzureBuilder;
#[cfg(feature = "parquet")]
use object_store::ObjectStore;
#[cfg(feature = "parquet")]
use object_store::{aws::AmazonS3Builder, ClientOptions};
use serde::{Deserialize, Serialize};
#[cfg(feature = "parquet")]
use std::sync::Arc;
#[cfg(feature = "parquet")]
use tokio::sync::RwLock;

#[cfg(feature = "parquet")]
lazy_static::lazy_static! {

    pub static ref OBJECT_STORE_CACHE_SETTINGS: Arc<RwLock<Option<Arc<dyn ObjectStore>>>> = Arc::new(RwLock::new(None));
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum LargeFileStorage {
    S3Storage(S3Storage),
    AzureBlobStorage(AzureBlobStorage),
    S3AwsOidc(S3Storage),
    AzureWorkloadIdentity(AzureBlobStorage),
    // TODO: Add a filesystem type here in the future if needed
}

#[derive(Serialize, Deserialize, Debug)]
pub struct S3Storage {
    pub s3_resource_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_resource: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AzureBlobStorage {
    pub azure_blob_resource_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_resource: Option<bool>,
}

#[derive(Clone, Debug)]
pub enum ObjectStoreResource {
    S3(S3Resource),
    Azure(AzureBlobResource),
}

#[derive(Deserialize, Debug)]
pub enum StorageResourceType {
    S3,
    AzureBlob,
    S3AwsOidc,
    AzureWorkloadIdentity,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct S3Resource {
    #[serde(rename = "bucket")]
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
    pub port: Option<u16>,

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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct S3AwsOidcResource {
    #[serde(rename = "bucket")]
    pub bucket: String,
    pub region: Option<String>,
    #[serde(rename = "roleArn")]
    pub role_arn: String,
    pub audience: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct S3Object {
    pub s3: String,
}

#[cfg(feature = "parquet")]
pub async fn get_etag_or_empty(
    object_store_resource: &ObjectStoreResource,
    s3_object: S3Object,
) -> Option<String> {
    let object_store_client = build_object_store_client(object_store_resource);
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
pub fn build_object_store_client(
    resource_ref: &ObjectStoreResource,
) -> error::Result<Arc<dyn ObjectStore>> {
    match resource_ref {
        ObjectStoreResource::S3(s3_resource_ref) => build_s3_client(&s3_resource_ref, None),
        ObjectStoreResource::Azure(azure_blob_resource_ref) => {
            build_azure_blob_client(&azure_blob_resource_ref)
        }
    }
}

#[cfg(feature = "parquet")]
use aws_config::{default_provider::credentials::DefaultCredentialsChain, Region};
#[cfg(feature = "parquet")]
use  object_store::CredentialProvider;

#[cfg(feature = "parquet")]
pub fn build_s3_client(s3_resource_ref: &S3Resource, credential_providers: Option<DefaultCredentialsChain>) -> error::Result<Arc<dyn ObjectStore>> {

    let s3_resource = s3_resource_ref.clone();
    let endpoint = render_endpoint(
        s3_resource.endpoint,
        s3_resource.use_ssl,
        s3_resource.port,
        s3_resource.path_style,
        s3_resource.bucket.clone(),
    );



    let mut store_builder = AmazonS3Builder::new()
        .with_client_options(ClientOptions::new().with_timeout_disabled()) // TODO: make it configurable maybe
        .with_region(s3_resource.region)
        .with_bucket_name(s3_resource.bucket)
        .with_endpoint(endpoint);

    if let Some(credentials_provider) = credential_providers {
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
        error::Error::InternalErr(format!(
            "Error building object store client: {}",
            err.to_string()
        ))
    })?;

    return Ok(Arc::new(store));
}

#[cfg(feature = "parquet")]
fn build_azure_blob_client(
    azure_blob_resource_ref: &AzureBlobResource,
) -> error::Result<Arc<dyn ObjectStore>> {
    let blob_resource = azure_blob_resource_ref.clone();

    let mut store_builder = MicrosoftAzureBuilder::new()
        .with_client_options(ClientOptions::new().with_timeout_disabled()) // TODO: make it configurable maybe
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
        error::Error::InternalErr(format!(
            "Error building object store client: {}",
            err.to_string()
        ))
    })?;

    return Ok(Arc::new(store));
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "typ", content = "value")]
pub enum ObjectStoreSettings {
    S3(S3Settings),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ObjectSettings {
    S3(S3Settings),
    Azure(AzureBlobResource),
}

#[cfg(feature = "parquet")]
pub async fn build_object_store_from_settings(settings: ObjectSettings) -> error::Result<Arc<dyn ObjectStore>> {
    match settings {
        ObjectSettings::S3(s3_settings) => build_s3_client_from_settings(s3_settings).await,
        ObjectSettings::Azure(azure_settings) => {
            let azure_blob_resource = azure_settings;
            build_azure_blob_client(&azure_blob_resource)
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
    pub store_logs: Option<bool>,
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
pub async fn build_s3_client_from_settings(settings: S3Settings) -> error::Result<Arc<dyn ObjectStore>> {
    let region = none_if_empty(settings.region).unwrap_or_else(|| std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()));
    let access_key = none_if_empty(settings.access_key);
    let secret_key = none_if_empty(settings.secret_key);
    let credentials_provider = if access_key.is_none() && secret_key.is_none() { 
        Some(DefaultCredentialsChain::builder()
            .region(Region::new(region.clone()))
            .build()
            .await)
     } else { None };
    let s3_resource = S3Resource {
        endpoint: none_if_empty(settings.endpoint).unwrap_or_else(|| std::env::var("S3_ENDPOINT")
            .unwrap_or_else(|_| format!("s3.{region}.amazonaws.com"))),
        bucket: settings.bucket.clone()
            .unwrap_or_else(|| std::env::var("S3_CACHE_BUCKET").unwrap_or_else(|_| "missingbucket".to_string())),
        region,
        access_key,
        secret_key,
        use_ssl: !settings.allow_http.unwrap_or(false),
        path_style: None,
        port: None,
        token: None,
    };
     

    build_s3_client(&s3_resource, credentials_provider)
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
        let creds = self.inner.provide_credentials().await.unwrap();
        Ok(Arc::new(Self::Credential {
            key_id: creds.access_key_id().to_string(),
            secret_key: creds.secret_access_key().to_string(),
            token: creds.session_token().map(|s| s.to_string()),
        }))
    }
}
