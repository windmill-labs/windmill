use crate::error;
use object_store::aws::AmazonS3Builder;
use object_store::ObjectStore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum LargeFileStorage {
    S3Storage(S3Storage),
    // TODO: Add a filesystem type here in the future if needed
}

#[derive(Serialize, Deserialize, Debug)]
pub struct S3Storage {
    pub s3_resource_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_resource: Option<bool>,
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

#[derive(Deserialize, Clone)]
pub struct S3Object {
    pub s3: String,
}

pub async fn get_etag_or_empty(s3_resource: &S3Resource, s3_object: S3Object) -> Option<String> {
    let s3_client = build_object_store_client(s3_resource);
    if s3_client.is_err() {
        return None;
    }

    let s3_key = object_store::path::Path::from(s3_object.s3);

    return s3_client
        .unwrap()
        .head(&s3_key)
        .await
        .ok()
        .map(|meta| meta.e_tag)
        .flatten();
}

pub fn render_endpoint(s3_resource: &S3Resource) -> String {
    let url_with_prefix = if s3_resource.endpoint.starts_with("http://")
        || s3_resource.endpoint.starts_with("https://")
    {
        s3_resource.endpoint.clone()
    } else if s3_resource.use_ssl {
        format!("https://{}", s3_resource.endpoint)
    } else {
        format!("http://{}", s3_resource.endpoint)
    };
    if s3_resource.port.is_some() {
        format!("{}:{}", url_with_prefix, s3_resource.port.unwrap())
    } else {
        url_with_prefix
    }
}

pub fn build_object_store_client(
    s3_resource_ref: &S3Resource,
) -> error::Result<Arc<dyn ObjectStore>> {
    let s3_resource = s3_resource_ref.clone();
    let endpoint = render_endpoint(&s3_resource);

    let mut store_builder = AmazonS3Builder::new()
        .with_region(s3_resource.region)
        .with_bucket_name(s3_resource.bucket)
        .with_endpoint(endpoint);

    if !s3_resource.use_ssl {
        store_builder = store_builder.with_allow_http(true)
    }

    if let Some(key) = s3_resource.access_key {
        store_builder = store_builder.with_access_key_id(key);
    }
    if let Some(secret_key) = s3_resource.secret_key {
        store_builder = store_builder.with_secret_access_key(secret_key);
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
