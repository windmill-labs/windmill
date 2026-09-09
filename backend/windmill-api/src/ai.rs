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
use windmill_ai::ai_types::MAX_MODEL_RATE;
use windmill_ai::credentials::ProviderCredentials;
#[cfg(feature = "bedrock")]
use windmill_ai::providers::bedrock::{
    handle_bedrock_proxy, BedrockProxyResponse, BedrockProxyResponseBody,
};
use windmill_ai::providers::{
    create_query_builder,
    google_ai::{
        handle_google_ai_chat_proxy, handle_google_ai_models_proxy, GoogleAIProxyResponse,
        GoogleAIProxyResponseBody,
    },
};
use windmill_ai::proxy::{
    fim::maybe_transform_fim_request, proxy_execution_mode, ProxyBuildArgs, ProxyExecutionMode,
    ProxyRequest,
};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::db::UserDB;
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

}

pub(crate) fn invalidate_ai_request_cache_for_workspace(workspace_id: &str) {
    AI_REQUEST_CACHE.retain(|(cached_workspace_id, _), _| cached_workspace_id != workspace_id);
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
/// pin — an IP-literal host, or a deployment that opted into private AI
/// endpoints via `ALLOW_PRIVATE_AI_BASE_URLS`. The same opt-out and error hint
/// as `get_base_url` apply, so the guard here is consistent with save-time
/// validation while additionally closing the connect-time window.
async fn pinned_ai_client_for(url: &str) -> Result<std::borrow::Cow<'static, Client>> {
    use std::borrow::Cow;
    use windmill_common::ssrf::SsrfValidationError;

    if *windmill_ai::ai_providers::ALLOW_PRIVATE_AI_BASE_URLS {
        return Ok(Cow::Borrowed(&HTTP_CLIENT));
    }

    let target = windmill_common::ssrf::validate_url_for_ssrf(url)
        .await
        .map_err(|e| match e {
            e @ SsrfValidationError::Private { .. } => Error::BadRequest(format!(
                "{e}. If you need to use private/internal AI endpoints, \
                 set the ALLOW_PRIVATE_AI_BASE_URLS=true environment variable"
            )),
            e => Error::from(e),
        })?;

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
    /// Platform (standard or google_vertex_ai)
    #[serde(default)]
    platform: AIPlatform,
    /// Custom HTTP headers to include in AI requests
    #[serde(default)]
    headers: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct OAuthTokens {
    access_token: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum AIResource {
    OAuth(AIOAuthResource),
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
        AIResource::Standard(resource) => {
            // Skip get_base_url for Bedrock - it uses SDK directly, not HTTP
            let base_url = if matches!(provider, AIProvider::AWSBedrock) {
                String::new()
            } else {
                provider.get_base_url(resource.base_url, db).await?
            };
            let api_key = if let Some(api_key) = resource.api_key {
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
                platform: resource.platform,
                custom_headers: resource.headers,
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

#[derive(Clone, Debug)]
pub struct ExpiringProviderCredentials {
    credentials: ProviderCredentials,
    expires_at: std::time::Instant,
    instance_ai_config_revision: Option<u64>,
}

impl ExpiringProviderCredentials {
    fn new(credentials: ProviderCredentials, instance_ai_config_revision: Option<u64>) -> Self {
        Self {
            credentials,
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
    /// Hides the Windmill AI assistant (chat, sessions, generation, completion, fixes) from
    /// the workspace UI. Only the workspace's own row is consulted: the flag holds even when
    /// the providers served come from the instance config or the free tier. AI agent steps
    /// and the AI sandbox are unaffected, so the providers stay in force.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub copilot_disabled: bool,
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

impl AIConfig {
    pub fn validate_model_pricing(&self) -> Result<()> {
        for (key, price) in self.model_pricing.iter().flatten() {
            price.validate(key)?;
        }
        Ok(())
    }

    pub fn has_providers(&self) -> bool {
        self.providers
            .as_ref()
            .is_some_and(|providers| !providers.is_empty())
    }
}

pub fn global_service() -> Router {
    Router::new().route("/proxy/{*ai}", post(global_proxy).get(global_proxy))
}

pub fn workspaced_service() -> Router {
    let router = Router::new()
        .route("/proxy/{*ai}", post(proxy).get(proxy))
        .route(
            "/usage",
            post(record_ai_usage)
                .get(list_ai_usage)
                // The handler caps how many events it *stores*, but Json deserializes
                // the whole array first — without a body limit an authenticated member
                // could make the server allocate and parse an arbitrarily large one.
                // Sized well above a full batch of the shape below.
                .layer(DefaultBodyLimit::max(AI_USAGE_BODY_LIMIT)),
        );

    #[cfg(feature = "bedrock")]
    let router = router.route("/check_bedrock_credentials", get(check_bedrock_credentials));

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

async fn global_proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(ai_path): Path<String>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
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

    let response = request.send().await.map_err(to_anyhow)?;

    audit_global_ai_request(&db, &authed).await?;

    if response.error_for_status_ref().is_err() {
        let err_msg = response.text().await.unwrap_or("".to_string());
        return Err(Error::AIError(err_msg));
    }

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
    let provider = headers
        .get("X-Provider")
        .map(|v| v.to_str().unwrap_or("").to_string());

    let provider = match provider {
        Some(provider) => AIProvider::try_from(provider.as_str())?,
        None => return Err(Error::BadRequest("Provider is required".to_string())),
    };

    let workspace_cache = AI_REQUEST_CACHE.get(&(w_id.clone(), provider.clone()));

    let forced_resource_path = headers
        .get("X-Resource-Path")
        .map(|v| v.to_str().unwrap_or("").to_string());
    let is_user_specified_resource = forced_resource_path.is_some();

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
    let mut credentials = 'cred: {
        match workspace_cache {
            Some(request_cache)
                if !request_cache.is_expired() && forced_resource_path.is_none() =>
            {
                request_cache.credentials
            }
            _ => {
                let (resource_path, save_to_cache, resource_workspace, instance_ai_config_revision) =
                    if let Some(resource_path) = forced_resource_path {
                        // forced resource path
                        (resource_path, false, w_id.clone(), None)
                    } else {
                        let workspace_ai_config = sqlx::query_scalar!(
                            "SELECT ai_config FROM workspace_settings WHERE workspace_id = $1",
                            &w_id
                        )
                        .fetch_one(&db)
                        .await?;

                        let (ai_config_value, resource_workspace, instance_ai_config_revision) = {
                            let ws_has_config = workspace_ai_config
                                .as_ref()
                                .and_then(|v| serde_json::from_value::<AIConfig>(v.clone()).ok())
                                .is_some_and(|config| config.has_providers());

                            if ws_has_config {
                                (workspace_ai_config.unwrap(), w_id.clone(), None)
                            } else {
                                let instance_config = sqlx::query_scalar!(
                                    "SELECT value FROM global_settings WHERE name = 'ai_config'"
                                )
                                .fetch_optional(&db)
                                .await?;

                                let instance_has_config =
                                    instance_config.as_ref().is_some_and(|v| {
                                        serde_json::from_value::<AIConfig>(v.clone())
                                            .ok()
                                            .is_some_and(|c| c.has_providers())
                                    });
                                match instance_config {
                                    // An instance `ai_config` row with no usable provider (e.g. `{}`
                                    // or `{"providers":{}}`) is treated as unconfigured, exactly as
                                    // build_copilot_settings_state does — otherwise its mere presence
                                    // would suppress the free-tier fallback below.
                                    Some(config) if instance_has_config => (
                                        config,
                                        "admins".to_string(),
                                        Some(current_instance_ai_config_revision()),
                                    ),
                                    _ => {
                                        // Nothing configured: fall back to Windmill's free AI tier
                                        // (EE-only) if a lent key is set and both the user's
                                        // one-time grant and the instance's daily cap have room.
                                        // Errors once the grant is spent, the day is capped, or the
                                        // user already has a request in flight; None otherwise.
                                        // Ineligible identities (e.g. service accounts) are refused
                                        // inside the helper, so every path treats them alike.
                                        let free =
                                            crate::ai_free_tier_oss::resolve_free_tier_credentials(
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
                                    }
                                }
                            }
                        };

                        let mut ai_config = serde_json::from_value::<AIConfig>(ai_config_value)
                            .map_err(|e| Error::BadRequest(e.to_string()))?;

                        let provider_config = ai_config
                            .providers
                            .as_mut()
                            .and_then(|providers| providers.remove(&provider))
                            .ok_or_else(|| {
                                Error::BadRequest(format!("Provider {:?} not configured", provider))
                            })?;

                        if provider_config.resource_path.is_empty() {
                            return Err(Error::BadRequest("Resource path is empty".to_string()));
                        }

                        (
                            provider_config.resource_path,
                            true,
                            resource_workspace,
                            instance_ai_config_revision,
                        )
                    };

                // For user-specified resources, fetch through an RLS-scoped
                // connection so PostgreSQL row-level security enforces the same
                // folder/group boundaries as the regular resource API. For the
                // workspace/instance ai_config path, the resource_path was already
                // validated by an admin/devops user when configuring the workspace,
                // so the raw pool is used.
                let resource = if is_user_specified_resource {
                let mut tx = user_db.clone().begin(&authed).await?;
                let res = sqlx::query_scalar::<_, Option<sqlx::types::Json<Box<RawValue>>>>(
                    "SELECT value FROM resource WHERE path = $1 AND workspace_id = $2",
                )
                .bind(&resource_path)
                .bind(&resource_workspace)
                .fetch_optional(&mut *tx)
                .await?;
                tx.commit().await?;
                res
            } else {
                sqlx::query_scalar::<_, Option<sqlx::types::Json<Box<RawValue>>>>(
                    "SELECT value FROM resource WHERE path = $1 AND workspace_id = $2",
                )
                .bind(&resource_path)
                .bind(&resource_workspace)
                .fetch_optional(&db)
                .await?
            }
            .ok_or_else(|| Error::NotFound(format!("Could not find the resource {}, update the resource path in the workspace settings", resource_path)))?
            .ok_or_else(|| Error::BadRequest(format!("Empty resource value for {}", resource_path)))?;

                let resource = serde_json::from_str::<AIResource>(resource.0.get())
                    .map_err(|e| Error::BadRequest(e.to_string()))?;

                // Enforce RLS on $var: resolution when the resource path was
                // user-specified (X-Resource-Path header) so users can only read
                // variables they have permission to access.
                let enforce_authed = if is_user_specified_resource {
                    Some(&authed)
                } else {
                    None
                };
                let credentials = resolve_provider_credentials(
                    &provider,
                    &db,
                    &resource_workspace,
                    resource,
                    enforce_authed,
                )
                .await?;
                if save_to_cache {
                    AI_REQUEST_CACHE.insert(
                        (w_id.clone(), provider.clone()),
                        ExpiringProviderCredentials::new(
                            credentials.clone(),
                            instance_ai_config_revision,
                        ),
                    );
                }
                credentials
            }
        }
    };

    // Free tier: pin the model and clamp max_tokens server-side before forwarding,
    // since the request body is otherwise client-controlled.
    if free_lease.is_some() {
        body = crate::ai_free_tier_oss::enforce_free_tier_body(&body)?;
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

    let response = request.send().await.map_err(to_anyhow)?;

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

    if response.error_for_status_ref().is_err() {
        let err_msg = response.text().await.unwrap_or("".to_string());
        return Err(Error::AIError(err_msg));
    }

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
            platform: AIPlatform::Standard,
            custom_headers: HashMap::new(),
        }
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
            ExpiringProviderCredentials::new(sample_provider_credentials(), None),
        );
        AI_REQUEST_CACHE.insert(
            ("workspace-a".to_string(), AIProvider::Anthropic),
            ExpiringProviderCredentials::new(sample_provider_credentials(), None),
        );
        AI_REQUEST_CACHE.insert(
            ("workspace-b".to_string(), AIProvider::OpenAI),
            ExpiringProviderCredentials::new(sample_provider_credentials(), None),
        );

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
    }

    #[test]
    fn instance_backed_cache_entries_expire_when_revision_changes() {
        let _guard = TEST_LOCK.lock().unwrap();
        AI_REQUEST_CACHE.clear();

        let cached = ExpiringProviderCredentials::new(
            sample_provider_credentials(),
            Some(current_instance_ai_config_revision()),
        );
        assert!(!cached.is_expired());

        bump_instance_ai_config_revision();

        assert!(cached.is_expired());
    }
}
