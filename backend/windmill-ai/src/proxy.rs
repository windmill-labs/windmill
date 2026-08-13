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

/// The headers a provider can carry its credential in. Any other resource header
/// is passed through untouched, so a header one provider authenticates with stays
/// an ordinary header for the providers that do not.
pub const CREDENTIAL_HEADERS: [&str; 4] =
    ["authorization", "x-api-key", "api-key", "x-goog-api-key"];

/// Whether a resource header takes over `built_in`, the credential header Windmill
/// would otherwise send. Outgoing headers are appended rather than replaced, so
/// keeping the built-in one alongside a resource-supplied credential would put two
/// on the wire, which endpoints reject.
///
/// An `authorization` header always takes over, whatever `built_in` is: every
/// endpoint reads it as the credential, so it can never coexist with another one.
pub fn resource_replaces_credential(credentials: &ProviderCredentials, built_in: &str) -> bool {
    credentials.custom_headers.keys().any(|header_name| {
        header_name.eq_ignore_ascii_case(built_in)
            || header_name.eq_ignore_ascii_case("authorization")
    })
}

/// The credential header for an outbound request, or `None` when the resource
/// supplies its own.
///
/// An api key goes in `built_in`, the header the provider authenticates keys
/// with. An OAuth token is always a bearer token instead, whatever header that
/// provider's keys use — Azure OpenAI reads keys from `api-key` but Entra ID
/// tokens only from `authorization`. `authorization` carries `Bearer <secret>`
/// and every other credential header carries the raw secret, for every provider.
pub fn credential_header(
    credentials: &ProviderCredentials,
    built_in: &str,
) -> Option<(String, String)> {
    // `resource_replaces_credential` also matches a resource `authorization`
    // header, so this covers the bearer case whatever `built_in` is.
    if resource_replaces_credential(credentials, built_in) {
        return None;
    }
    let (name, secret) = match (&credentials.api_key, &credentials.access_token) {
        (_, Some(access_token)) => ("authorization", access_token),
        (Some(api_key), None) => (built_in, api_key),
        (None, None) => return None,
    };
    let value = if name.eq_ignore_ascii_case("authorization") {
        format!("Bearer {}", secret)
    } else {
        secret.clone()
    };
    Some((name.to_string(), value))
}

/// Whether a resource authenticates by an OAuth exchange that only the API runs, so
/// a worker has no way to obtain its token.
///
/// `auth_headers` is what the provider's query builder produced: a resource can still
/// authenticate the request by supplying the credential header that provider actually
/// reads. A credential-shaped header it does not read (an `x-api-key` used for routing
/// by an OpenAI-compatible gateway, say) leaves the request unauthenticated, so it does
/// not count.
pub fn needs_unavailable_oauth_exchange(
    credentials: &ProviderCredentials,
    token_url: Option<&str>,
    auth_headers: &[(&'static str, String)],
) -> bool {
    token_url.is_some()
        && credentials.api_key.is_none()
        && !auth_headers.iter().any(|(header_name, _)| {
            CREDENTIAL_HEADERS
                .iter()
                .any(|name| header_name.eq_ignore_ascii_case(name))
                && resource_replaces_credential(credentials, header_name)
        })
}

/// Drop the query builder's built-in credential header when the resource carries
/// its own, or when there is no key at all — an endpoint may authenticate the
/// request another way (its own header, mTLS, or no auth), and an empty credential
/// must not go out in its place. This applies `credential_header`'s rule to headers a
/// query builder already built from `credentials.api_key`, so the agent step and the
/// proxy send the same credential. (An OAuth `access_token` never reaches a query
/// builder: only the API resolves one.) Non-credential headers always stay.
pub fn retain_effective_credentials(
    credentials: &ProviderCredentials,
    auth_headers: Vec<(&'static str, String)>,
) -> Vec<(&'static str, String)> {
    auth_headers
        .into_iter()
        .filter(|(header_name, _)| {
            let carries_credential = CREDENTIAL_HEADERS
                .iter()
                .any(|name| header_name.eq_ignore_ascii_case(name));
            !carries_credential
                || (credentials.api_key.is_some()
                    && !resource_replaces_credential(credentials, header_name))
        })
        .collect()
}

/// The headers every outbound AI request ends with: Windmill's own, then the
/// resource's, which come last so a resource can add to what the provider set.
pub fn common_outbound_headers(
    credentials: &ProviderCredentials,
) -> impl Iterator<Item = (String, String)> + '_ {
    AI_HTTP_HEADERS
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .chain(
            credentials
                .custom_headers
                .iter()
                .map(|(name, value)| (name.clone(), value.clone())),
        )
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

    headers.extend(credential_header(
        credentials,
        if is_azure { "api-key" } else { "authorization" },
    ));

    if let Some(org_id) = credentials.organization_id.as_ref() {
        headers.push(("OpenAI-Organization".to_string(), org_id.clone()));
    }

    headers.extend(common_outbound_headers(credentials));

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

    /// A resource that carries its own credential owns authentication; outgoing
    /// headers are appended, not replaced, so the built-in one must be dropped or
    /// two credentials reach the endpoint.
    #[test]
    fn resource_header_replaces_the_built_in_credential() {
        let mut credentials = credentials(AIProvider::CustomAI, "https://gateway.example/openai");
        credentials.custom_headers = HashMap::from([(
            "Authorization".to_string(),
            "Bearer gateway-token".to_string(),
        )]);
        let method = Method::POST;

        let request = build_openai_compatible_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "chat/completions",
            headers: &HeaderMap::new(),
            body: br#"{"model":"model","messages":[]}"#,
            credentials: &credentials,
        })
        .unwrap();

        assert_eq!(
            request
                .headers
                .iter()
                .filter(|(header_name, _)| header_name.eq_ignore_ascii_case("authorization"))
                .collect::<Vec<_>>(),
            vec![&(
                "Authorization".to_string(),
                "Bearer gateway-token".to_string()
            )]
        );
    }

    /// Only the header the provider authenticates keys with hands over. A
    /// credential-shaped header a provider does not read is an ordinary header,
    /// and suppressing the built-in credential on account of it strands the
    /// request with no credential at all.
    #[test]
    fn unrelated_credential_header_keeps_the_built_in_one() {
        let mut credentials = credentials(AIProvider::CustomAI, "https://gateway.example/openai");
        credentials.custom_headers =
            HashMap::from([("x-api-key".to_string(), "routing-key".to_string())]);
        let method = Method::POST;

        let request = build_openai_compatible_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "chat/completions",
            headers: &HeaderMap::new(),
            body: br#"{"model":"model","messages":[]}"#,
            credentials: &credentials,
        })
        .unwrap();

        assert!(request
            .headers
            .contains(&("authorization".to_string(), "Bearer api-key".to_string())));
        assert!(request
            .headers
            .contains(&("x-api-key".to_string(), "routing-key".to_string())));
    }

    /// The OAuth guard must not reject the workaround its error names, and must not be
    /// satisfied by a credential header the provider does not read.
    #[test]
    fn oauth_guard_follows_the_provider_credential_header() {
        let token_url = Some("https://login.example/token");
        let mut credentials = credentials(AIProvider::OpenAI, "https://api.openai.com/v1");
        credentials.api_key = None;
        let bearer = [("Authorization", String::new())];

        assert!(needs_unavailable_oauth_exchange(
            &credentials,
            token_url,
            &bearer
        ));

        // A routing header this provider never authenticates with leaves the request
        // unauthenticated, so it must not satisfy the guard.
        credentials.custom_headers =
            HashMap::from([("x-api-key".to_string(), "routing".to_string())]);
        assert!(needs_unavailable_oauth_exchange(
            &credentials,
            token_url,
            &bearer
        ));

        // The header the provider does read stands in for the token.
        credentials.custom_headers =
            HashMap::from([("Authorization".to_string(), "Bearer static".to_string())]);
        assert!(!needs_unavailable_oauth_exchange(
            &credentials,
            token_url,
            &bearer
        ));
        // An api-key resource never needed the exchange in the first place.
        assert!(!needs_unavailable_oauth_exchange(
            &credentials,
            None,
            &bearer
        ));
    }

    /// Azure reads api keys from `api-key` but Entra ID tokens only from
    /// `authorization`, so an OAuth resource must stay on the bearer header
    /// whatever the provider's key header is.
    #[test]
    fn oauth_token_is_sent_as_a_bearer_on_azure() {
        let mut credentials = credentials(
            AIProvider::AzureOpenAI,
            "https://example.openai.azure.com/openai",
        );
        credentials.api_key = None;
        credentials.access_token = Some("oauth-token".to_string());
        let method = Method::POST;

        let request = build_openai_compatible_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "chat/completions",
            headers: &HeaderMap::new(),
            body: br#"{"model":"deployment","messages":[]}"#,
            credentials: &credentials,
        })
        .unwrap();

        assert!(request.headers.contains(&(
            "authorization".to_string(),
            "Bearer oauth-token".to_string()
        )));
        assert!(!request
            .headers
            .iter()
            .any(|(header_name, _)| header_name.eq_ignore_ascii_case("api-key")));
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
