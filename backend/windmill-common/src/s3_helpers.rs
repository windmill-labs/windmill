#[cfg(feature = "parquet")]
use crate::error;
use crate::error::to_anyhow;
use crate::utils::rd_string;
#[cfg(feature = "parquet")]
use aws_sdk_sts::config::ProvideCredentials;
#[cfg(feature = "parquet")]
use axum::async_trait;
use bytes::Bytes;
use datafusion::arrow::array::{RecordBatch, RecordBatchWriter};
use datafusion::arrow::error::ArrowError;
use datafusion::arrow::json::writer::JsonArray;
use datafusion::arrow::{csv, json};
use datafusion::parquet::arrow::ArrowWriter;
use futures::{StreamExt, TryStreamExt};
#[cfg(feature = "parquet")]
use object_store::aws::AwsCredential;
#[cfg(feature = "parquet")]
use object_store::azure::MicrosoftAzureBuilder;
#[cfg(feature = "parquet")]
use object_store::ObjectStore;
#[cfg(feature = "parquet")]
use object_store::{aws::AmazonS3Builder, ClientOptions};
#[cfg(feature = "parquet")]
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::io::Write;
#[cfg(feature = "parquet")]
use std::sync::Arc;
use std::sync::Mutex;
#[cfg(feature = "parquet")]
use tokio::sync::RwLock;
use tokio::task;
use windmill_parser_sql::S3ModeFormat;

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
        s3_resource.endpoint,
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
        error::Error::internal_err(format!(
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
        error::Error::internal_err(format!(
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
pub async fn build_object_store_from_settings(
    settings: ObjectSettings,
) -> error::Result<Arc<dyn ObjectStore>> {
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
enum RecordBatchWriterEnum {
    Parquet(ArrowWriter<ChannelWriter>),
    Csv(csv::Writer<ChannelWriter>),
    Json(json::Writer<ChannelWriter, JsonArray>),
}

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

pub async fn convert_ndjson<E: Into<anyhow::Error>>(
    mut stream: impl TryStreamExt<Item = Result<serde_json::Value, E>> + Unpin,
    output_format: S3ModeFormat,
) -> anyhow::Result<impl StreamExt<Item = bytes::Bytes>> {
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
                ArrowWriter::try_new(ChannelWriter { sender: tx }, Arc::new(schema), None)
                    .map_err(to_anyhow)?,
            ),

            S3ModeFormat::Csv => {
                RecordBatchWriterEnum::Csv(csv::Writer::new(ChannelWriter { sender: tx }))
            }
            S3ModeFormat::Json => {
                RecordBatchWriterEnum::Json(json::Writer::<_, JsonArray>::new(ChannelWriter {
                    sender: tx,
                }))
            }
        })));

    // This spawn is so that the data is sent in the background. Else the function would deadlock
    // when hitting the mpsc channel limit
    task::spawn(async move {
        while let Some(batch_result) = datafusion_stream.next().await {
            let batch: RecordBatch = batch_result.map_err(to_anyhow)?;
            let writer = writer.clone();
            // Writer calls blocking_send which would crash if called from the async context
            task::spawn_blocking(move || {
                // SAFETY: We await so the code is actually sequential, lock unwrap cannot panic
                // Second unwrap is ok because we initialized the option with Some
                writer.lock().unwrap().as_mut().unwrap().write(&batch)
            })
            .await??;
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

struct ChannelWriter {
    sender: tokio::sync::mpsc::Sender<Bytes>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let data = buf.to_vec().into();
        self.sender.blocking_send(data).map_err(|e| {
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
