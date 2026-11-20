use crate::db::{ApiAuthed, DB};
use crate::utils::check_scopes;

#[cfg(feature = "bedrock")]
use axum::routing::get;
use axum::Json;
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query},
    response::IntoResponse,
    routing::post,
    Extension, Router,
};
use futures::StreamExt;
use http::{HeaderMap, Method, StatusCode};
use quick_cache::sync::Cache;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use std::time::Duration;
use windmill_ai::ai_cache::current_instance_ai_config_revision;
use windmill_ai::ai_providers::{
    empty_string_as_none, AIPlatform, AIProvider, ProviderConfig, ProviderModel,
};
use windmill_ai::ai_types::{validate_token_limit, CONTEXT_WINDOWS, MAX_MODEL_RATE, OUTPUT_LIMITS};
use windmill_ai::credentials::{AnthropicFederation, ProviderCredentials};
use windmill_ai::one_shot::{complete_text, OneShotRequest};
use windmill_ai::providers::anthropic::{
    exchange_federated_token, FederatedAccessToken, ANTHROPIC_API_BASE_URL,
};
#[cfg(feature = "bedrock")]
use windmill_ai::providers::bedrock::{
    handle_bedrock_proxy, BedrockProxyResponse, BedrockProxyResponseBody,
};
use windmill_ai::providers::{
    create_chat_completions_query_builder, create_query_builder,
    google_ai::{
        handle_google_ai_chat_proxy, handle_google_ai_models_proxy, GoogleAIProxyResponse,
        GoogleAIProxyResponseBody,
    },
    is_chat_completions_only, reads_max_completion_tokens,
};
use windmill_ai::proxy::{
    fim::maybe_transform_fim_request, proxy_execution_mode, validate_proxy_path, ProxyBuildArgs,
    ProxyExecutionMode, ProxyRequest,
};
use windmill_ai::retry::send_with_retries;
use windmill_ai::types::{OpenAIContent, OpenAIMessage};
use windmill_ai::vertex_workload_identity::{
    check_vertex_base_url, exchange_oidc_token, GcpAccessToken, VertexWorkloadIdentity,
};
use windmill_ai::web_search::{WebPage, WebSearchProvider, WebSearchResult};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::client::AuthedClient;
use windmill_common::db::UserDB;
use windmill_common::error::JsonResult;
use windmill_common::error::{to_anyhow, Error, Result};
use windmill_common::utils::{configure_client, require_admin};
use windmill_common::variables::{get_variable_or_self, get_variable_or_self_as};

// AI timeout configuration constants
const AI_TIMEOUT_MIN_SECS: u64 = 1;
const AI_TIMEOUT_MAX_SECS: u64 = 86400; // 24 hours
const AI_TIMEOUT_DEFAULT_SECS: u64 = 3600; // 1 hour
const HTTP_POOL_MAX_IDLE_PER_HOST: usize = 10;
const HTTP_POOL_IDLE_TIMEOUT_SECS: u64 = 90;
pub(crate) const KEEPALIVE_INTERVAL_SECS: u64 = 15;

lazy_static::lazy_static! {
    /// AI request timeout in seconds.
    ///
    /// This timeout applies to the TOTAL duration of AI HTTP requests,
    /// including streaming responses. Default is 3600 seconds (1 hour).
    ///
    /// Can be configured via AI_REQUEST_TIMEOUT_SECONDS environment variable.
    /// Valid range: 1-86400 seconds (24 hours).
    ///   - Minimum (1s): Prevents immediate timeout, allows minimal response time
    ///   - Maximum (24h): Prevents indefinite hangs while supporting long-running AI operations
    ///   - Default (1h): Balances responsiveness with support for complex AI tasks
    ///
    /// Note: This is a total request timeout, not an idle timeout.
    /// Long-running streaming responses that exceed this duration will be terminated,
    /// even if actively receiving data.
    ///
    /// CRITICAL: If using a reverse proxy (NGINX, Traefik, etc.), you MUST configure
    /// proxy timeouts to match or exceed this value. Without proper proxy configuration,
    /// connections will be terminated prematurely at the proxy layer regardless of this
    /// backend timeout setting.
    ///
    /// Example NGINX configuration:
    ///   location /api/ {
    ///     proxy_read_timeout 3600s;  # Must be >= AI_REQUEST_TIMEOUT_SECONDS
    ///     proxy_send_timeout 3600s;
    ///     proxy_connect_timeout 60s;
    ///   }
    static ref AI_TIMEOUT_SECS: u64 = {
        match std::env::var("AI_REQUEST_TIMEOUT_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
        {
            Some(timeout) if timeout >= AI_TIMEOUT_MIN_SECS && timeout <= AI_TIMEOUT_MAX_SECS => {
                tracing::info!("AI request timeout configured: {}s", timeout);
                timeout
            },
            Some(timeout) => {
                tracing::warn!(
                    "AI_REQUEST_TIMEOUT_SECONDS value {} is out of range ({}-{}), using default {}s",
                    timeout,
                    AI_TIMEOUT_MIN_SECS,
                    AI_TIMEOUT_MAX_SECS,
                    AI_TIMEOUT_DEFAULT_SECS
                );
                AI_TIMEOUT_DEFAULT_SECS
            },
            None => {
                tracing::info!(
                    "AI_REQUEST_TIMEOUT_SECONDS not set, using default {}s",
                    AI_TIMEOUT_DEFAULT_SECS
                );
                AI_TIMEOUT_DEFAULT_SECS
            },
        }
    };

    pub(crate) static ref HTTP_CLIENT: Client = ai_http_client_builder()
        .build()
        .expect("Failed to build AI HTTP client - check system TLS configuration");

    static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();

    pub static ref AI_REQUEST_CACHE: Cache<(String, AIProvider), ExpiringProviderCredentials> = Cache::new(500);

    /// Anthropic access tokens from a federated exchange, keyed by (workspace,
    /// federation settings, user email). Like BEDROCK_ASSUMED_ROLE_CACHE, an entry
    /// is never handed to another user than the one the OIDC token named.
    static ref ANTHROPIC_FEDERATED_TOKEN_CACHE: Cache<(String, AnthropicFederation, String), FederatedAccessToken> = Cache::new(500);

    /// Google access tokens from a Vertex Workload Identity Federation exchange,
    /// keyed like ANTHROPIC_FEDERATED_TOKEN_CACHE by (workspace, settings, user email).
    static ref VERTEX_WORKLOAD_IDENTITY_TOKEN_CACHE: Cache<(String, VertexWorkloadIdentity, String), GcpAccessToken> = Cache::new(500);
}

#[cfg(feature = "bedrock")]
lazy_static::lazy_static! {
    /// Temporary IAM credentials from a Bedrock OIDC role assumption, keyed by
    /// (workspace, role ARN, user email). The email is the identity the OIDC
    /// token is minted for, so unlike AI_REQUEST_CACHE an entry is never handed
    /// to a different user — that is what keeps AWS-side attribution per user.
    static ref BEDROCK_ASSUMED_ROLE_CACHE: Cache<(String, String, String), windmill_ai::ai_bedrock::AssumedRoleCredentials> = Cache::new(500);
}

pub(crate) fn invalidate_ai_request_cache_for_workspace(workspace_id: &str) {
    AI_REQUEST_CACHE.retain(|(cached_workspace_id, _), _| cached_workspace_id != workspace_id);
    ANTHROPIC_FEDERATED_TOKEN_CACHE
        .retain(|(cached_workspace_id, _, _), _| cached_workspace_id != workspace_id);
    VERTEX_WORKLOAD_IDENTITY_TOKEN_CACHE
        .retain(|(cached_workspace_id, _, _), _| cached_workspace_id != workspace_id);
    // Dropped alongside the credentials that named the role, so that changing a
    // workspace's AI settings does not leave a session assumed under the old ones
    // in play until STS expires it.
    #[cfg(feature = "bedrock")]
    BEDROCK_ASSUMED_ROLE_CACHE
        .retain(|(cached_workspace_id, _, _), _| cached_workspace_id != workspace_id);
}

/// Shared configuration for every outbound AI HTTP client (the pooled
/// [`HTTP_CLIENT`] and the per-request DNS-pinned clients).
///
/// Redirects are disabled: the SSRF check only validates the configured host, so
/// a public host that 3xx-es could otherwise bounce us to a private/internal
/// address. AI APIs respond directly and do not rely on redirects, so this holds
/// even for ALLOW_PRIVATE_AI_BASE_URLS deployments. DNS pinning likewise only
/// covers the original host, so following a redirect would reopen the hole.
fn ai_http_client_builder() -> reqwest::ClientBuilder {
    configure_client(
        reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(*AI_TIMEOUT_SECS))
            .pool_max_idle_per_host(HTTP_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(Some(std::time::Duration::from_secs(
                HTTP_POOL_IDLE_TIMEOUT_SECS,
            )))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("windmill/beta"),
    )
}

/// Build the client for a single outbound AI request to `url`, pinning DNS to
/// the SSRF-validated address so the connect cannot rebind to an internal IP
/// after the check (DNS-rebinding TOCTOU).
///
/// Returns the shared pooled [`HTTP_CLIENT`] unchanged when there is nothing to
/// pin — an IP-literal host, or a host `ALLOW_PRIVATE_AI_BASE_URLS` allows
/// by name or wholesale. The same allowlist and error hint as `get_base_url`
/// apply, so the guard here is consistent with save-time
/// validation while additionally closing the connect-time window.
async fn pinned_ai_client_for(url: &str) -> Result<std::borrow::Cow<'static, Client>> {
    use std::borrow::Cow;

    let target = windmill_ai::ai_providers::validate_ai_url(url).await?;

    if target.pinned_addrs().is_empty() {
        return Ok(Cow::Borrowed(&HTTP_CLIENT));
    }

    let client = target
        .apply_dns_pinning(ai_http_client_builder())
        .build()
        .map_err(to_anyhow)?;
    Ok(Cow::Owned(client))
}

#[derive(Deserialize, Debug)]
struct AIOAuthResource {
    client_id: String,
    client_secret: String,
    token_url: String,
    user: Option<String>,
}

#[derive(Deserialize, Debug)]
struct AIStandardResource {
    #[serde(alias = "baseUrl", default, deserialize_with = "empty_string_as_none")]
    base_url: Option<String>,
    #[serde(alias = "apiKey", default, deserialize_with = "empty_string_as_none")]
    api_key: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    organization_id: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    region: Option<String>,
    #[serde(
        alias = "awsAccessKeyId",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    aws_access_key_id: Option<String>,
    #[serde(
        alias = "awsSecretAccessKey",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    aws_secret_access_key: Option<String>,
    #[serde(
        alias = "awsSessionToken",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    aws_session_token: Option<String>,
    #[serde(
        alias = "oidcRoleArn",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    oidc_role_arn: Option<String>,
    /// Platform (standard or google_vertex_ai)
    #[serde(default)]
    platform: AIPlatform,
    /// Custom HTTP headers to include in AI requests
    #[serde(default)]
    headers: HashMap<String, String>,
    /// Set only by `vertex_ai_federated` resources, which authenticate to Vertex AI
    /// through Google Cloud Workload Identity Federation.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    workload_identity_provider: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    service_account_email: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    workload_identity_audience: Option<String>,
    /// The API token of a `cloudflare` resource, which names it `token` rather than `api_key`.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    token: Option<String>,
    /// The account of a `cloudflare` resource, which its Workers AI URL is built from.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    account_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OAuthTokens {
    access_token: String,
}

/// An `anthropic_federated` resource: the Claude Console federation rule,
/// organization, service account and (for a rule enabled in several workspaces)
/// workspace to exchange the requesting user's OIDC token against.
///
/// It has no base URL on purpose. The exchanged token is the identity of whoever
/// uses the resource, not of whoever wrote it, so a configurable host would let
/// the resource's author collect other users' tokens.
///
/// Only `federation_rule_id` decides that a value is one: the other fields are
/// optional here and checked afterwards, so a resource missing one is reported
/// rather than read as a standard resource with no key.
#[derive(Deserialize, Debug)]
struct AIAnthropicFederatedResource {
    federation_rule_id: String,
    #[serde(default)]
    organization_id: String,
    #[serde(default)]
    service_account_id: String,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    workspace_id: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum AIResource {
    OAuth(AIOAuthResource),
    AnthropicFederated(AIAnthropicFederatedResource),
    Standard(AIStandardResource),
}

/// Resolve a `$var:` reference. When `user_db`/`authed` are provided the query
/// goes through an RLS-scoped connection so the caller can only read variables
/// they are authorised to access.  Without auth context the raw pool is used
/// (appropriate for admin/system paths where the resource was already validated).
async fn resolve_var(
    path: String,
    db: &DB,
    w_id: &str,
    user_db: Option<&UserDB>,
    authed: Option<&ApiAuthed>,
) -> Result<String> {
    match (user_db, authed) {
        (Some(udb), Some(auth)) => Ok(get_variable_or_self_as(path, db, udb, auth, w_id).await?),
        _ => Ok(get_variable_or_self(path, db, w_id).await?),
    }
}

async fn resolve_optional_var(
    value: Option<String>,
    db: &DB,
    w_id: &str,
    user_db: Option<&UserDB>,
    authed: Option<&ApiAuthed>,
) -> Result<Option<String>> {
    match value {
        Some(value) => Ok(Some(resolve_var(value, db, w_id, user_db, authed).await?)),
        None => Ok(None),
    }
}

async fn resolve_provider_credentials(
    provider: &AIProvider,
    db: &DB,
    w_id: &str,
    resource: AIResource,
    authed: Option<&ApiAuthed>,
) -> Result<ProviderCredentials> {
    // When authed is provided, resolve $var: references through RLS so that
    // users can only read variables they have permission to access.
    let user_db = authed.map(|_| UserDB::new(db.clone()));

    match resource {
        AIResource::Standard(mut resource) => {
            if let Some(base_url) = resource.base_url {
                resource.base_url =
                    Some(resolve_var(base_url, db, w_id, user_db.as_ref(), authed).await?);
            }
            // Skip get_base_url for Bedrock - it uses SDK directly, not HTTP
            let base_url = if matches!(provider, AIProvider::AWSBedrock) {
                String::new()
            } else if *provider == AIProvider::Cloudflare && resource.base_url.is_none() {
                let account_id = if let Some(account_id) = resource.account_id {
                    Some(resolve_var(account_id, db, w_id, user_db.as_ref(), authed).await?)
                } else {
                    None
                };
                windmill_ai::ai_providers::cloudflare_workers_ai_base_url(account_id.as_deref())?
            } else {
                provider.get_base_url(resource.base_url, db).await?
            };
            let api_key = match provider {
                AIProvider::Cloudflare => resource.api_key.or(resource.token),
                _ => resource.api_key,
            };
            let api_key = if let Some(api_key) = api_key {
                Some(resolve_var(api_key, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let organization_id = if let Some(organization_id) = resource.organization_id {
                Some(resolve_var(organization_id, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let aws_access_key_id = if let Some(access_key_id) = resource.aws_access_key_id {
                Some(resolve_var(access_key_id, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let aws_secret_access_key =
                if let Some(secret_access_key) = resource.aws_secret_access_key {
                    Some(resolve_var(secret_access_key, db, w_id, user_db.as_ref(), authed).await?)
                } else {
                    None
                };
            let aws_session_token = if let Some(session_token) = resource.aws_session_token {
                Some(resolve_var(session_token, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let oidc_role_arn = if let Some(role_arn) = resource.oidc_role_arn {
                Some(resolve_var(role_arn, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let workload_identity_provider = resolve_optional_var(
                resource.workload_identity_provider,
                db,
                w_id,
                user_db.as_ref(),
                authed,
            )
            .await?;
            let service_account_email = resolve_optional_var(
                resource.service_account_email,
                db,
                w_id,
                user_db.as_ref(),
                authed,
            )
            .await?;
            let workload_identity_audience = resolve_optional_var(
                resource.workload_identity_audience,
                db,
                w_id,
                user_db.as_ref(),
                authed,
            )
            .await?;
            let vertex_workload_identity = VertexWorkloadIdentity::from_resource(
                provider,
                workload_identity_provider.as_deref(),
                service_account_email.as_deref(),
                workload_identity_audience.as_deref(),
            )?;
            let platform = if vertex_workload_identity.is_some() {
                AIPlatform::GoogleVertexAi
            } else {
                resource.platform
            };

            Ok(ProviderCredentials {
                provider: provider.clone(),
                base_url,
                api_key,
                access_token: None,
                organization_id,
                user: None,
                region: resource.region,
                aws_access_key_id,
                aws_secret_access_key,
                aws_session_token,
                oidc_role_arn,
                anthropic_federation: None,
                vertex_workload_identity,
                platform,
                custom_headers: resource.headers,
            })
        }
        AIResource::AnthropicFederated(resource) => {
            if *provider != AIProvider::Anthropic {
                return Err(Error::BadRequest(format!(
                    "An anthropic_federated resource can only be used for the Anthropic provider, not {provider:?}"
                )));
            }
            let required = [
                ("federation_rule_id", &resource.federation_rule_id),
                ("organization_id", &resource.organization_id),
                ("service_account_id", &resource.service_account_id),
            ];
            if let Some((field, _)) = required.iter().find(|(_, value)| value.is_empty()) {
                return Err(Error::BadRequest(format!(
                    "The anthropic_federated resource needs {field} to be set"
                )));
            }

            Ok(ProviderCredentials {
                provider: provider.clone(),
                base_url: ANTHROPIC_API_BASE_URL.to_string(),
                api_key: None,
                access_token: None,
                organization_id: None,
                user: None,
                region: None,
                aws_access_key_id: None,
                aws_secret_access_key: None,
                aws_session_token: None,
                oidc_role_arn: None,
                anthropic_federation: Some(AnthropicFederation {
                    federation_rule_id: resource.federation_rule_id,
                    organization_id: resource.organization_id,
                    service_account_id: resource.service_account_id,
                    workspace_id: resource.workspace_id,
                }),
                vertex_workload_identity: None,
                platform: AIPlatform::Standard,
                custom_headers: HashMap::new(),
            })
        }
        AIResource::OAuth(resource) => {
            let user = if let Some(user) = resource.user.clone() {
                Some(resolve_var(user, db, w_id, user_db.as_ref(), authed).await?)
            } else {
                None
            };
            let token = get_token_using_oauth(resource, db, w_id, user_db.as_ref(), authed).await?;
            let base_url = provider.get_base_url(None, db).await?;

            Ok(ProviderCredentials {
                provider: provider.clone(),
                base_url,
                api_key: None,
                access_token: Some(token),
                organization_id: None,
                user,
                region: None,
                aws_access_key_id: None,
                aws_secret_access_key: None,
                aws_session_token: None,
                oidc_role_arn: None,
                anthropic_federation: None,
                vertex_workload_identity: None,
                platform: AIPlatform::Standard,
                custom_headers: HashMap::new(),
            })
        }
    }
}

async fn get_token_using_oauth(
    mut resource: AIOAuthResource,
    db: &DB,
    w_id: &str,
    user_db: Option<&UserDB>,
    authed: Option<&ApiAuthed>,
) -> Result<String> {
    resource.client_id = resolve_var(resource.client_id, db, w_id, user_db, authed).await?;
    resource.client_secret = resolve_var(resource.client_secret, db, w_id, user_db, authed).await?;
    resource.token_url = resolve_var(resource.token_url, db, w_id, user_db, authed).await?;
    // Validate the resolved token_url against SSRF rules before issuing the request,
    // mirroring the protection applied to base_url in `get_base_url` (same
    // ALLOW_PRIVATE_AI_BASE_URLS opt-in). Without this a workspace member could
    // point token_url at an internal/metadata address. The returned client pins
    // DNS to the validated address so the connect cannot rebind after the check.
    let client = pinned_ai_client_for(&resource.token_url).await?;
    let mut params = HashMap::new();
    params.insert("grant_type", "client_credentials");
    params.insert("scope", "https://cognitiveservices.azure.com/.default");
    let response = client
        .post(resource.token_url)
        .form(&params)
        .basic_auth(resource.client_id, Some(resource.client_secret))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|err| {
            Error::internal_err(format!(
                "Failed to get access token using credentials flow: {}",
                err
            ))
        })?;
    let response = response.json::<OAuthTokens>().await.map_err(|err| {
        Error::internal_err(format!(
            "Failed to parse access token from credentials flow: {}",
            err
        ))
    })?;
    Ok(response.access_token)
}

/// Exchange the resource's OIDC role ARN for temporary IAM credentials, in place.
///
/// Only the API holds the OIDC signing key, so the exchange happens here rather
/// than in the Bedrock handler, which then sees ordinary IAM keys. It runs after
/// the workspace credential cache on purpose: the STS session name names the user
/// making the request, and a cache shared by the whole workspace would hand one
/// user's session to another.
#[cfg(feature = "bedrock")]
async fn resolve_bedrock_oidc_credentials(
    db: &DB,
    w_id: &str,
    authed: &ApiAuthed,
    credentials: &mut ProviderCredentials,
) -> Result<()> {
    use windmill_ai::ai_bedrock::{aws_role_session_name, bedrock_oidc_role_to_assume};

    // Only the Bedrock resource type carries a role ARN; leaving the field set on
    // any other resource inert keeps a hand-edited one from reaching STS.
    if credentials.provider != AIProvider::AWSBedrock {
        return Ok(());
    }

    let Some(role_arn) = bedrock_oidc_role_to_assume(
        credentials.api_key.as_deref(),
        credentials.aws_access_key_id.as_deref(),
        credentials.aws_secret_access_key.as_deref(),
        credentials.oidc_role_arn.as_deref(),
    ) else {
        return Ok(());
    };

    // Ahead of the cache: STS credentials are region-independent, so a resource
    // with the same role but no region would otherwise reuse an entry and sign a
    // request against the empty region the guard exists to reject.
    let region = windmill_ai::ai_bedrock::bedrock_oidc_region(credentials.region.as_deref())?;

    let session_name = aws_role_session_name("windmill-ai-copilot", &authed.username);
    // Keyed on the email, the identity the OIDC token is minted for, rather than
    // on the session name derived from it: that name is sanitised and length
    // capped, so it is a lossy stand-in for the caller.
    let cache_key = (w_id.to_string(), role_arn.to_string(), authed.email.clone());

    let assumed = match BEDROCK_ASSUMED_ROLE_CACHE
        .get(&cache_key)
        .filter(windmill_ai::ai_bedrock::AssumedRoleCredentials::is_fresh)
    {
        Some(cached) => cached,
        None => {
            let assumed =
                assume_bedrock_role(db, w_id, authed, role_arn, region, &session_name).await?;
            BEDROCK_ASSUMED_ROLE_CACHE.insert(cache_key, assumed.clone());
            assumed
        }
    };

    credentials.aws_access_key_id = Some(assumed.access_key_id);
    credentials.aws_secret_access_key = Some(assumed.secret_access_key);
    credentials.aws_session_token = Some(assumed.session_token);

    Ok(())
}

#[cfg(feature = "bedrock")]
async fn assume_bedrock_role(
    db: &DB,
    w_id: &str,
    authed: &ApiAuthed,
    role_arn: &str,
    region: &str,
    session_name: &str,
) -> Result<windmill_ai::ai_bedrock::AssumedRoleCredentials> {
    #[cfg(all(feature = "enterprise", feature = "openidconnect", feature = "private"))]
    {
        use windmill_common::oidc_oss::{generate_id_token, OidcCaller, WorkspaceClaim};

        let id_token = generate_id_token(
            OidcCaller::Bedrock,
            Some(db),
            WorkspaceClaim { workspace: w_id.to_string() },
            windmill_ai::ai_bedrock::AWS_OIDC_AUDIENCE,
            format!("{}::{}", authed.email, w_id),
            Some(authed.email.clone()),
            None,
        )
        .await?;

        windmill_ai::ai_bedrock::assume_role_with_oidc_token(
            role_arn,
            region,
            id_token.to_string(),
            session_name,
        )
        .await
    }
    #[cfg(not(all(feature = "enterprise", feature = "openidconnect", feature = "private")))]
    {
        let _ = (db, w_id, authed, role_arn, region, session_name);
        Err(Error::BadRequest(
            "Assuming an AWS role through Windmill's OIDC provider requires an enterprise license"
                .to_string(),
        ))
    }
}

/// Exchange an OIDC token minted for the requesting user for an Anthropic access
/// token, in place, when the resource is set up for Workload Identity Federation.
///
/// Runs after the workspace credential cache for the same reason the Bedrock
/// exchange does: the token's `sub` names the user, and is what a federation rule
/// matches and the Claude Console's authentication history records.
async fn resolve_anthropic_federated_credentials(
    db: &DB,
    w_id: &str,
    authed: &ApiAuthed,
    credentials: &mut ProviderCredentials,
) -> Result<()> {
    let Some(federation) = credentials.anthropic_federation.as_ref() else {
        return Ok(());
    };

    let cache_key = (w_id.to_string(), federation.clone(), authed.email.clone());
    let token = match ANTHROPIC_FEDERATED_TOKEN_CACHE
        .get(&cache_key)
        .filter(FederatedAccessToken::is_fresh)
    {
        Some(cached) => cached,
        None => {
            let assertion = mint_anthropic_oidc_token(db, w_id, authed).await?;
            let client = pinned_ai_client_for(&credentials.base_url).await?;
            let token =
                exchange_federated_token(&client, &credentials.base_url, federation, &assertion)
                    .await?;
            ANTHROPIC_FEDERATED_TOKEN_CACHE.insert(cache_key, token.clone());
            token
        }
    };

    credentials.access_token = Some(token.access_token);
    Ok(())
}

async fn mint_anthropic_oidc_token(db: &DB, w_id: &str, authed: &ApiAuthed) -> Result<String> {
    #[cfg(all(feature = "enterprise", feature = "openidconnect", feature = "private"))]
    {
        use windmill_ai::providers::anthropic::{
            ANTHROPIC_OIDC_AUDIENCE, ANTHROPIC_OIDC_TOKEN_LIFETIME_SECS,
        };
        use windmill_common::oidc_oss::{generate_id_token, OidcCaller, WorkspaceClaim};

        let id_token = generate_id_token(
            OidcCaller::Anthropic,
            Some(db),
            WorkspaceClaim { workspace: w_id.to_string() },
            ANTHROPIC_OIDC_AUDIENCE,
            format!("{}::{}", authed.email, w_id),
            Some(authed.email.clone()),
            Some(ANTHROPIC_OIDC_TOKEN_LIFETIME_SECS),
        )
        .await?;
        Ok(id_token.to_string())
    }
    #[cfg(not(all(feature = "enterprise", feature = "openidconnect", feature = "private")))]
    {
        let _ = (db, w_id, authed);
        Err(Error::BadRequest(
            "Anthropic Workload Identity Federation through Windmill's OIDC provider requires an enterprise license"
                .to_string(),
        ))
    }
}

/// Exchange an OIDC token minted for the requesting user for a Google access
/// token, sent as the Vertex bearer token, when the resource authenticates through
/// Workload Identity Federation. Runs after the workspace credential cache for the
/// same reason the Anthropic exchange does: the token names the user.
async fn resolve_vertex_workload_identity_credentials(
    db: &DB,
    w_id: &str,
    authed: &ApiAuthed,
    credentials: &mut ProviderCredentials,
) -> Result<()> {
    let Some(identity) = credentials.vertex_workload_identity.as_ref() else {
        return Ok(());
    };
    check_vertex_base_url(&credentials.base_url)?;

    let cache_key = (w_id.to_string(), identity.clone(), authed.email.clone());
    let token = match VERTEX_WORKLOAD_IDENTITY_TOKEN_CACHE
        .get(&cache_key)
        .filter(GcpAccessToken::is_fresh)
    {
        Some(cached) => cached,
        None => {
            let subject_token =
                mint_vertex_oidc_token(db, w_id, authed, &identity.oidc_audience()).await?;
            let token = exchange_oidc_token(identity, &subject_token).await?;
            VERTEX_WORKLOAD_IDENTITY_TOKEN_CACHE.insert(cache_key, token.clone());
            token
        }
    };

    credentials.api_key = Some(token.access_token);
    Ok(())
}

async fn mint_vertex_oidc_token(
    db: &DB,
    w_id: &str,
    authed: &ApiAuthed,
    audience: &str,
) -> Result<String> {
    #[cfg(all(feature = "enterprise", feature = "openidconnect", feature = "private"))]
    {
        use windmill_ai::vertex_workload_identity::VERTEX_OIDC_TOKEN_LIFETIME_SECS;
        use windmill_common::oidc_oss::{generate_id_token, OidcCaller, WorkspaceClaim};

        let id_token = generate_id_token(
            OidcCaller::GoogleVertex,
            Some(db),
            WorkspaceClaim { workspace: w_id.to_string() },
            audience,
            format!("{}::{}", authed.email, w_id),
            Some(authed.email.clone()),
            Some(VERTEX_OIDC_TOKEN_LIFETIME_SECS),
        )
        .await?;
        Ok(id_token.to_string())
    }
    #[cfg(not(all(feature = "enterprise", feature = "openidconnect", feature = "private")))]
    {
        let _ = (db, w_id, authed, audience);
        Err(Error::BadRequest(
            "Vertex AI Workload Identity Federation through Windmill's OIDC provider requires an enterprise license"
                .to_string(),
        ))
    }
}

#[derive(Clone, Debug)]
pub struct ExpiringProviderCredentials {
    credentials: ProviderCredentials,
    policy: ModelPolicy,
    expires_at: std::time::Instant,
    instance_ai_config_revision: Option<u64>,
}

/// The models a request served with the effective AI config's own key may ask one provider
/// for, and the output limit each is held to. The UI only offers what the settings enable;
/// this holds a direct API caller to it.
#[derive(Clone, Debug, Default)]
pub struct ModelPolicy {
    models: std::collections::HashSet<String>,
    max_tokens: HashMap<String, i64>,
}

/// The fields of a proxied body that [`ModelPolicy::check_body`] reads.
#[derive(Deserialize)]
struct ProxiedModel {
    model: String,
    models: Option<Box<RawValue>>,
    fallbacks: Option<Box<RawValue>>,
}

impl ModelPolicy {
    fn new(config: &AIConfig, provider: &AIProvider) -> Self {
        let enabled = config
            .providers
            .as_ref()
            .and_then(|providers| providers.get(provider))
            .into_iter()
            .flat_map(|p| p.models.iter());
        // Settings allow picking these without enabling them in the provider's list.
        let picked = [
            &config.default_model,
            &config.metadata_model,
            &config.code_completion_model,
        ]
        .into_iter()
        .flatten()
        .filter(|m| &m.provider == provider)
        .map(|m| &m.model);
        // The browser sends a legacy `x/thinking` entry as `x`.
        let models = enabled
            .chain(picked)
            .map(|m| m.strip_suffix("/thinking").unwrap_or(m).to_string())
            .collect();
        let prefix = format!(
            "{}:",
            serde_json::to_string(provider)
                .unwrap_or_default()
                .trim_matches('"')
        );
        let max_tokens = config
            .max_tokens_per_model
            .iter()
            .flatten()
            .filter_map(|(key, limit)| {
                Some((key.strip_prefix(&prefix)?.to_string(), i64::from(*limit)))
            })
            .collect();
        Self { models, max_tokens }
    }

    /// The output limit to send `model` with: `requested` capped at the configured limit, or
    /// the limit itself when none was requested. Below 1 counts as none, since `-1` or `0`
    /// mean "no limit" on servers such as Ollama. A model with no limit keeps `requested`.
    fn output_tokens(&self, model: &str, requested: Option<i64>) -> Option<i64> {
        let Some(&limit) = self.max_tokens.get(model) else {
            return requested;
        };
        Some(
            requested
                .filter(|n| *n >= 1)
                .map_or(limit, |n| n.min(limit)),
        )
    }

    /// Writes the configured limit into a body [`Self::check_body`] accepted for `model`. A
    /// model with no limit keeps its body byte for byte.
    fn limit_body(
        &self,
        model: &str,
        provider: &AIProvider,
        platform: &AIPlatform,
        ai_path: &str,
        body: &Bytes,
    ) -> Result<Bytes> {
        if !self.max_tokens.contains_key(model) {
            return Ok(body.clone());
        }
        let mut value: serde_json::Value = serde_json::from_slice(body)
            .map_err(|e| Error::BadRequest(format!("invalid AI request body: {e}")))?;
        let Some(obj) = value.as_object_mut() else {
            return Ok(body.clone());
        };
        // The limit holds per choice, so several choices would multiply it. Legacy completions
        // answer each prompt of a list; a flat list of tokens is one prompt.
        let batched = obj
            .get("prompt")
            .and_then(|p| p.as_array())
            .is_some_and(|a| a.len() > 1 && !a.iter().all(|v| v.is_number()));
        if batched
            || ["n", "best_of"].iter().any(|k| {
                obj.get(*k)
                    .is_some_and(|v| !v.is_null() && v.as_u64() != Some(1))
            })
        {
            return Err(Error::BadRequest(
                "only one prompt and one choice per request are supported with the workspace's AI key"
                    .to_string(),
            ));
        }
        // Bedrock and Anthropic on Vertex generate for any POST, whatever its path; FIM is
        // matched as its conversion matches it. A path that neither generates nor counts is
        // refused rather than forwarded without the limit.
        let path = ai_path.trim_end_matches('/');
        let fim = path.contains("fim/completions");
        let any_post_generates = *provider == AIProvider::AWSBedrock
            || *provider == AIProvider::Anthropic && *platform == AIPlatform::GoogleVertexAi;
        let generates = any_post_generates
            || fim
            || ["completions", "responses", "messages"]
                .iter()
                .any(|p| path.ends_with(p));
        if !generates {
            if ["count_tokens", "input_tokens", "embeddings"]
                .iter()
                .any(|p| path.ends_with(p))
            {
                return Ok(body.clone());
            }
            return Err(Error::BadRequest(format!(
                "\"{ai_path}\" can't be held to the token limit set for \"{model}\""
            )));
        }
        // A caller's `max_tokens` stays where it is: any provider's base URL may point at a
        // server that reads only `max_tokens` (Ollama), and the browser sends it there.
        let field = if any_post_generates
            || fim
            || path.ends_with("messages")
            || *provider == AIProvider::GoogleAI
        {
            "max_tokens"
        } else if path.ends_with("responses") {
            "max_output_tokens"
        } else if path.ends_with("chat/completions")
            && reads_max_completion_tokens(provider)
            && !obj.contains_key("max_tokens")
        {
            "max_completion_tokens"
        } else {
            "max_tokens"
        };
        // The smallest asked for that `output_tokens` counts as a request.
        let requested = ["max_tokens", "max_completion_tokens", "max_output_tokens"]
            .iter()
            .filter_map(|f| obj.remove(*f)?.as_i64())
            .filter(|n| *n >= 1)
            .min();
        obj.insert(
            field.to_string(),
            self.output_tokens(model, requested).into(),
        );
        Ok(Bytes::from(serde_json::to_vec(&value).map_err(to_anyhow)?))
    }

    fn check_model(&self, model: &str) -> Result<()> {
        if self.models.contains(model) {
            Ok(())
        } else {
            Err(Error::BadRequest(format!(
                "model \"{model}\" is not enabled for this workspace"
            )))
        }
    }

    /// A body-less GET (a model listing) passes; any other request must be a JSON object
    /// naming an enabled model, which is returned.
    fn check_body(&self, method: &Method, body: &Bytes) -> Result<Option<String>> {
        if method == Method::GET && body.is_empty() {
            return Ok(None);
        }
        let invalid = |e: String| {
            Error::BadRequest(format!(
                "with the workspace's AI key, the request body must be a JSON object with a string \"model\": {e}"
            ))
        };
        // A struct also deserializes from a positional array, which the limit can't be written into.
        if !body.trim_ascii_start().starts_with(b"{") {
            return Err(invalid("not an object".to_string()));
        }
        let parsed: ProxiedModel =
            serde_json::from_slice(body).map_err(|e| invalid(e.to_string()))?;
        // Gateways fall back to the models listed here (OpenRouter `models`, LiteLLM
        // `fallbacks`), which would run unchecked.
        for (field, value) in [("models", &parsed.models), ("fallbacks", &parsed.fallbacks)] {
            if value.is_some() {
                return Err(Error::BadRequest(format!(
                    "\"{field}\" (fallback models) is not supported with the workspace's AI key"
                )));
            }
        }
        self.check_model(&parsed.model)?;
        Ok(Some(parsed.model))
    }
}

impl ExpiringProviderCredentials {
    fn new(
        credentials: ProviderCredentials,
        policy: ModelPolicy,
        instance_ai_config_revision: Option<u64>,
    ) -> Self {
        Self {
            credentials,
            policy,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(60),
            instance_ai_config_revision,
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at < std::time::Instant::now()
            || self
                .instance_ai_config_revision
                .is_some_and(|revision| revision != current_instance_ai_config_revision())
    }
}

/// Set on the copilot config when the workspace has no AI provider of its own and is
/// running on Windmill's free tier, so the client can label the lent model as free, warn
/// before the grant runs out, and tell the user to add their own key once it has — rather
/// than showing the same "no provider configured" state a never-configured workspace gets.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct FreeTierInfo {
    /// The grant is spent: no provider is served and the user must bring their own key.
    pub exhausted: bool,
    /// Fraction of the grant consumed, 0.0..=1.0. A ratio, not a dollar amount — the
    /// pricing model stays server-side.
    pub used_ratio: f64,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AIConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<HashMap<AIProvider, ProviderConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<ProviderModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_model: Option<ProviderModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_completion_model: Option<ProviderModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_prompts: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_model: Option<HashMap<String, i32>>,
    /// Response-only: this same struct is the request body for saving a workspace's AI
    /// config, and `skip_deserializing` is what stops a client from storing a forged
    /// free-tier marker. Only the server sets it, per-request.
    #[serde(skip_serializing_if = "Option::is_none", skip_deserializing)]
    pub free_tier: Option<FreeTierInfo>,
    /// Per-model price overrides, keyed `provider:model` like `max_tokens_per_model`.
    /// Only models whose rates differ from the built-in table are stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_pricing: Option<HashMap<String, ModelPriceOverride>>,
    /// Per-model context windows the chat budgets its history against, keyed
    /// `provider:model` like `max_tokens_per_model`. Overrides the client's built-in
    /// table, which a model served through a custom endpoint is usually missing from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window_per_model: Option<HashMap<String, i32>>,
    /// Hides the Windmill AI assistant (chat, sessions, generation, completion, fixes) from
    /// the workspace UI. Only the workspace's own row is consulted: the flag holds even when
    /// the providers served come from the instance config or the free tier. AI agent steps
    /// and the AI sandbox are unaffected, so the providers stay in force.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub copilot_disabled: bool,
    /// Stops browsers from backing their AI sessions up to the workspace's object storage
    /// (`ai_sessions.rs`). Read from the workspace's own row like `copilot_disabled`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sessions_storage_disabled: bool,
    /// The server's sweep (`ai_sessions.rs`) deletes the backup of a session no push has
    /// reached for this many days. The copies in members' browsers are untouched. Unset
    /// keeps backups until the user deletes the session. Read from the workspace's own row
    /// like `copilot_disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sessions_retention_days: Option<u32>,
    /// The search API the AI chat searches the web and reads pages through when the model's
    /// provider has no built-in search, or the workspace turned it off. Read from the
    /// workspace's own row like `copilot_disabled`, its resource from that workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_search: Option<WebSearchConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebSearchConfig {
    pub provider: WebSearchProvider,
    pub resource_path: String,
}

/// Negotiated rates in USD per million tokens. An unset cache rate is read as the
/// provider's own multiple of the input rate where the model has a published one,
/// and as the input rate itself where it does not — an unstated discount is never
/// filled in from another vendor's.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelPriceOverride {
    pub input: f64,
    pub output: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
}

impl ModelPriceOverride {
    pub fn validate(&self, key: &str) -> Result<()> {
        for (field, rate) in [
            ("input", Some(self.input)),
            ("output", Some(self.output)),
            ("cache_read", self.cache_read),
            ("cache_write", self.cache_write),
        ] {
            let Some(rate) = rate else { continue };
            if !rate.is_finite() || rate < 0.0 || rate > MAX_MODEL_RATE {
                return Err(Error::BadRequest(format!(
                    "Price override for {}: {} must be between 0 and {}",
                    key, field, MAX_MODEL_RATE
                )));
            }
        }
        Ok(())
    }
}

/// Ten years: past any plausible retention, and well within what a day count is turned into.
pub const MAX_SESSIONS_RETENTION_DAYS: u32 = 3650;

impl AIConfig {
    pub fn validate_model_pricing(&self) -> Result<()> {
        for (key, price) in self.model_pricing.iter().flatten() {
            price.validate(key)?;
        }
        Ok(())
    }

    pub fn validate_token_limits(&self) -> Result<()> {
        for (entries, bounds) in [
            (&self.context_window_per_model, &CONTEXT_WINDOWS),
            (&self.max_tokens_per_model, &OUTPUT_LIMITS),
        ] {
            for (key, tokens) in entries.iter().flatten() {
                validate_token_limit(bounds, key, i64::from(*tokens)).map_err(Error::BadRequest)?;
            }
        }
        Ok(())
    }

    pub fn validate_sessions_retention(&self) -> Result<()> {
        match self.sessions_retention_days {
            Some(days) if !(1..=MAX_SESSIONS_RETENTION_DAYS).contains(&days) => {
                Err(Error::BadRequest(format!(
                    "AI session retention must be between 1 and {MAX_SESSIONS_RETENTION_DAYS} days (got {days})"
                )))
            }
            _ => Ok(()),
        }
    }

    pub fn has_providers(&self) -> bool {
        self.providers
            .as_ref()
            .is_some_and(|providers| !providers.is_empty())
    }
}

/// The AI config requests in `w_id` are served from: the workspace's own when it
/// configures a provider, else the instance's, with the workspace the configured resource
/// paths live in and, for the instance config, the revision its cache entries are tied to.
/// `None` when neither configures a provider.
async fn effective_ai_config(
    db: &DB,
    w_id: &str,
) -> Result<Option<(AIConfig, String, Option<u64>)>> {
    let workspace_ai_config = sqlx::query_scalar!(
        "SELECT ai_config FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_one(db)
    .await?;
    if let Some(config) = workspace_ai_config
        .and_then(|v| serde_json::from_value::<AIConfig>(v).ok())
        .filter(AIConfig::has_providers)
    {
        return Ok(Some((config, w_id.to_string(), None)));
    }
    let instance_config =
        sqlx::query_scalar!("SELECT value FROM global_settings WHERE name = 'ai_config'")
            .fetch_optional(db)
            .await?;
    // An instance `ai_config` row with no usable provider (e.g. `{}` or
    // `{"providers":{}}`) is treated as unconfigured, exactly as
    // build_copilot_settings_state does — otherwise its mere presence would suppress the
    // free-tier fallback.
    Ok(instance_config
        .and_then(|v| serde_json::from_value::<AIConfig>(v).ok())
        .filter(AIConfig::has_providers)
        .map(|config| {
            (
                config,
                "admins".to_string(),
                Some(current_instance_ai_config_revision()),
            )
        }))
}

/// Credentials for `provider` from the effective AI config, or `None` when no provider is
/// configured at all.
async fn configured_credentials(
    db: &DB,
    w_id: &str,
    provider: &AIProvider,
) -> Result<Option<(ProviderCredentials, ModelPolicy)>> {
    let cache_key = (w_id.to_string(), provider.clone());
    if let Some(cached) = AI_REQUEST_CACHE.get(&cache_key).filter(|c| !c.is_expired()) {
        return Ok(Some((cached.credentials, cached.policy)));
    }
    let Some((mut ai_config, resource_workspace, instance_ai_config_revision)) =
        effective_ai_config(db, w_id).await?
    else {
        return Ok(None);
    };
    let policy = ModelPolicy::new(&ai_config, provider);
    let provider_config = ai_config
        .providers
        .as_mut()
        .and_then(|providers| providers.remove(provider))
        .ok_or_else(|| Error::BadRequest(format!("Provider {:?} not configured", provider)))?;
    let resource_path = provider_config.resource_path;
    if resource_path.is_empty() {
        return Err(Error::BadRequest("Resource path is empty".to_string()));
    }
    // The resource path was validated by an admin/devops user when configuring the AI
    // config, so the raw pool is used.
    let res = sqlx::query_scalar::<_, Option<sqlx::types::Json<Box<RawValue>>>>(
        "SELECT value FROM resource WHERE path = $1 AND workspace_id = $2",
    )
    .bind(&resource_path)
    .bind(&resource_workspace)
    .fetch_optional(db)
    .await?;
    let resource = parse_ai_resource(res, &resource_path)?;
    let credentials =
        resolve_provider_credentials(provider, db, &resource_workspace, resource, None).await?;
    AI_REQUEST_CACHE.insert(
        cache_key,
        ExpiringProviderCredentials::new(
            credentials.clone(),
            policy.clone(),
            instance_ai_config_revision,
        ),
    );
    Ok(Some((credentials, policy)))
}

fn parse_ai_resource(
    row: Option<Option<sqlx::types::Json<Box<RawValue>>>>,
    resource_path: &str,
) -> Result<AIResource> {
    let value = row
        .ok_or_else(|| {
            Error::NotFound(format!(
                "Could not find the resource {}, update the resource path in the workspace settings",
                resource_path
            ))
        })?
        .ok_or_else(|| Error::BadRequest(format!("Empty resource value for {}", resource_path)))?;
    serde_json::from_str::<AIResource>(value.0.get()).map_err(|e| Error::BadRequest(e.to_string()))
}

/// The workspace's search API and its key. The resource was picked by a workspace admin, so,
/// like a provider's, it is read with the raw pool: a member searches without read access to
/// the key.
async fn web_search_credentials(
    authed: &ApiAuthed,
    db: &DB,
    w_id: &str,
) -> Result<(WebSearchProvider, String)> {
    let config = sqlx::query_scalar!(
        "SELECT ai_config FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_one(db)
    .await?
    .and_then(|v| serde_json::from_value::<AIConfig>(v).ok())
    .and_then(|c| c.web_search)
    .ok_or_else(|| {
        Error::BadRequest(
            "No web search API is configured for this workspace: a workspace admin can set one in the workspace's AI settings".to_string(),
        )
    })?;
    check_scopes(authed, || {
        format!("resources:read:{}", config.resource_path)
    })?;
    let value = sqlx::query_scalar::<_, Option<sqlx::types::Json<serde_json::Value>>>(
        "SELECT value FROM resource WHERE path = $1 AND workspace_id = $2",
    )
    .bind(&config.resource_path)
    .bind(w_id)
    .fetch_optional(db)
    .await?
    .flatten()
    .ok_or_else(|| {
        Error::NotFound(format!(
            "Could not find the web search resource {}, update it in the workspace's AI settings",
            config.resource_path
        ))
    })?;
    // `exa` predates the hub's snake_case keys; newer API-key types name the key `token`.
    let api_key = ["token", "api_key", "apiKey"]
        .iter()
        .find_map(|key| {
            value
                .get(*key)
                .and_then(serde_json::Value::as_str)
                .filter(|k| !k.is_empty())
        })
        .ok_or_else(|| {
            Error::BadRequest(format!(
                "The web search resource {} has no API key",
                config.resource_path
            ))
        })?;
    let api_key = get_variable_or_self(api_key.to_string(), db, w_id).await?;
    Ok((config.provider, api_key))
}

fn log_web_search_usage(provider: WebSearchProvider, action: &str) {
    windmill_common::feature_usage::log_feature_usage(
        "ai_chat",
        "web_search_backend",
        &format!("{}:{action}", provider.id()),
    );
}

#[derive(Deserialize)]
struct WebSearchRequest {
    query: String,
    #[serde(default)]
    max_results: Option<usize>,
}

#[derive(Serialize)]
struct WebSearchResponse {
    results: Vec<WebSearchResult>,
}

async fn web_search(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(request): Json<WebSearchRequest>,
) -> JsonResult<WebSearchResponse> {
    let query = request.query.trim();
    if query.is_empty() {
        return Err(Error::BadRequest("The search query is empty".to_string()));
    }
    let (provider, api_key) = web_search_credentials(&authed, &db, &w_id).await?;
    let results = provider
        .search(&api_key, query, request.max_results)
        .await?;
    log_web_search_usage(provider, "search");
    Ok(Json(WebSearchResponse { results }))
}

#[derive(Deserialize)]
struct WebFetchRequest {
    url: String,
}

async fn web_fetch(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(request): Json<WebFetchRequest>,
) -> JsonResult<WebPage> {
    let url = reqwest::Url::parse(request.url.trim())
        .ok()
        .filter(|u| matches!(u.scheme(), "http" | "https"))
        .ok_or_else(|| Error::BadRequest("The URL must be an http(s) URL".to_string()))?;
    let (provider, api_key) = web_search_credentials(&authed, &db, &w_id).await?;
    let page = provider.fetch(&api_key, url.as_str()).await?;
    log_web_search_usage(provider, "fetch");
    Ok(Json(page))
}

pub fn global_service() -> Router {
    Router::new().route("/proxy/{*ai}", post(global_proxy).get(global_proxy))
}

pub fn workspaced_service() -> Router {
    let router = Router::new()
        .route("/proxy/{*ai}", post(proxy).get(proxy))
        .route("/complete", post(complete))
        .route("/web_search", post(web_search))
        .route("/web_fetch", post(web_fetch))
        .route(
            "/usage",
            post(record_ai_usage)
                .get(list_ai_usage)
                // The handler caps how many events it *stores*, but Json deserializes
                // the whole array first — without a body limit an authenticated member
                // could make the server allocate and parse an arbitrarily large one.
                // Sized well above a full batch of the shape below.
                .layer(DefaultBodyLimit::max(AI_USAGE_BODY_LIMIT)),
        )
        .nest(
            "/shared_artifacts",
            crate::ai_shared_artifacts::workspaced_service(),
        )
        .nest(
            "/shared_sessions",
            crate::ai_shared_sessions::workspaced_service(),
        );

    #[cfg(feature = "bedrock")]
    let router = router.route("/check_bedrock_credentials", get(check_bedrock_credentials));

    #[cfg(feature = "parquet")]
    let router = router.nest("/sessions", crate::ai_sessions::workspaced_service());

    router
}

/// One provider request's worth of tokens, as counted by the chat client.
#[derive(Deserialize)]
struct AIUsageEvent {
    provider: String,
    model: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    cache_read_tokens: i64,
    #[serde(default)]
    cache_write_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
    /// Only the providers that bill back an exact figure set this.
    #[serde(default)]
    reported_cost_nano_usd: Option<i64>,
    #[serde(default)]
    requests: Option<i64>,
}

#[derive(Deserialize)]
struct RecordAIUsagePayload {
    events: Vec<AIUsageEvent>,
}

const MAX_AI_USAGE_EVENTS: usize = 50;
/// 64 KiB — a 50-event batch is a few kB even with the longest model ids.
const AI_USAGE_BODY_LIMIT: usize = 64 * 1024;
/// Well above any single conversation and far below an i64 overflow, so a client
/// bug caps out at one absurd row instead of poisoning the running total.
const MAX_TOKENS_PER_EVENT: i64 = 100_000_000;
/// $1000 in nano-USD.
const MAX_REPORTED_COST_PER_EVENT: i64 = 1_000_000_000_000;

/// Model ids carry vendor prefixes and variant suffixes (`anthropic/claude-opus-5:thinking`),
/// so the shape check is looser than an identifier but still excludes whitespace and
/// anything that would not be a model id.
fn is_model_shaped(s: &str, max_len: usize) -> bool {
    !s.is_empty()
        && s.len() <= max_len
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ':' | '.' | '/' | '~'))
}

/// Accumulate one workspace's AI token spend. Values are clamped and the caller's
/// email comes from the session, never the payload — the client is trusted to
/// report its own usage, not to attribute it to someone else.
async fn record_ai_usage(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(payload): Json<RecordAIUsagePayload>,
) -> Result<StatusCode> {
    // Pre-sum duplicate keys: two rows hitting the same conflict target in a single
    // INSERT error out ("cannot affect row a second time").
    let mut agg: HashMap<(String, String, String), AIUsageTotals> = HashMap::new();
    for e in payload.events.into_iter().take(MAX_AI_USAGE_EVENTS) {
        if AIProvider::try_from(e.provider.as_str()).is_err()
            || !is_model_shaped(&e.model, 255)
            || !(e.session_id.is_empty() || is_model_shaped(&e.session_id, 50))
        {
            continue;
        }
        let totals = agg
            .entry((e.provider, e.model, e.session_id))
            .or_insert_with(AIUsageTotals::default);
        totals.input += e.input_tokens.clamp(0, MAX_TOKENS_PER_EVENT);
        totals.cache_read += e.cache_read_tokens.clamp(0, MAX_TOKENS_PER_EVENT);
        totals.cache_write += e.cache_write_tokens.clamp(0, MAX_TOKENS_PER_EVENT);
        totals.output += e.output_tokens.clamp(0, MAX_TOKENS_PER_EVENT);
        totals.requests += e.requests.unwrap_or(1).clamp(0, MAX_AI_USAGE_EVENTS as i64);
        if let Some(cost) = e.reported_cost_nano_usd {
            totals.reported_cost = Some(
                totals.reported_cost.unwrap_or(0) + cost.clamp(0, MAX_REPORTED_COST_PER_EVENT),
            );
        }
    }
    if agg.is_empty() {
        return Ok(StatusCode::NO_CONTENT);
    }

    let mut providers = Vec::with_capacity(agg.len());
    let mut models = Vec::with_capacity(agg.len());
    let mut session_ids = Vec::with_capacity(agg.len());
    let mut inputs = Vec::with_capacity(agg.len());
    let mut cache_reads = Vec::with_capacity(agg.len());
    let mut cache_writes = Vec::with_capacity(agg.len());
    let mut outputs = Vec::with_capacity(agg.len());
    let mut reported_costs: Vec<Option<i64>> = Vec::with_capacity(agg.len());
    let mut requests = Vec::with_capacity(agg.len());
    for ((provider, model, session_id), totals) in agg {
        providers.push(provider);
        models.push(model);
        session_ids.push(session_id);
        inputs.push(totals.input);
        cache_reads.push(totals.cache_read);
        cache_writes.push(totals.cache_write);
        outputs.push(totals.output);
        reported_costs.push(totals.reported_cost);
        requests.push(totals.requests);
    }

    sqlx::query!(
        "INSERT INTO ai_token_usage (workspace_id, email, provider, model, session_id, \
            input_tokens, cache_read_tokens, cache_write_tokens, output_tokens, \
            reported_cost_nano_usd, requests)
         SELECT $1, $2, * FROM UNNEST($3::text[], $4::text[], $5::text[], $6::bigint[], \
            $7::bigint[], $8::bigint[], $9::bigint[], $10::bigint[], $11::bigint[])
         ON CONFLICT (workspace_id, day, email, provider, model, session_id)
         DO UPDATE SET
            input_tokens = ai_token_usage.input_tokens + EXCLUDED.input_tokens,
            cache_read_tokens = ai_token_usage.cache_read_tokens + EXCLUDED.cache_read_tokens,
            cache_write_tokens = ai_token_usage.cache_write_tokens + EXCLUDED.cache_write_tokens,
            output_tokens = ai_token_usage.output_tokens + EXCLUDED.output_tokens,
            reported_cost_nano_usd = CASE
                WHEN EXCLUDED.reported_cost_nano_usd IS NULL
                    THEN ai_token_usage.reported_cost_nano_usd
                ELSE COALESCE(ai_token_usage.reported_cost_nano_usd, 0)
                    + EXCLUDED.reported_cost_nano_usd
            END,
            requests = ai_token_usage.requests + EXCLUDED.requests,
            updated_at = now()",
        &w_id,
        &authed.email,
        &providers,
        &models,
        &session_ids,
        &inputs,
        &cache_reads,
        &cache_writes,
        &outputs,
        &reported_costs as &[Option<i64>],
        &requests
    )
    .execute(&db)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Default)]
struct AIUsageTotals {
    input: i64,
    cache_read: i64,
    cache_write: i64,
    output: i64,
    reported_cost: Option<i64>,
    requests: i64,
}

#[derive(Deserialize)]
struct ListAIUsageQuery {
    days: Option<i32>,
    group_by: Option<String>,
    scope: Option<String>,
}

/// A bucket always carries its provider and model: the caller prices it from a
/// per-model rate table, which a bucket spanning several models could not be
/// resolved against.
#[derive(Serialize)]
struct AITokenUsageBucket {
    key: String,
    provider: String,
    model: String,
    input_tokens: i64,
    cache_read_tokens: i64,
    cache_write_tokens: i64,
    output_tokens: i64,
    reported_cost_nano_usd: Option<i64>,
    requests: i64,
}

/// Grouping by day over a long range, or by model across many models, can produce
/// more buckets than a table is worth rendering, so the listing is capped.
/// `truncated` says so explicitly — a caller that sums the rows into a total must be
/// able to tell that the total is partial rather than silently under-reporting spend.
#[derive(Serialize)]
struct AITokenUsageListing {
    buckets: Vec<AITokenUsageBucket>,
    truncated: bool,
}

const AI_USAGE_MAX_BUCKETS: i64 = 1000;

async fn list_ai_usage(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(query): Query<ListAIUsageQuery>,
) -> Result<Json<AITokenUsageListing>> {
    // Reading the whole workspace's spend is an admin view; reading your own is
    // not, so a member can see what they are costing without being shown their
    // colleagues'. The filter is the session's email, never a parameter.
    let own_email = match query.scope.as_deref().unwrap_or("workspace") {
        "workspace" => {
            require_admin(authed.is_admin, &authed.username)?;
            None
        }
        "self" => Some(authed.email.clone()),
        scope => return Err(Error::BadRequest(format!("Unsupported scope: {}", scope))),
    };

    let days = query.days.unwrap_or(30).clamp(1, 365);
    let group_by = query.group_by.as_deref().unwrap_or("day");
    // No `session`: a session is identified by a client-generated id whose name
    // lives only in the browser that made it, so a bucket keyed on one is a label
    // nobody can resolve. `session_id` is still stored, at the grain the client
    // batches on, should sessions ever gain a server-side name.
    if !matches!(group_by, "day" | "user" | "model") {
        return Err(Error::BadRequest(format!(
            "Unsupported group_by: {}",
            group_by
        )));
    }

    // Fetch one past the cap to detect truncation. Ordering is by token volume, not
    // by cost: rates are applied by the caller, so this query cannot know what a
    // bucket cost. Volume is the closest proxy available here, and the caller is told
    // the listing was capped rather than being left to sum a partial set silently.
    let mut rows = sqlx::query_as!(
        AITokenUsageBucket,
        r#"SELECT
            (CASE $3::text
                WHEN 'day' THEN day::text
                WHEN 'user' THEN email
                ELSE ''
            END) AS "key!",
            provider AS "provider!",
            model AS "model!",
            SUM(input_tokens)::bigint AS "input_tokens!",
            SUM(cache_read_tokens)::bigint AS "cache_read_tokens!",
            SUM(cache_write_tokens)::bigint AS "cache_write_tokens!",
            SUM(output_tokens)::bigint AS "output_tokens!",
            SUM(reported_cost_nano_usd)::bigint AS "reported_cost_nano_usd",
            SUM(requests)::bigint AS "requests!"
         FROM ai_token_usage
         WHERE workspace_id = $1 AND day > CURRENT_DATE - $2::int
            AND ($5::text IS NULL OR email = $5)
         GROUP BY 1, provider, model
         ORDER BY SUM(input_tokens + cache_read_tokens + cache_write_tokens + output_tokens) DESC
         LIMIT $4"#,
        &w_id,
        days,
        group_by,
        AI_USAGE_MAX_BUCKETS + 1,
        own_email.as_deref()
    )
    .fetch_all(&db)
    .await?;

    let truncated = rows.len() as i64 > AI_USAGE_MAX_BUCKETS;
    rows.truncate(AI_USAGE_MAX_BUCKETS as usize);

    Ok(Json(AITokenUsageListing { buckets: rows, truncated }))
}

/// Check if AWS Bedrock credentials are available from environment variables.
#[cfg(feature = "bedrock")]
async fn check_bedrock_credentials(
    _authed: ApiAuthed,
    Path(_w_id): Path<String>,
) -> Result<Json<windmill_ai::ai_bedrock::BedrockCredentialsCheck>> {
    let response = windmill_ai::ai_bedrock::check_env_credentials().await;
    Ok(Json(response))
}

fn is_sse_response(headers: &HeaderMap) -> bool {
    headers
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/event-stream"))
        .unwrap_or(false)
}

fn proxy_request_to_request_builder(
    client: &Client,
    proxy_request: ProxyRequest,
) -> RequestBuilder {
    let mut request = client.request(proxy_request.method.clone(), &proxy_request.url);
    for (header_name, header_value) in &proxy_request.headers {
        request = request.header(header_name.as_str(), header_value.as_str());
    }
    request.body(proxy_request.body)
}

async fn audit_global_ai_request(db: &DB, authed: &ApiAuthed) -> Result<()> {
    let mut tx = db.begin().await?;

    audit_log(
        &mut *tx,
        authed,
        "ai.global_request",
        ActionKind::Execute,
        "global",
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(())
}

fn google_ai_proxy_response_to_body(
    response: GoogleAIProxyResponse,
) -> (http::StatusCode, HeaderMap, axum::body::Body) {
    let body = match response.body {
        GoogleAIProxyResponseBody::Fixed(body) => axum::body::Body::from(body),
        GoogleAIProxyResponseBody::Stream(stream) => axum::body::Body::from_stream(
            inject_keepalives(stream, Duration::from_secs(KEEPALIVE_INTERVAL_SECS)),
        ),
    };

    (response.status_code, response.headers, body)
}

#[cfg(feature = "bedrock")]
fn bedrock_proxy_response_to_body(
    response: BedrockProxyResponse,
) -> (http::StatusCode, HeaderMap, axum::body::Body) {
    let body = match response.body {
        BedrockProxyResponseBody::Fixed(body) => axum::body::Body::from(body),
        BedrockProxyResponseBody::Stream(stream) => axum::body::Body::from_stream(stream),
    };

    (response.status_code, response.headers, body)
}

pub(crate) fn inject_keepalives<S>(
    upstream: S,
    interval: Duration,
) -> impl futures::Stream<Item = std::result::Result<Bytes, reqwest::Error>>
where
    S: futures::Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin,
{
    async_stream::stream! {
        tokio::pin!(upstream);
        loop {
            tokio::select! {
                biased;
                chunk = upstream.next() => {
                    match chunk {
                        Some(item) => yield item,
                        None => break,
                    }
                }
                _ = tokio::time::sleep(interval) => {
                    yield Ok(Bytes::from(": keepalive\n\n"));
                }
            }
        }
    }
}

/// Paths of every `$var:` reference found anywhere in a resource value.
fn var_references(value: &serde_json::Value) -> Vec<&str> {
    match value {
        serde_json::Value::String(s) => s.strip_prefix("$var:").into_iter().collect(),
        serde_json::Value::Array(items) => items.iter().flat_map(var_references).collect(),
        serde_json::Value::Object(fields) => fields.values().flat_map(var_references).collect(),
        _ => vec![],
    }
}

#[derive(Deserialize)]
struct CompleteRequest {
    system: String,
    user: String,
    /// Defaults to the AI config's `default_model`.
    #[serde(default)]
    model: Option<ProviderModel>,
    #[serde(default)]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct CompleteResponse {
    text: String,
    model: ProviderModel,
}

/// One text completion with the workspace's AI config, for callers that are not a browser
/// talking a provider's own API through the proxy (the CLI, jobs). Provider differences are
/// handled by the same query builders AI agent steps use.
async fn complete(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(request): Json<CompleteRequest>,
) -> JsonResult<CompleteResponse> {
    let not_configured =
        || Error::BadRequest("AI is not configured for this workspace".to_string());
    let mut model = match request.model {
        Some(model) => model,
        None => effective_ai_config(&db, &w_id)
            .await?
            .ok_or_else(not_configured)?
            .0
            .default_model
            .ok_or_else(|| {
                Error::BadRequest(
                    "No model given, and the AI settings set no default model".to_string(),
                )
            })?,
    };
    if let Some(base) = model.model.strip_suffix("/thinking") {
        model.model = base.to_string();
    }
    if model.provider.is_decision_provider() {
        return Err(Error::BadRequest(format!(
            "{:?} serves decision models, which answer typed questions rather than messages",
            model.provider
        )));
    }
    let (mut credentials, policy) = configured_credentials(&db, &w_id, &model.provider)
        .await?
        .ok_or_else(not_configured)?;
    policy.check_model(&model.model)?;
    let max_tokens = policy
        .output_tokens(&model.model, request.max_tokens.map(i64::from))
        .map(|n| u32::try_from(n).unwrap_or(u32::MAX));
    #[cfg(feature = "bedrock")]
    resolve_bedrock_oidc_credentials(&db, &w_id, &authed, &mut credentials).await?;
    resolve_anthropic_federated_credentials(&db, &w_id, &authed, &mut credentials).await?;
    resolve_vertex_workload_identity_credentials(&db, &w_id, &authed, &mut credentials).await?;
    // The query builders send the api key; only a federated Anthropic token has a way in.
    if credentials.access_token.is_some() && credentials.anthropic_federation.is_none() {
        return Err(Error::BadRequest(format!(
            "The {:?} resource authenticates with OAuth, which this endpoint does not support. \
             Set an API key on the resource, or carry the provider's credential header in its \
             `headers`.",
            credentials.provider
        )));
    }

    let base_url = credentials.base_url.clone();
    let mut query_builder = create_query_builder(&credentials, &model.model);
    if query_builder.supports_chat_completions_fallback(&base_url)
        && is_chat_completions_only(&base_url, &model.model)
    {
        query_builder = create_chat_completions_query_builder(&credentials);
    }

    let mut tx = db.begin().await?;
    audit_log(
        &mut *tx,
        &authed,
        "ai.request",
        ActionKind::Execute,
        &w_id,
        Some(&authed.email),
        Some([("ai_config_path", "complete")].into()),
    )
    .await?;
    tx.commit().await?;

    let messages =
        [("system", request.system), ("user", request.user)].map(|(role, text)| OpenAIMessage {
            role: role.to_string(),
            content: Some(OpenAIContent::Text(text)),
            ..Default::default()
        });
    // Attachments are the only thing a query builder reaches the API for, and a one-shot
    // request carries none.
    let client = AuthedClient::new(String::new(), w_id.clone(), String::new(), None);
    let (text, _usage) = complete_text(
        &messages,
        &OneShotRequest {
            query_builder: query_builder.as_ref(),
            credentials: &credentials,
            model: &model.model,
            max_tokens,
            reasoning_effort: None,
            timeout: Duration::from_secs(*AI_TIMEOUT_SECS),
            client: &client,
            workspace_id: &w_id,
            federated_token: credentials
                .anthropic_federation
                .as_ref()
                .and(credentials.access_token.as_deref()),
            include_usage: true,
        },
    )
    .await?;
    Ok(Json(CompleteResponse { text, model }))
}

async fn global_proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(ai_path): Path<String>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    validate_proxy_path(&ai_path)?;
    let provider = headers
        .get("X-Provider")
        .map(|v| v.to_str().unwrap_or("").to_string());
    let api_key = headers
        .get("X-API-Key")
        .map(|v| v.to_str().unwrap_or("").to_string());

    let provider = match provider {
        Some(provider) => AIProvider::try_from(provider.as_str())?,
        None => return Err(Error::BadRequest("Provider is required".to_string())),
    };

    let Some(api_key) = api_key else {
        return Err(Error::BadRequest("API key is required".to_string()));
    };

    let proxy_mode = proxy_execution_mode(&provider);

    if matches!(proxy_mode, ProxyExecutionMode::NativeAwsBedrock) {
        return Err(Error::BadRequest(
            "AWS Bedrock global proxy is not supported; use a workspace AI resource with a region"
                .to_string(),
        ));
    }

    let base_url = provider.get_base_url(None, &db).await?;
    let credentials = ProviderCredentials {
        provider: provider.clone(),
        base_url,
        api_key: Some(api_key.clone()),
        access_token: None,
        organization_id: None,
        user: None,
        region: None,
        aws_access_key_id: None,
        aws_secret_access_key: None,
        aws_session_token: None,
        oidc_role_arn: None,
        anthropic_federation: None,
        vertex_workload_identity: None,
        platform: AIPlatform::Standard,
        custom_headers: HashMap::new(),
    };

    let client = pinned_ai_client_for(&credentials.base_url).await?;

    if matches!(proxy_mode, ProxyExecutionMode::NativeGoogleAi) {
        let proxy_args = ProxyBuildArgs {
            method: &method,
            path: &ai_path,
            headers: &headers,
            body: &body,
            credentials: &credentials,
        };

        audit_global_ai_request(&db, &authed).await?;

        let response = match ai_path.as_str() {
            "chat/completions" => handle_google_ai_chat_proxy(&client, &proxy_args).await,
            "models" => handle_google_ai_models_proxy(&client, &proxy_args).await,
            _ => Err(Error::BadRequest(format!(
                "Unsupported Google AI path: {}",
                ai_path
            ))),
        }?;

        return Ok(google_ai_proxy_response_to_body(response));
    }

    let request = match proxy_mode {
        ProxyExecutionMode::HttpForward => {
            // Azure AI Foundry routes Claude deployments through the Anthropic
            // Messages API, so the builder is chosen from the request's model.
            let model = AIProvider::extract_model_from_body(&body).unwrap_or_default();
            let query_builder = create_query_builder(&credentials, &model);
            let proxy_request = query_builder.build_proxy_request(&ProxyBuildArgs {
                method: &method,
                path: &ai_path,
                headers: &headers,
                body: &body,
                credentials: &credentials,
            })?;
            proxy_request_to_request_builder(&client, proxy_request)
        }
        ProxyExecutionMode::NativeGoogleAi | ProxyExecutionMode::NativeAwsBedrock => {
            return Err(Error::BadRequest(format!(
                "Unsupported global proxy mode for provider {:?}",
                provider
            )))
        }
    };

    let response = send_with_retries(request).await;

    audit_global_ai_request(&db, &authed).await?;

    let response = response?;
    let status_code = response.status();
    let headers = response.headers().clone();
    let stream = response.bytes_stream();
    let body = if is_sse_response(&headers) {
        axum::body::Body::from_stream(inject_keepalives(
            stream,
            Duration::from_secs(KEEPALIVE_INTERVAL_SECS),
        ))
    } else {
        axum::body::Body::from_stream(stream)
    };
    Ok((status_code, headers, body))
}

async fn proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, mut ai_path)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    mut body: Bytes,
) -> impl IntoResponse {
    validate_proxy_path(&ai_path)?;
    let provider = headers
        .get("X-Provider")
        .map(|v| v.to_str().unwrap_or("").to_string());

    let provider = match provider {
        Some(provider) => AIProvider::try_from(provider.as_str())?,
        None => return Err(Error::BadRequest("Provider is required".to_string())),
    };

    let forced_resource_path = headers
        .get("X-Resource-Path")
        .map(|v| v.to_str().unwrap_or("").to_string());

    // When the caller supplies X-Resource-Path, the resource is treated as if it
    // were being read through the normal resource API: scope and RLS checks must
    // apply so that a low-privilege user cannot point the proxy at a restricted
    // AI resource (e.g. one in a folder they cannot read) to exfiltrate the
    // resource's provider credentials or use them via the proxy.
    if let Some(resource_path) = forced_resource_path.as_ref() {
        check_scopes(&authed, || format!("resources:read:{}", resource_path))?;
    }

    // Set when serving the request through Windmill's free AI tier (the lent key). Holds
    // the per-user concurrency lock and drives response metering.
    let mut free_lease: Option<crate::ai_free_tier_oss::FreeTierLease> = None;
    // Set when serving a request that names a model with the effective AI config's own key.
    let mut policy: Option<(ModelPolicy, String)> = None;
    // An unsaved resource value (percent-encoded JSON), so a resource can be tested
    // before it is stored. `$var:` references resolve as the caller, and since the
    // caller also picks the base URL they are sent to, a scoped token needs the same
    // scopes as storing the resource and reading each variable would.
    let inline_resource = headers
        .get("X-Resource-Value")
        .map(|v| {
            let invalid = |e: String| Error::BadRequest(format!("Invalid X-Resource-Value: {e}"));
            let decoded = urlencoding::decode(v.to_str().unwrap_or(""))
                .map_err(|e| invalid(e.to_string()))?;
            let value = serde_json::from_str::<serde_json::Value>(&decoded)
                .map_err(|e| invalid(e.to_string()))?;
            check_scopes(&authed, || "resources:write".to_string())?;
            for path in var_references(&value) {
                check_scopes(&authed, || format!("variables:read:{path}"))?;
            }
            serde_json::from_value::<AIResource>(value).map_err(|e| invalid(e.to_string()))
        })
        .transpose()?;

    let mut credentials = 'cred: {
        if let Some(resource) = inline_resource {
            break 'cred resolve_provider_credentials(
                &provider,
                &db,
                &w_id,
                resource,
                Some(&authed),
            )
            .await?;
        }
        if let Some(resource_path) = forced_resource_path {
            // For user-specified resources, fetch through an RLS-scoped connection so
            // PostgreSQL row-level security enforces the same folder/group boundaries as
            // the regular resource API.
            let mut tx = user_db.clone().begin(&authed).await?;
            let res = sqlx::query_scalar::<_, Option<sqlx::types::Json<Box<RawValue>>>>(
                "SELECT value FROM resource WHERE path = $1 AND workspace_id = $2",
            )
            .bind(&resource_path)
            .bind(&w_id)
            .fetch_optional(&mut *tx)
            .await?;
            tx.commit().await?;
            let resource = parse_ai_resource(res, &resource_path)?;
            // RLS also applies to the resource's $var: references, so users can only
            // read variables they have permission to access.
            break 'cred resolve_provider_credentials(
                &provider,
                &db,
                &w_id,
                resource,
                Some(&authed),
            )
            .await?;
        }
        if let Some((credentials, configured)) =
            configured_credentials(&db, &w_id, &provider).await?
        {
            policy = configured
                .check_body(&method, &body)?
                .map(|model| (configured, model));
            break 'cred credentials;
        }
        // Nothing configured: fall back to Windmill's free AI tier (EE-only) if a lent
        // key is set and both the user's one-time grant and the instance's daily cap have
        // room. Errors once the grant is spent, the day is capped, or the user already has
        // a request in flight; None otherwise. Ineligible identities (e.g. service
        // accounts) are refused inside the helper, so every path treats them alike.
        let free = crate::ai_free_tier_oss::resolve_free_tier_credentials(
            &provider,
            &db,
            &ai_path,
            &authed.email,
            &body,
        )
        .await?;
        if let Some((free_credentials, lease)) = free {
            free_lease = Some(lease);
            break 'cred free_credentials;
        }
        return Err(Error::internal_err(
            "AI resource not configured".to_string(),
        ));
    };

    #[cfg(feature = "bedrock")]
    resolve_bedrock_oidc_credentials(&db, &w_id, &authed, &mut credentials).await?;
    resolve_anthropic_federated_credentials(&db, &w_id, &authed, &mut credentials).await?;
    resolve_vertex_workload_identity_credentials(&db, &w_id, &authed, &mut credentials).await?;

    // Free tier: pin the model and clamp max_tokens server-side before forwarding,
    // since the request body is otherwise client-controlled.
    if free_lease.is_some() {
        body = crate::ai_free_tier_oss::enforce_free_tier_body(&body)?;
    }
    if let Some((policy, model)) = &policy {
        body = policy.limit_body(model, &provider, &credentials.platform, &ai_path, &body)?;
    }

    if let Some(fim_transform) =
        maybe_transform_fim_request(&provider, &ai_path, &credentials.base_url, &body)?
    {
        if fim_transform.base_url.is_some() {
            tracing::debug!(
                "Routing native FIM request through provider-specific endpoint for {:?}",
                provider
            );
        } else {
            tracing::debug!(
                "Transforming FIM request to chat/completions with FIM tokens for provider {:?}",
                provider
            );
        }
        if let Some(base_url) = fim_transform.base_url {
            credentials.base_url = base_url;
        }
        body = fim_transform.body;
        ai_path = fim_transform.path;
    }

    let proxy_mode = proxy_execution_mode(&provider);

    // Handle GoogleAI (Gemini) using the native Gemini API
    if matches!(proxy_mode, ProxyExecutionMode::NativeGoogleAi) {
        let mut tx = db.begin().await?;
        audit_log(
            &mut *tx,
            &authed,
            "ai.request",
            ActionKind::Execute,
            &w_id,
            Some(&authed.email),
            Some([("ai_config_path", &format!("{:?}", ai_path)[..])].into()),
        )
        .await?;
        tx.commit().await?;

        let proxy_args = ProxyBuildArgs {
            method: &method,
            path: &ai_path,
            headers: &headers,
            body: &body,
            credentials: &credentials,
        };

        let client = pinned_ai_client_for(&credentials.base_url).await?;

        let response = match ai_path.as_str() {
            "chat/completions" => handle_google_ai_chat_proxy(&client, &proxy_args).await,
            "models" => handle_google_ai_models_proxy(&client, &proxy_args).await,
            _ => Err(Error::BadRequest(format!(
                "Unsupported Google AI path: {}",
                ai_path
            ))),
        }?;

        return Ok(google_ai_proxy_response_to_body(response));
    }

    // Handle Bedrock-specific logic when the feature is enabled
    #[cfg(feature = "bedrock")]
    if matches!(proxy_mode, ProxyExecutionMode::NativeAwsBedrock) {
        let mut tx = db.begin().await?;
        audit_log(
            &mut *tx,
            &authed,
            "ai.request",
            ActionKind::Execute,
            &w_id,
            Some(&authed.email),
            Some([("ai_config_path", &format!("{:?}", ai_path)[..])].into()),
        )
        .await?;
        tx.commit().await?;

        let response = handle_bedrock_proxy(&ProxyBuildArgs {
            method: &method,
            path: &ai_path,
            headers: &headers,
            body: &body,
            credentials: &credentials,
        })
        .await?;

        return Ok(bedrock_proxy_response_to_body(response));
    }

    // When bedrock feature is disabled, return error for Bedrock provider
    #[cfg(not(feature = "bedrock"))]
    if matches!(proxy_mode, ProxyExecutionMode::NativeAwsBedrock) {
        return Err(Error::BadRequest(
            "AWS Bedrock support is not enabled. Build with 'bedrock' feature.".to_string(),
        ));
    }

    let request = match proxy_mode {
        ProxyExecutionMode::HttpForward => {
            // Azure AI Foundry routes Claude deployments through the Anthropic
            // Messages API, so the builder is chosen from the request's model.
            let model = AIProvider::extract_model_from_body(&body).unwrap_or_default();
            let query_builder = create_query_builder(&credentials, &model);
            let proxy_request = query_builder.build_proxy_request(&ProxyBuildArgs {
                method: &method,
                path: &ai_path,
                headers: &headers,
                body: &body,
                credentials: &credentials,
            })?;
            let client = pinned_ai_client_for(&credentials.base_url).await?;
            proxy_request_to_request_builder(&client, proxy_request)
        }
        ProxyExecutionMode::NativeGoogleAi => {
            return Err(Error::internal_err(
                "Google AI proxy route was not handled".to_string(),
            ))
        }
        ProxyExecutionMode::NativeAwsBedrock => {
            return Err(Error::BadRequest(
                "Unsupported AWS Bedrock proxy request".to_string(),
            ))
        }
    };

    let response = send_with_retries(request).await;

    let mut tx = db.begin().await?;

    audit_log(
        &mut *tx,
        &authed,
        "ai.request",
        ActionKind::Execute,
        &w_id,
        Some(&authed.email),
        Some([("ai_config_path", &format!("{:?}", ai_path)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    let response = response?;
    let status_code = response.status();
    let headers = response.headers().clone();
    let is_sse = is_sse_response(&headers);

    // Free tier: reconcile the cost reserved up-front against what the response actually
    // used, holding the per-user lock (via the lease) until it is recorded. The chat
    // streams (SSE), where the usage report only arrives in the final chunk; the
    // non-streaming JSON path is handled for completeness.
    if let Some(lease) = free_lease {
        let body = if is_sse {
            axum::body::Body::from_stream(inject_keepalives(
                Box::pin(crate::ai_free_tier_oss::meter_usage(
                    response.bytes_stream(),
                    db.clone(),
                    lease,
                )),
                Duration::from_secs(KEEPALIVE_INTERVAL_SECS),
            ))
        } else {
            let bytes = response.bytes().await.map_err(to_anyhow)?;
            crate::ai_free_tier_oss::record_json_usage(db.clone(), lease, &bytes);
            axum::body::Body::from(bytes)
        };
        return Ok((status_code, headers, body));
    }

    let stream = response.bytes_stream();
    let body = if is_sse {
        axum::body::Body::from_stream(inject_keepalives(
            stream,
            Duration::from_secs(KEEPALIVE_INTERVAL_SECS),
        ))
    } else {
        axum::body::Body::from_stream(stream)
    };
    Ok((status_code, headers, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};
    use windmill_ai::ai_cache::bump_instance_ai_config_revision;
    use windmill_ai::ai_providers::AIPlatform;

    static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn sample_provider_credentials() -> ProviderCredentials {
        ProviderCredentials {
            provider: AIProvider::OpenAI,
            base_url: "https://example.com".to_string(),
            api_key: None,
            access_token: None,
            organization_id: None,
            user: None,
            region: None,
            aws_access_key_id: None,
            aws_secret_access_key: None,
            aws_session_token: None,
            oidc_role_arn: None,
            anthropic_federation: None,
            vertex_workload_identity: None,
            platform: AIPlatform::Standard,
            custom_headers: HashMap::new(),
        }
    }

    #[test]
    fn only_a_federation_rule_makes_a_resource_anthropic_federated() {
        let federated = serde_json::json!({
            "federation_rule_id": "fdrl_x",
            "organization_id": "00000000-0000-0000-0000-000000000000",
            "service_account_id": "svac_x",
            "workspace_id": ""
        });
        assert!(matches!(
            serde_json::from_value::<AIResource>(federated),
            Ok(AIResource::AnthropicFederated(
                AIAnthropicFederatedResource { workspace_id: None, .. }
            ))
        ));

        let incomplete = serde_json::json!({"federation_rule_id": "fdrl_x"});
        assert!(matches!(
            serde_json::from_value::<AIResource>(incomplete),
            Ok(AIResource::AnthropicFederated(_))
        ));

        // `organization_id` is also the OpenAI organization of a standard resource.
        let standard = serde_json::json!({"api_key": "k", "organization_id": "org-x"});
        assert!(matches!(
            serde_json::from_value::<AIResource>(standard),
            Ok(AIResource::Standard(_))
        ));
    }

    #[test]
    fn ai_standard_resource_ignores_legacy_enable_1m_context_keys() {
        // Resources created before the field was removed still carry the legacy key
        // (lowercase `enable_1m_context` or the frontend alias `enable_1M_context`).
        // The struct has no `deny_unknown_fields`, so both must be silently ignored.
        let json = r#"{
            "base_url": "https://api.anthropic.com",
            "enable_1m_context": true,
            "enable_1M_context": true
        }"#;

        let resource: AIStandardResource =
            serde_json::from_str(json).expect("legacy resource must still deserialize");

        assert_eq!(
            resource.base_url.as_deref(),
            Some("https://api.anthropic.com")
        );
    }

    #[test]
    fn invalidates_all_cached_providers_for_workspace() {
        let _guard = TEST_LOCK.lock().unwrap();
        AI_REQUEST_CACHE.clear();
        AI_REQUEST_CACHE.insert(
            ("workspace-a".to_string(), AIProvider::OpenAI),
            ExpiringProviderCredentials::new(
                sample_provider_credentials(),
                ModelPolicy::default(),
                None,
            ),
        );
        AI_REQUEST_CACHE.insert(
            ("workspace-a".to_string(), AIProvider::Anthropic),
            ExpiringProviderCredentials::new(
                sample_provider_credentials(),
                ModelPolicy::default(),
                None,
            ),
        );
        AI_REQUEST_CACHE.insert(
            ("workspace-b".to_string(), AIProvider::OpenAI),
            ExpiringProviderCredentials::new(
                sample_provider_credentials(),
                ModelPolicy::default(),
                None,
            ),
        );
        #[cfg(feature = "bedrock")]
        let assumed_role_key = {
            let key = (
                "workspace-a".to_string(),
                "arn:aws:iam::123456789012:role/bedrock".to_string(),
                "someone@windmill.dev".to_string(),
            );
            BEDROCK_ASSUMED_ROLE_CACHE.insert(
                key.clone(),
                windmill_ai::ai_bedrock::AssumedRoleCredentials {
                    access_key_id: "AKIA".to_string(),
                    secret_access_key: "secret".to_string(),
                    session_token: "session".to_string(),
                    expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
                },
            );
            key
        };

        invalidate_ai_request_cache_for_workspace("workspace-a");

        assert!(AI_REQUEST_CACHE
            .get(&("workspace-a".to_string(), AIProvider::OpenAI))
            .is_none());
        assert!(AI_REQUEST_CACHE
            .get(&("workspace-a".to_string(), AIProvider::Anthropic))
            .is_none());
        assert!(AI_REQUEST_CACHE
            .get(&("workspace-b".to_string(), AIProvider::OpenAI))
            .is_some());
        #[cfg(feature = "bedrock")]
        assert!(BEDROCK_ASSUMED_ROLE_CACHE.get(&assumed_role_key).is_none());
    }

    #[test]
    fn configured_key_serves_only_enabled_models() {
        let config: AIConfig = serde_json::from_value(serde_json::json!({
            "providers": {
                "openai": {"resource_path": "r", "models": ["gpt-4o"]},
                "anthropic": {"resource_path": "r", "models": ["claude-x"]}
            },
            "default_model": {"provider": "openai", "model": "picked/thinking"}
        }))
        .unwrap();
        let policy = ModelPolicy::new(&config, &AIProvider::OpenAI);
        let check = |body: serde_json::Value| {
            policy.check_body(&Method::POST, &Bytes::from(body.to_string()))
        };

        let refused = check(serde_json::json!({"model": "claude-x"})).unwrap_err();
        assert!(refused.to_string().contains("\"claude-x\" is not enabled"));
        assert!(check(serde_json::json!({"model": "picked"})).is_ok());
        assert!(policy.check_body(&Method::GET, &Bytes::new()).is_ok());
        assert!(policy.check_body(&Method::POST, &Bytes::new()).is_err());
        assert!(check(serde_json::json!({"model": "gpt-4o", "models": ["other"]})).is_err());
        assert!(check(serde_json::json!({"model": "gpt-4o", "fallbacks": ["other"]})).is_err());
        assert!(check(serde_json::json!({"messages": []})).is_err());
        assert!(check(serde_json::json!(["gpt-4o", null, null])).is_err());
        assert!(policy
            .check_body(&Method::POST, &Bytes::from_static(b"--boundary"))
            .is_err());
    }

    #[test]
    fn configured_key_holds_requests_to_token_limits() {
        let config: AIConfig = serde_json::from_value(serde_json::json!({
            "providers": {
                "openai": {"resource_path": "r", "models": ["gpt-4o", "gpt-5", "unlimited"]},
                "googleai": {"resource_path": "r", "models": ["gemini-x"]},
                "anthropic": {"resource_path": "r", "models": ["claude-x"]},
                "aws_bedrock": {"resource_path": "r", "models": ["bedrock-x"]},
                "azure_openai": {"resource_path": "r", "models": ["production"]},
                "azure_foundry": {"resource_path": "r", "models": ["mistral-x"]},
                "customai": {"resource_path": "r", "models": ["llama"]},
                "deepseek": {"resource_path": "r", "models": ["deepseek-chat"]}
            },
            "max_tokens_per_model": {
                "openai:gpt-4o": 4000, "openai:gpt-5": 4000,
                "googleai:gemini-x": 4000, "anthropic:claude-x": 4000,
                "aws_bedrock:bedrock-x": 4000, "azure_openai:production": 4000,
                "azure_foundry:mistral-x": 4000, "customai:llama": 4000,
                "deepseek:deepseek-chat": 4000
            }
        }))
        .unwrap();
        let run_on =
            |provider: AIProvider, platform: AIPlatform, path: &str, body: serde_json::Value| {
                let policy = ModelPolicy::new(&config, &provider);
                let body = Bytes::from(body.to_string());
                let model = policy.check_body(&Method::POST, &body)?.unwrap();
                policy.limit_body(&model, &provider, &platform, path, &body)
            };
        let run = |path: &str, body: serde_json::Value| {
            run_on(AIProvider::OpenAI, AIPlatform::Standard, path, body)
        };

        // Only the field the upstream reads reaches it, at the lowest valid value asked for, capped.
        let limits = |body: Bytes| {
            serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(&body)
                .unwrap()
                .into_iter()
                .filter(|(k, _)| k.starts_with("max_"))
                .collect::<serde_json::Map<_, _>>()
        };
        use serde_json::json;
        use AIPlatform::{GoogleVertexAi as Vertex, Standard};
        for (provider, platform, path, model, asked, sent) in [
            (
                AIProvider::GoogleAI,
                Standard,
                "chat/completions",
                "gemini-x",
                json!({"max_completion_tokens": 10}),
                json!({"max_tokens": 10}),
            ),
            (
                AIProvider::Anthropic,
                Standard,
                "messages",
                "claude-x",
                json!({"max_completion_tokens": 100000}),
                json!({"max_tokens": 4000}),
            ),
            (
                AIProvider::OpenAI,
                Standard,
                "fim/completions",
                "gpt-4o",
                json!({"max_completion_tokens": 100000}),
                json!({"max_tokens": 4000}),
            ),
            (
                AIProvider::OpenAI,
                Standard,
                "fim/completions/x",
                "gpt-4o",
                json!({}),
                json!({"max_tokens": 4000}),
            ),
            (
                AIProvider::CustomAI,
                Standard,
                "chat/completions",
                "llama",
                json!({"max_tokens": 100000, "max_completion_tokens": 9000}),
                json!({"max_tokens": 4000}),
            ),
            (
                AIProvider::CustomAI,
                Standard,
                "chat/completions",
                "llama",
                json!({}),
                json!({"max_completion_tokens": 4000}),
            ),
            (
                AIProvider::AzureFoundry,
                Standard,
                "chat/completions",
                "mistral-x",
                json!({"max_completion_tokens": 100000}),
                json!({"max_completion_tokens": 4000}),
            ),
            (
                AIProvider::DeepSeek,
                Standard,
                "chat/completions",
                "deepseek-chat",
                json!({"max_completion_tokens": 100000}),
                json!({"max_tokens": 4000}),
            ),
            (
                AIProvider::OpenAI,
                Standard,
                "chat/completions",
                "gpt-4o",
                json!({"max_tokens": 100000, "max_completion_tokens": 10}),
                json!({"max_tokens": 10}),
            ),
            (
                AIProvider::OpenAI,
                Standard,
                "chat/completions",
                "gpt-4o",
                json!({"max_tokens": -1, "max_output_tokens": 0}),
                json!({"max_tokens": 4000}),
            ),
            (
                AIProvider::AzureOpenAI,
                Standard,
                "chat/completions",
                "production",
                json!({}),
                json!({"max_completion_tokens": 4000}),
            ),
            (
                AIProvider::OpenAI,
                Standard,
                "responses",
                "gpt-5",
                json!({"max_tokens": 100000}),
                json!({"max_output_tokens": 4000}),
            ),
            (
                AIProvider::OpenAI,
                Standard,
                "completions",
                "gpt-4o",
                json!({"max_tokens": 500}),
                json!({"max_tokens": 500}),
            ),
            // Bedrock and Anthropic on Vertex generate for any POST, whatever its path.
            (
                AIProvider::AWSBedrock,
                Standard,
                "responses",
                "bedrock-x",
                json!({"max_output_tokens": 100000}),
                json!({"max_tokens": 4000}),
            ),
            (
                AIProvider::AWSBedrock,
                Standard,
                "x",
                "bedrock-x",
                json!({}),
                json!({"max_tokens": 4000}),
            ),
            (
                AIProvider::Anthropic,
                Vertex,
                "messages/count_tokens",
                "claude-x",
                json!({}),
                json!({"max_tokens": 4000}),
            ),
            (
                AIProvider::Anthropic,
                Standard,
                "messages/count_tokens",
                "claude-x",
                json!({}),
                json!({}),
            ),
            (
                AIProvider::OpenAI,
                Standard,
                "embeddings",
                "gpt-4o",
                json!({}),
                json!({}),
            ),
        ] {
            let mut body = asked.clone();
            body["model"] = model.into();
            let out = run_on(provider.clone(), platform, path, body).unwrap();
            assert_eq!(
                serde_json::Value::Object(limits(out)),
                sent,
                "{provider:?} {path} {asked}"
            );
        }

        // A model with no limit is forwarded byte for byte.
        let raw = Bytes::from_static(br#"{ "model": "unlimited", "max_tokens": 9 }"#);
        let policy = ModelPolicy::new(&config, &AIProvider::OpenAI);
        assert_eq!(
            policy
                .limit_body(
                    "unlimited",
                    &AIProvider::OpenAI,
                    &Standard,
                    "chat/completions",
                    &raw
                )
                .unwrap(),
            raw
        );
        assert_eq!(policy.output_tokens("gpt-4o", Some(100000)), Some(4000));
        assert_eq!(policy.output_tokens("gpt-4o", None), Some(4000));
        assert_eq!(policy.output_tokens("unlimited", Some(9)), Some(9));
        assert_eq!(policy.output_tokens("unlimited", None), None);
        assert_eq!(policy.output_tokens("unlimited", Some(0)), Some(0));
        for no_limit in [0, -1] {
            assert_eq!(policy.output_tokens("gpt-4o", Some(no_limit)), Some(4000));
        }

        // Where the limit can't be placed, the request is refused rather than sent without it.
        assert!(run_on(
            AIProvider::Anthropic,
            Standard,
            "conversations",
            json!({"model": "claude-x"})
        )
        .is_err());
        for many in [
            json!({"model": "gpt-4o", "n": 2}),
            json!({"model": "gpt-4o", "best_of": 2}),
            json!({"model": "gpt-4o", "prompt": ["a", "b"]}),
            json!({"model": "gpt-4o", "prompt": [[1, 2], [3]]}),
        ] {
            assert!(run("completions", many).is_err());
        }
        for one in [
            json!({"model": "gpt-4o", "n": 1}),
            json!({"model": "gpt-4o", "prompt": "a"}),
            json!({"model": "gpt-4o", "prompt": ["a"]}),
            json!({"model": "gpt-4o", "prompt": [1, 2, 3]}),
        ] {
            assert!(run("completions", one).is_ok());
        }
    }

    #[test]
    fn instance_backed_cache_entries_expire_when_revision_changes() {
        let _guard = TEST_LOCK.lock().unwrap();
        AI_REQUEST_CACHE.clear();

        let cached = ExpiringProviderCredentials::new(
            sample_provider_credentials(),
            ModelPolicy::default(),
            Some(current_instance_ai_config_revision()),
        );
        assert!(!cached.is_expired());

        bump_instance_ai_config_revision();

        assert!(cached.is_expired());
    }
}
