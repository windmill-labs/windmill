use async_trait::async_trait;
use windmill_common::{client::AuthedClient, error::Error, s3_helpers::S3Object};

use crate::ai::{
    providers::{
        anthropic::AnthropicQueryBuilder, google_ai::GoogleAIQueryBuilder,
        openai::OpenAIQueryBuilder, openrouter::OpenRouterQueryBuilder,
    },
    types::*,
};

/// Arguments for building an AI request
pub struct BuildRequestArgs<'a> {
    pub messages: &'a [OpenAIMessage],
    pub tools: Option<&'a [ToolDef]>,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub output_schema: Option<&'a OpenAPISchema>,
    pub output_type: &'a OutputType,
    pub system_prompt: Option<&'a str>,
    pub user_message: &'a str,
    pub images: Option<&'a [S3Object]>,
    pub api_key: &'a str,
    pub base_url: &'a str,
}

/// Response from AI provider
pub enum ParsedResponse {
    Text { content: Option<String>, tool_calls: Vec<OpenAIToolCall> },
    Image { base64_data: String },
}

/// Trait for building provider-specific AI requests
#[async_trait]
pub trait QueryBuilder: Send + Sync {
    /// Check if this provider supports tools with the given output type
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool;

    /// Build the request body for the provider
    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error>;

    /// Parse the response from the provider
    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error>;

    /// Get the API endpoint for this provider
    fn get_endpoint(&self, base_url: &str, model: &str, output_type: &OutputType) -> String;

    /// Get the authentication headers for this provider
    fn get_auth_headers(
        &self,
        api_key: &str,
        output_type: &OutputType,
    ) -> Vec<(&'static str, String)>;
}

/// Factory function to create the appropriate query builder for a provider
pub fn create_query_builder(provider: &ProviderWithResource) -> Box<dyn QueryBuilder> {
    use windmill_common::ai_providers::AIProvider;

    match provider.kind {
        AIProvider::OpenAI => Box::new(OpenAIQueryBuilder::new()),
        AIProvider::Anthropic => Box::new(AnthropicQueryBuilder::new()),
        AIProvider::GoogleAI => Box::new(GoogleAIQueryBuilder::new()),
        AIProvider::OpenRouter => Box::new(OpenRouterQueryBuilder::new()),
        _ => Box::new(DefaultQueryBuilder::new()), // Fallback for other providers
    }
}

// Default implementation that follows OpenAI API format
pub struct DefaultQueryBuilder;

impl DefaultQueryBuilder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl QueryBuilder for DefaultQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        matches!(output_type, OutputType::Text)
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        _client: &AuthedClient,
        _workspace_id: &str,
    ) -> Result<String, Error> {
        let request = OpenAIRequest {
            model: args.model,
            messages: args.messages,
            tools: args.tools,
            temperature: args.temperature,
            max_completion_tokens: args.max_tokens,
            response_format: None,
        };

        serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to parse response: {}", e)))?;

        let first_choice = openai_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal_err("No response from API"))?;

        Ok(ParsedResponse::Text {
            content: first_choice.message.content.map(|c| match c {
                OpenAIContent::Text(text) => text,
                OpenAIContent::Parts(_) => String::new(), // For simplicity in default impl
            }),
            tool_calls: first_choice.message.tool_calls.unwrap_or_default(),
        })
    }

    fn get_endpoint(&self, base_url: &str, _model: &str, _output_type: &OutputType) -> String {
        format!("{}/chat/completions", base_url)
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        vec![("Authorization", format!("Bearer {}", api_key))]
    }
}
