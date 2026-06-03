use std::collections::HashMap;

use http::{HeaderMap, Method};
use serde_json::value::RawValue;
use windmill_common::error::{Error, Result};

use crate::ai_providers::{AIPlatform, AIProvider, OPENAI_CHATGPT_ACCOUNT_BASE_URL};
use crate::credentials::ProviderCredentials;
use crate::providers::codex::{
    add_codex_responses_defaults, CodexRequestContext, CODEX_MODELS_PATH, CODEX_RESPONSES_PATH,
};
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
            | AIProvider::OpenAIChatGPTAccount
            | AIProvider::AzureOpenAI
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
        | AIProvider::OpenAIChatGPTAccount
        | AIProvider::AzureOpenAI
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
    if args.credentials.provider == AIProvider::OpenAIChatGPTAccount {
        return build_openai_chatgpt_account_proxy_request(args);
    }

    let credentials = args.credentials;
    let body = if let Some(user) = credentials.user.as_ref() {
        add_user_to_body(args.body, user)?
    } else {
        args.body.to_vec()
    };

    let base_url = credentials.base_url.trim_end_matches('/');
    let is_azure = credentials.provider.is_azure_openai(base_url);
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

fn build_openai_chatgpt_account_proxy_request(args: &ProxyBuildArgs<'_>) -> Result<ProxyRequest> {
    let credentials = args.credentials;
    let base_url = credentials
        .base_url
        .trim_end_matches('/')
        .trim_end_matches("/v1");
    let base_url = if base_url.is_empty() {
        OPENAI_CHATGPT_ACCOUNT_BASE_URL
    } else {
        base_url
    };

    let path = match args.path.trim_start_matches('/') {
        "models" => CODEX_MODELS_PATH.to_string(),
        "responses" => CODEX_RESPONSES_PATH.to_string(),
        unsupported => {
            return Err(Error::BadRequest(format!(
                "Unsupported OpenAI ChatGPT Account proxy path: {unsupported}"
            )))
        }
    };

    let context = CodexRequestContext::new();
    let (body, is_streaming_response) = if args.path.trim_start_matches('/') == "responses" {
        let body = build_codex_responses_body(args.body, &context)?;
        let is_streaming_response = serde_json::from_slice::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| value.get("stream").and_then(serde_json::Value::as_bool))
            .unwrap_or(false);
        (body, is_streaming_response)
    } else {
        (args.body.to_vec(), false)
    };

    let mut headers = vec![
        ("content-type".to_string(), "application/json".to_string()),
    ];
    headers.extend(
        context
            .headers()
            .into_iter()
            .map(|(name, value)| (name.to_string(), value)),
    );

    if is_streaming_response {
        headers.push(("accept".to_string(), "text/event-stream".to_string()));
    }

    if let Some(access_token) = credentials
        .access_token
        .as_ref()
        .or(credentials.api_key.as_ref())
    {
        headers.push((
            "authorization".to_string(),
            format!("Bearer {}", access_token),
        ));
    }

    for (header_name, header_value) in AI_HTTP_HEADERS.iter() {
        headers.push((header_name.clone(), header_value.clone()));
    }

    for (header_name, header_value) in &credentials.custom_headers {
        headers.push((header_name.clone(), header_value.clone()));
    }

    Ok(ProxyRequest {
        method: args.method.clone(),
        url: format!("{}/{}", base_url, path),
        headers,
        body,
    })
}

fn build_codex_responses_body(body: &[u8], context: &CodexRequestContext) -> Result<Vec<u8>> {
    let value = serde_json::from_slice(body)
        .map_err(|e| Error::internal_err(format!("Failed to parse request body: {}", e)))?;
    let value = add_codex_responses_defaults(value, context, None)?;

    serde_json::to_vec(&value)
        .map_err(|e| Error::internal_err(format!("Failed to reserialize request body: {}", e)))
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
            enable_1m_context: false,
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
    fn builds_openai_chatgpt_account_proxy_request() {
        let mut credentials = credentials(
            AIProvider::OpenAIChatGPTAccount,
            "https://chatgpt.com/backend-api",
        );
        credentials.api_key = None;
        credentials.access_token = Some("access-token".to_string());
        credentials
            .custom_headers
            .insert("chatgpt-account-id".to_string(), "account-id".to_string());

        let method = Method::POST;
        let headers = HeaderMap::new();

        let request = build_openai_compatible_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "responses",
            headers: &headers,
            body: br#"{"model":"gpt-5.4","input":[],"stream":true,"max_output_tokens":100,"tools":[{"type":"function","name":"set_flow_json","parameters":{"type":"object","properties":{}}}]}"#,
            credentials: &credentials,
        })
        .unwrap();

        assert_eq!(
            request.url,
            "https://chatgpt.com/backend-api/codex/responses"
        );
        assert!(request.headers.contains(&(
            "authorization".to_string(),
            "Bearer access-token".to_string()
        )));
        assert!(request
            .headers
            .contains(&("chatgpt-account-id".to_string(), "account-id".to_string())));
        assert!(request.headers.contains(&(
            "accept".to_string(),
            "text/event-stream".to_string()
        )));
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], true);
        assert_eq!(body["tool_choice"], "auto");
        assert_eq!(body["parallel_tool_calls"], true);
        assert!(body.get("max_output_tokens").is_none());
    }

    #[test]
    fn preserves_non_streaming_openai_chatgpt_account_proxy_request() {
        let mut credentials = credentials(
            AIProvider::OpenAIChatGPTAccount,
            "https://chatgpt.com/backend-api",
        );
        credentials.api_key = None;
        credentials.access_token = Some("access-token".to_string());

        let method = Method::POST;
        let headers = HeaderMap::new();

        let request = build_openai_compatible_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "responses",
            headers: &headers,
            body: br#"{"model":"gpt-5.4","input":[]}"#,
            credentials: &credentials,
        })
        .unwrap();

        assert!(!request.headers.contains(&(
            "accept".to_string(),
            "text/event-stream".to_string()
        )));
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert!(body.get("stream").is_none());
    }

    #[test]
    fn query_builder_proxy_support_includes_anthropic() {
        let cases = [
            (AIProvider::OpenAI, ProxyExecutionMode::HttpForward),
            (
                AIProvider::OpenAIChatGPTAccount,
                ProxyExecutionMode::HttpForward,
            ),
            (AIProvider::AzureOpenAI, ProxyExecutionMode::HttpForward),
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
}
