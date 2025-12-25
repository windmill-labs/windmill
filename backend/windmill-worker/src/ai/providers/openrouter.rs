use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::prepare_messages_for_api,
    providers::other::OtherQueryBuilder,
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    types::*,
};

// OpenRouter-specific types
#[derive(Serialize)]
pub struct OpenRouterChatRequest<'a> {
    pub model: &'a str,
    pub messages: &'a [OpenAIMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<&'a [ToolDef]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<&'a str>>,
}

#[derive(Deserialize)]
pub struct OpenRouterImageResponse {
    pub choices: Vec<OpenRouterImageChoice>,
}

#[derive(Deserialize)]
pub struct OpenRouterImageChoice {
    pub message: OpenRouterImageResponseMessage,
}

#[derive(Deserialize)]
pub struct OpenRouterImageResponseMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<OpenRouterImageData>>,
}

#[derive(Deserialize)]
pub struct OpenRouterImageData {
    pub image_url: OpenRouterImageUrl,
}

#[derive(Deserialize)]
pub struct OpenRouterImageUrl {
    pub url: String, // data:image/png;base64,... format
}

pub struct OpenRouterQueryBuilder {
    // OpenRouter uses OpenAI-compatible completion API, so we delegate most work to Other builder
    other_builder: OtherQueryBuilder,
}

impl OpenRouterQueryBuilder {
    pub fn new() -> Self {
        Self { other_builder: OtherQueryBuilder::new(AIProvider::OpenRouter) }
    }
}

#[async_trait]
impl QueryBuilder for OpenRouterQueryBuilder {
    fn supports_tools_with_output_type(&self, _output_type: &OutputType) -> bool {
        // OpenRouter supports tools for both text and image output (via OpenAI-compatible API)
        true
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        match args.output_type {
            OutputType::Text => {
                self.other_builder
                    .build_request(args, client, workspace_id)
                    .await
            }
            OutputType::Image => {
                let prepared_messages =
                    prepare_messages_for_api(args.messages, client, workspace_id).await?;
                let request = OpenRouterChatRequest {
                    model: args.model,
                    messages: &prepared_messages,
                    tools: args.tools,
                    temperature: args.temperature,
                    max_completion_tokens: args.max_tokens,
                    response_format: None,
                    modalities: Some(vec!["image", "text"]),
                };

                serde_json::to_string(&request)
                    .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
            }
        }
    }

    async fn parse_image_response(
        &self,
        response: reqwest::Response,
    ) -> Result<ParsedResponse, Error> {
        let image_response: OpenRouterImageResponse = response.json().await.map_err(|e| {
            Error::internal_err(format!("Failed to parse OpenRouter image response: {}", e))
        })?;

        if let Some(image) = image_response
            .choices
            .first()
            .and_then(|choice| choice.message.images.as_ref())
            .and_then(|images| images.first())
        {
            if let Some(base64_data) = image.image_url.url.strip_prefix("data:image/png;base64,") {
                return Ok(ParsedResponse::Image { base64_data: base64_data.to_string() });
            }
        }

        Err(Error::internal_err(
            "No image data received from OpenRouter API".to_string(),
        ))
    }

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        self.other_builder
            .parse_streaming_response(response, stream_event_processor)
            .await
    }

    fn get_endpoint(&self, base_url: &str, _model: &str, _output_type: &OutputType) -> String {
        format!("{}/chat/completions", base_url)
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        _base_url: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        vec![("Authorization", format!("Bearer {}", api_key))]
    }
}
