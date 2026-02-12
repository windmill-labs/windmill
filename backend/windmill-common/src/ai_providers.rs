/*
 * This file contains shared AI provider utilities used by both the API and worker.
 */

use crate::db::DB;
use crate::error::{Error, Result};
use serde::{Deserialize, Deserializer, Serialize};

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
}

pub const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
pub const GOOGLE_AI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Empty string signals BedrockClient::from_env() to use the region from AWS environment/config
/// (e.g., AWS_REGION or AWS_DEFAULT_REGION env vars, or ~/.aws/config)
pub const USE_ENV_REGION: &str = "";

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AIProvider {
    OpenAI,
    #[serde(rename = "azure_openai")]
    AzureOpenAI,
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

                Ok(azure_base_path.unwrap_or("https://api.openai.com/v1".to_string()))
            }
            AIProvider::DeepSeek => Ok("https://api.deepseek.com/v1".to_string()),
            AIProvider::GoogleAI => Ok(GOOGLE_AI_BASE_URL.to_string()),
            AIProvider::Groq => Ok("https://api.groq.com/openai/v1".to_string()),
            AIProvider::OpenRouter => Ok("https://openrouter.ai/api/v1".to_string()),
            AIProvider::TogetherAI => Ok("https://api.together.xyz/v1".to_string()),
            AIProvider::Anthropic => Ok("https://api.anthropic.com/v1".to_string()),
            AIProvider::Mistral => Ok("https://api.mistral.ai/v1".to_string()),
            p @ (AIProvider::CustomAI | AIProvider::AzureOpenAI) => Err(Error::BadRequest(
                format!("{:?} provider requires a base URL in the resource", p),
            )),
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

    /// Check if this provider/URL combination represents Azure OpenAI
    pub fn is_azure_openai(&self, base_url: &str) -> bool {
        (matches!(self, AIProvider::OpenAI) && base_url != OPENAI_BASE_URL)
            || matches!(self, AIProvider::AzureOpenAI)
    }

    /// Build Azure OpenAI URL with deployment model path
    pub fn build_azure_openai_url(base_url: &str, path: &str) -> String {
        let base_url = base_url.trim_end_matches('/');
        if base_url.ends_with("/openai") {
            format!("{}/v1/{}", base_url, path)
        } else if base_url.ends_with("/deployments") {
            format!("{}/v1/{}", base_url.trim_end_matches("/deployments"), path)
        } else {
            format!("{}/{}", base_url, path)
        }
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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProviderModel {
    pub model: String,
    pub provider: AIProvider,
}
