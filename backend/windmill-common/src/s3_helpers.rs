use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::config::Credentials;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum LargeFileStorage {
    S3Storage(S3Storage),
    // TODO: Add a filesystem type here in the future if needed
}

#[derive(Serialize, Deserialize, Debug)]
pub struct S3Storage {
    pub s3_resource_path: String,
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
    let s3_client = build_s3_client(s3_resource);

    let s3_bucket = s3_resource.bucket.clone();
    let s3_key = s3_object.s3;
    let s3_object_metadata = s3_client
        .head_object()
        .bucket(s3_bucket)
        .key(s3_key)
        .send()
        .await;
    s3_object_metadata
        .ok()
        .map(|res| res.e_tag().map(|et| et.to_string()))
        .flatten()
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

pub fn build_s3_client(s3_resource_ref: &S3Resource) -> aws_sdk_s3::Client {
    let s3_resource = s3_resource_ref.clone();

    let endpoint = render_endpoint(&s3_resource);
    let mut s3_config_builder = aws_sdk_s3::Config::builder()
        .endpoint_url(endpoint)
        .behavior_version(BehaviorVersion::latest())
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
