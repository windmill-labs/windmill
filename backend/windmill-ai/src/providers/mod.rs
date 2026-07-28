pub mod anthropic;
#[cfg(feature = "bedrock")]
pub mod bedrock;
pub mod google_ai;
pub mod openai;
pub mod openrouter;
pub mod other;

use crate::{
    ai_providers::{AIPlatform, AIProvider},
    credentials::ProviderCredentials,
    query_builder::QueryBuilder,
};

use self::{
    anthropic::AnthropicQueryBuilder, google_ai::GoogleAIQueryBuilder, openai::OpenAIQueryBuilder,
    openrouter::OpenRouterQueryBuilder, other::OtherQueryBuilder,
};

/// Factory function to create the appropriate query builder from resolved credentials.
///
/// `model` is the deployment/model name of the request. It matters only for Azure AI
/// Foundry, which fronts multiple model families under one resource: Claude
/// deployments speak the Anthropic Messages API while everything else is
/// OpenAI-compatible, so the builder is chosen per model rather than per provider.
pub fn create_query_builder(
    credentials: &ProviderCredentials,
    model: &str,
) -> Box<dyn QueryBuilder> {
    match credentials.provider {
        AIProvider::GoogleAI => Box::new(GoogleAIQueryBuilder::new(credentials.platform.clone())),
        AIProvider::OpenAI => Box::new(OpenAIQueryBuilder::new(credentials.provider.clone())),
        AIProvider::Anthropic => Box::new(AnthropicQueryBuilder::new(
            credentials.provider.clone(),
            credentials.platform.clone(),
        )),
        AIProvider::OpenRouter => Box::new(OpenRouterQueryBuilder::new()),
        AIProvider::AzureFoundry if AIProvider::is_anthropic_model(model) => Box::new(
            AnthropicQueryBuilder::new(AIProvider::AzureFoundry, AIPlatform::Standard),
        ),
        _ => Box::new(OtherQueryBuilder::new(credentials.provider.clone())),
    }
}

/// The proxy (workspace/instance AI settings) and the query builder (AI agent step)
/// each derive the endpoint and the credential header for the same resource. They
/// must agree, or a resource authenticates in one and 401s in the other.
#[cfg(test)]
mod parity_tests {
    use super::*;
    use crate::proxy::{
        credential_header, proxy_execution_mode, retain_effective_credentials, ProxyBuildArgs,
        ProxyExecutionMode, CREDENTIAL_HEADERS,
    };
    use crate::types::OutputType;
    use http::{HeaderMap, Method};
    use std::collections::HashMap;

    struct ParityCase {
        provider: AIProvider,
        platform: AIPlatform,
        base_url: &'static str,
        model: &'static str,
        /// The path the client SDK sends for the endpoint the query builder targets.
        proxy_path: &'static str,
    }

    fn case(
        provider: AIProvider,
        base_url: &'static str,
        model: &'static str,
        proxy_path: &'static str,
    ) -> ParityCase {
        ParityCase { provider, platform: AIPlatform::Standard, base_url, model, proxy_path }
    }

    fn credentials(kase: &ParityCase) -> ProviderCredentials {
        ProviderCredentials {
            provider: kase.provider.clone(),
            base_url: kase.base_url.to_string(),
            api_key: Some("secret".to_string()),
            access_token: None,
            organization_id: None,
            user: None,
            region: None,
            aws_access_key_id: None,
            aws_secret_access_key: None,
            aws_session_token: None,
            platform: kase.platform.clone(),
            custom_headers: HashMap::new(),
        }
    }

    fn only_credentials<I: IntoIterator<Item = (String, String)>>(
        headers: I,
    ) -> Vec<(String, String)> {
        let mut found = headers
            .into_iter()
            .filter(|(name, _)| {
                CREDENTIAL_HEADERS
                    .iter()
                    .any(|credential| name.eq_ignore_ascii_case(credential))
            })
            .map(|(name, value)| (name.to_ascii_lowercase(), value))
            .collect::<Vec<_>>();
        found.sort();
        found
    }

    fn cases() -> Vec<ParityCase> {
        vec![
            case(
                AIProvider::Anthropic,
                "https://api.anthropic.com/v1",
                "claude-sonnet-5",
                "v1/messages",
            ),
            // A gateway base URL without the `/v1` must resolve the same way in both.
            case(
                AIProvider::Anthropic,
                "https://gateway.example/proxy/anthropic",
                "claude-sonnet-5",
                "v1/messages",
            ),
            ParityCase {
                provider: AIProvider::Anthropic,
                platform: AIPlatform::GoogleVertexAi,
                base_url: "https://europe-west1-aiplatform.googleapis.com/v1/projects/p/locations/europe-west1/publishers/anthropic/models",
                model: "claude-sonnet-5",
                proxy_path: "v1/messages",
            },
            case(
                AIProvider::AzureFoundry,
                "https://example.services.ai.azure.com",
                "claude-sonnet-5",
                "v1/messages",
            ),
            case(
                AIProvider::AzureOpenAI,
                "https://example.openai.azure.com/openai",
                "gpt-4o",
                "chat/completions",
            ),
            case(
                AIProvider::OpenAI,
                "https://api.openai.com/v1",
                "gpt-5.6-terra",
                "responses",
            ),
            // An OpenAI-compatible gateway keeps bearer auth and the plain path.
            case(
                AIProvider::OpenAI,
                "https://gateway.example/proxy/openai",
                "gpt-5.6-terra",
                "responses",
            ),
            // An OpenAI resource pointed at Azure through `openai_azure_base_path`.
            case(
                AIProvider::OpenAI,
                "https://example.openai.azure.com/openai/deployments/gpt-4o",
                "gpt-4o",
                "responses",
            ),
            case(
                AIProvider::CustomAI,
                "https://litellm.example/v1",
                "gpt-4o",
                "chat/completions",
            ),
            // A base URL stored with a trailing slash must not double it.
            case(
                AIProvider::CustomAI,
                "https://litellm.example/v1/",
                "gpt-4o",
                "chat/completions",
            ),
            case(
                AIProvider::OpenRouter,
                "https://openrouter.ai/api/v1",
                "anthropic/claude-sonnet-5",
                "chat/completions",
            ),
        ]
    }

    /// The agent step assembles credential headers itself (`ai_executor`), so the
    /// rule it applies must match `credential_header`: the built-in credential is
    /// dropped when the resource carries its own and when there is no key at all,
    /// and an empty credential never goes out.
    #[test]
    fn both_paths_agree_on_when_the_built_in_credential_is_dropped() {
        let mut kase = case(
            AIProvider::CustomAI,
            "https://gateway.example/v1",
            "gpt-4o",
            "chat/completions",
        );
        kase.platform = AIPlatform::Standard;

        for (api_key, resource_header) in [
            // Keyless endpoint: no credential from either path.
            (None, None),
            // Resource carries its own: only that one travels.
            (None, Some(("x-api-key", "resource-key"))),
            (Some("secret"), Some(("Authorization", "Bearer resource"))),
            // Nothing to defer to: the built-in credential travels.
            (Some("secret"), None),
        ] {
            let mut credentials = credentials(&kase);
            credentials.api_key = api_key.map(str::to_string);
            if let Some((name, value)) = resource_header {
                credentials.custom_headers = HashMap::from([(name.to_string(), value.to_string())]);
            }
            let label = format!("api_key={api_key:?} resource_header={resource_header:?}");

            let from_proxy = credential_header(&credentials, "authorization");
            let from_agent_step = agent_step_credential(&credentials, &kase);

            assert_eq!(
                from_proxy, from_agent_step,
                "{label}: proxy and agent step disagree on the built-in credential"
            );
            assert!(
                from_agent_step
                    .as_ref()
                    .is_none_or(|(_, value)| value != "Bearer "),
                "{label}: an empty credential must not be sent"
            );
        }
    }

    /// The credential the agent step is left with, through the same helper it uses.
    fn agent_step_credential(
        credentials: &ProviderCredentials,
        kase: &ParityCase,
    ) -> Option<(String, String)> {
        let built = create_query_builder(credentials, kase.model).get_auth_headers(
            credentials.api_key.as_deref().unwrap_or(""),
            &credentials.base_url,
            &OutputType::Text,
        );
        only_credentials(
            retain_effective_credentials(credentials, built)
                .into_iter()
                .map(|(name, value)| (name.to_string(), value)),
        )
        .pop()
    }

    #[test]
    fn both_paths_agree_on_endpoint_and_credential() {
        for kase in cases() {
            let credentials = credentials(&kase);
            let builder = create_query_builder(&credentials, kase.model);
            let label = format!("{:?} {:?} {}", kase.provider, kase.platform, kase.base_url);

            assert_eq!(
                proxy_execution_mode(&kase.provider),
                ProxyExecutionMode::HttpForward,
                "{label}: only forwarded providers have both paths to compare"
            );

            let headers = HeaderMap::new();
            // Vertex reads the model from the body to build its URL, so the body must
            // name the same model the query builder is given.
            let body = format!(r#"{{"model":"{}","messages":[]}}"#, kase.model);
            let args = ProxyBuildArgs {
                method: &Method::POST,
                path: kase.proxy_path,
                headers: &headers,
                body: body.as_bytes(),
                credentials: &credentials,
            };
            let proxied = builder
                .build_proxy_request(&args)
                .unwrap_or_else(|e| panic!("{label}: proxy request failed: {e}"));

            assert_eq!(
                builder.get_endpoint(&credentials.base_url, kase.model, &OutputType::Text),
                proxied.url,
                "{label}: agent step and proxy disagree on the endpoint"
            );

            let from_builder = builder
                .get_auth_headers("secret", &credentials.base_url, &OutputType::Text)
                .into_iter()
                .map(|(name, value)| (name.to_string(), value));
            assert_eq!(
                only_credentials(from_builder),
                only_credentials(proxied.headers),
                "{label}: agent step and proxy disagree on the credential header"
            );
        }
    }
}
