//! Azure Workload Identity Federation.
//!
//! The Kubernetes-projected service account token of the pod is exchanged with Entra
//! ID for an access token, which Azure-hosted databases accept in place of a password.
//! Nothing long-lived is stored on the Windmill instance.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
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

/// A token with less life than this left is not worth handing to a connection.
const TOKEN_MIN_LIFETIME: Duration = Duration::from_secs(30);

lazy_static::lazy_static! {
    /// Access tokens keyed by identity and scope, shared by every job on the worker.
    static ref TOKEN_CACHE: Mutex<HashMap<String, Arc<TokenSlot>>> = Mutex::new(HashMap::new());
}

#[derive(Default)]
struct TokenSlot {
    /// Read on the hit path, so it must never be held across the exchange.
    token: RwLock<Option<CachedToken>>,
    /// Held for the whole exchange, so a burst of jobs on a cold or expiring entry
    /// makes one request to Entra ID instead of one per job.
    refreshing: tokio::sync::Mutex<()>,
}

struct CachedToken {
    token: String,
    expires_at: Instant,
}

impl TokenSlot {
    fn token_valid_for(&self, remaining: Duration) -> Option<String> {
        let cached = self.token.read().unwrap();
        cached
            .as_ref()
            .filter(|cached| Instant::now() + remaining < cached.expires_at)
            .map(|cached| cached.token.clone())
    }
}

/// The federated credentials of the identity the worker authenticates as. The Azure
/// workload identity webhook injects them as env vars; a resource may override the
/// tenant and client so one worker can reach databases behind distinct identities.
pub struct WorkloadIdentityConfig {
    tenant_id: String,
    client_id: String,
    federated_token_file: String,
    authority_host: String,
}

impl WorkloadIdentityConfig {
    pub fn resolve(tenant_id: Option<&str>, client_id: Option<&str>) -> Result<Self> {
        fn from_env(env_var: &str) -> Option<String> {
            std::env::var(env_var).ok().filter(|v| !v.is_empty())
        }

        fn missing(env_var: &str, overridable: bool) -> Error {
            Error::BadConfig(format!(
                "Workload identity authentication requires the {} env var on the worker \
                 (injected by the Azure workload identity webhook){}",
                env_var,
                if overridable {
                    " or the matching field on the resource"
                } else {
                    ""
                }
            ))
        }

        fn resolve_field(resource: Option<&str>, env_var: &str) -> Result<String> {
            resource
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .or_else(|| from_env(env_var))
                .ok_or_else(|| missing(env_var, true))
        }

        Ok(Self {
            tenant_id: resolve_field(tenant_id, "AZURE_TENANT_ID")?,
            client_id: resolve_field(client_id, "AZURE_CLIENT_ID")?,
            // Deliberately env-only: the projected token's path is the webhook's to
            // choose, and a resource-supplied path would let whoever runs a script
            // probe the worker filesystem through the resulting error.
            federated_token_file: from_env("AZURE_FEDERATED_TOKEN_FILE")
                .ok_or_else(|| missing("AZURE_FEDERATED_TOKEN_FILE", false))?,
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
            "{}|{}|{}|{}",
            self.authority_host, self.tenant_id, self.client_id, scope
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
        let slot = self.slot(scope);
        if let Some(token) = slot.token_valid_for(TOKEN_REFRESH_BUFFER) {
            return Ok(token);
        }

        let _refreshing = slot.refreshing.lock().await;
        // Whoever held the lock may have just refreshed it.
        if let Some(token) = slot.token_valid_for(TOKEN_REFRESH_BUFFER) {
            return Ok(token);
        }

        match self.request_token(scope).await {
            Ok(fresh) => {
                let token = fresh.token.clone();
                *slot.token.write().unwrap() = Some(fresh);
                Ok(token)
            }
            // Inside the refresh buffer the previous token still works, and Entra ID
            // throttles bursts: a failed renewal must not fail an otherwise fine job.
            Err(e) => match slot.token_valid_for(TOKEN_MIN_LIFETIME) {
                Some(token) => {
                    tracing::warn!("Keeping the current Entra ID token, renewal failed: {e:#}");
                    Ok(token)
                }
                None => Err(e),
            },
        }
    }

    fn slot(&self, scope: &str) -> Arc<TokenSlot> {
        let mut cache = TOKEN_CACHE.lock().unwrap();
        // Identities come off resources, so the key space is caller-controlled: drop
        // what has expired and that no in-flight request is holding.
        cache.retain(|_, slot| {
            Arc::strong_count(slot) > 1 || slot.token_valid_for(TOKEN_MIN_LIFETIME).is_some()
        });
        cache.entry(self.cache_key(scope)).or_default().clone()
    }

    async fn request_token(&self, scope: &str) -> Result<CachedToken> {
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

        Ok(CachedToken { token, expires_at: Instant::now() + Duration::from_secs(expires_in) })
    }
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

    /// A token inside the refresh buffer is no longer served as fresh, but is still
    /// good enough to fall back on when the renewal itself fails.
    #[test]
    fn test_token_within_refresh_buffer_is_stale_but_usable() {
        let slot = TokenSlot::default();
        *slot.token.write().unwrap() = Some(CachedToken {
            token: "tok".to_string(),
            expires_at: Instant::now() + TOKEN_REFRESH_BUFFER - Duration::from_secs(60),
        });

        assert_eq!(slot.token_valid_for(TOKEN_REFRESH_BUFFER), None);
        assert_eq!(
            slot.token_valid_for(TOKEN_MIN_LIFETIME),
            Some("tok".to_string())
        );
    }
}
