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
    /// AWS IAM role to assume through Windmill's OIDC provider. Only consulted
    /// when no bearer key and no explicit IAM keys are set — see
    /// [`crate::ai_bedrock::bedrock_oidc_role_to_assume`]. The API proxy
    /// exchanges it for temporary keys before it builds the request; the worker
    /// exchanges it at the Bedrock call, because only the API can sign an OIDC
    /// token.
    ///
    /// The two paths therefore present different tokens to AWS: the proxy a
    /// workspace claim with `sub = "<email>::<workspace>"`, the worker the job
    /// claim `/oidc/token` issues, `sub = "<email>::<path>::<flow_path>::<workspace>"`.
    /// A role trust policy that conditions on `sub` has to admit both, or only
    /// one of the copilot and the AI agent step will be able to assume the role.
    pub oidc_role_arn: Option<String>,
    pub platform: AIPlatform,
    pub custom_headers: HashMap<String, String>,
}
