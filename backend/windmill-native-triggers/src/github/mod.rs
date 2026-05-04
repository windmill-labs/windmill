use serde::{Deserialize, Serialize};

pub mod external;
pub mod routes;

/// GitHub API base URL
pub const GITHUB_API_BASE: &str = "https://api.github.com";

/// Handler struct for GitHub triggers (stateless, used for routing)
#[derive(Copy, Clone)]
pub struct GitHub;

/// OAuth data for GitHub.
/// Assembled by decrypt_oauth_data() from the three-table pattern.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GithubOAuthData {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// User-provided configuration for a GitHub webhook trigger.
/// Stored as JSON in native_trigger.service_config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubServiceConfig {
    pub owner: String,
    pub repo: String,
    pub events: Vec<String>,
}

/// Data returned by the GitHub API when fetching a webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubTriggerData {
    /// Webhook ID on GitHub (same as external_id)
    #[serde(skip_serializing)]
    pub id: i64,
    pub active: bool,
    pub events: Vec<String>,
    /// Owner extracted from the webhook URL config
    pub owner: String,
    /// Repo extracted from the webhook URL config
    pub repo: String,
}

/// Response from GitHub when creating a webhook via POST /repos/{owner}/{repo}/hooks.
#[derive(Debug, Deserialize)]
pub struct CreateWebhookResponse {
    pub id: i64,
}

/// GitHub API webhook response (full shape for deserialization).
#[derive(Debug, Deserialize)]
pub struct GithubWebhookApiResponse {
    pub id: i64,
    pub active: bool,
    pub events: Vec<String>,
    pub config: GithubWebhookApiConfig,
}

#[derive(Debug, Deserialize)]
pub struct GithubWebhookApiConfig {
    pub url: Option<String>,
    pub content_type: Option<String>,
    pub insecure_ssl: Option<String>,
}

/// Simplified repo entry returned by the /repos additional route.
#[derive(Debug, Serialize, Deserialize)]
pub struct GithubRepoEntry {
    pub full_name: String,
    pub name: String,
    pub owner: String,
    pub private: bool,
}

/// GitHub API repo response shape.
#[derive(Debug, Deserialize)]
pub struct GithubApiRepoResponse {
    pub full_name: String,
    pub name: String,
    pub owner: GithubApiOwner,
    pub private: bool,
}

#[derive(Debug, Deserialize)]
pub struct GithubApiOwner {
    pub login: String,
}
