use chrono::{DateTime, Utc};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FilesystemSettings {
    pub root_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FilesystemStorage {
    pub root_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_resource: Option<bool>,
    pub advanced_permissions: Option<Vec<S3PermissionRule>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum LargeFileStorage {
    S3Storage(S3Storage),
    AzureBlobStorage(AzureBlobStorage),
    S3AwsOidc(S3Storage),
    AzureWorkloadIdentity(AzureBlobStorage),
    GoogleCloudStorage(GoogleCloudStorage),
    FilesystemStorage(FilesystemStorage),
}

impl LargeFileStorage {
    pub fn get_s3_resource_path(&self) -> &str {
        match self {
            LargeFileStorage::S3Storage(s3_lfs) => &s3_lfs.s3_resource_path,
            LargeFileStorage::S3AwsOidc(s3_lfs) => &s3_lfs.s3_resource_path,
            LargeFileStorage::AzureBlobStorage(az_lfs) => &az_lfs.azure_blob_resource_path,
            LargeFileStorage::AzureWorkloadIdentity(az_lfs) => &az_lfs.azure_blob_resource_path,
            LargeFileStorage::GoogleCloudStorage(gcs_lfs) => &gcs_lfs.gcs_resource_path,
            LargeFileStorage::FilesystemStorage(fs_lfs) => &fs_lfs.root_path,
        }
    }
    pub fn is_public_resource(&self) -> bool {
        match self {
            LargeFileStorage::S3Storage(lfs) => lfs.public_resource,
            LargeFileStorage::S3AwsOidc(lfs) => lfs.public_resource,
            LargeFileStorage::AzureBlobStorage(lfs) => lfs.public_resource,
            LargeFileStorage::AzureWorkloadIdentity(lfs) => lfs.public_resource,
            LargeFileStorage::GoogleCloudStorage(glfs) => glfs.public_resource,
            LargeFileStorage::FilesystemStorage(fs_lfs) => fs_lfs.public_resource,
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
            LargeFileStorage::FilesystemStorage(fs_lfs) => fs_lfs.advanced_permissions.as_ref(),
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
    pub allow: S3Permission,
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
                        _ => S3Permission::empty(),
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
    Filesystem(FilesystemSettings),
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
    Filesystem,
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

#[derive(Debug, PartialEq)]
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
    Filesystem(FilesystemSettings),
}

impl ObjectSettings {
    pub fn get_bucket(&self) -> Option<&String> {
        match self {
            ObjectSettings::S3(s3_settings) => s3_settings.bucket.as_ref(),
            ObjectSettings::Azure(azure_settings) => Some(&azure_settings.container_name),
            ObjectSettings::AwsOidc(s3_aws_oidc_settings) => Some(&s3_aws_oidc_settings.bucket),
            ObjectSettings::Gcs(gcs_settings) => Some(&gcs_settings.bucket),
            ObjectSettings::Filesystem(fs_settings) => Some(&fs_settings.root_path),
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
    pub allow_http: Option<bool>,
    pub path_style: Option<bool>,
    pub store_logs: Option<bool>,
    pub port: Option<u16>,
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

#[derive(Clone, Copy, Debug)]
pub enum S3ModeFormat {
    Json,
    Csv,
    Parquet,
}

pub fn s3_mode_extension(format: S3ModeFormat) -> &'static str {
    match format {
        S3ModeFormat::Json => "json",
        S3ModeFormat::Csv => "csv",
        S3ModeFormat::Parquet => "parquet",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_permission_serde_custom_format() {
        let perm = S3Permission::READ | S3Permission::WRITE;
        let serialized = serde_json::to_string(&perm).unwrap();
        assert_eq!(serialized, "\"read,write\"");

        let deserialized: S3Permission = serde_json::from_str("\"read,write\"").unwrap();
        assert_eq!(deserialized, S3Permission::READ | S3Permission::WRITE);

        // Unknown permissions are silently ignored
        let deserialized: S3Permission =
            serde_json::from_str("\"read,unknown,delete\"").unwrap();
        assert_eq!(deserialized, S3Permission::READ | S3Permission::DELETE);

        // All four permissions
        let all: S3Permission =
            serde_json::from_str("\"read,write,delete,list\"").unwrap();
        assert_eq!(
            all,
            S3Permission::READ | S3Permission::WRITE | S3Permission::DELETE | S3Permission::LIST
        );

        // Empty string gives empty permission set
        let empty: S3Permission = serde_json::from_str("\"\"").unwrap();
        assert_eq!(empty, S3Permission::empty());
    }

    #[test]
    fn test_s3_resource_endpoint_fallback() {
        let resource = S3Resource {
            bucket: "b".to_string(),
            region: "".to_string(),
            endpoint: "".to_string(),
            use_ssl: true,
            access_key: None,
            secret_key: None,
            path_style: None,
            token: None,
            expiration: None,
            port: None,
        };

        // Both empty, no fallback → defaults to us-east-1
        assert_eq!(
            resource.endpoint_with_region_fallback(None),
            "s3.us-east-1.amazonaws.com"
        );

        // Both empty, fallback provided
        assert_eq!(
            resource.endpoint_with_region_fallback(Some("eu-west-1".to_string())),
            "s3.eu-west-1.amazonaws.com"
        );

        // Region set, endpoint empty → use region
        let with_region = S3Resource {
            region: "ap-southeast-1".to_string(),
            ..resource.clone()
        };
        assert_eq!(
            with_region.endpoint_with_region_fallback(Some("ignored".to_string())),
            "s3.ap-southeast-1.amazonaws.com"
        );

        // Endpoint set → return as-is
        let with_endpoint = S3Resource {
            endpoint: "custom.s3.endpoint.com".to_string(),
            ..resource.clone()
        };
        assert_eq!(
            with_endpoint.endpoint_with_region_fallback(Some("ignored".to_string())),
            "custom.s3.endpoint.com"
        );
    }

    #[test]
    fn test_lfs_methods_filesystem() {
        let rules = vec![S3PermissionRule {
            pattern: "**/*.csv".to_string(),
            allow: S3Permission::READ,
        }];
        let lfs = LargeFileStorage::FilesystemStorage(FilesystemStorage {
            root_path: "/data/workspace".to_string(),
            public_resource: Some(true),
            advanced_permissions: Some(rules.clone()),
        });

        assert_eq!(lfs.get_s3_resource_path(), "/data/workspace");
        assert!(lfs.is_public_resource());
        let perms = lfs.get_advanced_permissions().unwrap();
        assert_eq!(perms.len(), 1);
        assert_eq!(perms[0].pattern, "**/*.csv");

        // Default public_resource = false when None
        let lfs_default = LargeFileStorage::FilesystemStorage(FilesystemStorage {
            root_path: "/tmp".to_string(),
            public_resource: None,
            advanced_permissions: None,
        });
        assert!(!lfs_default.is_public_resource());
        assert!(lfs_default.get_advanced_permissions().is_none());
    }

    #[test]
    fn test_bundle_format_from_string() {
        assert_eq!(BundleFormat::from_string("esm"), Some(BundleFormat::Esm));
        assert_eq!(BundleFormat::from_string("cjs"), Some(BundleFormat::Cjs));
        assert_eq!(BundleFormat::from_string("unknown"), None);
        assert_eq!(BundleFormat::from_string(""), None);
        assert_eq!(BundleFormat::from_string("ESM"), None); // case sensitive
    }
}
