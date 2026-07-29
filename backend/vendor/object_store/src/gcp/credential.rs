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

use super::client::GoogleCloudStorageClient;
use crate::client::builder::HttpRequestBuilder;
use crate::client::retry::RetryExt;
use crate::client::token::TemporaryToken;
use crate::client::{HttpClient, HttpError, TokenProvider};
use crate::gcp::{GcpSigningCredentialProvider, STORE};
use crate::util::{hex_digest, hex_encode, STRICT_ENCODE_SET};
use crate::{RetryConfig, StaticCredentialProvider};
use async_trait::async_trait;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use futures::TryFutureExt;
use http::{HeaderMap, Method};
use itertools::Itertools;
use percent_encoding::utf8_percent_encode;
use ring::signature::RsaKeyPair;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;
use url::Url;

pub(crate) const DEFAULT_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

pub(crate) const DEFAULT_GCS_BASE_URL: &str = "https://storage.googleapis.com";

const DEFAULT_GCS_PLAYLOAD_STRING: &str = "UNSIGNED-PAYLOAD";
const DEFAULT_GCS_SIGN_BLOB_HOST: &str = "storage.googleapis.com";

const DEFAULT_METADATA_HOST: &str = "metadata.google.internal";
const DEFAULT_METADATA_IP: &str = "169.254.169.254";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unable to open service account file from {}: {}", path.display(), source)]
    OpenCredentials {
        source: std::io::Error,
        path: PathBuf,
    },

    #[error("Unable to decode service account file: {}", source)]
    DecodeCredentials { source: serde_json::Error },

    #[error("No RSA key found in pem file")]
    MissingKey,

    #[error("Invalid RSA key: {}", source)]
    InvalidKey {
        #[from]
        source: ring::error::KeyRejected,
    },

    #[error("Error signing: {}", source)]
    Sign { source: ring::error::Unspecified },

    #[error("Error encoding jwt payload: {}", source)]
    Encode { source: serde_json::Error },

    #[error("Unsupported key encoding: {}", encoding)]
    UnsupportedKey { encoding: String },

    #[error("Error performing token request: {}", source)]
    TokenRequest {
        source: crate::client::retry::RetryError,
    },

    #[error("Error getting token response body: {}", source)]
    TokenResponseBody { source: HttpError },
}

impl From<Error> for crate::Error {
    fn from(value: Error) -> Self {
        Self::Generic {
            store: STORE,
            source: Box::new(value),
        }
    }
}

/// A Google Cloud Storage Credential for signing
#[derive(Debug)]
pub struct GcpSigningCredential {
    /// The email of the service account
    pub email: String,

    /// An optional RSA private key
    ///
    /// If provided this will be used to sign the URL, otherwise a call will be made to
    /// [`iam.serviceAccounts.signBlob`]. This allows supporting credential sources
    /// that don't expose the service account private key, e.g. [IMDS].
    ///
    /// [IMDS]: https://cloud.google.com/docs/authentication/get-id-token#metadata-server
    /// [`iam.serviceAccounts.signBlob`]: https://cloud.google.com/storage/docs/authentication/creating-signatures
    pub private_key: Option<ServiceAccountKey>,
}

/// A private RSA key for a service account
#[derive(Debug)]
pub struct ServiceAccountKey(RsaKeyPair);

impl ServiceAccountKey {
    /// Parses a pem-encoded RSA key
    pub fn from_pem(encoded: &[u8]) -> Result<Self> {
        use rustls_pemfile::Item;
        use std::io::Cursor;

        let mut cursor = Cursor::new(encoded);
        let mut reader = BufReader::new(&mut cursor);

        // Reading from string is infallible
        match rustls_pemfile::read_one(&mut reader).unwrap() {
            Some(Item::Pkcs8Key(key)) => Self::from_pkcs8(key.secret_pkcs8_der()),
            Some(Item::Pkcs1Key(key)) => Self::from_der(key.secret_pkcs1_der()),
            _ => Err(Error::MissingKey),
        }
    }

    /// Parses an unencrypted PKCS#8-encoded RSA private key.
    pub fn from_pkcs8(key: &[u8]) -> Result<Self> {
        Ok(Self(RsaKeyPair::from_pkcs8(key)?))
    }

    /// Parses an unencrypted PKCS#8-encoded RSA private key.
    pub fn from_der(key: &[u8]) -> Result<Self> {
        Ok(Self(RsaKeyPair::from_der(key)?))
    }

    fn sign(&self, string_to_sign: &str) -> Result<String> {
        let mut signature = vec![0; self.0.public().modulus_len()];
        self.0
            .sign(
                &ring::signature::RSA_PKCS1_SHA256,
                &ring::rand::SystemRandom::new(),
                string_to_sign.as_bytes(),
                &mut signature,
            )
            .map_err(|source| Error::Sign { source })?;

        Ok(hex_encode(&signature))
    }
}

/// A Google Cloud Storage Credential
#[derive(Debug, Eq, PartialEq)]
pub struct GcpCredential {
    /// An HTTP bearer token
    pub bearer: String,
}

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Default, serde::Serialize)]
pub(crate) struct JwtHeader<'a> {
    /// The type of JWS: it can only be "JWT" here
    ///
    /// Defined in [RFC7515#4.1.9](https://tools.ietf.org/html/rfc7515#section-4.1.9).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typ: Option<&'a str>,
    /// The algorithm used
    ///
    /// Defined in [RFC7515#4.1.1](https://tools.ietf.org/html/rfc7515#section-4.1.1).
    pub alg: &'a str,
    /// Content type
    ///
    /// Defined in [RFC7519#5.2](https://tools.ietf.org/html/rfc7519#section-5.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cty: Option<&'a str>,
    /// JSON Key URL
    ///
    /// Defined in [RFC7515#4.1.2](https://tools.ietf.org/html/rfc7515#section-4.1.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jku: Option<&'a str>,
    /// Key ID
    ///
    /// Defined in [RFC7515#4.1.4](https://tools.ietf.org/html/rfc7515#section-4.1.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kid: Option<&'a str>,
    /// X.509 URL
    ///
    /// Defined in [RFC7515#4.1.5](https://tools.ietf.org/html/rfc7515#section-4.1.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5u: Option<&'a str>,
    /// X.509 certificate thumbprint
    ///
    /// Defined in [RFC7515#4.1.7](https://tools.ietf.org/html/rfc7515#section-4.1.7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5t: Option<&'a str>,
}

#[derive(serde::Serialize)]
struct TokenClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    scope: &'a str,
    exp: u64,
    iat: u64,
}

#[derive(serde::Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    id_token: Option<String>,
}

/// Self-signed JWT (JSON Web Token).
///
/// # References
/// - <https://google.aip.dev/auth/4111>
#[derive(Debug)]
pub(crate) struct SelfSignedJwt {
    issuer: String,
    scope: String,
    private_key: ServiceAccountKey,
    key_id: String,
}

impl SelfSignedJwt {
    /// Create a new [`SelfSignedJwt`]
    pub(crate) fn new(
        key_id: String,
        issuer: String,
        private_key: ServiceAccountKey,
        scope: String,
    ) -> Result<Self> {
        Ok(Self {
            issuer,
            scope,
            private_key,
            key_id,
        })
    }
}

#[async_trait]
impl TokenProvider for SelfSignedJwt {
    type Credential = GcpCredential;

    /// Fetch a fresh token
    async fn fetch_token(
        &self,
        _client: &HttpClient,
        _retry: &RetryConfig,
    ) -> crate::Result<TemporaryToken<Arc<GcpCredential>>> {
        let now = seconds_since_epoch();
        let exp = now + 3600;

        let claims = TokenClaims {
            iss: &self.issuer,
            sub: &self.issuer,
            scope: &self.scope,
            iat: now,
            exp,
        };

        let jwt_header = b64_encode_obj(&JwtHeader {
            alg: "RS256",
            typ: Some("JWT"),
            kid: Some(&self.key_id),
            ..Default::default()
        })?;

        let claim_str = b64_encode_obj(&claims)?;
        let message = [jwt_header.as_ref(), claim_str.as_ref()].join(".");
        let mut sig_bytes = vec![0; self.private_key.0.public().modulus_len()];
        self.private_key
            .0
            .sign(
                &ring::signature::RSA_PKCS1_SHA256,
                &ring::rand::SystemRandom::new(),
                message.as_bytes(),
                &mut sig_bytes,
            )
            .map_err(|source| Error::Sign { source })?;

        let signature = BASE64_URL_SAFE_NO_PAD.encode(sig_bytes);
        let bearer = [message, signature].join(".");

        Ok(TemporaryToken {
            token: Arc::new(GcpCredential { bearer }),
            expiry: Some(Instant::now() + Duration::from_secs(3600)),
        })
    }
}

fn read_credentials_file<T>(service_account_path: impl AsRef<std::path::Path>) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let file = File::open(&service_account_path).map_err(|source| {
        let path = service_account_path.as_ref().to_owned();
        Error::OpenCredentials { source, path }
    })?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|source| Error::DecodeCredentials { source })
}

/// A deserialized `service-account-********.json`-file.
#[derive(serde::Deserialize, Debug, Clone)]
pub(crate) struct ServiceAccountCredentials {
    /// The private key in RSA format.
    pub private_key: String,

    /// The private key ID
    pub private_key_id: String,

    /// The email address associated with the service account.
    pub client_email: String,

    /// Base URL for GCS
    #[serde(default)]
    pub gcs_base_url: Option<String>,

    /// Disable oauth and use empty tokens.
    #[serde(default)]
    pub disable_oauth: bool,
}

impl ServiceAccountCredentials {
    /// Create a new [`ServiceAccountCredentials`] from a file.
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        read_credentials_file(path)
    }

    /// Create a new [`ServiceAccountCredentials`] from a string.
    pub(crate) fn from_key(key: &str) -> Result<Self> {
        serde_json::from_str(key).map_err(|source| Error::DecodeCredentials { source })
    }

    /// Create a [`SelfSignedJwt`] from this credentials struct.
    ///
    /// We use a scope of [`DEFAULT_SCOPE`] as opposed to an audience
    /// as GCS appears to not support audience
    ///
    /// # References
    /// - <https://stackoverflow.com/questions/63222450/service-account-authorization-without-oauth-can-we-get-file-from-google-cloud/71834557#71834557>
    /// - <https://www.codejam.info/2022/05/google-cloud-service-account-authorization-without-oauth.html>
    pub(crate) fn token_provider(self) -> crate::Result<SelfSignedJwt> {
        Ok(SelfSignedJwt::new(
            self.private_key_id,
            self.client_email,
            ServiceAccountKey::from_pem(self.private_key.as_bytes())?,
            DEFAULT_SCOPE.to_string(),
        )?)
    }

    pub(crate) fn signing_credentials(self) -> crate::Result<GcpSigningCredentialProvider> {
        Ok(Arc::new(StaticCredentialProvider::new(
            GcpSigningCredential {
                email: self.client_email,
                private_key: Some(ServiceAccountKey::from_pem(self.private_key.as_bytes())?),
            },
        )))
    }
}

/// Returns the number of seconds since unix epoch
fn seconds_since_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn b64_encode_obj<T: serde::Serialize>(obj: &T) -> Result<String> {
    let string = serde_json::to_string(obj).map_err(|source| Error::Encode { source })?;
    Ok(BASE64_URL_SAFE_NO_PAD.encode(string))
}

/// A provider that uses the Google Cloud Platform metadata server to fetch a token.
///
/// <https://cloud.google.com/docs/authentication/get-id-token#metadata-server>
#[derive(Debug, Default)]
pub(crate) struct InstanceCredentialProvider {}

/// Make a request to the metadata server to fetch a token, using a a given hostname.
async fn make_metadata_request(
    client: &HttpClient,
    hostname: &str,
    retry: &RetryConfig,
) -> crate::Result<TokenResponse> {
    let url =
        format!("http://{hostname}/computeMetadata/v1/instance/service-accounts/default/token");
    let response: TokenResponse = client
        .get(url)
        .header("Metadata-Flavor", "Google")
        .query(&[("audience", "https://www.googleapis.com/oauth2/v4/token")])
        .send_retry(retry)
        .await
        .map_err(|source| Error::TokenRequest { source })?
        .into_body()
        .json()
        .await
        .map_err(|source| Error::TokenResponseBody { source })?;
    Ok(response)
}

#[async_trait]
impl TokenProvider for InstanceCredentialProvider {
    type Credential = GcpCredential;

    /// Fetch a token from the metadata server.
    /// Since the connection is local we need to enable http access and don't actually use the client object passed in.
    /// Respects the `GCE_METADATA_HOST`, `GCE_METADATA_ROOT`, and `GCE_METADATA_IP`
    /// environment variables.
    ///
    /// References: <https://googleapis.dev/python/google-auth/latest/reference/google.auth.environment_vars.html>
    async fn fetch_token(
        &self,
        client: &HttpClient,
        retry: &RetryConfig,
    ) -> crate::Result<TemporaryToken<Arc<GcpCredential>>> {
        let metadata_host = if let Ok(host) = env::var("GCE_METADATA_HOST") {
            host
        } else if let Ok(host) = env::var("GCE_METADATA_ROOT") {
            host
        } else {
            DEFAULT_METADATA_HOST.to_string()
        };
        let metadata_ip = if let Ok(ip) = env::var("GCE_METADATA_IP") {
            ip
        } else {
            DEFAULT_METADATA_IP.to_string()
        };

        info!("fetching token from metadata server");
        let response = make_metadata_request(client, &metadata_host, retry)
            .or_else(|_| make_metadata_request(client, &metadata_ip, retry))
            .await?;

        let token = TemporaryToken {
            token: Arc::new(GcpCredential {
                bearer: response.access_token,
            }),
            expiry: Some(Instant::now() + Duration::from_secs(response.expires_in)),
        };
        Ok(token)
    }
}

/// Make a request to the metadata server to fetch the client email, using a given hostname.
async fn make_metadata_request_for_email(
    client: &HttpClient,
    hostname: &str,
    retry: &RetryConfig,
) -> crate::Result<String> {
    let url =
        format!("http://{hostname}/computeMetadata/v1/instance/service-accounts/default/email",);
    let response = client
        .get(url)
        .header("Metadata-Flavor", "Google")
        .send_retry(retry)
        .await
        .map_err(|source| Error::TokenRequest { source })?
        .into_body()
        .text()
        .await
        .map_err(|source| Error::TokenResponseBody { source })?;
    Ok(response)
}

/// A provider that uses the Google Cloud Platform metadata server to fetch a email for signing.
///
/// <https://cloud.google.com/appengine/docs/legacy/standard/java/accessing-instance-metadata>
#[derive(Debug, Default)]
pub(crate) struct InstanceSigningCredentialProvider {}

#[async_trait]
impl TokenProvider for InstanceSigningCredentialProvider {
    type Credential = GcpSigningCredential;

    /// Fetch a token from the metadata server.
    /// Since the connection is local we need to enable http access and don't actually use the client object passed in.
    /// Respects the `GCE_METADATA_HOST`, `GCE_METADATA_ROOT`, and `GCE_METADATA_IP`
    /// environment variables.
    ///
    /// References: <https://googleapis.dev/python/google-auth/latest/reference/google.auth.environment_vars.html>
    async fn fetch_token(
        &self,
        client: &HttpClient,
        retry: &RetryConfig,
    ) -> crate::Result<TemporaryToken<Arc<GcpSigningCredential>>> {
        let metadata_host = if let Ok(host) = env::var("GCE_METADATA_HOST") {
            host
        } else if let Ok(host) = env::var("GCE_METADATA_ROOT") {
            host
        } else {
            DEFAULT_METADATA_HOST.to_string()
        };

        let metadata_ip = if let Ok(ip) = env::var("GCE_METADATA_IP") {
            ip
        } else {
            DEFAULT_METADATA_IP.to_string()
        };

        info!("fetching token from metadata server");

        let email = make_metadata_request_for_email(client, &metadata_host, retry)
            .or_else(|_| make_metadata_request_for_email(client, &metadata_ip, retry))
            .await?;

        let token = TemporaryToken {
            token: Arc::new(GcpSigningCredential {
                email,
                private_key: None,
            }),
            expiry: None,
        };
        Ok(token)
    }
}

/// A deserialized `application_default_credentials.json`-file.
///
/// # References
/// - <https://cloud.google.com/docs/authentication/application-default-credentials#personal>
/// - <https://google.aip.dev/auth/4110>
#[derive(serde::Deserialize, Clone)]
#[serde(tag = "type")]
pub(crate) enum ApplicationDefaultCredentials {
    /// Service Account.
    ///
    /// # References
    /// - <https://google.aip.dev/auth/4112>
    #[serde(rename = "service_account")]
    ServiceAccount(ServiceAccountCredentials),
    /// Authorized user via "gcloud CLI Integration".
    ///
    /// # References
    /// - <https://google.aip.dev/auth/4113>
    #[serde(rename = "authorized_user")]
    AuthorizedUser(AuthorizedUserCredentials),
}

impl ApplicationDefaultCredentials {
    const CREDENTIALS_PATH: &'static str = if cfg!(windows) {
        "gcloud/application_default_credentials.json"
    } else {
        ".config/gcloud/application_default_credentials.json"
    };

    // Create a new application default credential in the following situations:
    //  1. a file is passed in and the type matches.
    //  2. without argument if the well-known configuration file is present.
    pub(crate) fn read(path: Option<&str>) -> Result<Option<Self>, Error> {
        if let Some(path) = path {
            return read_credentials_file::<Self>(path).map(Some);
        }

        let home_var = if cfg!(windows) { "APPDATA" } else { "HOME" };
        if let Some(home) = env::var_os(home_var) {
            let path = Path::new(&home).join(Self::CREDENTIALS_PATH);

            // It's expected for this file to not exist unless it has been explicitly configured by the user.
            if path.exists() {
                return read_credentials_file::<Self>(path).map(Some);
            }
        }
        Ok(None)
    }
}

const DEFAULT_TOKEN_GCP_URI: &str = "https://accounts.google.com/o/oauth2/token";

/// <https://google.aip.dev/auth/4113>
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct AuthorizedUserCredentials {
    client_id: String,
    client_secret: String,
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthorizedUserSigningCredentials {
    credential: AuthorizedUserCredentials,
}

///<https://oauth2.googleapis.com/tokeninfo?access_token=ACCESS_TOKEN>
#[derive(Debug, Deserialize)]
struct EmailResponse {
    email: String,
}

#[derive(Debug, Deserialize)]
struct IdTokenClaims {
    email: String,
}

async fn get_token_response(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
    client: &HttpClient,
    retry: &RetryConfig,
) -> Result<TokenResponse> {
    client
        .post(DEFAULT_TOKEN_GCP_URI)
        .form([
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
        ])
        .retryable(retry)
        .idempotent(true)
        .send()
        .await
        .map_err(|source| Error::TokenRequest { source })?
        .into_body()
        .json::<TokenResponse>()
        .await
        .map_err(|source| Error::TokenResponseBody { source })
}

impl AuthorizedUserSigningCredentials {
    pub(crate) fn from(credential: AuthorizedUserCredentials) -> crate::Result<Self> {
        Ok(Self { credential })
    }

    async fn client_email(
        &self,
        client: &HttpClient,
        retry: &RetryConfig,
    ) -> crate::Result<String> {
        let response = get_token_response(
            &self.credential.client_id,
            &self.credential.client_secret,
            &self.credential.refresh_token,
            client,
            retry,
        )
        .await?;

        // Extract email from id_token if available
        if let Some(id_token) = response.id_token {
            // Split the JWT string by dots to get the payload section
            let parts: Vec<&str> = id_token.split('.').collect();
            if parts.len() == 3 {
                // Decode the base64-encoded payload (middle part)
                if let Ok(payload) = BASE64_URL_SAFE_NO_PAD.decode(parts[1]) {
                    // Parse the payload as JSON and extract the email
                    if let Ok(claims) = serde_json::from_slice::<IdTokenClaims>(&payload) {
                        return Ok(claims.email);
                    }
                }
                // If any of the parsing steps fail, fallback to other method
            }
        }

        // Fallback to the original method if id_token is not available or invalid
        let response = client
            .get("https://oauth2.googleapis.com/tokeninfo")
            .query(&[("access_token", response.access_token)])
            .send_retry(retry)
            .await
            .map_err(|source| Error::TokenRequest { source })?
            .into_body()
            .json::<EmailResponse>()
            .await
            .map_err(|source: HttpError| Error::TokenResponseBody { source })?;

        Ok(response.email)
    }
}

#[async_trait]
impl TokenProvider for AuthorizedUserSigningCredentials {
    type Credential = GcpSigningCredential;

    async fn fetch_token(
        &self,
        client: &HttpClient,
        retry: &RetryConfig,
    ) -> crate::Result<TemporaryToken<Arc<GcpSigningCredential>>> {
        let email = self.client_email(client, retry).await?;
        Ok(TemporaryToken {
            token: Arc::new(GcpSigningCredential {
                email,
                private_key: None,
            }),
            expiry: None,
        })
    }
}

#[async_trait]
impl TokenProvider for AuthorizedUserCredentials {
    type Credential = GcpCredential;

    async fn fetch_token(
        &self,
        client: &HttpClient,
        retry: &RetryConfig,
    ) -> crate::Result<TemporaryToken<Arc<GcpCredential>>> {
        let response = get_token_response(
            &self.client_id,
            &self.client_secret,
            &self.refresh_token,
            client,
            retry,
        )
        .await?;

        Ok(TemporaryToken {
            token: Arc::new(GcpCredential {
                bearer: response.access_token,
            }),
            expiry: Some(Instant::now() + Duration::from_secs(response.expires_in)),
        })
    }
}

/// Trim whitespace from header values
fn trim_header_value(value: &str) -> String {
    let mut ret = value.to_string();
    ret.retain(|c| !c.is_whitespace());
    ret
}

/// A Google Cloud Storage Authorizer for generating signed URL using [Google SigV4]
///
/// [Google SigV4]: https://cloud.google.com/storage/docs/access-control/signed-urls
#[derive(Debug)]
pub(crate) struct GCSAuthorizer {
    date: Option<DateTime<Utc>>,
    credential: Arc<GcpSigningCredential>,
}

impl GCSAuthorizer {
    /// Create a new [`GCSAuthorizer`]
    pub(crate) fn new(credential: Arc<GcpSigningCredential>) -> Self {
        Self {
            date: None,
            credential,
        }
    }

    pub(crate) async fn sign(
        &self,
        method: Method,
        url: &mut Url,
        expires_in: Duration,
        client: &GoogleCloudStorageClient,
    ) -> crate::Result<()> {
        let email = &self.credential.email;
        let date = self.date.unwrap_or_else(Utc::now);
        let scope = self.scope(date);
        let credential_with_scope = format!("{}/{}", email, scope);

        let mut headers = HeaderMap::new();
        headers.insert("host", DEFAULT_GCS_SIGN_BLOB_HOST.parse().unwrap());

        let (_, signed_headers) = Self::canonicalize_headers(&headers);

        url.query_pairs_mut()
            .append_pair("X-Goog-Algorithm", "GOOG4-RSA-SHA256")
            .append_pair("X-Goog-Credential", &credential_with_scope)
            .append_pair("X-Goog-Date", &date.format("%Y%m%dT%H%M%SZ").to_string())
            .append_pair("X-Goog-Expires", &expires_in.as_secs().to_string())
            .append_pair("X-Goog-SignedHeaders", &signed_headers);

        let string_to_sign = self.string_to_sign(date, &method, url, &headers);
        let signature = match &self.credential.private_key {
            Some(key) => key.sign(&string_to_sign)?,
            None => client.sign_blob(&string_to_sign, email).await?,
        };

        url.query_pairs_mut()
            .append_pair("X-Goog-Signature", &signature);
        Ok(())
    }

    /// Get scope for the request
    ///
    /// <https://cloud.google.com/storage/docs/authentication/signatures#credential-scope>
    fn scope(&self, date: DateTime<Utc>) -> String {
        format!("{}/auto/storage/goog4_request", date.format("%Y%m%d"),)
    }

    /// Canonicalizes query parameters into the GCP canonical form
    /// form like:
    ///```plaintext
    ///HTTP_VERB
    ///PATH_TO_RESOURCE
    ///CANONICAL_QUERY_STRING
    ///CANONICAL_HEADERS
    ///
    ///SIGNED_HEADERS
    ///PAYLOAD
    ///```
    ///
    /// <https://cloud.google.com/storage/docs/authentication/canonical-requests>
    fn canonicalize_request(url: &Url, method: &Method, headers: &HeaderMap) -> String {
        let verb = method.as_str();
        let path = url.path();
        let query = Self::canonicalize_query(url);
        let (canonical_headers, signed_headers) = Self::canonicalize_headers(headers);

        format!(
            "{}\n{}\n{}\n{}\n\n{}\n{}",
            verb, path, query, canonical_headers, signed_headers, DEFAULT_GCS_PLAYLOAD_STRING
        )
    }

    /// Canonicalizes query parameters into the GCP canonical form
    /// form like `max-keys=2&prefix=object`
    ///
    /// <https://cloud.google.com/storage/docs/authentication/canonical-requests#about-query-strings>
    fn canonicalize_query(url: &Url) -> String {
        url.query_pairs()
            .sorted_unstable_by(|a, b| a.0.cmp(&b.0))
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    utf8_percent_encode(k.as_ref(), &STRICT_ENCODE_SET),
                    utf8_percent_encode(v.as_ref(), &STRICT_ENCODE_SET)
                )
            })
            .join("&")
    }

    /// Canonicalizes header into the GCP canonical form
    ///
    /// <https://cloud.google.com/storage/docs/authentication/canonical-requests#about-headers>
    fn canonicalize_headers(header_map: &HeaderMap) -> (String, String) {
        //FIXME add error handling for invalid header values
        let mut headers = BTreeMap::<String, Vec<&str>>::new();
        for (k, v) in header_map {
            headers
                .entry(k.as_str().to_lowercase())
                .or_default()
                .push(std::str::from_utf8(v.as_bytes()).unwrap());
        }

        let canonicalize_headers = headers
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}:{}",
                    k.trim(),
                    v.iter().map(|v| trim_header_value(v)).join(",")
                )
            })
            .join("\n");

        let signed_headers = headers.keys().join(";");

        (canonicalize_headers, signed_headers)
    }

    ///construct the string to sign
    ///form like:
    ///```plaintext
    ///SIGNING_ALGORITHM
    ///ACTIVE_DATETIME
    ///CREDENTIAL_SCOPE
    ///HASHED_CANONICAL_REQUEST
    ///```
    ///`ACTIVE_DATETIME` format:`YYYYMMDD'T'HHMMSS'Z'`
    /// <https://cloud.google.com/storage/docs/authentication/signatures#string-to-sign>
    pub(crate) fn string_to_sign(
        &self,
        date: DateTime<Utc>,
        request_method: &Method,
        url: &Url,
        headers: &HeaderMap,
    ) -> String {
        let canonical_request = Self::canonicalize_request(url, request_method, headers);
        let hashed_canonical_req = hex_digest(canonical_request.as_bytes());
        let scope = self.scope(date);

        format!(
            "{}\n{}\n{}\n{}",
            "GOOG4-RSA-SHA256",
            date.format("%Y%m%dT%H%M%SZ"),
            scope,
            hashed_canonical_req
        )
    }
}

pub(crate) trait CredentialExt {
    /// Apply bearer authentication to the request if the credential is not None
    fn with_bearer_auth(self, credential: Option<&GcpCredential>) -> Self;
}

impl CredentialExt for HttpRequestBuilder {
    fn with_bearer_auth(self, credential: Option<&GcpCredential>) -> Self {
        match credential {
            Some(credential) => {
                if credential.bearer.is_empty() {
                    self
                } else {
                    self.bearer_auth(&credential.bearer)
                }
            }
            None => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonicalize_headers() {
        let mut input_header = HeaderMap::new();
        input_header.insert("content-type", "text/plain".parse().unwrap());
        input_header.insert("host", "storage.googleapis.com".parse().unwrap());
        input_header.insert("x-goog-meta-reviewer", "jane".parse().unwrap());
        input_header.append("x-goog-meta-reviewer", "john".parse().unwrap());
        assert_eq!(
            GCSAuthorizer::canonicalize_headers(&input_header),
            (
                "content-type:text/plain
host:storage.googleapis.com
x-goog-meta-reviewer:jane,john"
                    .into(),
                "content-type;host;x-goog-meta-reviewer".to_string()
            )
        );
    }

    #[test]
    fn test_canonicalize_query() {
        let mut url = Url::parse("https://storage.googleapis.com/bucket/object").unwrap();
        url.query_pairs_mut()
            .append_pair("max-keys", "2")
            .append_pair("prefix", "object");
        assert_eq!(
            GCSAuthorizer::canonicalize_query(&url),
            "max-keys=2&prefix=object".to_string()
        );
    }
}
