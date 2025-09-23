use async_trait::async_trait;
use windmill_common::{
    client::AuthedClient, error::Error, s3_helpers::S3Object, worker::Connection,
};
use windmill_queue::MiniPulledJob;

use crate::{
    ai::{
        providers::{
            google_ai::GoogleAIQueryBuilder,
            openai::{OpenAIQueryBuilder, OpenAIToolCall},
            openrouter::OpenRouterQueryBuilder,
        },
        types::*,
    },
    job_logger::append_result_stream,
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
    Text { content: Option<String>, tool_calls: Vec<OpenAIToolCall>, events_str: Option<String> },
    Image { base64_data: String },
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
        _args: &BuildRequestArgs<'_>,
        _client: &AuthedClient,
        _workspace_id: &str,
    ) -> Result<String, Error> {
        return Err(Error::internal_err(
            "Streaming is not supported for this provider".to_string(),
        ));
    }

    /// Parse the response from the provider
    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error>;

    /// Parse streaming response from the provider
    async fn parse_streaming_response(
        &self,
        _response: reqwest::Response,
        _stream_event_channel: StreamEventChannel,
    ) -> Result<ParsedResponse, Error> {
        return Err(Error::internal_err(
            "Streaming is not supported for this provider".to_string(),
        ));
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

#[derive(Clone)]
pub struct StreamEventChannel {
    tx: tokio::sync::mpsc::Sender<String>,
}

impl StreamEventChannel {
    pub fn new(conn: &Connection, job: &MiniPulledJob) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);
        let conn = conn.clone();
        let job_id = job.id.clone();
        let workspace_id = job.workspace_id.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(err) = append_result_stream(&conn, &workspace_id, &job_id, &event).await
                {
                    tracing::error!("Failed to send event: {}", err);
                }
            }
        });

        Self { tx }
    }

    pub async fn send(&self, event: StreamingEvent, events_str: &mut String) -> Result<(), Error> {
        match serde_json::to_string(&event) {
            Ok(event_json) => {
                let event_json = format!("{}\n", event_json);
                events_str.push_str(&event_json);
                self.tx
                    .send(event_json.clone())
                    .await
                    .map_err(|e| Error::internal_err(format!("Failed to send event: {}", e)))?;
                Ok(())
            }
            Err(e) => Err(Error::internal_err(format!(
                "Failed to serialize streaming event {:#?}, error is: {}",
                event, e
            ))),
        }
    }
}
