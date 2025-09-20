use async_trait::async_trait;
use windmill_common::{client::AuthedClient, error::Error, s3_helpers::S3Object};

use crate::ai::{
    providers::{
        google_ai::GoogleAIQueryBuilder,
        openai::{OpenAIQueryBuilder, OpenAIToolCall},
        openrouter::OpenRouterQueryBuilder,
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
}

/// Response from AI provider
pub enum ParsedResponse {
    Text { content: Option<String>, tool_calls: Vec<OpenAIToolCall> },
    Image { base64_data: String },
}

/// Streaming response from AI provider
pub struct StreamingResponse {
    pub response: reqwest::Response,
}

/// Trait for building provider-specific AI requests
#[async_trait]
pub trait QueryBuilder: Send + Sync {
    /// Check if this provider supports tools with the given output type
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool;

    /// Check if this provider supports streaming
    fn supports_streaming(&self) -> bool {
        false // Default implementation returns false
    }

    /// Build the request body for the provider
    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error>;

    /// Build the request body for streaming
    async fn build_streaming_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // Default implementation falls back to regular request
        self.build_request(args, client, workspace_id).await
    }

    /// Parse the response from the provider
    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error>;

    /// Parse streaming response from the provider
    fn parse_streaming_response(&self, response: reqwest::Response) -> Result<StreamingResponse, Error> {
        // Default implementation just wraps the response
        Ok(StreamingResponse { response })
    }

    /// Get the API endpoint for this provider
    fn get_endpoint(&self, base_url: &str, model: &str, output_type: &OutputType) -> String;

    /// Get the authentication headers for this provider
    fn get_auth_headers(
        &self,
        api_key: &str,
        base_url: &str,
        output_type: &OutputType,
    ) -> Vec<(&'static str, String)>;
}

/// Factory function to create the appropriate query builder for a provider
pub fn create_query_builder(provider: &ProviderWithResource) -> Box<dyn QueryBuilder> {
    use windmill_common::ai_providers::AIProvider;

    match provider.kind {
        AIProvider::GoogleAI => Box::new(GoogleAIQueryBuilder::new()),
        AIProvider::OpenRouter => Box::new(OpenRouterQueryBuilder::new()),
        _ => Box::new(OpenAIQueryBuilder::new(provider.kind.clone())), // Pass provider kind for Azure handling
    }
}
