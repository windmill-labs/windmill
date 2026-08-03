//! Azure Workload Identity Federation.
//!
//! The Kubernetes-projected service account token of the pod is exchanged with Entra
//! ID for an access token, which Azure-hosted databases accept in place of a password.
//! Nothing long-lived is stored on the Windmill instance.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::error::{Error, Result};
use crate::utils::HTTP_CLIENT;

/// Entra ID scope of Azure SQL / SQL Server.
pub const AZURE_SQL_SCOPE: &str = "https://database.windows.net/.default";

/// Entra ID scope of Azure Database for PostgreSQL / MySQL.
pub const AZURE_OSSRDBMS_SCOPE: &str = "https://ossrdbms-aad.database.windows.net/.default";

/// Renew an access token this long before it expires.
const TOKEN_REFRESH_BUFFER: Duration = Duration::from_secs(5 * 60);

lazy_static::lazy_static! {
    /// Access tokens keyed by identity and scope, shared by every job on the worker.
    static ref TOKEN_CACHE: Mutex<HashMap<String, CachedToken>> = Mutex::new(HashMap::new());
}

struct CachedToken {
    token: String,
    expires_at: Instant,
}

/// The federated credentials of the identity the worker authenticates as. The Azure
/// workload identity webhook injects all of them as env vars; a resource may override
/// them so one worker can reach databases behind distinct identities.
pub struct WorkloadIdentityConfig {
    tenant_id: String,
    client_id: String,
    federated_token_file: String,
    authority_host: String,
}

impl WorkloadIdentityConfig {
    pub fn resolve(
        tenant_id: Option<&str>,
        client_id: Option<&str>,
        federated_token_file: Option<&str>,
    ) -> Result<Self> {
        fn resolve_field(resource: Option<&str>, env_var: &str) -> Result<String> {
            resource
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .or_else(|| std::env::var(env_var).ok().filter(|v| !v.is_empty()))
                .ok_or_else(|| {
                    Error::BadConfig(format!(
                        "Workload identity authentication requires the {} env var on the worker \
                         (injected by the Azure workload identity webhook) or the matching field \
                         on the resource",
                        env_var
                    ))
                })
        }

        Ok(Self {
            tenant_id: resolve_field(tenant_id, "AZURE_TENANT_ID")?,
            client_id: resolve_field(client_id, "AZURE_CLIENT_ID")?,
            federated_token_file: resolve_field(
                federated_token_file,
                "AZURE_FEDERATED_TOKEN_FILE",
            )?,
            authority_host: std::env::var("AZURE_AUTHORITY_HOST")
                .ok()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "login.microsoftonline.com".to_string()),
        })
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    fn cache_key(&self, scope: &str) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.authority_host, self.tenant_id, self.client_id, self.federated_token_file, scope
        )
    }

    /// AZURE_AUTHORITY_HOST is injected with a scheme and a trailing slash
    /// (`https://login.microsoftonline.com/`), neither of which belongs in the path.
    fn token_endpoint(&self) -> String {
        let authority = self
            .authority_host
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/');
        format!("https://{}/{}/oauth2/v2.0/token", authority, self.tenant_id)
    }

    /// An Entra ID access token for `scope`, reusing the cached one while it is valid.
    pub async fn access_token(&self, scope: &str) -> Result<String> {
        let cache_key = self.cache_key(scope);
        if let Some(token) = cached_token(&cache_key) {
            return Ok(token);
        }

        // The projected token rotates on disk, so it must be re-read on every exchange.
        let assertion = tokio::fs::read_to_string(&self.federated_token_file)
            .await
            .map_err(|e| {
                Error::ExecutionErr(format!(
                    "Failed to read the federated token at {}: {}",
                    self.federated_token_file, e
                ))
            })?;

        let url = self.token_endpoint();
        let response = HTTP_CLIENT
            .post(&url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", self.client_id.as_str()),
                (
                    "client_assertion_type",
                    "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
                ),
                ("client_assertion", assertion.trim()),
                ("scope", scope),
            ])
            .send()
            .await
            .map_err(|e| {
                Error::ExecutionErr(format!(
                    "Failed to request an Entra ID token from {url}: {e}"
                ))
            })?;

        let status = response.status();
        let body: Value = response.json().await.map_err(|e| {
            Error::ExecutionErr(format!("Failed to parse the Entra ID token response: {e}"))
        })?;

        if !status.is_success() {
            return Err(Error::ExecutionErr(format!(
                "Entra ID token request failed ({}): {} - {}",
                status,
                body["error"].as_str().unwrap_or("unknown"),
                body["error_description"]
                    .as_str()
                    .unwrap_or("no description")
            )));
        }

        let token = body["access_token"]
            .as_str()
            .ok_or_else(|| {
                Error::ExecutionErr("Entra ID token response is missing access_token".to_string())
            })?
            .to_string();
        let expires_in = body["expires_in"].as_u64().unwrap_or(3600);

        TOKEN_CACHE.lock().unwrap().insert(
            cache_key,
            CachedToken {
                token: token.clone(),
                expires_at: Instant::now() + Duration::from_secs(expires_in),
            },
        );

        Ok(token)
    }
}

fn cached_token(cache_key: &str) -> Option<String> {
    let cache = TOKEN_CACHE.lock().unwrap();
    cache
        .get(cache_key)
        .filter(|cached| Instant::now() + TOKEN_REFRESH_BUFFER < cached.expires_at)
        .map(|cached| cached.token.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(authority_host: &str) -> WorkloadIdentityConfig {
        WorkloadIdentityConfig {
            tenant_id: "tenant".to_string(),
            client_id: "client".to_string(),
            federated_token_file: "/var/run/secrets/azure/tokens/azure-identity-token".to_string(),
            authority_host: authority_host.to_string(),
        }
    }

    #[test]
    fn test_token_endpoint() {
        let expected = "https://login.microsoftonline.com/tenant/oauth2/v2.0/token";
        assert_eq!(
            config("login.microsoftonline.com").token_endpoint(),
            expected
        );
        // The shape the workload identity webhook actually injects.
        assert_eq!(
            config("https://login.microsoftonline.com/").token_endpoint(),
            expected
        );
    }

    #[test]
    fn test_cache_key_is_scoped() {
        let config = config("login.microsoftonline.com");
        assert_ne!(
            config.cache_key(AZURE_SQL_SCOPE),
            config.cache_key(AZURE_OSSRDBMS_SCOPE)
        );
    }
}
