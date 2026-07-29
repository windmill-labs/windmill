// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use crate::aws::client::{S3Client, S3Config};
use crate::aws::credential::{
    EKSPodCredentialProvider, InstanceCredentialProvider, SessionProvider, TaskCredentialProvider,
    WebIdentityProvider,
};
use crate::aws::{
    AmazonS3, AwsCredential, AwsCredentialProvider, Checksum, S3ConditionalPut, S3CopyIfNotExists,
    STORE,
};
use crate::client::{http_connector, HttpConnector, TokenCredentialProvider};
use crate::config::ConfigValue;
use crate::{ClientConfigKey, ClientOptions, Result, RetryConfig, StaticCredentialProvider};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use itertools::Itertools;
use md5::{Digest, Md5};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use url::Url;

/// Default metadata endpoint
static DEFAULT_METADATA_ENDPOINT: &str = "http://169.254.169.254";

/// A specialized `Error` for object store-related errors
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Missing bucket name")]
    MissingBucketName,

    #[error("Missing AccessKeyId")]
    MissingAccessKeyId,

    #[error("Missing SecretAccessKey")]
    MissingSecretAccessKey,

    #[error("Unable parse source url. Url: {}, Error: {}", url, source)]
    UnableToParseUrl {
        source: url::ParseError,
        url: String,
    },

    #[error(
        "Unknown url scheme cannot be parsed into storage location: {}",
        scheme
    )]
    UnknownUrlScheme { scheme: String },

    #[error("URL did not match any known pattern for scheme: {}", url)]
    UrlNotRecognised { url: String },

    #[error("Configuration key: '{}' is not known.", key)]
    UnknownConfigurationKey { key: String },

    #[error("Invalid Zone suffix for bucket '{bucket}'")]
    ZoneSuffix { bucket: String },

    #[error("Invalid encryption type: {}. Valid values are \"AES256\", \"sse:kms\", \"sse:kms:dsse\" and \"sse-c\".", passed)]
    InvalidEncryptionType { passed: String },

    #[error(
        "Invalid encryption header values. Header: {}, source: {}",
        header,
        source
    )]
    InvalidEncryptionHeader {
        header: &'static str,
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
}

impl From<Error> for crate::Error {
    fn from(source: Error) -> Self {
        match source {
            Error::UnknownConfigurationKey { key } => {
                Self::UnknownConfigurationKey { store: STORE, key }
            }
            _ => Self::Generic {
                store: STORE,
                source: Box::new(source),
            },
        }
    }
}

/// Configure a connection to Amazon S3 using the specified credentials in
/// the specified Amazon region and bucket.
///
/// # Example
/// ```
/// # let REGION = "foo";
/// # let BUCKET_NAME = "foo";
/// # let ACCESS_KEY_ID = "foo";
/// # let SECRET_KEY = "foo";
/// # use object_store::aws::AmazonS3Builder;
/// let s3 = AmazonS3Builder::new()
///  .with_region(REGION)
///  .with_bucket_name(BUCKET_NAME)
///  .with_access_key_id(ACCESS_KEY_ID)
///  .with_secret_access_key(SECRET_KEY)
///  .build();
/// ```
#[derive(Debug, Default, Clone)]
pub struct AmazonS3Builder {
    /// Access key id
    access_key_id: Option<String>,
    /// Secret access_key
    secret_access_key: Option<String>,
    /// Region
    region: Option<String>,
    /// Bucket name
    bucket_name: Option<String>,
    /// Endpoint for communicating with AWS S3
    endpoint: Option<String>,
    /// Token to use for requests
    token: Option<String>,
    /// Url
    url: Option<String>,
    /// Retry config
    retry_config: RetryConfig,
    /// When set to true, fallback to IMDSv1
    imdsv1_fallback: ConfigValue<bool>,
    /// When set to true, virtual hosted style request has to be used
    virtual_hosted_style_request: ConfigValue<bool>,
    /// When set to true, S3 express is used
    s3_express: ConfigValue<bool>,
    /// When set to true, unsigned payload option has to be used
    unsigned_payload: ConfigValue<bool>,
    /// Checksum algorithm which has to be used for object integrity check during upload
    checksum_algorithm: Option<ConfigValue<Checksum>>,
    /// Metadata endpoint, see <https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-instance-metadata.html>
    metadata_endpoint: Option<String>,
    /// Container credentials URL, see <https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task-iam-roles.html>
    container_credentials_relative_uri: Option<String>,
    /// Container credentials full URL, see <https://docs.aws.amazon.com/sdkref/latest/guide/feature-container-credentials.html>
    container_credentials_full_uri: Option<String>,
    /// Container authorization token file, see <https://docs.aws.amazon.com/sdkref/latest/guide/feature-container-credentials.html>
    container_authorization_token_file: Option<String>,
    /// Client options
    client_options: ClientOptions,
    /// Credentials
    credentials: Option<AwsCredentialProvider>,
    /// Skip signing requests
    skip_signature: ConfigValue<bool>,
    /// Copy if not exists
    copy_if_not_exists: Option<ConfigValue<S3CopyIfNotExists>>,
    /// Put precondition
    conditional_put: ConfigValue<S3ConditionalPut>,
    /// Ignore tags
    disable_tagging: ConfigValue<bool>,
    /// Encryption (See [`S3EncryptionConfigKey`])
    encryption_type: Option<ConfigValue<S3EncryptionType>>,
    encryption_kms_key_id: Option<String>,
    encryption_bucket_key_enabled: Option<ConfigValue<bool>>,
    /// base64-encoded 256-bit customer encryption key for SSE-C.
    encryption_customer_key_base64: Option<String>,
    /// When set to true, charge requester for bucket operations
    request_payer: ConfigValue<bool>,
    /// The [`HttpConnector`] to use
    http_connector: Option<Arc<dyn HttpConnector>>,
}

/// Configuration keys for [`AmazonS3Builder`]
///
/// Configuration via keys can be done via [`AmazonS3Builder::with_config`]
///
/// # Example
/// ```
/// # use object_store::aws::{AmazonS3Builder, AmazonS3ConfigKey};
/// let builder = AmazonS3Builder::new()
///     .with_config("aws_access_key_id".parse().unwrap(), "my-access-key-id")
///     .with_config(AmazonS3ConfigKey::DefaultRegion, "my-default-region");
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Debug, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AmazonS3ConfigKey {
    /// AWS Access Key
    ///
    /// See [`AmazonS3Builder::with_access_key_id`] for details.
    ///
    /// Supported keys:
    /// - `aws_access_key_id`
    /// - `access_key_id`
    AccessKeyId,

    /// Secret Access Key
    ///
    /// See [`AmazonS3Builder::with_secret_access_key`] for details.
    ///
    /// Supported keys:
    /// - `aws_secret_access_key`
    /// - `secret_access_key`
    SecretAccessKey,

    /// Region
    ///
    /// See [`AmazonS3Builder::with_region`] for details.
    ///
    /// Supported keys:
    /// - `aws_region`
    /// - `region`
    Region,

    /// Default region
    ///
    /// See [`AmazonS3Builder::with_region`] for details.
    ///
    /// Supported keys:
    /// - `aws_default_region`
    /// - `default_region`
    DefaultRegion,

    /// Bucket name
    ///
    /// See [`AmazonS3Builder::with_bucket_name`] for details.
    ///
    /// Supported keys:
    /// - `aws_bucket`
    /// - `aws_bucket_name`
    /// - `bucket`
    /// - `bucket_name`
    Bucket,

    /// Sets custom endpoint for communicating with AWS S3.
    ///
    /// See [`AmazonS3Builder::with_endpoint`] for details.
    ///
    /// Supported keys:
    /// - `aws_endpoint`
    /// - `aws_endpoint_url`
    /// - `endpoint`
    /// - `endpoint_url`
    Endpoint,

    /// Token to use for requests (passed to underlying provider)
    ///
    /// See [`AmazonS3Builder::with_token`] for details.
    ///
    /// Supported keys:
    /// - `aws_session_token`
    /// - `aws_token`
    /// - `session_token`
    /// - `token`
    Token,

    /// Fall back to ImdsV1
    ///
    /// See [`AmazonS3Builder::with_imdsv1_fallback`] for details.
    ///
    /// Supported keys:
    /// - `aws_imdsv1_fallback`
    /// - `imdsv1_fallback`
    ImdsV1Fallback,

    /// If virtual hosted style request has to be used
    ///
    /// See [`AmazonS3Builder::with_virtual_hosted_style_request`] for details.
    ///
    /// Supported keys:
    /// - `aws_virtual_hosted_style_request`
    /// - `virtual_hosted_style_request`
    VirtualHostedStyleRequest,

    /// Avoid computing payload checksum when calculating signature.
    ///
    /// See [`AmazonS3Builder::with_unsigned_payload`] for details.
    ///
    /// Supported keys:
    /// - `aws_unsigned_payload`
    /// - `unsigned_payload`
    UnsignedPayload,

    /// Set the checksum algorithm for this client
    ///
    /// See [`AmazonS3Builder::with_checksum_algorithm`]
    Checksum,

    /// Set the instance metadata endpoint
    ///
    /// See [`AmazonS3Builder::with_metadata_endpoint`] for details.
    ///
    /// Supported keys:
    /// - `aws_metadata_endpoint`
    /// - `metadata_endpoint`
    MetadataEndpoint,

    /// Set the container credentials relative URI when used in ECS
    ///
    /// <https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task-iam-roles.html>
    ContainerCredentialsRelativeUri,

    /// Set the container credentials full URI when used in EKS
    ///
    /// <https://docs.aws.amazon.com/sdkref/latest/guide/feature-container-credentials.html>
    ContainerCredentialsFullUri,

    /// Set the authorization token in plain text when used in EKS to authenticate with ContainerCredentialsFullUri
    ///
    /// <https://docs.aws.amazon.com/sdkref/latest/guide/feature-container-credentials.html>
    ContainerAuthorizationTokenFile,

    /// Configure how to provide `copy_if_not_exists`
    ///
    /// See [`S3CopyIfNotExists`]
    CopyIfNotExists,

    /// Configure how to provide conditional put operations
    ///
    /// See [`S3ConditionalPut`]
    ConditionalPut,

    /// Skip signing request
    SkipSignature,

    /// Disable tagging objects
    ///
    /// This can be desirable if not supported by the backing store
    ///
    /// Supported keys:
    /// - `aws_disable_tagging`
    /// - `disable_tagging`
    DisableTagging,

    /// Enable Support for S3 Express One Zone
    ///
    /// Supported keys:
    /// - `aws_s3_express`
    /// - `s3_express`
    S3Express,

    /// Enable Support for S3 Requester Pays
    ///
    /// Supported keys:
    /// - `aws_request_payer`
    /// - `request_payer`
    RequestPayer,

    /// Client options
    Client(ClientConfigKey),

    /// Encryption options
    Encryption(S3EncryptionConfigKey),
}

impl AsRef<str> for AmazonS3ConfigKey {
    fn as_ref(&self) -> &str {
        match self {
            Self::AccessKeyId => "aws_access_key_id",
            Self::SecretAccessKey => "aws_secret_access_key",
            Self::Region => "aws_region",
            Self::Bucket => "aws_bucket",
            Self::Endpoint => "aws_endpoint",
            Self::Token => "aws_session_token",
            Self::ImdsV1Fallback => "aws_imdsv1_fallback",
            Self::VirtualHostedStyleRequest => "aws_virtual_hosted_style_request",
            Self::S3Express => "aws_s3_express",
            Self::DefaultRegion => "aws_default_region",
            Self::MetadataEndpoint => "aws_metadata_endpoint",
            Self::UnsignedPayload => "aws_unsigned_payload",
            Self::Checksum => "aws_checksum_algorithm",
            Self::ContainerCredentialsRelativeUri => "aws_container_credentials_relative_uri",
            Self::ContainerCredentialsFullUri => "aws_container_credentials_full_uri",
            Self::ContainerAuthorizationTokenFile => "aws_container_authorization_token_file",
            Self::SkipSignature => "aws_skip_signature",
            Self::CopyIfNotExists => "aws_copy_if_not_exists",
            Self::ConditionalPut => "aws_conditional_put",
            Self::DisableTagging => "aws_disable_tagging",
            Self::RequestPayer => "aws_request_payer",
            Self::Client(opt) => opt.as_ref(),
            Self::Encryption(opt) => opt.as_ref(),
        }
    }
}

impl FromStr for AmazonS3ConfigKey {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aws_access_key_id" | "access_key_id" => Ok(Self::AccessKeyId),
            "aws_secret_access_key" | "secret_access_key" => Ok(Self::SecretAccessKey),
            "aws_default_region" | "default_region" => Ok(Self::DefaultRegion),
            "aws_region" | "region" => Ok(Self::Region),
            "aws_bucket" | "aws_bucket_name" | "bucket_name" | "bucket" => Ok(Self::Bucket),
            "aws_endpoint_url" | "aws_endpoint" | "endpoint_url" | "endpoint" => Ok(Self::Endpoint),
            "aws_session_token" | "aws_token" | "session_token" | "token" => Ok(Self::Token),
            "aws_virtual_hosted_style_request" | "virtual_hosted_style_request" => {
                Ok(Self::VirtualHostedStyleRequest)
            }
            "aws_s3_express" | "s3_express" => Ok(Self::S3Express),
            "aws_imdsv1_fallback" | "imdsv1_fallback" => Ok(Self::ImdsV1Fallback),
            "aws_metadata_endpoint" | "metadata_endpoint" => Ok(Self::MetadataEndpoint),
            "aws_unsigned_payload" | "unsigned_payload" => Ok(Self::UnsignedPayload),
            "aws_checksum_algorithm" | "checksum_algorithm" => Ok(Self::Checksum),
            "aws_container_credentials_relative_uri" => Ok(Self::ContainerCredentialsRelativeUri),
            "aws_container_credentials_full_uri" => Ok(Self::ContainerCredentialsFullUri),
            "aws_container_authorization_token_file" => Ok(Self::ContainerAuthorizationTokenFile),
            "aws_skip_signature" | "skip_signature" => Ok(Self::SkipSignature),
            "aws_copy_if_not_exists" | "copy_if_not_exists" => Ok(Self::CopyIfNotExists),
            "aws_conditional_put" | "conditional_put" => Ok(Self::ConditionalPut),
            "aws_disable_tagging" | "disable_tagging" => Ok(Self::DisableTagging),
            "aws_request_payer" | "request_payer" => Ok(Self::RequestPayer),
            // Backwards compatibility
            "aws_allow_http" => Ok(Self::Client(ClientConfigKey::AllowHttp)),
            "aws_server_side_encryption" => Ok(Self::Encryption(
                S3EncryptionConfigKey::ServerSideEncryption,
            )),
            "aws_sse_kms_key_id" => Ok(Self::Encryption(S3EncryptionConfigKey::KmsKeyId)),
            "aws_sse_bucket_key_enabled" => {
                Ok(Self::Encryption(S3EncryptionConfigKey::BucketKeyEnabled))
            }
            "aws_sse_customer_key_base64" => Ok(Self::Encryption(
                S3EncryptionConfigKey::CustomerEncryptionKey,
            )),
            _ => match s.strip_prefix("aws_").unwrap_or(s).parse() {
                Ok(key) => Ok(Self::Client(key)),
                Err(_) => Err(Error::UnknownConfigurationKey { key: s.into() }.into()),
            },
        }
    }
}

impl AmazonS3Builder {
    /// Create a new [`AmazonS3Builder`] with default values.
    pub fn new() -> Self {
        Default::default()
    }

    /// Fill the [`AmazonS3Builder`] with regular AWS environment variables
    ///
    /// All environment variables starting with `AWS_` will be evaluated. Names must
    /// match acceptable input to [`AmazonS3ConfigKey::from_str`]. Only upper-case environment
    /// variables are accepted.
    ///
    /// Some examples of variables extracted from environment:
    /// * `AWS_ACCESS_KEY_ID` -> access_key_id
    /// * `AWS_SECRET_ACCESS_KEY` -> secret_access_key
    /// * `AWS_DEFAULT_REGION` -> region
    /// * `AWS_ENDPOINT` -> endpoint
    /// * `AWS_SESSION_TOKEN` -> token
    /// * `AWS_CONTAINER_CREDENTIALS_RELATIVE_URI` -> <https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task-iam-roles.html>
    /// * `AWS_CONTAINER_CREDENTIALS_FULL_URI` -> <https://docs.aws.amazon.com/sdkref/latest/guide/feature-container-credentials.html>
    /// * `AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE` -> <https://docs.aws.amazon.com/sdkref/latest/guide/feature-container-credentials.html>
    /// * `AWS_ALLOW_HTTP` -> set to "true" to permit HTTP connections without TLS
    /// * `AWS_REQUEST_PAYER` -> set to "true" to permit operations on requester-pays buckets.
    /// # Example
    /// ```
    /// use object_store::aws::AmazonS3Builder;
    ///
    /// let s3 = AmazonS3Builder::from_env()
    ///     .with_bucket_name("foo")
    ///     .build();
    /// ```
    pub fn from_env() -> Self {
        let mut builder: Self = Default::default();

        for (os_key, os_value) in std::env::vars_os() {
            if let (Some(key), Some(value)) = (os_key.to_str(), os_value.to_str()) {
                if key.starts_with("AWS_") {
                    if let Ok(config_key) = key.to_ascii_lowercase().parse() {
                        builder = builder.with_config(config_key, value);
                    }
                }
            }
        }

        builder
    }

    /// Parse available connection info form a well-known storage URL.
    ///
    /// The supported url schemes are:
    ///
    /// - `s3://<bucket>/<path>`
    /// - `s3a://<bucket>/<path>`
    /// - `https://s3.<region>.amazonaws.com/<bucket>`
    /// - `https://<bucket>.s3.<region>.amazonaws.com`
    /// - `https://ACCOUNT_ID.r2.cloudflarestorage.com/bucket`
    ///
    /// Note: Settings derived from the URL will override any others set on this builder
    ///
    /// # Example
    /// ```
    /// use object_store::aws::AmazonS3Builder;
    ///
    /// let s3 = AmazonS3Builder::from_env()
    ///     .with_url("s3://bucket/path")
    ///     .build();
    /// ```
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set an option on the builder via a key - value pair.
    pub fn with_config(mut self, key: AmazonS3ConfigKey, value: impl Into<String>) -> Self {
        match key {
            AmazonS3ConfigKey::AccessKeyId => self.access_key_id = Some(value.into()),
            AmazonS3ConfigKey::SecretAccessKey => self.secret_access_key = Some(value.into()),
            AmazonS3ConfigKey::Region => self.region = Some(value.into()),
            AmazonS3ConfigKey::Bucket => self.bucket_name = Some(value.into()),
            AmazonS3ConfigKey::Endpoint => self.endpoint = Some(value.into()),
            AmazonS3ConfigKey::Token => self.token = Some(value.into()),
            AmazonS3ConfigKey::ImdsV1Fallback => self.imdsv1_fallback.parse(value),
            AmazonS3ConfigKey::VirtualHostedStyleRequest => {
                self.virtual_hosted_style_request.parse(value)
            }
            AmazonS3ConfigKey::S3Express => self.s3_express.parse(value),
            AmazonS3ConfigKey::DefaultRegion => {
                self.region = self.region.or_else(|| Some(value.into()))
            }
            AmazonS3ConfigKey::MetadataEndpoint => self.metadata_endpoint = Some(value.into()),
            AmazonS3ConfigKey::UnsignedPayload => self.unsigned_payload.parse(value),
            AmazonS3ConfigKey::Checksum => {
                self.checksum_algorithm = Some(ConfigValue::Deferred(value.into()))
            }
            AmazonS3ConfigKey::ContainerCredentialsRelativeUri => {
                self.container_credentials_relative_uri = Some(value.into())
            }
            AmazonS3ConfigKey::ContainerCredentialsFullUri => {
                self.container_credentials_full_uri = Some(value.into());
            }
            AmazonS3ConfigKey::ContainerAuthorizationTokenFile => {
                self.container_authorization_token_file = Some(value.into());
            }
            AmazonS3ConfigKey::Client(key) => {
                self.client_options = self.client_options.with_config(key, value)
            }
            AmazonS3ConfigKey::SkipSignature => self.skip_signature.parse(value),
            AmazonS3ConfigKey::DisableTagging => self.disable_tagging.parse(value),
            AmazonS3ConfigKey::CopyIfNotExists => {
                self.copy_if_not_exists = Some(ConfigValue::Deferred(value.into()))
            }
            AmazonS3ConfigKey::ConditionalPut => {
                self.conditional_put = ConfigValue::Deferred(value.into())
            }
            AmazonS3ConfigKey::RequestPayer => {
                self.request_payer = ConfigValue::Deferred(value.into())
            }
            AmazonS3ConfigKey::Encryption(key) => match key {
                S3EncryptionConfigKey::ServerSideEncryption => {
                    self.encryption_type = Some(ConfigValue::Deferred(value.into()))
                }
                S3EncryptionConfigKey::KmsKeyId => self.encryption_kms_key_id = Some(value.into()),
                S3EncryptionConfigKey::BucketKeyEnabled => {
                    self.encryption_bucket_key_enabled = Some(ConfigValue::Deferred(value.into()))
                }
                S3EncryptionConfigKey::CustomerEncryptionKey => {
                    self.encryption_customer_key_base64 = Some(value.into())
                }
            },
        };
        self
    }

    /// Get config value via a [`AmazonS3ConfigKey`].
    ///
    /// # Example
    /// ```
    /// use object_store::aws::{AmazonS3Builder, AmazonS3ConfigKey};
    ///
    /// let builder = AmazonS3Builder::from_env()
    ///     .with_bucket_name("foo");
    /// let bucket_name = builder.get_config_value(&AmazonS3ConfigKey::Bucket).unwrap_or_default();
    /// assert_eq!("foo", &bucket_name);
    /// ```
    pub fn get_config_value(&self, key: &AmazonS3ConfigKey) -> Option<String> {
        match key {
            AmazonS3ConfigKey::AccessKeyId => self.access_key_id.clone(),
            AmazonS3ConfigKey::SecretAccessKey => self.secret_access_key.clone(),
            AmazonS3ConfigKey::Region | AmazonS3ConfigKey::DefaultRegion => self.region.clone(),
            AmazonS3ConfigKey::Bucket => self.bucket_name.clone(),
            AmazonS3ConfigKey::Endpoint => self.endpoint.clone(),
            AmazonS3ConfigKey::Token => self.token.clone(),
            AmazonS3ConfigKey::ImdsV1Fallback => Some(self.imdsv1_fallback.to_string()),
            AmazonS3ConfigKey::VirtualHostedStyleRequest => {
                Some(self.virtual_hosted_style_request.to_string())
            }
            AmazonS3ConfigKey::S3Express => Some(self.s3_express.to_string()),
            AmazonS3ConfigKey::MetadataEndpoint => self.metadata_endpoint.clone(),
            AmazonS3ConfigKey::UnsignedPayload => Some(self.unsigned_payload.to_string()),
            AmazonS3ConfigKey::Checksum => {
                self.checksum_algorithm.as_ref().map(ToString::to_string)
            }
            AmazonS3ConfigKey::Client(key) => self.client_options.get_config_value(key),
            AmazonS3ConfigKey::ContainerCredentialsRelativeUri => {
                self.container_credentials_relative_uri.clone()
            }
            AmazonS3ConfigKey::ContainerCredentialsFullUri => {
                self.container_credentials_full_uri.clone()
            }
            AmazonS3ConfigKey::ContainerAuthorizationTokenFile => {
                self.container_authorization_token_file.clone()
            }
            AmazonS3ConfigKey::SkipSignature => Some(self.skip_signature.to_string()),
            AmazonS3ConfigKey::CopyIfNotExists => {
                self.copy_if_not_exists.as_ref().map(ToString::to_string)
            }
            AmazonS3ConfigKey::ConditionalPut => Some(self.conditional_put.to_string()),
            AmazonS3ConfigKey::DisableTagging => Some(self.disable_tagging.to_string()),
            AmazonS3ConfigKey::RequestPayer => Some(self.request_payer.to_string()),
            AmazonS3ConfigKey::Encryption(key) => match key {
                S3EncryptionConfigKey::ServerSideEncryption => {
                    self.encryption_type.as_ref().map(ToString::to_string)
                }
                S3EncryptionConfigKey::KmsKeyId => self.encryption_kms_key_id.clone(),
                S3EncryptionConfigKey::BucketKeyEnabled => self
                    .encryption_bucket_key_enabled
                    .as_ref()
                    .map(ToString::to_string),
                S3EncryptionConfigKey::CustomerEncryptionKey => {
                    self.encryption_customer_key_base64.clone()
                }
            },
        }
    }

    /// Sets properties on this builder based on a URL
    ///
    /// This is a separate member function to allow fallible computation to
    /// be deferred until [`Self::build`] which in turn allows deriving [`Clone`]
    fn parse_url(&mut self, url: &str) -> Result<()> {
        let parsed = Url::parse(url).map_err(|source| {
            let url = url.into();
            Error::UnableToParseUrl { url, source }
        })?;

        let host = parsed
            .host_str()
            .ok_or_else(|| Error::UrlNotRecognised { url: url.into() })?;

        match parsed.scheme() {
            "s3" | "s3a" => self.bucket_name = Some(host.to_string()),
            "https" => match host.splitn(4, '.').collect_tuple() {
                Some(("s3", region, "amazonaws", "com")) => {
                    self.region = Some(region.to_string());
                    let bucket = parsed.path_segments().into_iter().flatten().next();
                    if let Some(bucket) = bucket {
                        self.bucket_name = Some(bucket.into());
                    }
                }
                Some((bucket, "s3", region, "amazonaws.com")) => {
                    self.bucket_name = Some(bucket.to_string());
                    self.region = Some(region.to_string());
                    self.virtual_hosted_style_request = true.into();
                }
                Some((account, "r2", "cloudflarestorage", "com")) => {
                    self.region = Some("auto".to_string());
                    let endpoint = format!("https://{account}.r2.cloudflarestorage.com");
                    self.endpoint = Some(endpoint);

                    let bucket = parsed.path_segments().into_iter().flatten().next();
                    if let Some(bucket) = bucket {
                        self.bucket_name = Some(bucket.into());
                    }
                }
                _ => return Err(Error::UrlNotRecognised { url: url.into() }.into()),
            },
            scheme => {
                let scheme = scheme.into();
                return Err(Error::UnknownUrlScheme { scheme }.into());
            }
        };
        Ok(())
    }

    /// Set the AWS Access Key
    pub fn with_access_key_id(mut self, access_key_id: impl Into<String>) -> Self {
        self.access_key_id = Some(access_key_id.into());
        self
    }

    /// Set the AWS Secret Access Key
    pub fn with_secret_access_key(mut self, secret_access_key: impl Into<String>) -> Self {
        self.secret_access_key = Some(secret_access_key.into());
        self
    }

    /// Set the AWS Session Token to use for requests
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the region, defaults to `us-east-1`
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the bucket_name (required)
    pub fn with_bucket_name(mut self, bucket_name: impl Into<String>) -> Self {
        self.bucket_name = Some(bucket_name.into());
        self
    }

    /// Sets the endpoint for communicating with AWS S3, defaults to the [region endpoint]
    ///
    /// For example, this might be set to `"http://localhost:4566:`
    /// for testing against a localstack instance.
    ///
    /// The `endpoint` field should be consistent with [`Self::with_virtual_hosted_style_request`],
    /// i.e. if `virtual_hosted_style_request` is set to true then `endpoint`
    /// should have the bucket name included.
    ///
    /// By default, only HTTPS schemes are enabled. To connect to an HTTP endpoint, enable
    /// [`Self::with_allow_http`].
    ///
    /// [region endpoint]: https://docs.aws.amazon.com/general/latest/gr/s3.html
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set the credential provider overriding any other options
    pub fn with_credentials(mut self, credentials: AwsCredentialProvider) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// Sets what protocol is allowed. If `allow_http` is :
    /// * false (default):  Only HTTPS are allowed
    /// * true:  HTTP and HTTPS are allowed
    pub fn with_allow_http(mut self, allow_http: bool) -> Self {
        self.client_options = self.client_options.with_allow_http(allow_http);
        self
    }

    /// Sets if virtual hosted style request has to be used.
    ///
    /// If `virtual_hosted_style_request` is:
    /// * false (default):  Path style request is used
    /// * true:  Virtual hosted style request is used
    ///
    /// If the `endpoint` is provided then it should be
    /// consistent with `virtual_hosted_style_request`.
    /// i.e. if `virtual_hosted_style_request` is set to true
    /// then `endpoint` should have bucket name included.
    pub fn with_virtual_hosted_style_request(mut self, virtual_hosted_style_request: bool) -> Self {
        self.virtual_hosted_style_request = virtual_hosted_style_request.into();
        self
    }

    /// Configure this as an S3 Express One Zone Bucket
    pub fn with_s3_express(mut self, s3_express: bool) -> Self {
        self.s3_express = s3_express.into();
        self
    }

    /// Set the retry configuration
    pub fn with_retry(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    /// By default instance credentials will only be fetched over [IMDSv2], as AWS recommends
    /// against having IMDSv1 enabled on EC2 instances as it is vulnerable to [SSRF attack]
    ///
    /// However, certain deployment environments, such as those running old versions of kube2iam,
    /// may not support IMDSv2. This option will enable automatic fallback to using IMDSv1
    /// if the token endpoint returns a 403 error indicating that IMDSv2 is not supported.
    ///
    /// This option has no effect if not using instance credentials
    ///
    /// [IMDSv2]: https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/configuring-instance-metadata-service.html
    /// [SSRF attack]: https://aws.amazon.com/blogs/security/defense-in-depth-open-firewalls-reverse-proxies-ssrf-vulnerabilities-ec2-instance-metadata-service/
    ///
    pub fn with_imdsv1_fallback(mut self) -> Self {
        self.imdsv1_fallback = true.into();
        self
    }

    /// Sets if unsigned payload option has to be used.
    /// See [unsigned payload option](https://docs.aws.amazon.com/AmazonS3/latest/API/sig-v4-header-based-auth.html)
    /// * false (default): Signed payload option is used, where the checksum for the request body is computed and included when constructing a canonical request.
    /// * true: Unsigned payload option is used. `UNSIGNED-PAYLOAD` literal is included when constructing a canonical request,
    pub fn with_unsigned_payload(mut self, unsigned_payload: bool) -> Self {
        self.unsigned_payload = unsigned_payload.into();
        self
    }

    /// If enabled, [`AmazonS3`] will not fetch credentials and will not sign requests
    ///
    /// This can be useful when interacting with public S3 buckets that deny authorized requests
    pub fn with_skip_signature(mut self, skip_signature: bool) -> Self {
        self.skip_signature = skip_signature.into();
        self
    }

    /// Sets the [checksum algorithm] which has to be used for object integrity check during upload.
    ///
    /// [checksum algorithm]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/checking-object-integrity.html
    pub fn with_checksum_algorithm(mut self, checksum_algorithm: Checksum) -> Self {
        // Convert to String to enable deferred parsing of config
        self.checksum_algorithm = Some(checksum_algorithm.into());
        self
    }

    /// Set the [instance metadata endpoint](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-instance-metadata.html),
    /// used primarily within AWS EC2.
    ///
    /// This defaults to the IPv4 endpoint: http://169.254.169.254. One can alternatively use the IPv6
    /// endpoint http://fd00:ec2::254.
    pub fn with_metadata_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.metadata_endpoint = Some(endpoint.into());
        self
    }

    /// Set the proxy_url to be used by the underlying client
    pub fn with_proxy_url(mut self, proxy_url: impl Into<String>) -> Self {
        self.client_options = self.client_options.with_proxy_url(proxy_url);
        self
    }

    /// Set a trusted proxy CA certificate
    pub fn with_proxy_ca_certificate(mut self, proxy_ca_certificate: impl Into<String>) -> Self {
        self.client_options = self
            .client_options
            .with_proxy_ca_certificate(proxy_ca_certificate);
        self
    }

    /// Set a list of hosts to exclude from proxy connections
    pub fn with_proxy_excludes(mut self, proxy_excludes: impl Into<String>) -> Self {
        self.client_options = self.client_options.with_proxy_excludes(proxy_excludes);
        self
    }

    /// Sets the client options, overriding any already set
    pub fn with_client_options(mut self, options: ClientOptions) -> Self {
        self.client_options = options;
        self
    }

    /// Configure how to provide `copy_if_not_exists`
    pub fn with_copy_if_not_exists(mut self, config: S3CopyIfNotExists) -> Self {
        self.copy_if_not_exists = Some(config.into());
        self
    }

    /// Configure how to provide conditional put operations.
    /// if not set, the default value will be `S3ConditionalPut::ETagMatch`
    pub fn with_conditional_put(mut self, config: S3ConditionalPut) -> Self {
        self.conditional_put = config.into();
        self
    }

    /// If set to `true` will ignore any tags provided to put_opts
    pub fn with_disable_tagging(mut self, ignore: bool) -> Self {
        self.disable_tagging = ignore.into();
        self
    }

    /// Use SSE-KMS for server side encryption.
    pub fn with_sse_kms_encryption(mut self, kms_key_id: impl Into<String>) -> Self {
        self.encryption_type = Some(ConfigValue::Parsed(S3EncryptionType::SseKms));
        if let Some(kms_key_id) = kms_key_id.into().into() {
            self.encryption_kms_key_id = Some(kms_key_id);
        }
        self
    }

    /// Use dual server side encryption for server side encryption.
    pub fn with_dsse_kms_encryption(mut self, kms_key_id: impl Into<String>) -> Self {
        self.encryption_type = Some(ConfigValue::Parsed(S3EncryptionType::DsseKms));
        if let Some(kms_key_id) = kms_key_id.into().into() {
            self.encryption_kms_key_id = Some(kms_key_id);
        }
        self
    }

    /// Use SSE-C for server side encryption.
    /// Must pass the *base64-encoded* 256-bit customer encryption key.
    pub fn with_ssec_encryption(mut self, customer_key_base64: impl Into<String>) -> Self {
        self.encryption_type = Some(ConfigValue::Parsed(S3EncryptionType::SseC));
        self.encryption_customer_key_base64 = customer_key_base64.into().into();
        self
    }

    /// Set whether to enable bucket key for server side encryption. This overrides
    /// the bucket default setting for bucket keys.
    ///
    /// When bucket keys are disabled, each object is encrypted with a unique data key.
    /// When bucket keys are enabled, a single data key is used for the entire bucket,
    /// reducing overhead of encryption.
    pub fn with_bucket_key(mut self, enabled: bool) -> Self {
        self.encryption_bucket_key_enabled = Some(ConfigValue::Parsed(enabled));
        self
    }

    /// Set whether to charge requester for bucket operations.
    ///
    /// <https://docs.aws.amazon.com/AmazonS3/latest/userguide/RequesterPaysBuckets.html>
    pub fn with_request_payer(mut self, enabled: bool) -> Self {
        self.request_payer = ConfigValue::Parsed(enabled);
        self
    }

    /// The [`HttpConnector`] to use
    ///
    /// On non-WASM32 platforms uses [`reqwest`] by default, on WASM32 platforms must be provided
    pub fn with_http_connector<C: HttpConnector>(mut self, connector: C) -> Self {
        self.http_connector = Some(Arc::new(connector));
        self
    }

    /// Create a [`AmazonS3`] instance from the provided values,
    /// consuming `self`.
    pub fn build(mut self) -> Result<AmazonS3> {
        if let Some(url) = self.url.take() {
            self.parse_url(&url)?;
        }

        let http = http_connector(self.http_connector)?;

        let bucket = self.bucket_name.ok_or(Error::MissingBucketName)?;
        let region = self.region.unwrap_or_else(|| "us-east-1".to_string());
        let checksum = self.checksum_algorithm.map(|x| x.get()).transpose()?;
        let copy_if_not_exists = self.copy_if_not_exists.map(|x| x.get()).transpose()?;

        let credentials = if let Some(credentials) = self.credentials {
            credentials
        } else if self.access_key_id.is_some() || self.secret_access_key.is_some() {
            match (self.access_key_id, self.secret_access_key, self.token) {
                (Some(key_id), Some(secret_key), token) => {
                    info!("Using Static credential provider");
                    let credential = AwsCredential {
                        key_id,
                        secret_key,
                        token,
                    };
                    Arc::new(StaticCredentialProvider::new(credential)) as _
                }
                (None, Some(_), _) => return Err(Error::MissingAccessKeyId.into()),
                (Some(_), None, _) => return Err(Error::MissingSecretAccessKey.into()),
                (None, None, _) => unreachable!(),
            }
        } else if let (Ok(token_path), Ok(role_arn)) = (
            std::env::var("AWS_WEB_IDENTITY_TOKEN_FILE"),
            std::env::var("AWS_ROLE_ARN"),
        ) {
            // TODO: Replace with `AmazonS3Builder::credentials_from_env`
            info!("Using WebIdentity credential provider");

            let session_name = std::env::var("AWS_ROLE_SESSION_NAME")
                .unwrap_or_else(|_| "WebIdentitySession".to_string());

            let endpoint = format!("https://sts.{region}.amazonaws.com");

            // Disallow non-HTTPs requests
            let options = self.client_options.clone().with_allow_http(false);

            let token = WebIdentityProvider {
                token_path,
                session_name,
                role_arn,
                endpoint,
            };

            Arc::new(TokenCredentialProvider::new(
                token,
                http.connect(&options)?,
                self.retry_config.clone(),
            )) as _
        } else if let Some(uri) = self.container_credentials_relative_uri {
            info!("Using Task credential provider");

            let options = self.client_options.clone().with_allow_http(true);

            Arc::new(TaskCredentialProvider {
                url: format!("http://169.254.170.2{uri}"),
                retry: self.retry_config.clone(),
                // The instance metadata endpoint is access over HTTP
                client: http.connect(&options)?,
                cache: Default::default(),
            }) as _
        } else if let (Some(full_uri), Some(token_file)) = (
            self.container_credentials_full_uri,
            self.container_authorization_token_file,
        ) {
            info!("Using EKS Pod Identity credential provider");

            let options = self.client_options.clone().with_allow_http(true);

            Arc::new(EKSPodCredentialProvider {
                url: full_uri,
                token_file,
                retry: self.retry_config.clone(),
                client: http.connect(&options)?,
                cache: Default::default(),
            }) as _
        } else {
            info!("Using Instance credential provider");

            let token = InstanceCredentialProvider {
                imdsv1_fallback: self.imdsv1_fallback.get()?,
                metadata_endpoint: self
                    .metadata_endpoint
                    .unwrap_or_else(|| DEFAULT_METADATA_ENDPOINT.into()),
            };

            Arc::new(TokenCredentialProvider::new(
                token,
                http.connect(&self.client_options.metadata_options())?,
                self.retry_config.clone(),
            )) as _
        };

        let (session_provider, zonal_endpoint) = match self.s3_express.get()? {
            true => {
                let zone = parse_bucket_az(&bucket).ok_or_else(|| {
                    let bucket = bucket.clone();
                    Error::ZoneSuffix { bucket }
                })?;

                // https://docs.aws.amazon.com/AmazonS3/latest/userguide/s3-express-Regions-and-Zones.html
                let endpoint = format!("https://{bucket}.s3express-{zone}.{region}.amazonaws.com");

                let session = Arc::new(
                    TokenCredentialProvider::new(
                        SessionProvider {
                            endpoint: endpoint.clone(),
                            region: region.clone(),
                            credentials: Arc::clone(&credentials),
                        },
                        http.connect(&self.client_options)?,
                        self.retry_config.clone(),
                    )
                    .with_min_ttl(Duration::from_secs(60)), // Credentials only valid for 5 minutes
                );
                (Some(session as _), Some(endpoint))
            }
            false => (None, None),
        };

        // If `endpoint` is provided it's assumed to be consistent with `virtual_hosted_style_request` or `s3_express`.
        // For example, if `virtual_hosted_style_request` is true then `endpoint` should have bucket name included.
        let virtual_hosted = self.virtual_hosted_style_request.get()?;
        let bucket_endpoint = match (&self.endpoint, zonal_endpoint, virtual_hosted) {
            (Some(endpoint), _, true) => endpoint.clone(),
            (Some(endpoint), _, false) => format!("{}/{}", endpoint.trim_end_matches("/"), bucket),
            (None, Some(endpoint), _) => endpoint,
            (None, None, true) => format!("https://{bucket}.s3.{region}.amazonaws.com"),
            (None, None, false) => format!("https://s3.{region}.amazonaws.com/{bucket}"),
        };

        let encryption_headers = if let Some(encryption_type) = self.encryption_type {
            S3EncryptionHeaders::try_new(
                &encryption_type.get()?,
                self.encryption_kms_key_id,
                self.encryption_bucket_key_enabled
                    .map(|val| val.get())
                    .transpose()?,
                self.encryption_customer_key_base64,
            )?
        } else {
            S3EncryptionHeaders::default()
        };

        let config = S3Config {
            region,
            endpoint: self.endpoint,
            bucket,
            bucket_endpoint,
            credentials,
            session_provider,
            retry_config: self.retry_config,
            client_options: self.client_options,
            sign_payload: !self.unsigned_payload.get()?,
            skip_signature: self.skip_signature.get()?,
            disable_tagging: self.disable_tagging.get()?,
            checksum,
            copy_if_not_exists,
            conditional_put: self.conditional_put.get()?,
            encryption_headers,
            request_payer: self.request_payer.get()?,
        };

        let http_client = http.connect(&config.client_options)?;
        let client = Arc::new(S3Client::new(config, http_client));

        Ok(AmazonS3 { client })
    }
}

/// Extracts the AZ from a S3 Express One Zone bucket name
///
/// <https://docs.aws.amazon.com/AmazonS3/latest/userguide/directory-bucket-naming-rules.html>
fn parse_bucket_az(bucket: &str) -> Option<&str> {
    Some(bucket.strip_suffix("--x-s3")?.rsplit_once("--")?.1)
}

/// Encryption configuration options for S3.
///
/// These options are used to configure server-side encryption for S3 objects.
/// To configure them, pass them to [`AmazonS3Builder::with_config`].
///
/// [SSE-S3]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/UsingServerSideEncryption.html
/// [SSE-KMS]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/UsingKMSEncryption.html
/// [DSSE-KMS]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/UsingDSSEncryption.html
/// [SSE-C]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/ServerSideEncryptionCustomerKeys.html
#[derive(PartialEq, Eq, Hash, Clone, Debug, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum S3EncryptionConfigKey {
    /// Type of encryption to use. If set, must be one of "AES256" (SSE-S3), "aws:kms" (SSE-KMS), "aws:kms:dsse" (DSSE-KMS) or "sse-c".
    ServerSideEncryption,
    /// The KMS key ID to use for server-side encryption. If set, ServerSideEncryption
    /// must be "aws:kms" or "aws:kms:dsse".
    KmsKeyId,
    /// If set to true, will use the bucket's default KMS key for server-side encryption.
    /// If set to false, will disable the use of the bucket's default KMS key for server-side encryption.
    BucketKeyEnabled,

    /// The base64 encoded, 256-bit customer encryption key to use for server-side encryption.
    /// If set, ServerSideEncryption must be "sse-c".
    CustomerEncryptionKey,
}

impl AsRef<str> for S3EncryptionConfigKey {
    fn as_ref(&self) -> &str {
        match self {
            Self::ServerSideEncryption => "aws_server_side_encryption",
            Self::KmsKeyId => "aws_sse_kms_key_id",
            Self::BucketKeyEnabled => "aws_sse_bucket_key_enabled",
            Self::CustomerEncryptionKey => "aws_sse_customer_key_base64",
        }
    }
}

#[derive(Debug, Clone)]
enum S3EncryptionType {
    S3,
    SseKms,
    DsseKms,
    SseC,
}

impl crate::config::Parse for S3EncryptionType {
    fn parse(s: &str) -> Result<Self> {
        match s {
            "AES256" => Ok(Self::S3),
            "aws:kms" => Ok(Self::SseKms),
            "aws:kms:dsse" => Ok(Self::DsseKms),
            "sse-c" => Ok(Self::SseC),
            _ => Err(Error::InvalidEncryptionType { passed: s.into() }.into()),
        }
    }
}

impl From<&S3EncryptionType> for &'static str {
    fn from(value: &S3EncryptionType) -> Self {
        match value {
            S3EncryptionType::S3 => "AES256",
            S3EncryptionType::SseKms => "aws:kms",
            S3EncryptionType::DsseKms => "aws:kms:dsse",
            S3EncryptionType::SseC => "sse-c",
        }
    }
}

impl std::fmt::Display for S3EncryptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.into())
    }
}

/// A sequence of headers to be sent for write requests that specify server-side
/// encryption.
///
/// Whether these headers are sent depends on both the kind of encryption set
/// and the kind of request being made.
#[derive(Default, Clone, Debug)]
pub(super) struct S3EncryptionHeaders(pub HeaderMap);

impl S3EncryptionHeaders {
    fn try_new(
        encryption_type: &S3EncryptionType,
        encryption_kms_key_id: Option<String>,
        bucket_key_enabled: Option<bool>,
        encryption_customer_key_base64: Option<String>,
    ) -> Result<Self> {
        let mut headers = HeaderMap::new();
        match encryption_type {
            S3EncryptionType::S3 | S3EncryptionType::SseKms | S3EncryptionType::DsseKms => {
                headers.insert(
                    "x-amz-server-side-encryption",
                    HeaderValue::from_static(encryption_type.into()),
                );
                if let Some(key_id) = encryption_kms_key_id {
                    headers.insert(
                        "x-amz-server-side-encryption-aws-kms-key-id",
                        key_id
                            .try_into()
                            .map_err(|err| Error::InvalidEncryptionHeader {
                                header: "kms-key-id",
                                source: Box::new(err),
                            })?,
                    );
                }
                if let Some(bucket_key_enabled) = bucket_key_enabled {
                    headers.insert(
                        "x-amz-server-side-encryption-bucket-key-enabled",
                        HeaderValue::from_static(if bucket_key_enabled { "true" } else { "false" }),
                    );
                }
            }
            S3EncryptionType::SseC => {
                headers.insert(
                    "x-amz-server-side-encryption-customer-algorithm",
                    HeaderValue::from_static("AES256"),
                );
                if let Some(key) = encryption_customer_key_base64 {
                    let mut header_value: HeaderValue =
                        key.clone()
                            .try_into()
                            .map_err(|err| Error::InvalidEncryptionHeader {
                                header: "x-amz-server-side-encryption-customer-key",
                                source: Box::new(err),
                            })?;
                    header_value.set_sensitive(true);
                    headers.insert("x-amz-server-side-encryption-customer-key", header_value);

                    let decoded_key = BASE64_STANDARD.decode(key.as_bytes()).map_err(|err| {
                        Error::InvalidEncryptionHeader {
                            header: "x-amz-server-side-encryption-customer-key",
                            source: Box::new(err),
                        }
                    })?;
                    let mut hasher = Md5::new();
                    hasher.update(decoded_key);
                    let md5 = BASE64_STANDARD.encode(hasher.finalize());
                    let mut md5_header_value: HeaderValue =
                        md5.try_into()
                            .map_err(|err| Error::InvalidEncryptionHeader {
                                header: "x-amz-server-side-encryption-customer-key-MD5",
                                source: Box::new(err),
                            })?;
                    md5_header_value.set_sensitive(true);
                    headers.insert(
                        "x-amz-server-side-encryption-customer-key-MD5",
                        md5_header_value,
                    );
                } else {
                    return Err(Error::InvalidEncryptionHeader {
                        header: "x-amz-server-side-encryption-customer-key",
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "Missing customer key",
                        )),
                    }
                    .into());
                }
            }
        }
        Ok(Self(headers))
    }
}

impl From<S3EncryptionHeaders> for HeaderMap {
    fn from(headers: S3EncryptionHeaders) -> Self {
        headers.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn s3_test_config_from_map() {
        let aws_access_key_id = "object_store:fake_access_key_id".to_string();
        let aws_secret_access_key = "object_store:fake_secret_key".to_string();
        let aws_default_region = "object_store:fake_default_region".to_string();
        let aws_endpoint = "object_store:fake_endpoint".to_string();
        let aws_session_token = "object_store:fake_session_token".to_string();
        let options = HashMap::from([
            ("aws_access_key_id", aws_access_key_id.clone()),
            ("aws_secret_access_key", aws_secret_access_key),
            ("aws_default_region", aws_default_region.clone()),
            ("aws_endpoint", aws_endpoint.clone()),
            ("aws_session_token", aws_session_token.clone()),
            ("aws_unsigned_payload", "true".to_string()),
            ("aws_checksum_algorithm", "sha256".to_string()),
        ]);

        let builder = options
            .into_iter()
            .fold(AmazonS3Builder::new(), |builder, (key, value)| {
                builder.with_config(key.parse().unwrap(), value)
            })
            .with_config(AmazonS3ConfigKey::SecretAccessKey, "new-secret-key");

        assert_eq!(builder.access_key_id.unwrap(), aws_access_key_id.as_str());
        assert_eq!(builder.secret_access_key.unwrap(), "new-secret-key");
        assert_eq!(builder.region.unwrap(), aws_default_region);
        assert_eq!(builder.endpoint.unwrap(), aws_endpoint);
        assert_eq!(builder.token.unwrap(), aws_session_token);
        assert_eq!(
            builder.checksum_algorithm.unwrap().get().unwrap(),
            Checksum::SHA256
        );
        assert!(builder.unsigned_payload.get().unwrap());
    }

    #[test]
    fn s3_test_config_get_value() {
        let aws_access_key_id = "object_store:fake_access_key_id".to_string();
        let aws_secret_access_key = "object_store:fake_secret_key".to_string();
        let aws_default_region = "object_store:fake_default_region".to_string();
        let aws_endpoint = "object_store:fake_endpoint".to_string();
        let aws_session_token = "object_store:fake_session_token".to_string();

        let builder = AmazonS3Builder::new()
            .with_config(AmazonS3ConfigKey::AccessKeyId, &aws_access_key_id)
            .with_config(AmazonS3ConfigKey::SecretAccessKey, &aws_secret_access_key)
            .with_config(AmazonS3ConfigKey::DefaultRegion, &aws_default_region)
            .with_config(AmazonS3ConfigKey::Endpoint, &aws_endpoint)
            .with_config(AmazonS3ConfigKey::Token, &aws_session_token)
            .with_config(AmazonS3ConfigKey::UnsignedPayload, "true")
            .with_config("aws_server_side_encryption".parse().unwrap(), "AES256")
            .with_config("aws_sse_kms_key_id".parse().unwrap(), "some_key_id")
            .with_config("aws_sse_bucket_key_enabled".parse().unwrap(), "true")
            .with_config(
                "aws_sse_customer_key_base64".parse().unwrap(),
                "some_customer_key",
            );

        assert_eq!(
            builder
                .get_config_value(&AmazonS3ConfigKey::AccessKeyId)
                .unwrap(),
            aws_access_key_id
        );
        assert_eq!(
            builder
                .get_config_value(&AmazonS3ConfigKey::SecretAccessKey)
                .unwrap(),
            aws_secret_access_key
        );
        assert_eq!(
            builder
                .get_config_value(&AmazonS3ConfigKey::DefaultRegion)
                .unwrap(),
            aws_default_region
        );
        assert_eq!(
            builder
                .get_config_value(&AmazonS3ConfigKey::Endpoint)
                .unwrap(),
            aws_endpoint
        );
        assert_eq!(
            builder.get_config_value(&AmazonS3ConfigKey::Token).unwrap(),
            aws_session_token
        );
        assert_eq!(
            builder
                .get_config_value(&AmazonS3ConfigKey::UnsignedPayload)
                .unwrap(),
            "true"
        );
        assert_eq!(
            builder
                .get_config_value(&"aws_server_side_encryption".parse().unwrap())
                .unwrap(),
            "AES256"
        );
        assert_eq!(
            builder
                .get_config_value(&"aws_sse_kms_key_id".parse().unwrap())
                .unwrap(),
            "some_key_id"
        );
        assert_eq!(
            builder
                .get_config_value(&"aws_sse_bucket_key_enabled".parse().unwrap())
                .unwrap(),
            "true"
        );
        assert_eq!(
            builder
                .get_config_value(&"aws_sse_customer_key_base64".parse().unwrap())
                .unwrap(),
            "some_customer_key"
        );
    }

    #[test]
    fn s3_default_region() {
        let builder = AmazonS3Builder::new()
            .with_bucket_name("foo")
            .build()
            .unwrap();
        assert_eq!(builder.client.config.region, "us-east-1");
    }

    #[test]
    fn s3_test_bucket_endpoint() {
        let builder = AmazonS3Builder::new()
            .with_endpoint("http://some.host:1234")
            .with_bucket_name("foo")
            .build()
            .unwrap();
        assert_eq!(
            builder.client.config.bucket_endpoint,
            "http://some.host:1234/foo"
        );

        let builder = AmazonS3Builder::new()
            .with_endpoint("http://some.host:1234/")
            .with_bucket_name("foo")
            .build()
            .unwrap();
        assert_eq!(
            builder.client.config.bucket_endpoint,
            "http://some.host:1234/foo"
        );
    }

    #[test]
    fn s3_test_urls() {
        let mut builder = AmazonS3Builder::new();
        builder.parse_url("s3://bucket/path").unwrap();
        assert_eq!(builder.bucket_name, Some("bucket".to_string()));

        let mut builder = AmazonS3Builder::new();
        builder
            .parse_url("s3://buckets.can.have.dots/path")
            .unwrap();
        assert_eq!(
            builder.bucket_name,
            Some("buckets.can.have.dots".to_string())
        );

        let mut builder = AmazonS3Builder::new();
        builder
            .parse_url("https://s3.region.amazonaws.com")
            .unwrap();
        assert_eq!(builder.region, Some("region".to_string()));

        let mut builder = AmazonS3Builder::new();
        builder
            .parse_url("https://s3.region.amazonaws.com/bucket")
            .unwrap();
        assert_eq!(builder.region, Some("region".to_string()));
        assert_eq!(builder.bucket_name, Some("bucket".to_string()));

        let mut builder = AmazonS3Builder::new();
        builder
            .parse_url("https://s3.region.amazonaws.com/bucket.with.dot/path")
            .unwrap();
        assert_eq!(builder.region, Some("region".to_string()));
        assert_eq!(builder.bucket_name, Some("bucket.with.dot".to_string()));

        let mut builder = AmazonS3Builder::new();
        builder
            .parse_url("https://bucket.s3.region.amazonaws.com")
            .unwrap();
        assert_eq!(builder.bucket_name, Some("bucket".to_string()));
        assert_eq!(builder.region, Some("region".to_string()));
        assert!(builder.virtual_hosted_style_request.get().unwrap());

        let mut builder = AmazonS3Builder::new();
        builder
            .parse_url("https://account123.r2.cloudflarestorage.com/bucket-123")
            .unwrap();

        assert_eq!(builder.bucket_name, Some("bucket-123".to_string()));
        assert_eq!(builder.region, Some("auto".to_string()));
        assert_eq!(
            builder.endpoint,
            Some("https://account123.r2.cloudflarestorage.com".to_string())
        );

        let err_cases = [
            "mailto://bucket/path",
            "https://s3.bucket.mydomain.com",
            "https://s3.bucket.foo.amazonaws.com",
            "https://bucket.mydomain.region.amazonaws.com",
            "https://bucket.s3.region.bar.amazonaws.com",
            "https://bucket.foo.s3.amazonaws.com",
        ];
        let mut builder = AmazonS3Builder::new();
        for case in err_cases {
            builder.parse_url(case).unwrap_err();
        }
    }

    #[tokio::test]
    async fn s3_test_proxy_url() {
        let s3 = AmazonS3Builder::new()
            .with_access_key_id("access_key_id")
            .with_secret_access_key("secret_access_key")
            .with_region("region")
            .with_bucket_name("bucket_name")
            .with_allow_http(true)
            .with_proxy_url("https://example.com")
            .build();

        assert!(s3.is_ok());

        let err = AmazonS3Builder::new()
            .with_access_key_id("access_key_id")
            .with_secret_access_key("secret_access_key")
            .with_region("region")
            .with_bucket_name("bucket_name")
            .with_allow_http(true)
            .with_proxy_url("asdf://example.com")
            .build()
            .unwrap_err()
            .to_string();

        assert_eq!("Generic HTTP client error: builder error", err);
    }

    #[test]
    fn test_invalid_config() {
        let err = AmazonS3Builder::new()
            .with_config(AmazonS3ConfigKey::ImdsV1Fallback, "enabled")
            .with_bucket_name("bucket")
            .with_region("region")
            .build()
            .unwrap_err()
            .to_string();

        assert_eq!(
            err,
            "Generic Config error: failed to parse \"enabled\" as boolean"
        );

        let err = AmazonS3Builder::new()
            .with_config(AmazonS3ConfigKey::Checksum, "md5")
            .with_bucket_name("bucket")
            .with_region("region")
            .build()
            .unwrap_err()
            .to_string();

        assert_eq!(
            err,
            "Generic Config error: \"md5\" is not a valid checksum algorithm"
        );
    }

    #[test]
    fn test_parse_bucket_az() {
        let cases = [
            ("bucket-base-name--usw2-az1--x-s3", Some("usw2-az1")),
            ("bucket-base--name--azid--x-s3", Some("azid")),
            ("bucket-base-name", None),
            ("bucket-base-name--x-s3", None),
        ];

        for (bucket, expected) in cases {
            assert_eq!(parse_bucket_az(bucket), expected)
        }
    }

    #[test]
    fn aws_test_client_opts() {
        let key = "AWS_PROXY_URL";
        if let Ok(config_key) = key.to_ascii_lowercase().parse() {
            assert_eq!(
                AmazonS3ConfigKey::Client(ClientConfigKey::ProxyUrl),
                config_key
            );
        } else {
            panic!("{} not propagated as ClientConfigKey", key);
        }
    }

    #[test]
    fn test_builder_eks_with_config() {
        let builder = AmazonS3Builder::new()
            .with_bucket_name("some-bucket")
            .with_config(
                AmazonS3ConfigKey::ContainerCredentialsFullUri,
                "https://127.0.0.1/eks-credentials",
            )
            .with_config(
                AmazonS3ConfigKey::ContainerAuthorizationTokenFile,
                "/tmp/fake-bearer-token",
            );

        let s3 = builder.build().expect("should build successfully");
        let creds = &s3.client.config.credentials;
        let debug_str = format!("{:?}", creds);
        assert!(
            debug_str.contains("EKSPodCredentialProvider"),
            "expected EKS provider but got: {debug_str}"
        );
    }
}
