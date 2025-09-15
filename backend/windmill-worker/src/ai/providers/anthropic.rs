use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use windmill_common::{client::AuthedClient, error::Error};

use crate::ai::{
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder},
    types::*,
};

// Anthropic-specific types
#[derive(Serialize, Deserialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<AnthropicToolChoice>,
}

#[derive(Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: Vec<AnthropicContent>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: AnthropicImageSource },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String, content: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicImageSource {
    pub r#type: String, // "base64"
    pub media_type: String,
    pub data: String,
}

#[derive(Serialize, Deserialize)]
pub struct AnthropicTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicToolChoice {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "tool")]
    Tool { name: String },
}

#[derive(Deserialize)]
pub struct AnthropicResponse {
    pub content: Vec<AnthropicContent>,
}

pub struct AnthropicQueryBuilder;

impl AnthropicQueryBuilder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl QueryBuilder for AnthropicQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        // Anthropic only supports tools for text output
        matches!(output_type, OutputType::Text)
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        match args.output_type {
            OutputType::Text => {
                // For text output, use OpenAI-compatible format
                // This allows Anthropic to work with the standard OpenAI API format
                let openai_builder = super::openai::OpenAIQueryBuilder::new();
                openai_builder
                    .build_request(args, client, workspace_id)
                    .await
            }
            OutputType::Image => Err(Error::internal_err(
                "Anthropic does not support image generation".to_string(),
            )),
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // For text responses, parse as OpenAI format
        let openai_builder = super::openai::OpenAIQueryBuilder::new();
        openai_builder.parse_response(response).await
    }

    fn get_endpoint(&self, base_url: &str, _model: &str, output_type: &OutputType) -> String {
        match output_type {
            OutputType::Text => format!("{}/chat/completions", base_url), // Use OpenAI-compatible endpoint
            OutputType::Image => format!("{}/messages", base_url), // Not used, but keep for consistency
        }
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        vec![("Authorization", format!("Bearer {}", api_key))]
    }
}
