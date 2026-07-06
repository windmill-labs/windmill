use std::collections::HashMap;

use http::{HeaderMap, Method};
use serde_json::value::RawValue;
use windmill_common::error::{Error, Result};

use crate::ai_providers::AIProvider;
use crate::credentials::ProviderCredentials;
use crate::utils::AI_HTTP_HEADERS;

pub mod fim;

/// Inputs needed to transform an OpenAI-compatible proxy request for a provider.
pub struct ProxyBuildArgs<'a> {
    pub method: &'a Method,
    pub path: &'a str,
    pub headers: &'a HeaderMap,
    pub body: &'a [u8],
    pub credentials: &'a ProviderCredentials,
}

/// Provider-specific request produced by proxy request builders.
#[derive(Clone, Debug)]
pub struct ProxyRequest {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// How the API proxy should execute a request for a provider.
///
/// Most providers can be represented as a transformed HTTP request. Google AI
/// and Bedrock need native execution because their proxy paths also transform
/// responses or call an SDK rather than forwarding an HTTP request directly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProxyExecutionMode {
    HttpForward,
    NativeGoogleAi,
    NativeAwsBedrock,
}

impl ProxyExecutionMode {
    pub fn uses_query_builder_proxy(self) -> bool {
        matches!(self, Self::HttpForward)
    }
}

pub fn supports_openai_compatible_proxy(provider: &AIProvider) -> bool {
    matches!(
        provider,
        AIProvider::OpenAI
            | AIProvider::AzureOpenAI
            | AIProvider::AzureFoundry
            | AIProvider::Mistral
            | AIProvider::DeepSeek
            | AIProvider::Groq
            | AIProvider::OpenRouter
            | AIProvider::TogetherAI
            | AIProvider::CustomAI
    )
}

pub fn proxy_execution_mode(provider: &AIProvider) -> ProxyExecutionMode {
    match provider {
        AIProvider::OpenAI
        | AIProvider::AzureOpenAI
        | AIProvider::AzureFoundry
        | AIProvider::Anthropic
        | AIProvider::Mistral
        | AIProvider::DeepSeek
        | AIProvider::Groq
        | AIProvider::OpenRouter
        | AIProvider::TogetherAI
        | AIProvider::CustomAI => ProxyExecutionMode::HttpForward,
        AIProvider::GoogleAI => ProxyExecutionMode::NativeGoogleAi,
        AIProvider::AWSBedrock => ProxyExecutionMode::NativeAwsBedrock,
    }
}

pub fn supports_query_builder_proxy(provider: &AIProvider) -> bool {
    proxy_execution_mode(provider).uses_query_builder_proxy()
}

pub fn build_openai_compatible_proxy_request(args: &ProxyBuildArgs<'_>) -> Result<ProxyRequest> {
    let credentials = args.credentials;
    let body = if let Some(user) = credentials.user.as_ref() {
        add_user_to_body(args.body, user)?
    } else {
        args.body.to_vec()
    };

    let base_url = credentials.base_url.trim_end_matches('/');
    let is_azure = credentials.provider.is_azure(base_url);
    let url = if is_azure {
        AIProvider::build_azure_openai_url(base_url, args.path)
    } else {
        format!("{}/{}", base_url, args.path)
    };

    let mut headers = vec![("content-type".to_string(), "application/json".to_string())];

    if let Some(api_key) = credentials.api_key.as_ref() {
        if is_azure {
            headers.push(("api-key".to_string(), api_key.clone()));
        } else {
            headers.push(("authorization".to_string(), format!("Bearer {}", api_key)));
        }
    }

    if let Some(access_token) = credentials.access_token.as_ref() {
        headers.push((
            "authorization".to_string(),
            format!("Bearer {}", access_token),
        ));
    }

    if let Some(org_id) = credentials.organization_id.as_ref() {
        headers.push(("OpenAI-Organization".to_string(), org_id.clone()));
    }

    for (header_name, header_value) in AI_HTTP_HEADERS.iter() {
        headers.push((header_name.clone(), header_value.clone()));
    }

    for (header_name, header_value) in &credentials.custom_headers {
        headers.push((header_name.clone(), header_value.clone()));
    }

    Ok(ProxyRequest { method: args.method.clone(), url, headers, body })
}

pub(crate) fn add_user_to_body(body: &[u8], user: &str) -> Result<Vec<u8>> {
    tracing::debug!("Adding user to request body");
    let mut json_body: HashMap<String, Box<RawValue>> = serde_json::from_slice(body)
        .map_err(|e| Error::internal_err(format!("Failed to parse request body: {}", e)))?;

    let user_json_string = serde_json::Value::String(user.to_string()).to_string();

    json_body.insert(
        "user".to_string(),
        RawValue::from_string(user_json_string)
            .map_err(|e| Error::internal_err(format!("Failed to parse user: {}", e)))?,
    );

    serde_json::to_vec(&json_body)
        .map_err(|e| Error::internal_err(format!("Failed to reserialize request body: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::ai_providers::AIPlatform;

    fn credentials(provider: AIProvider, base_url: &str) -> ProviderCredentials {
        ProviderCredentials {
            provider,
            base_url: base_url.to_string(),
            api_key: Some("api-key".to_string()),
            access_token: None,
            organization_id: Some("org-id".to_string()),
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
    fn builds_openai_compatible_proxy_request() {
        let credentials = credentials(AIProvider::OpenRouter, "https://openrouter.ai/api/v1/");
        let method = Method::POST;
        let headers = HeaderMap::new();
        let body = br#"{"model":"openrouter/model","messages":[]}"#;

        let request = build_openai_compatible_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "chat/completions",
            headers: &headers,
            body,
            credentials: &credentials,
        })
        .unwrap();

        assert_eq!(request.method, Method::POST);
        assert_eq!(request.url, "https://openrouter.ai/api/v1/chat/completions");
        assert_eq!(request.body, body.to_vec());
        assert!(request
            .headers
            .contains(&("authorization".to_string(), "Bearer api-key".to_string())));
        assert!(request
            .headers
            .contains(&("OpenAI-Organization".to_string(), "org-id".to_string())));
    }

    #[test]
    fn query_builder_proxy_support_includes_anthropic() {
        let cases = [
            (AIProvider::OpenAI, ProxyExecutionMode::HttpForward),
            (AIProvider::AzureOpenAI, ProxyExecutionMode::HttpForward),
            (AIProvider::AzureFoundry, ProxyExecutionMode::HttpForward),
            (AIProvider::Anthropic, ProxyExecutionMode::HttpForward),
            (AIProvider::Mistral, ProxyExecutionMode::HttpForward),
            (AIProvider::DeepSeek, ProxyExecutionMode::HttpForward),
            (AIProvider::Groq, ProxyExecutionMode::HttpForward),
            (AIProvider::OpenRouter, ProxyExecutionMode::HttpForward),
            (AIProvider::TogetherAI, ProxyExecutionMode::HttpForward),
            (AIProvider::CustomAI, ProxyExecutionMode::HttpForward),
            (AIProvider::GoogleAI, ProxyExecutionMode::NativeGoogleAi),
            (AIProvider::AWSBedrock, ProxyExecutionMode::NativeAwsBedrock),
        ];

        for (provider, expected_mode) in cases {
            let mode = proxy_execution_mode(&provider);
            assert_eq!(
                mode, expected_mode,
                "unexpected proxy mode for {provider:?}"
            );
            assert_eq!(
                supports_query_builder_proxy(&provider),
                mode.uses_query_builder_proxy(),
                "query-builder support drifted for {provider:?}"
            );
        }
    }

    #[test]
    fn builds_azure_openai_proxy_request() {
        let credentials = credentials(
            AIProvider::AzureOpenAI,
            "https://example.openai.azure.com/openai",
        );
        let method = Method::POST;
        let headers = HeaderMap::new();

        let request = build_openai_compatible_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "chat/completions",
            headers: &headers,
            body: br#"{"model":"deployment","messages":[]}"#,
            credentials: &credentials,
        })
        .unwrap();

        assert_eq!(
            request.url,
            "https://example.openai.azure.com/openai/v1/chat/completions"
        );
        assert!(request
            .headers
            .contains(&("api-key".to_string(), "api-key".to_string())));
    }

    #[test]
    fn builds_azure_foundry_proxy_request() {
        // Foundry's OpenAI-compatible endpoint uses the same Azure conventions
        // (api-key header, /openai -> /openai/v1 path) as Azure OpenAI.
        let credentials = credentials(
            AIProvider::AzureFoundry,
            "https://example.services.ai.azure.com/openai",
        );
        let method = Method::POST;
        let headers = HeaderMap::new();

        let request = build_openai_compatible_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "chat/completions",
            headers: &headers,
            body: br#"{"model":"gpt-4o","messages":[]}"#,
            credentials: &credentials,
        })
        .unwrap();

        assert_eq!(
            request.url,
            "https://example.services.ai.azure.com/openai/v1/chat/completions"
        );
        assert!(request
            .headers
            .contains(&("api-key".to_string(), "api-key".to_string())));
    }

    #[test]
    fn injects_user_into_proxy_body() {
        let mut credentials = credentials(AIProvider::OpenAI, "https://api.openai.com/v1");
        credentials.user = Some("user-1".to_string());
        let method = Method::POST;
        let headers = HeaderMap::new();

        let request = build_openai_compatible_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "chat/completions",
            headers: &headers,
            body: br#"{"model":"gpt-4o","messages":[]}"#,
            credentials: &credentials,
        })
        .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();

        assert_eq!(body["user"], "user-1");
        assert_eq!(body["model"], "gpt-4o");
    }

    #[test]
    fn foundry_routes_claude_to_anthropic_messages_api() {
        use crate::providers::create_query_builder;
        use crate::types::OutputType;

        let creds = credentials(
            AIProvider::AzureFoundry,
            "https://wm-test-ai.services.ai.azure.com/openai/v1",
        );

        // Claude deployment -> Anthropic Messages API surface + x-api-key auth.
        let claude = create_query_builder(&creds, "claude-sonnet-5");
        assert_eq!(
            claude.get_endpoint(&creds.base_url, "claude-sonnet-5", &OutputType::Text),
            "https://wm-test-ai.services.ai.azure.com/anthropic/v1/messages"
        );
        let auth = claude.get_auth_headers("api-key", &creds.base_url, &OutputType::Text);
        assert!(auth.contains(&("x-api-key", "api-key".to_string())));

        // OpenAI-compatible deployment -> chat completions surface.
        let gpt = create_query_builder(&creds, "gpt-4o");
        assert_eq!(
            gpt.get_endpoint(&creds.base_url, "gpt-4o", &OutputType::Text),
            "https://wm-test-ai.services.ai.azure.com/openai/v1/chat/completions"
        );
    }
}
