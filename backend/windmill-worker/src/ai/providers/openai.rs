use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::{download_and_encode_s3_image, prepare_messages_for_api},
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    sse::{OpenAISSEParser, SSEParser},
    types::*,
    utils::should_use_structured_output_tool,
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAIFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAIToolCall {
    pub id: String,
    pub function: OpenAIFunction,
    pub r#type: String,
}

#[derive(Deserialize)]
pub struct OpenAIChoice {
    pub message: OpenAIMessage,
}

#[derive(Deserialize)]
pub struct OpenAIResponse {
    pub choices: Vec<OpenAIChoice>,
}

#[derive(Serialize)]
pub struct ImageGenerationTool {
    pub r#type: String,
    pub quality: Option<String>,
    pub background: Option<String>,
}

// Input content for image generation - supports both text and images
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageGenerationContent {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_image")]
    InputImage { image_url: String },
}

#[derive(Serialize)]
pub struct ImageGenerationMessage {
    pub role: String,
    pub content: Vec<ImageGenerationContent>,
}

#[derive(Serialize)]
pub struct ImageGenerationRequest<'a> {
    pub model: &'a str,
    pub input: Vec<ImageGenerationMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<&'a str>,
    pub tools: Vec<ImageGenerationTool>,
}

#[derive(Deserialize)]
pub struct OpenAIImageResponse {
    pub output: Vec<OpenAIImageOutput>,
}

#[derive(Deserialize)]
pub struct OpenAIImageOutput {
    pub r#type: String, // Expected to be "image_generation_call"
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>, // Base64 encoded image, None if not completed
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    #[allow(dead_code)]
    Auto,
    Required,
}

#[derive(Serialize)]
pub struct OpenAIRequest<'a> {
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
    pub tool_choice: Option<ToolChoice>,
    pub stream: bool,
}

pub struct OpenAIQueryBuilder {
    provider_kind: AIProvider,
}

impl OpenAIQueryBuilder {
    pub fn new(provider_kind: AIProvider) -> Self {
        Self { provider_kind }
    }

    async fn build_text_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
        stream: bool,
    ) -> Result<String, Error> {
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

        let should_use_structured_output_tool =
            should_use_structured_output_tool(&self.provider_kind, args.model);
        // Force usage of structured output tool for Claude models when structured output provided
        let tool_choice = if should_use_structured_output_tool && response_format.is_some() {
            Some(ToolChoice::Required)
        } else {
            None
        };

        let request = OpenAIRequest {
            model: args.model,
            messages: &prepared_messages,
            tools: args.tools,
            temperature: args.temperature,
            max_completion_tokens: args.max_tokens,
            response_format,
            tool_choice,
            stream,
        };

        serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
    }

    async fn build_image_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // Build content array with text and optional image
        let mut content =
            vec![ImageGenerationContent::InputText { text: args.user_message.to_string() }];

        // Add images if provided
        if let Some(images) = args.images {
            for image in images.iter() {
                if !image.s3.is_empty() {
                    let (mime_type, image_bytes) =
                        download_and_encode_s3_image(image, client, workspace_id).await?;
                    content.push(ImageGenerationContent::InputImage {
                        image_url: format!("data:{};base64,{}", mime_type, image_bytes),
                    });
                }
            }
        }

        // Build the request with tools if provided
        let tools = vec![ImageGenerationTool {
            r#type: "image_generation".to_string(),
            quality: Some("low".to_string()),
            background: None,
        }];

        // TODO: OpenAI's image generation API doesn't support custom tools in the same way as chat completions
        // This would require a different approach, potentially using chat completions with image output
        // For now, we'll use the standard image generation without custom tools

        let image_request = ImageGenerationRequest {
            model: args.model,
            input: vec![ImageGenerationMessage { role: "user".to_string(), content }],
            instructions: args.system_prompt,
            tools,
        };

        serde_json::to_string(&image_request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
    }
}

#[async_trait]
impl QueryBuilder for OpenAIQueryBuilder {
    fn supports_tools_with_output_type(&self, _output_type: &OutputType) -> bool {
        // OpenAI supports tools for both text and image output
        true
    }

    fn supports_streaming(&self) -> bool {
        // OpenAI supports streaming for text output
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
                self.build_text_request(args, client, workspace_id, stream)
                    .await
            }
            OutputType::Image => self.build_image_request(args, client, workspace_id).await,
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // Check if this is an image response
        let url = response.url().path();
        if url.contains("/responses") {
            // Parse image generation response
            let response_text = response
                .text()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to read response text: {}", e)))?;

            let image_response: OpenAIImageResponse = serde_json::from_str(&response_text)
                .map_err(|e| {
                    Error::internal_err(format!(
                        "Failed to parse OpenAI image response: {}. Raw response: {}",
                        e, response_text
                    ))
                })?;

            // Find the first completed image generation output
            let image_generation_call = image_response
                .output
                .iter()
                .find(|output| {
                    output.r#type == "image_generation_call" && output.status == "completed"
                })
                .and_then(|output| output.result.as_ref());

            if let Some(base64_image) = image_generation_call {
                Ok(ParsedResponse::Image { base64_data: base64_image.clone() })
            } else {
                Err(Error::internal_err(
                    "No completed image output received from OpenAI".to_string(),
                ))
            }
        } else {
            // Parse text/chat completion response
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
                    OpenAIContent::Parts(parts) => {
                        // Extract text from parts
                        parts
                            .into_iter()
                            .filter_map(|part| match part {
                                ContentPart::Text { text } => Some(text),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                }),
                tool_calls: first_choice.message.tool_calls.unwrap_or_default(),
                events_str: None,
            })
        }
    }

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        let mut openai_sse_parser = OpenAISSEParser::new(stream_event_processor);
        openai_sse_parser.parse_events(response).await?;

        let OpenAISSEParser {
            accumulated_content,
            accumulated_tool_calls,
            mut events_str,
            stream_event_processor,
        } = openai_sse_parser;

        // Process streaming events with error handling

        for tool_call in accumulated_tool_calls.values() {
            let event = StreamingEvent::ToolCallArguments {
                call_id: tool_call.id.clone(),
                function_name: tool_call.function.name.clone(),
                arguments: tool_call.function.arguments.clone(),
            };
            stream_event_processor.send(event, &mut events_str).await?;
        }

        Ok(ParsedResponse::Text {
            content: if accumulated_content.is_empty() {
                None
            } else {
                Some(accumulated_content)
            },
            tool_calls: accumulated_tool_calls.into_values().collect(),
            events_str: Some(events_str),
        })
    }

    fn get_endpoint(
        &self,
        base_url: &str,
        _model: &str,
        output_type: &OutputType,
        _stream: bool,
    ) -> String {
        let path = match output_type {
            OutputType::Text => "chat/completions",
            OutputType::Image => "responses",
        };

        if self.provider_kind.is_azure_openai(base_url) {
            AIProvider::build_azure_openai_url(base_url, path)
        } else {
            format!("{}/{}", base_url, path)
        }
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        base_url: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        if self.provider_kind.is_azure_openai(base_url) {
            vec![("api-key", api_key.to_string())]
        } else {
            vec![("Authorization", format!("Bearer {}", api_key))]
        }
    }
}
