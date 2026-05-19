use std::collections::HashMap;

use http::{HeaderMap, Method};
use serde_json::value::RawValue;
use windmill_common::error::{Error, Result};

use crate::ai_providers::{AIPlatform, AIProvider};
use crate::utils::AI_HTTP_HEADERS;

/// Resolved provider credentials and proxy-specific context.
///
/// This is intentionally separate from the worker's `ProviderWithResource`: API
/// proxy credentials are already resolved from workspace or instance resources.
#[derive(Clone, Debug)]
pub struct ProviderCredentials {
    pub provider: AIProvider,
    pub base_url: String,
    pub api_key: Option<String>,
    pub access_token: Option<String>,
    pub organization_id: Option<String>,
    pub user: Option<String>,
    pub region: Option<String>,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_session_token: Option<String>,
    pub platform: AIPlatform,
    pub enable_1m_context: bool,
    pub custom_headers: HashMap<String, String>,
}

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

pub fn supports_openai_compatible_proxy(provider: &AIProvider) -> bool {
    matches!(
        provider,
        AIProvider::OpenAI
            | AIProvider::AzureOpenAI
            | AIProvider::Mistral
            | AIProvider::DeepSeek
            | AIProvider::Groq
            | AIProvider::OpenRouter
            | AIProvider::TogetherAI
            | AIProvider::CustomAI
    )
}

pub fn supports_query_builder_proxy(provider: &AIProvider) -> bool {
    supports_openai_compatible_proxy(provider) || matches!(provider, AIProvider::Anthropic)
}

pub fn build_openai_compatible_proxy_request(args: &ProxyBuildArgs<'_>) -> Result<ProxyRequest> {
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
    fn query_builder_proxy_support_includes_anthropic() {
        assert!(supports_query_builder_proxy(&AIProvider::OpenAI));
        assert!(supports_query_builder_proxy(&AIProvider::Anthropic));
        assert!(!supports_query_builder_proxy(&AIProvider::GoogleAI));
        assert!(!supports_query_builder_proxy(&AIProvider::AWSBedrock));
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
