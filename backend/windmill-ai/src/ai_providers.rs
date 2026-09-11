/*
 * This file contains shared AI provider utilities used by both the API and worker.
 */

use serde::{Deserialize, Deserializer, Serialize};
use windmill_common::db::DB;
use windmill_common::error::{Error, Result};

/// Deserializes an Option<String> where empty strings become None.
/// Use with `#[serde(default, deserialize_with = "empty_string_as_none")]`
pub fn empty_string_as_none<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.is_empty()))
}

lazy_static::lazy_static! {
    static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();
    pub static ref ALLOW_PRIVATE_AI_BASE_URLS: bool = std::env::var("ALLOW_PRIVATE_AI_BASE_URLS")
        .ok()
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    /// Drops the cache breakpoints from agent-step requests on every Anthropic platform,
    /// not just the one that motivates it: a Google Cloud project can have explicit prompt
    /// caching turned off (by request to Cloud support), and Vertex then rejects any request
    /// carrying breakpoints. An instance that sets this to unblock such a project also gives
    /// up caching on its direct-Anthropic and Foundry resources.
    pub static ref DISABLE_ANTHROPIC_PROMPT_CACHING: bool =
        std::env::var("DISABLE_ANTHROPIC_PROMPT_CACHING")
            .ok()
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
}

pub const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";
pub const GOOGLE_AI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Hosts that serve the OpenAI API with Azure conventions: Azure OpenAI
/// (`*.openai.azure.com`), AI Foundry (`*.services.ai.azure.com`,
/// `*.cognitiveservices.azure.com`), their sovereign-cloud counterparts, and API
/// Management fronting either.
const AZURE_HOST_SUFFIXES: &[&str] = &[
    ".azure.com",
    ".azure.us",
    ".azure.cn",
    ".azure-api.net",
    ".azure-api.us",
    ".azure-api.cn",
];

/// The deployment path Azure OpenAI serves under. It identifies an Azure endpoint
/// reached through a custom domain, which `AZURE_HOST_SUFFIXES` cannot catch — it
/// is the format `openai_azure_base_path` documents.
const AZURE_DEPLOYMENTS_PATH: &str = "/openai/deployments";

/// Where the deployment path starts in `base_url`, i.e. the end of the resource root.
/// ASCII lowercasing preserves byte offsets, so the index is a char boundary in
/// `base_url` and slicing on it cannot panic; `to_lowercase` would not be.
fn azure_deployments_index(base_url: &str) -> Option<usize> {
    base_url.to_ascii_lowercase().find(AZURE_DEPLOYMENTS_PATH)
}

/// Whether an OpenAI-API base URL is served by Azure, which authenticates with the
/// `api-key` header and lays its routes out under `/openai/...`.
///
/// An OpenAI-compatible endpoint on an Azure-owned domain (e.g. a self-hosted
/// server behind API Management) is misread as Azure; such a resource has to use
/// the `customai` provider.
fn is_azure_endpoint(base_url: &str) -> bool {
    let authority = base_url
        .split_once("://")
        .map_or(base_url, |(_, rest)| rest)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = host_port
        .rsplit_once(':')
        .map_or(host_port, |(host, _)| host)
        .trim_end_matches('.')
        .to_ascii_lowercase();

    AZURE_HOST_SUFFIXES
        .iter()
        .any(|suffix| host.ends_with(suffix))
        || azure_deployments_index(base_url).is_some()
}

/// Empty string signals BedrockClient::from_env() to use the region from AWS environment/config
/// (e.g., AWS_REGION or AWS_DEFAULT_REGION env vars, or ~/.aws/config)
pub const USE_ENV_REGION: &str = "";

/// Platform variant for providers that support Google Vertex AI (Anthropic, GoogleAI).
#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AIPlatform {
    #[default]
    Standard,
    GoogleVertexAi,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AIProvider {
    OpenAI,
    #[serde(rename = "azure_openai")]
    AzureOpenAI,
    #[serde(rename = "azure_foundry")]
    AzureFoundry,
    Anthropic,
    Mistral,
    DeepSeek,
    GoogleAI,
    Groq,
    OpenRouter,
    TogetherAI,
    CustomAI,
    #[serde(rename = "aws_bedrock")]
    AWSBedrock,
}

impl AIProvider {
    /// Get the base URL for the AI provider
    pub async fn get_base_url(&self, resource_base_url: Option<String>, db: &DB) -> Result<String> {
        if let Some(base_url) = resource_base_url {
            if !*ALLOW_PRIVATE_AI_BASE_URLS {
                use windmill_common::ssrf::SsrfValidationError;
                windmill_common::ssrf::validate_url_for_ssrf(&base_url)
                    .await
                    .map_err(|e| match e {
                        // The env-var hint is only actionable when the URL is
                        // well-formed but blocked for targeting a private
                        // address. For a malformed URL or bad scheme, surface
                        // the real error so users fix the URL (issue #9171).
                        e @ SsrfValidationError::Private { .. } => Error::BadRequest(format!(
                            "{e}. If you need to use private/internal AI endpoints, \
                             set the ALLOW_PRIVATE_AI_BASE_URLS=true environment variable"
                        )),
                        e => Error::from(e),
                    })?;
            }
            return Ok(base_url);
        }

        match self {
            AIProvider::OpenAI => {
                // Check for Azure base path override
                let azure_base_path = sqlx::query_scalar!(
                    "SELECT value
                    FROM global_settings
                    WHERE name = 'openai_azure_base_path'",
                )
                .fetch_optional(db)
                .await?;

                let azure_base_path = if let Some(azure_base_path) = azure_base_path {
                    Some(
                        serde_json::from_value::<String>(azure_base_path).map_err(|e| {
                            Error::internal_err(format!("validating openai azure base path {e:#}"))
                        })?,
                    )
                } else {
                    OPENAI_AZURE_BASE_PATH.clone()
                };

                Ok(azure_base_path.unwrap_or_else(|| OPENAI_BASE_URL.to_string()))
            }
            AIProvider::DeepSeek => Ok(DEEPSEEK_BASE_URL.to_string()),
            AIProvider::GoogleAI => Ok(GOOGLE_AI_BASE_URL.to_string()),
            AIProvider::Groq => Ok("https://api.groq.com/openai/v1".to_string()),
            AIProvider::OpenRouter => Ok("https://openrouter.ai/api/v1".to_string()),
            AIProvider::TogetherAI => Ok("https://api.together.xyz/v1".to_string()),
            AIProvider::Anthropic => Ok("https://api.anthropic.com/v1".to_string()),
            AIProvider::Mistral => Ok("https://api.mistral.ai/v1".to_string()),
            p @ (AIProvider::CustomAI | AIProvider::AzureOpenAI | AIProvider::AzureFoundry) => {
                Err(Error::BadRequest(format!(
                    "{:?} provider requires a base URL in the resource",
                    p
                )))
            }
            AIProvider::AWSBedrock => {
                // AWS Bedrock uses the SDK directly, not HTTP base URL
                Err(Error::internal_err(
                    "AWS Bedrock uses SDK directly, not HTTP base URL".to_string(),
                ))
            }
        }
    }

    /// Check if this provider is Anthropic (needs special handling)
    pub fn is_anthropic(&self) -> bool {
        matches!(self, AIProvider::Anthropic)
    }

    /// Check whether this provider/URL combination uses Azure conventions
    /// (the `api-key` auth header and Azure URL building). This covers Azure
    /// OpenAI, Azure AI Foundry, and the `OpenAI` provider pointed at an Azure
    /// endpoint through `openai_azure_base_path` or a resource base URL.
    ///
    /// A custom `OpenAI` base URL that is not an Azure endpoint is an
    /// OpenAI-compatible one (gateway, proxy, self-hosted server) and keeps bearer
    /// auth and the plain `<base>/<path>` layout.
    pub fn is_azure(&self, base_url: &str) -> bool {
        match self {
            AIProvider::AzureOpenAI | AIProvider::AzureFoundry => true,
            AIProvider::OpenAI => is_azure_endpoint(base_url),
            _ => false,
        }
    }

    /// Build an Azure-style OpenAI-compatible URL (Azure OpenAI / Azure AI Foundry)
    /// for the given path. The resource base URL may be stored as the bare resource
    /// root (e.g. `https://<res>.services.ai.azure.com`) or with a legacy `/openai`
    /// or `/openai/v1` suffix (older Foundry resources shipped that way), or as a
    /// deployment path (`.../openai/deployments/<id>`). All of those resolve to the
    /// canonical `<root>/openai/v1/<path>`. Any other explicit path is preserved as-is
    /// with only `/<path>` appended.
    pub fn build_azure_openai_url(base_url: &str, path: &str) -> String {
        let base_url = base_url.trim_end_matches('/');
        if base_url.ends_with("/openai/v1") {
            format!("{}/{}", base_url, path)
        } else if base_url.ends_with("/openai") {
            format!("{}/v1/{}", base_url, path)
        } else if let Some(index) = azure_deployments_index(base_url) {
            // A base naming the deployment surface, pinned to a deployment or not,
            // resolves to the resource root: that surface serves only with an
            // `api-version` query and 404s without one, while the v1 surface takes
            // the deployment from the request body.
            format!("{}/openai/v1/{}", &base_url[..index], path)
        } else if Self::is_bare_host(base_url) {
            // A resource root with no path (Foundry convention, or an Azure OpenAI
            // resource root) targets the OpenAI-compatible v1 surface.
            format!("{}/openai/v1/{}", base_url, path)
        } else {
            format!("{}/{}", base_url, path)
        }
    }

    /// Build an Anthropic Messages API URL. The API lives under `<root>/v1/`, but a
    /// resource may store the base URL with or without that `/v1` and a caller may
    /// pass the path with or without it (the Anthropic SDK sends `v1/messages`,
    /// `get_endpoint` asks for `messages`). Both forms resolve to the same endpoint
    /// so a resource reads the same way from the proxy and from an agent step.
    ///
    /// This assumes the API is served under `/v1`, as api.anthropic.com and the
    /// gateways in front of it are.
    pub fn build_anthropic_api_url(base_url: &str, path: &str) -> String {
        let base_url = base_url.trim_end_matches('/');
        let root = base_url.strip_suffix("/v1").unwrap_or(base_url);
        let path = path.trim_start_matches('/');
        let path = path.strip_prefix("v1/").unwrap_or(path);
        format!("{}/v1/{}", root.trim_end_matches('/'), path)
    }

    /// Whether the URL is a bare scheme+host with no path component, e.g.
    /// `https://x.services.ai.azure.com` (vs `https://x.openai.azure.com/openai/deployments/y`).
    fn is_bare_host(url: &str) -> bool {
        let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
        !after_scheme.contains('/')
    }

    /// Strip any known Foundry API sub-path from the resource base URL to recover the
    /// resource root, so the correct per-model-family path can be appended. Handles
    /// both the current root-URL convention and legacy `/openai/v1`-style values.
    fn azure_foundry_root(base_url: &str) -> &str {
        let base_url = base_url.trim_end_matches('/');
        if let Some(index) = azure_deployments_index(base_url) {
            return &base_url[..index];
        }
        for suffix in [
            "/openai/v1",
            "/anthropic/v1",
            "/openai",
            "/anthropic",
            "/models",
        ] {
            if let Some(root) = base_url.strip_suffix(suffix) {
                return root.trim_end_matches('/');
            }
        }
        base_url
    }

    /// Build an Azure AI Foundry Anthropic Messages API URL. Claude deployments on
    /// Foundry are served only through `<root>/anthropic/v1/...`, not the
    /// OpenAI-compatible `/openai/v1` surface.
    pub fn build_azure_foundry_anthropic_url(base_url: &str, path: &str) -> String {
        format!(
            "{}/anthropic/v1/{}",
            Self::azure_foundry_root(base_url),
            path
        )
    }

    /// Whether a Foundry deployment name refers to an Anthropic (Claude) model,
    /// which requires the Anthropic Messages API rather than OpenAI chat completions.
    pub fn is_anthropic_model(model: &str) -> bool {
        model.to_lowercase().starts_with("claude")
    }

    /// Extract model from request body (needed for Azure deployments)
    pub fn extract_model_from_body(body: &[u8]) -> Result<String> {
        #[derive(serde::Deserialize)]
        struct ModelRequest {
            model: String,
        }

        let model_request: ModelRequest = serde_json::from_slice(body).map_err(|e| {
            Error::internal_err(format!("Failed to parse request body for model: {}", e))
        })?;

        Ok(model_request.model)
    }
}

impl TryFrom<&str> for AIProvider {
    type Error = Error;
    fn try_from(s: &str) -> Result<Self> {
        let s = serde_json::from_value::<AIProvider>(serde_json::Value::String(s.to_string()))
            .map_err(|e| Error::BadRequest(format!("Invalid AI provider: {}", e)))?;
        Ok(s)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProviderConfig {
    pub resource_path: String,
    pub models: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_enabled: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProviderModel {
    pub model: String,
    pub provider: AIProvider,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn azure_openai_url_handles_root_and_legacy_suffixes() {
        // Current convention: resource stores the bare root.
        assert_eq!(
            AIProvider::build_azure_openai_url(
                "https://wm-test-ai.services.ai.azure.com",
                "chat/completions"
            ),
            "https://wm-test-ai.services.ai.azure.com/openai/v1/chat/completions"
        );
        // Legacy resources that already baked in /openai/v1 must still resolve.
        assert_eq!(
            AIProvider::build_azure_openai_url(
                "https://wm-test-ai.services.ai.azure.com/openai/v1/",
                "chat/completions"
            ),
            "https://wm-test-ai.services.ai.azure.com/openai/v1/chat/completions"
        );
        // Azure OpenAI resources typically end in /openai.
        assert_eq!(
            AIProvider::build_azure_openai_url(
                "https://example.openai.azure.com/openai",
                "chat/completions"
            ),
            "https://example.openai.azure.com/openai/v1/chat/completions"
        );
        assert_eq!(
            AIProvider::build_azure_openai_url(
                "https://example.openai.azure.com/openai/deployments",
                "chat/completions"
            ),
            "https://example.openai.azure.com/openai/v1/chat/completions"
        );
        // A base pinned to a deployment resolves to the v1 surface too.
        assert_eq!(
            AIProvider::build_azure_openai_url(
                "https://example.openai.azure.com/openai/deployments/my-deployment",
                "chat/completions"
            ),
            "https://example.openai.azure.com/openai/v1/chat/completions"
        );
    }

    /// An OpenAI resource with a custom base URL is an OpenAI-compatible endpoint
    /// unless it is an Azure one. Azure treatment swaps bearer auth for the
    /// `api-key` header and rewrites the path, so both directions must hold.
    #[test]
    fn openai_is_azure_only_for_azure_endpoints() {
        for base_url in [
            // A gateway path ending in /openai must not read as Azure.
            "https://gateway-eu.pydantic.dev/proxy/openai",
            "https://openrouter.ai/api/v1",
            "http://localhost:4000/v1",
            OPENAI_BASE_URL,
        ] {
            assert!(
                !AIProvider::OpenAI.is_azure(base_url),
                "{base_url} must not be treated as Azure"
            );
        }

        for base_url in [
            "https://example.openai.azure.com/openai/deployments/my-deployment",
            "https://wm-test-ai.services.ai.azure.com",
            "https://contoso.azure-api.cn/openai",
            // Azure OpenAI behind a custom domain, the format
            // `openai_azure_base_path` documents.
            "https://openai.contoso.com/openai/deployments/gpt-4o",
        ] {
            assert!(
                AIProvider::OpenAI.is_azure(base_url),
                "{base_url} must be treated as Azure"
            );
        }
    }

    #[test]
    fn azure_foundry_anthropic_url_from_root_and_legacy() {
        // Root URL.
        assert_eq!(
            AIProvider::build_azure_foundry_anthropic_url(
                "https://wm-test-ai.services.ai.azure.com",
                "messages"
            ),
            "https://wm-test-ai.services.ai.azure.com/anthropic/v1/messages"
        );
        // Legacy /openai/v1 base is normalized back to the root, then routed to
        // the Anthropic Messages API.
        assert_eq!(
            AIProvider::build_azure_foundry_anthropic_url(
                "https://wm-test-ai.services.ai.azure.com/openai/v1",
                "messages"
            ),
            "https://wm-test-ai.services.ai.azure.com/anthropic/v1/messages"
        );
        // A deployment-pinned base recovers the same root, so a Claude model on such
        // a resource is routed like every other model on it.
        assert_eq!(
            AIProvider::build_azure_foundry_anthropic_url(
                "https://wm-test-ai.services.ai.azure.com/openai/deployments/claude-sonnet-5",
                "messages"
            ),
            "https://wm-test-ai.services.ai.azure.com/anthropic/v1/messages"
        );
    }

    #[test]
    fn detects_anthropic_models() {
        assert!(AIProvider::is_anthropic_model("claude-sonnet-5"));
        assert!(AIProvider::is_anthropic_model("Claude-Opus-4-8"));
        assert!(!AIProvider::is_anthropic_model("gpt-4o"));
        assert!(!AIProvider::is_anthropic_model("DeepSeek-R1"));
    }
}
