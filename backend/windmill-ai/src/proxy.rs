use std::collections::HashMap;

use http::{HeaderMap, Method};

use crate::ai_providers::{AIPlatform, AIProvider};

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
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}
