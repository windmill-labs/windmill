use crate::error;
use object_store::azure::MicrosoftAzureBuilder;
use object_store::ObjectStore;
use object_store::{aws::AmazonS3Builder, ClientOptions};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum LargeFileStorage {
    S3Storage(S3Storage),
    AzureBlobStorage(AzureBlobStorage),
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
    S3Resource(S3Resource),
    AzureBlobResource(AzureBlobResource),
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
    pub path_style: bool,
    pub port: Option<u16>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AzureBlobResource {
    #[serde(rename = "endpoint")]
    pub endpoint: Option<String>,
    #[serde(rename = "useSSL")]
    pub use_ssl: bool,
    #[serde(rename = "accountName")]
    pub account_name: String,
    #[serde(rename = "containerName")]
    pub container_name: String,
    #[serde(rename = "accessKey")]
    pub access_key: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct S3Object {
    pub s3: String,
}

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

pub fn render_endpoint(raw_endpoint: String, use_ssl: bool, port: Option<u16>) -> String {
    let url_with_prefix =
        if raw_endpoint.starts_with("http://") || raw_endpoint.starts_with("https://") {
            raw_endpoint.clone()
        } else if use_ssl {
            format!("https://{}", raw_endpoint)
        } else {
            format!("http://{}", raw_endpoint)
        };
    if port.is_some() {
        format!("{}:{}", url_with_prefix, port.unwrap())
    } else {
        url_with_prefix
    }
}

pub fn build_object_store_client(
    resource_ref: &ObjectStoreResource,
) -> error::Result<Arc<dyn ObjectStore>> {
    match resource_ref {
        ObjectStoreResource::S3Resource(s3_resource_ref) => build_s3_client(&s3_resource_ref),
        ObjectStoreResource::AzureBlobResource(azure_blob_resource_ref) => {
            build_azure_blob_client(&azure_blob_resource_ref)
        }
    }
}

fn build_s3_client(s3_resource_ref: &S3Resource) -> error::Result<Arc<dyn ObjectStore>> {
    let s3_resource = s3_resource_ref.clone();
    let endpoint = render_endpoint(s3_resource.endpoint, s3_resource.use_ssl, s3_resource.port);

    let mut store_builder = AmazonS3Builder::new()
        .with_client_options(ClientOptions::new().with_timeout_disabled()) // TODO: make it configurable maybe
        .with_region(s3_resource.region)
        .with_bucket_name(s3_resource.bucket)
        .with_endpoint(endpoint);

    if !s3_resource.use_ssl {
        store_builder = store_builder.with_allow_http(true)
    }

    if let Some(key) = s3_resource.access_key {
        if key != "" {
            store_builder = store_builder.with_access_key_id(key);
        }
    }
    if let Some(secret_key) = s3_resource.secret_key {
        if secret_key != "" {
            store_builder = store_builder.with_secret_access_key(secret_key);
        }
    }
    if !s3_resource.path_style {
        store_builder = store_builder.with_virtual_hosted_style_request(s3_resource.path_style);
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

fn build_azure_blob_client(
    azure_blob_resource_ref: &AzureBlobResource,
) -> error::Result<Arc<dyn ObjectStore>> {
    let blob_resource = azure_blob_resource_ref.clone();

    let mut store_builder = MicrosoftAzureBuilder::new()
        .with_client_options(ClientOptions::new().with_timeout_disabled()) // TODO: make it configurable maybe
        .with_account(blob_resource.account_name)
        .with_container_name(blob_resource.container_name);

    if let Some(endpoint) = blob_resource.endpoint {
        if endpoint != "" {
            let endpoint = render_endpoint(endpoint, blob_resource.use_ssl, None);
            store_builder = store_builder.with_endpoint(endpoint)
        }
    }

    if !blob_resource.use_ssl {
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
