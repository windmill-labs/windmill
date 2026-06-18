use std::collections::HashMap;

use crate::ai_providers::{AIPlatform, AIProvider};

/// Resolved provider credentials shared by API proxy and worker execution.
///
/// Raw API resources and worker agent payloads convert into this shape at their
/// execution boundaries. Request-specific state such as the selected model stays
/// outside this type.
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
    pub custom_headers: HashMap<String, String>,
}
