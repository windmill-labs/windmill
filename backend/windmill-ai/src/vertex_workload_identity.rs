//! Vertex AI through Google Cloud Workload Identity Federation.
//!
//! A Vertex resource can name a workload identity pool provider instead of
//! carrying an access token. Windmill then mints an OIDC token for the caller,
//! exchanges it at Google's STS for a federated token and, when the resource names
//! a service account, impersonates it. The result is sent as the Vertex bearer
//! token, exactly as an `api_key` would be.

use serde::Deserialize;
use windmill_common::client::AuthedClient;
use windmill_common::error::Error;

use crate::ai_providers::AIProvider;
use crate::credentials::ProviderCredentials;

pub const GCP_STS_TOKEN_URL: &str = "https://sts.googleapis.com/v1/token";
pub const GCP_IAM_CREDENTIALS_URL: &str = "https://iamcredentials.googleapis.com/v1";
const CLOUD_PLATFORM_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

/// Lifetime of the OIDC token presented to STS. It is only read once, at the
/// exchange, so there is no reason to let it outlive the access token it buys.
pub const VERTEX_OIDC_TOKEN_LIFETIME_SECS: i64 = 1800;

/// The longest lifetime `generateAccessToken` grants without an organization
/// policy extending it.
const IMPERSONATED_TOKEN_LIFETIME_SECS: u64 = 3600;

/// Exchange again this long before the access token expires, so a long streamed
/// response that starts on a reused token cannot outlive it. Capped at half the
/// token's lifetime so a short-lived token is not exchanged on every request.
const REFRESH_MARGIN: std::time::Duration = std::time::Duration::from_secs(300);

/// The settings a Vertex resource authenticates with through Workload Identity
/// Federation. None of them is a secret: what authorises the exchange is the OIDC
/// token, and what it may do is the pool provider's and service account's policy.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VertexWorkloadIdentity {
    /// `projects/<number>/locations/global/workloadIdentityPools/<pool>/providers/<provider>`.
    pub provider: String,
    /// The service account to impersonate. Without one, Vertex is called with the
    /// federated token itself, which needs the pool's principals granted on the
    /// project directly.
    pub service_account_email: Option<String>,
    /// Audience of the OIDC token, for a provider configured with custom allowed
    /// audiences. Defaults to the provider's own resource name, which is what a
    /// provider accepts when none is configured.
    pub audience: Option<String>,
}

impl VertexWorkloadIdentity {
    /// The workload identity settings of a `vertex_ai_federated` resource, or `None`
    /// for any other: only `workload_identity_provider` identifies one. Such a
    /// resource is always on Vertex, so callers set the platform from the result.
    pub fn from_resource(
        provider: &AIProvider,
        workload_identity_provider: Option<&str>,
        service_account_email: Option<&str>,
        audience: Option<&str>,
    ) -> Result<Option<Self>, Error> {
        let non_empty = |v: Option<&str>| {
            v.map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
        };
        let Some(pool_provider) = non_empty(workload_identity_provider) else {
            return Ok(None);
        };
        if !matches!(provider, AIProvider::Anthropic | AIProvider::GoogleAI) {
            return Err(Error::BadRequest(format!(
                "A vertex_ai_federated resource can only be used for the Anthropic or Google AI \
                 provider, not {provider:?}"
            )));
        }

        let pool_provider = pool_provider
            .trim_start_matches("https:")
            .trim_start_matches("//iam.googleapis.com/")
            .to_string();
        let segments: Vec<&str> = pool_provider.split('/').collect();
        let well_formed = matches!(
            segments.as_slice(),
            ["projects", p, "locations", l, "workloadIdentityPools", pool, "providers", name]
                if [p, l, pool, name].iter().all(|s| !s.is_empty())
        );
        if !well_formed {
            return Err(Error::BadRequest(format!(
                "workload_identity_provider must be of the form \
                 projects/<project number>/locations/global/workloadIdentityPools/<pool>/providers/<provider>, \
                 got {pool_provider}"
            )));
        }

        let service_account_email = non_empty(service_account_email);
        if let Some(email) = service_account_email.as_deref() {
            // It becomes a path segment of the impersonation URL.
            let valid = email.contains('@')
                && email
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '-' | '_'));
            if !valid {
                return Err(Error::BadRequest(format!(
                    "service_account_email is not a service account email: {email}"
                )));
            }
        }

        Ok(Some(Self {
            provider: pool_provider,
            service_account_email,
            audience: non_empty(audience),
        }))
    }

    pub fn oidc_audience(&self) -> String {
        self.audience
            .clone()
            .unwrap_or_else(|| format!("https://iam.googleapis.com/{}", self.provider))
    }
}

/// The exchanged token is the identity of whoever uses the resource, not of
/// whoever wrote it, so it is only ever sent to Google: a resource author must not
/// be able to point another user's token at a host they read.
pub fn check_vertex_base_url(base_url: &str) -> Result<(), Error> {
    let on_google = reqwest::Url::parse(base_url).is_ok_and(|url| {
        url.scheme() == "https"
            && url
                .host_str()
                .is_some_and(|host| host == "googleapis.com" || host.ends_with(".googleapis.com"))
    });
    if on_google {
        Ok(())
    } else {
        Err(Error::BadRequest(format!(
            "A Vertex AI resource that authenticates through workload_identity_provider must set \
             base_url to a https://*.googleapis.com Vertex endpoint, got {base_url:?}"
        )))
    }
}

/// A Google access token from a Workload Identity Federation exchange.
#[derive(Clone, Debug)]
pub struct GcpAccessToken {
    pub access_token: String,
    refresh_at: std::time::SystemTime,
}

impl GcpAccessToken {
    fn new(access_token: String, lifetime: std::time::Duration) -> Self {
        // Google tokens live at most 12 hours; the clamp keeps an out-of-range
        // `expires_in` from overflowing the clock arithmetic.
        let lifetime = lifetime.min(std::time::Duration::from_secs(43200));
        let margin = REFRESH_MARGIN.min(lifetime / 2);
        Self { access_token, refresh_at: std::time::SystemTime::now() + lifetime - margin }
    }

    pub fn is_fresh(&self) -> bool {
        std::time::SystemTime::now() < self.refresh_at
    }
}

#[derive(Deserialize)]
struct StsResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateAccessTokenResponse {
    access_token: String,
}

/// Exchange an OIDC token for a Google access token: STS first, then the
/// service account impersonation when the resource names one.
pub async fn exchange_oidc_token(
    identity: &VertexWorkloadIdentity,
    subject_token: &str,
) -> Result<GcpAccessToken, Error> {
    let exchanged = request_access_token(identity, subject_token).await;
    windmill_common::feature_usage::log_feature_usage(
        "vertex_workload_identity",
        "token_exchange",
        if exchanged.is_ok() { "ok" } else { "failed" },
    );
    exchanged
}

async fn request_access_token(
    identity: &VertexWorkloadIdentity,
    subject_token: &str,
) -> Result<GcpAccessToken, Error> {
    let http = &*windmill_common::utils::HTTP_CLIENT;

    let response = http
        .post(GCP_STS_TOKEN_URL)
        .form(&[
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:token-exchange",
            ),
            ("subject_token", subject_token),
            ("subject_token_type", "urn:ietf:params:oauth:token-type:jwt"),
            (
                "audience",
                &format!("//iam.googleapis.com/{}", identity.provider),
            ),
            (
                "requested_token_type",
                "urn:ietf:params:oauth:token-type:access_token",
            ),
            ("scope", CLOUD_PLATFORM_SCOPE),
        ])
        .send()
        .await
        .map_err(|e| Error::internal_err(format!("Google STS token exchange failed: {e}")))?;
    let federated = parse_response::<StsResponse>(response, "Google STS token exchange").await?;

    let Some(email) = identity.service_account_email.as_deref() else {
        return Ok(GcpAccessToken::new(
            federated.access_token,
            std::time::Duration::from_secs(federated.expires_in),
        ));
    };

    let response = http
        .post(format!(
            "{GCP_IAM_CREDENTIALS_URL}/projects/-/serviceAccounts/{email}:generateAccessToken"
        ))
        .bearer_auth(&federated.access_token)
        .json(&serde_json::json!({
            "scope": [CLOUD_PLATFORM_SCOPE],
            "lifetime": format!("{IMPERSONATED_TOKEN_LIFETIME_SECS}s"),
        }))
        .send()
        .await
        .map_err(|e| {
            Error::internal_err(format!("Impersonating service account {email} failed: {e}"))
        })?;
    let impersonated = parse_response::<GenerateAccessTokenResponse>(
        response,
        &format!("Impersonating service account {email}"),
    )
    .await?;

    Ok(GcpAccessToken::new(
        impersonated.access_token,
        std::time::Duration::from_secs(IMPERSONATED_TOKEN_LIFETIME_SECS),
    ))
}

async fn parse_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
    what: &str,
) -> Result<T, Error> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(Error::BadRequest(format!(
            "{what} failed with status {status}: {body}"
        )));
    }
    response
        .json::<T>()
        .await
        .map_err(|e| Error::internal_err(format!("Failed to parse the {what} response: {e}")))
}

/// The Vertex access token of a federating resource for a job, reusing `cached`
/// while it is still fresh, so the agent loop exchanges once per job rather than
/// per turn. Returns `None` for a resource that does not federate.
pub async fn refresh_vertex_access_token<'a>(
    credentials: &ProviderCredentials,
    cached: &'a mut Option<GcpAccessToken>,
    client: &AuthedClient,
) -> Result<Option<&'a str>, Error> {
    let Some(identity) = credentials.vertex_workload_identity.as_ref() else {
        return Ok(None);
    };
    check_vertex_base_url(&credentials.base_url)?;
    if !cached.as_ref().is_some_and(GcpAccessToken::is_fresh) {
        // A worker holds no OIDC signing key, so it asks the API to mint the token for this job.
        let subject_token = client
            .get_id_token_expiring_in(
                &identity.oidc_audience(),
                Some(VERTEX_OIDC_TOKEN_LIFETIME_SECS),
            )
            .await
            .map_err(|e| Error::ExecutionErr(format!("Failed to get an OIDC token: {e}")))?;
        *cached = Some(exchange_oidc_token(identity, &subject_token).await?);
    }
    Ok(cached.as_ref().map(|token| token.access_token.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROVIDER: &str =
        "projects/123/locations/global/workloadIdentityPools/pool/providers/windmill";

    fn vertex(
        provider: Option<&str>,
        email: Option<&str>,
    ) -> Result<Option<VertexWorkloadIdentity>, Error> {
        VertexWorkloadIdentity::from_resource(&AIProvider::GoogleAI, provider, email, None)
    }

    #[test]
    fn provider_is_normalized_and_validated() {
        for form in [
            PROVIDER.to_string(),
            format!("//iam.googleapis.com/{PROVIDER}"),
            format!("https://iam.googleapis.com/{PROVIDER}"),
        ] {
            let identity = vertex(Some(&form), None).unwrap().unwrap();
            assert_eq!(identity.provider, PROVIDER);
            assert_eq!(
                identity.oidc_audience(),
                format!("https://iam.googleapis.com/{PROVIDER}")
            );
        }
        assert!(vertex(Some("projects/123/providers/windmill"), None).is_err());
    }

    #[test]
    fn service_account_email_cannot_escape_its_path_segment() {
        let sa = "vertex@my-project.iam.gserviceaccount.com";
        assert!(vertex(Some(PROVIDER), Some(sa)).is_ok());
        assert!(vertex(Some(PROVIDER), Some("a@b/../../other")).is_err());
        assert!(vertex(Some(PROVIDER), Some("a@b?x=1")).is_err());
    }

    #[test]
    fn token_only_goes_to_google() {
        assert!(check_vertex_base_url(
            "https://us-east5-aiplatform.googleapis.com/v1/projects/p/locations/us-east5/publishers/anthropic/models"
        )
        .is_ok());
        for url in [
            "http://us-east5-aiplatform.googleapis.com/v1",
            "https://googleapis.com.evil.example/v1",
            "https://evil-googleapis.com/v1",
            "https://api.anthropic.com/v1",
            "",
        ] {
            assert!(check_vertex_base_url(url).is_err(), "{url}");
        }
    }
}
