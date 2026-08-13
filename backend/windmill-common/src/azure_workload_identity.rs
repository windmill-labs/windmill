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

/// A database resource whose password is this authenticates as the worker's workload
/// identity, the way a `DATABASE_URL` whose password is `entraid` does for the instance
/// database. Carrying the mode in the password is what lets an existing deployment turn
/// it on: the resource type schemas live on the hub, and every database form already has
/// a password field. Anyone whose password is literally this string authenticates as the
/// worker's identity instead of failing to log in, which is why it is deliberately less
/// password-shaped than the instance's `entraid`: that one is set by whoever runs the
/// instance, this one sits where users keep their own secrets.
///
/// Compare it trimmed. A near-miss is otherwise forwarded to the server as a real
/// password, and the resulting rejection is indistinguishable from an ordinary bad
/// login, so the mode having been intended at all leaves no trace.
pub const WORKLOAD_IDENTITY_PASSWORD: &str = "ms_entraid";

/// Renew an access token this long before it expires.
const TOKEN_REFRESH_BUFFER: Duration = Duration::from_secs(5 * 60);

/// A token with less life than this left is not worth handing to a connection: opening
/// one takes up to 20 seconds, and it authenticates at the far end of that.
const TOKEN_MIN_LIFETIME: Duration = Duration::from_secs(60);

/// How long a failed renewal suppresses the next attempt, as long as the current token
/// still works. Without it every queued job retries the exchange in turn, so a
/// throttling Entra ID would cost each of them the request's full latency.
const REFRESH_RETRY_BACKOFF: Duration = Duration::from_secs(30);

lazy_static::lazy_static! {
    /// Access tokens keyed by identity and scope, shared by every job on the worker.
    static ref TOKEN_CACHE: Mutex<HashMap<String, Arc<TokenSlot>>> = Mutex::new(HashMap::new());
}

#[derive(Default)]
struct TokenSlot {
    /// Read on the hit path, so it must never be held across the exchange.
    token: RwLock<Option<CachedToken>>,
    /// Held for the whole exchange, so a burst of jobs on a cold or expiring entry
    /// makes one request to Entra ID instead of one per job. Guards the time of the
    /// last failed exchange, which is what the waiting jobs need to see.
    refreshing: tokio::sync::Mutex<Option<Instant>>,
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

/// Whether to go to Entra ID, given the last failed exchange and whether the current
/// token would still serve. A cold slot always tries: there is nothing to fall back on.
fn should_attempt_refresh(last_failure: Option<Instant>, has_usable_token: bool) -> bool {
    match last_failure {
        Some(at) if has_usable_token => at.elapsed() >= REFRESH_RETRY_BACKOFF,
        _ => true,
    }
}

/// The federated credentials of the identity the worker authenticates as, all injected
/// into the pod by the Azure workload identity webhook. The identity is the worker's,
/// not the resource's: reaching two databases as two identities means two worker groups.
pub struct WorkloadIdentityConfig {
    tenant_id: String,
    client_id: String,
    federated_token_file: String,
    authority_host: String,
}

impl WorkloadIdentityConfig {
    pub fn resolve() -> Result<Self> {
        fn required(env_var: &str) -> Result<String> {
            std::env::var(env_var)
                .ok()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    Error::BadConfig(format!(
                        "Workload identity authentication requires the {} env var on the worker, \
                         injected by the Azure workload identity webhook",
                        env_var
                    ))
                })
        }

        Ok(Self {
            tenant_id: required("AZURE_TENANT_ID")?,
            client_id: required("AZURE_CLIENT_ID")?,
            federated_token_file: required("AZURE_FEDERATED_TOKEN_FILE")?,
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

        let mut last_failure = slot.refreshing.lock().await;
        // Whoever held the lock may have just refreshed it.
        if let Some(token) = slot.token_valid_for(TOKEN_REFRESH_BUFFER) {
            return Ok(token);
        }

        // Inside the refresh buffer the previous token still works, and Entra ID
        // throttles bursts: a failed renewal must not fail an otherwise fine job.
        let usable = slot.token_valid_for(TOKEN_MIN_LIFETIME);
        if !should_attempt_refresh(*last_failure, usable.is_some()) {
            return Ok(usable.unwrap());
        }

        match self.request_token(scope).await {
            Ok(fresh) => {
                let token = fresh.token.clone();
                *slot.token.write().unwrap() = Some(fresh);
                *last_failure = None;
                Ok(token)
            }
            Err(e) => {
                *last_failure = Some(Instant::now());
                // Check again rather than trusting the pre-request value: a request that
                // times out eats 20 seconds of whatever the token had left.
                match slot.token_valid_for(TOKEN_MIN_LIFETIME) {
                    Some(token) => {
                        tracing::warn!("Keeping the current Entra ID token, renewal failed: {e:#}");
                        Ok(token)
                    }
                    None => Err(e),
                }
            }
        }
    }

    fn slot(&self, scope: &str) -> Arc<TokenSlot> {
        let mut cache = TOKEN_CACHE.lock().unwrap();
        // Drop what has expired and that no in-flight request is holding.
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

    /// A recent failure must not be re-attempted by every job queued behind the
    /// refresh lock, but only while there is a token left to serve them.
    #[test]
    fn test_failed_refresh_is_not_retried_by_every_caller() {
        assert!(!should_attempt_refresh(Some(Instant::now()), true));
        assert!(should_attempt_refresh(Some(Instant::now()), false));
        assert!(should_attempt_refresh(
            Instant::now().checked_sub(REFRESH_RETRY_BACKOFF),
            true
        ));
        assert!(should_attempt_refresh(None, true));
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
