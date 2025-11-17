use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::prepare_messages_for_api,
    providers::openai::{OpenAIQueryBuilder, OpenAIResponse},
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
    // OpenRouter uses OpenAI-compatible API, so we delegate most work to OpenAI builder
    openai_builder: OpenAIQueryBuilder,
}

impl OpenRouterQueryBuilder {
    pub fn new() -> Self {
        Self { openai_builder: OpenAIQueryBuilder::new(AIProvider::OpenRouter) }
    }
}

#[async_trait]
impl QueryBuilder for OpenRouterQueryBuilder {
    fn supports_tools_with_output_type(&self, _output_type: &OutputType) -> bool {
        // OpenRouter supports tools for both text and image output (via OpenAI-compatible API)
        true
    }

    fn supports_streaming(&self) -> bool {
        // OpenRouter supports streaming for text output
        true
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
        stream: bool,
    ) -> Result<String, Error> {
        match args.output_type {
            OutputType::Text => {
                // For text, use standard OpenAI format without modalities
                self.openai_builder
                    .build_request(args, client, workspace_id, stream)
                    .await
            }
            OutputType::Image => {
                // For image generation, we need to add modalities field
                // First, prepare the messages
                let prepared_messages =
                    prepare_messages_for_api(args.messages, client, workspace_id).await?;

                // Check if we need to add response_format for structured output
                let has_output_properties = args
                    .output_schema
                    .and_then(|schema| schema.properties.as_ref())
                    .map(|props| !props.is_empty())
                    .unwrap_or(false);

                let response_format = if has_output_properties && args.output_schema.is_some() {
                    let schema = args.output_schema.unwrap();
                    let strict_schema = schema.clone().make_strict();
                    Some(ResponseFormat {
                        r#type: "json_schema".to_string(),
                        json_schema: JsonSchemaFormat {
                            name: "structured_output".to_string(),
                            schema: strict_schema,
                            strict: Some(true),
                        },
                    })
                } else {
                    None
                };

                // Build OpenRouter-specific request with modalities
                let request = OpenRouterChatRequest {
                    model: args.model,
                    messages: &prepared_messages,
                    tools: args.tools,
                    temperature: args.temperature,
                    max_completion_tokens: args.max_tokens,
                    response_format,
                    modalities: Some(vec!["image", "text"]),
                };

                serde_json::to_string(&request)
                    .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
            }
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        let response_text = response
            .text()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to read response text: {}", e)))?;

        // First try to parse as OpenRouter image response
        if let Ok(image_response) = serde_json::from_str::<OpenRouterImageResponse>(&response_text)
        {
            // Extract base64 image from the first choice
            let image_url = image_response
                .choices
                .get(0)
                .and_then(|choice| choice.message.images.as_ref())
                .and_then(|images| images.get(0))
                .map(|image| &image.image_url.url);

            if let Some(data_url) = image_url {
                // Extract base64 data from data URL format: data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA...
                if let Some(base64_start) = data_url.find("base64,") {
                    let base64_data = &data_url[base64_start + 7..]; // Skip "base64," prefix
                    return Ok(ParsedResponse::Image { base64_data: base64_data.to_string() });
                }
            }
        }

        // If not an image response or parsing failed, try as regular OpenAI response
        let openai_response: OpenAIResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                Error::internal_err(format!(
                    "Failed to parse response: {}. Raw response: {}",
                    e, response_text
                ))
            })?;

        let first_choice = openai_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal_err("No response from API"))?;

        Ok(ParsedResponse::Text {
            content: first_choice.message.content.map(|c| match c {
                OpenAIContent::Text(text) => text,
                OpenAIContent::Parts(parts) => parts
                    .into_iter()
                    .filter_map(|part| match part {
                        ContentPart::Text { text } => Some(text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            }),
            tool_calls: first_choice.message.tool_calls.unwrap_or_default(),
            events_str: None,
        })
    }

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        self.openai_builder
            .parse_streaming_response(response, stream_event_processor)
            .await
    }

    fn get_endpoint(
        &self,
        base_url: &str,
        _model: &str,
        _output_type: &OutputType,
        _stream: bool,
    ) -> String {
        // OpenRouter uses the same endpoint for both text and image generation
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
