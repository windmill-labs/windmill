use async_trait::async_trait;
use windmill_common::{client::AuthedClient, error::Error, s3_helpers::S3Object};

use crate::ai::types::*;

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
    pub image: Option<&'a S3Object>,
    pub api_key: &'a str,
    pub base_url: &'a str,
}

/// Response from AI provider
pub enum ParsedResponse {
    Text {
        content: Option<String>,
        tool_calls: Vec<OpenAIToolCall>,
    },
    Image {
        base64_data: String,
    },
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
    fn get_endpoint(&self, base_url: &str, output_type: &OutputType) -> String;
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
        let openai_response: OpenAIResponse = response.json().await
            .map_err(|e| Error::internal_err(format!("Failed to parse response: {}", e)))?;
        
        let first_choice = openai_response.choices.into_iter().next()
            .ok_or_else(|| Error::internal_err("No response from API"))?;
        
        Ok(ParsedResponse::Text {
            content: first_choice.message.content.map(|c| match c {
                OpenAIContent::Text(text) => text,
                OpenAIContent::Parts(_) => String::new(), // For simplicity in default impl
            }),
            tool_calls: first_choice.message.tool_calls.unwrap_or_default(),
        })
    }
    
    fn get_endpoint(&self, base_url: &str, _output_type: &OutputType) -> String {
        format!("{}/chat/completions", base_url)
    }
}

// Import the actual OpenAI implementation
use crate::ai::providers::openai::OpenAIQueryBuilder;

pub struct AnthropicQueryBuilder;
impl AnthropicQueryBuilder {
    pub fn new() -> Self {
        Self
    }
}

// For now, use default implementation
#[async_trait]
impl QueryBuilder for AnthropicQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        matches!(output_type, OutputType::Text)
    }
    
    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // For now, delegate to default implementation
        DefaultQueryBuilder.build_request(args, client, workspace_id).await
    }
    
    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // For now, delegate to default implementation
        DefaultQueryBuilder.parse_response(response).await
    }
    
    fn get_endpoint(&self, base_url: &str, _output_type: &OutputType) -> String {
        format!("{}/messages", base_url)
    }
}

pub struct GoogleAIQueryBuilder;
impl GoogleAIQueryBuilder {
    pub fn new() -> Self {
        Self
    }
}

// For now, use default implementation
#[async_trait]
impl QueryBuilder for GoogleAIQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        matches!(output_type, OutputType::Text)
    }
    
    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // For now, delegate to default implementation
        DefaultQueryBuilder.build_request(args, client, workspace_id).await
    }
    
    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // For now, delegate to default implementation
        DefaultQueryBuilder.parse_response(response).await
    }
    
    fn get_endpoint(&self, base_url: &str, _output_type: &OutputType) -> String {
        format!("{}/chat/completions", base_url)
    }
}

pub struct OpenRouterQueryBuilder;
impl OpenRouterQueryBuilder {
    pub fn new() -> Self {
        Self
    }
}

// For now, use default implementation
#[async_trait]
impl QueryBuilder for OpenRouterQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        // OpenRouter supports tools for both text and image output
        true
    }
    
    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // For now, delegate to default implementation
        DefaultQueryBuilder.build_request(args, client, workspace_id).await
    }
    
    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // For now, delegate to default implementation
        DefaultQueryBuilder.parse_response(response).await
    }
    
    fn get_endpoint(&self, base_url: &str, _output_type: &OutputType) -> String {
        format!("{}/chat/completions", base_url)
    }
}