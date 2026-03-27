use async_trait::async_trait;
use windmill_common::{client::AuthedClient, error::Error};
use windmill_types::s3::S3Object;

use crate::ai_types::OpenAIToolCall;
use crate::types::*;

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
    pub attachments: Option<&'a [S3Object]>,
    pub has_websearch: bool,
}

/// Response from AI provider
pub enum ParsedResponse {
    Text {
        content: Option<String>,
        tool_calls: Vec<OpenAIToolCall>,
        events_str: Option<String>,
        annotations: Vec<UrlCitation>,
        used_websearch: bool,
        usage: Option<TokenUsage>,
    },
    Image {
        base64_data: String,
    },
}

/// Trait for streaming AI events to a sink (e.g., database persistence).
/// Implemented by the worker's StreamEventProcessor.
#[async_trait]
pub trait StreamEventSink: Send + Sync {
    async fn send(&self, event: StreamingEvent, events_str: &mut String) -> Result<(), Error>;
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

    /// Build the request body without usage tracking (for retry on incompatible providers)
    /// Default implementation just calls build_request (most providers don't need this)
    async fn build_request_without_usage(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        self.build_request(args, client, workspace_id).await
    }

    /// Whether this provider supports retry without usage tracking
    /// Only OtherQueryBuilder (OpenAI-compatible providers) needs this
    fn supports_retry_without_usage(&self) -> bool {
        false
    }

    /// Parse the image response from the provider
    async fn parse_image_response(
        &self,
        response: reqwest::Response,
    ) -> Result<ParsedResponse, Error>;

    /// Parse streaming response from the provider
    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_sink: Box<dyn StreamEventSink>,
    ) -> Result<ParsedResponse, Error>;

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
