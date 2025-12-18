use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use tracing::info;
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::download_and_encode_s3_image,
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    sse::{OpenAIResponsesSSEParser, SSEParser},
    types::*,
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAIFunction {
    pub name: String,
    pub arguments: String,
}

/// Google-specific extra content for thought signatures (Gemini 3 Pro / 2.5)
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct GoogleExtraContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

/// Extra content for provider-specific metadata (e.g., Google thought signatures)
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct ExtraContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google: Option<GoogleExtraContent>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAIToolCall {
    pub id: String,
    pub function: OpenAIFunction,
    pub r#type: String,
    /// Extra content for provider-specific metadata (e.g., Google Gemini thought signatures)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_content: Option<ExtraContent>,
}

// Responses API structures
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ResponsesMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<OpenAIContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
}

/// URL citation annotation for web search results
#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct UrlCitation {
    pub r#type: String,
    pub start_index: usize,
    pub end_index: usize,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Output text content item (used in "message" type outputs)
#[derive(Deserialize, Clone, Debug)]
pub struct OutputTextContent {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default)]
    pub annotations: Vec<UrlCitation>,
}

#[derive(Deserialize)]
pub struct ResponsesOutput {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>, // Base64 encoded image for image_generation_call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<ResponsesMessage>, // Message for message_call
    // Fields for "message" type output (used with web search)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<OutputTextContent>>,
    // Fields for "function_call" type output (direct tool call from Responses API)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ResponsesApiResponse {
    pub output: Vec<ResponsesOutput>,
}

#[derive(Serialize)]
#[allow(dead_code)]
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

/// Input items for OpenAI Responses API - supports messages, function calls, and function outputs
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponsesApiInputItem {
    /// User/assistant/system message
    Message {
        role: String,
        content: Vec<ImageGenerationContent>,
    },
    /// Function call from model (must echo back from previous response)
    FunctionCall {
        call_id: String,
        name: String,
        arguments: String,
    },
    /// Tool result output linked by call_id
    FunctionCallOutput {
        call_id: String,
        output: String,
    },
}

#[derive(Serialize)]
pub struct ImageGenerationMessage {
    pub role: String,
    pub content: Vec<ImageGenerationContent>,
}

/// Tool definition for OpenAI Responses API (flat structure, not nested like Chat Completions)
#[derive(Serialize, Debug)]
pub struct ResponsesApiFunctionTool {
    pub r#type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: Box<RawValue>,
}

impl From<&ToolDef> for ResponsesApiFunctionTool {
    fn from(tool: &ToolDef) -> Self {
        ResponsesApiFunctionTool {
            r#type: tool.r#type.clone(),
            name: tool.function.name.clone(),
            description: tool.function.description.clone(),
            parameters: tool.function.parameters.clone(),
        }
    }
}

/// Built-in tool for OpenAI Responses API (web_search, image_generation, etc.)
#[derive(Serialize, Debug)]
pub struct ResponsesApiBuiltInTool {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
}

/// Tool types for OpenAI Responses API
#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum ResponsesApiTool {
    Function(ResponsesApiFunctionTool),
    BuiltIn(ResponsesApiBuiltInTool),
}

#[derive(Serialize)]
pub struct ResponsesApiRequest<'a> {
    pub model: &'a str,
    pub input: Vec<ResponsesApiInputItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<&'a str>,
    pub tools: Vec<ResponsesApiTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum ToolChoice {
    Auto,
    Required,
}

/// Query builder for OpenAI and Azure OpenAI using the Responses API
pub struct OpenAIQueryBuilder {
    provider_kind: AIProvider,
}

/// Convert OpenAIContent to ImageGenerationContent array
fn convert_content_to_responses_format(content: &Option<OpenAIContent>) -> Vec<ImageGenerationContent> {
    match content {
        Some(OpenAIContent::Text(text)) => {
            vec![ImageGenerationContent::InputText { text: text.clone() }]
        }
        Some(OpenAIContent::Parts(parts)) => {
            parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Text { text } => {
                        Some(ImageGenerationContent::InputText { text: text.clone() })
                    }
                    ContentPart::ImageUrl { image_url } => {
                        Some(ImageGenerationContent::InputImage { image_url: image_url.url.clone() })
                    }
                    // S3 objects should have been resolved earlier, but handle gracefully
                    ContentPart::S3Object { .. } => None,
                })
                .collect()
        }
        None => Vec::new(),
    }
}

/// Extract text content as a string from OpenAIContent
fn extract_text_content(content: &Option<OpenAIContent>) -> String {
    match content {
        Some(OpenAIContent::Text(text)) => text.clone(),
        Some(OpenAIContent::Parts(parts)) => {
            parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        None => String::new(),
    }
}

/// Convert OpenAIMessage array to Responses API input items
/// Following the same pattern as frontend openai-responses.ts:convertMessagesToResponsesInput
fn convert_messages_to_responses_input(messages: &[OpenAIMessage]) -> Vec<ResponsesApiInputItem> {
    let mut input = Vec::new();

    for m in messages {
        match m.role.as_str() {
            "system" | "user" => {
                // Regular message with content
                let content = convert_content_to_responses_format(&m.content);
                if !content.is_empty() {
                    input.push(ResponsesApiInputItem::Message {
                        role: m.role.clone(),
                        content,
                    });
                }
            }
            "assistant" => {
                // Assistant may have text content
                if m.content.is_some() {
                    let content = convert_content_to_responses_format(&m.content);
                    if !content.is_empty() {
                        input.push(ResponsesApiInputItem::Message {
                            role: "assistant".to_string(),
                            content,
                        });
                    }
                }
                // Echo back function calls so model knows what it previously requested
                if let Some(ref tool_calls) = m.tool_calls {
                    for tc in tool_calls {
                        // Skip tool calls with empty names
                        if tc.function.name.is_empty() {
                            continue;
                        }
                        input.push(ResponsesApiInputItem::FunctionCall {
                            call_id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        });
                    }
                }
            }
            "tool" => {
                // Tool result - linked by tool_call_id
                if let Some(ref call_id) = m.tool_call_id {
                    let output = extract_text_content(&m.content);
                    input.push(ResponsesApiInputItem::FunctionCallOutput {
                        call_id: call_id.clone(),
                        output,
                    });
                }
            }
            _ => {
                // Unknown role, skip
            }
        }
    }

    input
}

impl OpenAIQueryBuilder {
    pub fn new(provider_kind: AIProvider) -> Self {
        Self { provider_kind }
    }

    async fn build_text_request(
        &self,
        args: &BuildRequestArgs<'_>,
        _client: &AuthedClient,
        _workspace_id: &str,
        stream: bool,
    ) -> Result<String, Error> {
        // Convert full message history to Responses API input format
        // (following frontend pattern from openai-responses.ts)
        let input_items = convert_messages_to_responses_input(args.messages);

        // Build tools array using typed structs
        let mut tools: Vec<ResponsesApiTool> = Vec::new();

        // Add websearch tool if enabled
        if args.has_websearch {
            tools.push(ResponsesApiTool::BuiltIn(ResponsesApiBuiltInTool {
                r#type: "web_search".to_string(),
                quality: None,
            }));
        }

        // Convert ToolDef to ResponsesApiFunctionTool (flat structure for Responses API)
        if let Some(tool_defs) = args.tools {
            for tool in tool_defs {
                tools.push(ResponsesApiTool::Function(ResponsesApiFunctionTool::from(tool)));
            }
        }

        let request = ResponsesApiRequest {
            model: args.model,
            input: input_items,
            instructions: args.system_prompt, // System prompt goes to instructions field
            tools,
            stream: if stream { Some(true) } else { None },
            temperature: args.temperature,
            max_completion_tokens: args.max_tokens,
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))?;

        info!("[OPENAI_RESPONSES_API] Request: {}", request_json);

        Ok(request_json)
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

        // Build the request with image generation tool
        let tools: Vec<ResponsesApiTool> = vec![ResponsesApiTool::BuiltIn(ResponsesApiBuiltInTool {
            r#type: "image_generation".to_string(),
            quality: Some("low".to_string()),
        })];

        let request = ResponsesApiRequest {
            model: args.model,
            input: vec![ResponsesApiInputItem::Message {
                role: "user".to_string(),
                content,
            }],
            instructions: args.system_prompt,
            tools,
            stream: None, // Image generation doesn't use streaming
            temperature: args.temperature,
            max_completion_tokens: args.max_tokens,
        };

        serde_json::to_string(&request)
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
        true
    }

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        let mut parser = OpenAIResponsesSSEParser::new(stream_event_processor);
        parser.parse_events(response).await?;

        Ok(ParsedResponse::Text {
            content: if parser.accumulated_content.is_empty() {
                None
            } else {
                Some(parser.accumulated_content)
            },
            tool_calls: parser.accumulated_tool_calls.into_values().collect(),
            events_str: Some(parser.events_str),
        })
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
        // Both text and image use the same responses API format
        let response_text = response
            .text()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to read response text: {}", e)))?;

        info!("[OPENAI_RESPONSES_API] Raw response: {}", response_text);

        let responses_response: ResponsesApiResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                Error::internal_err(format!(
                    "Failed to parse OpenAI responses API response: {}. Raw response: {}",
                    e, response_text
                ))
            })?;

        info!(
            "[OPENAI_RESPONSES_API] Parsed {} outputs",
            responses_response.output.len()
        );

        // Collect function_call outputs (for parallel tool calls)
        let mut collected_tool_calls: Vec<OpenAIToolCall> = Vec::new();

        for (idx, output) in responses_response.output.iter().enumerate() {
            info!(
                "[OPENAI_RESPONSES_API] Output[{}]: type={}, status={:?}, has_message={}, has_content={}, has_result={}, has_name={}, has_call_id={}",
                idx,
                output.r#type,
                output.status,
                output.message.is_some(),
                output.content.is_some(),
                output.result.is_some(),
                output.name.is_some(),
                output.call_id.is_some()
            );

            match output.r#type.as_str() {
                "image_generation_call" => {
                    if output.status.as_deref() == Some("completed") {
                        if let Some(ref base64_image) = output.result {
                            info!("[OPENAI_RESPONSES_API] Returning image_generation_call result (len={})", base64_image.len());
                            return Ok(ParsedResponse::Image { base64_data: base64_image.clone() });
                        }
                    }
                    info!("[OPENAI_RESPONSES_API] Skipping image_generation_call: status={:?}, has_result={}", output.status, output.result.is_some());
                }
                // Handle "function_call" type output (direct tool call from Responses API)
                "function_call" => {
                    if output.status.as_deref() == Some("completed") {
                        if let (Some(name), Some(arguments), Some(call_id)) =
                            (&output.name, &output.arguments, &output.call_id)
                        {
                            info!(
                                "[OPENAI_RESPONSES_API] Collecting function_call: name={}, call_id={}",
                                name, call_id
                            );
                            collected_tool_calls.push(OpenAIToolCall {
                                id: call_id.clone(),
                                function: OpenAIFunction {
                                    name: name.clone(),
                                    arguments: arguments.clone(),
                                },
                                r#type: "function".to_string(),
                                extra_content: None,
                            });
                        } else {
                            info!(
                                "[OPENAI_RESPONSES_API] Skipping function_call: missing fields - has_name={}, has_args={}, has_call_id={}",
                                output.name.is_some(), output.arguments.is_some(), output.call_id.is_some()
                            );
                        }
                    } else {
                        info!(
                            "[OPENAI_RESPONSES_API] Skipping function_call: status={:?}",
                            output.status
                        );
                    }
                }
                "message_call" => {
                    if let Some(ref message) = output.message {
                        info!(
                            "[OPENAI_RESPONSES_API] Returning message_call: has_content={}, has_tool_calls={}",
                            message.content.is_some(),
                            message.tool_calls.is_some()
                        );
                        return Ok(ParsedResponse::Text {
                            content: message.content.clone().map(|c| match c {
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
                            tool_calls: message.tool_calls.clone().unwrap_or_default(),
                            events_str: None,
                        });
                    }
                    info!("[OPENAI_RESPONSES_API] Skipping message_call: no message field");
                }
                // Handle "message" type output (used with web search responses)
                "message" => {
                    if output.status.as_deref() == Some("completed") {
                        if let Some(ref content_items) = output.content {
                            info!(
                                "[OPENAI_RESPONSES_API] message output has {} content items",
                                content_items.len()
                            );
                            // Extract text from output_text content items
                            let text_content: String = content_items
                                .iter()
                                .filter(|item| item.r#type == "output_text")
                                .filter_map(|item| item.text.as_ref())
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(" ");

                            if !text_content.is_empty() {
                                info!("[OPENAI_RESPONSES_API] Returning message with text (len={})", text_content.len());
                                return Ok(ParsedResponse::Text {
                                    content: Some(text_content),
                                    tool_calls: Vec::new(),
                                    events_str: None,
                                });
                            }
                        }
                    }
                    info!("[OPENAI_RESPONSES_API] Skipping message: status={:?}, has_content={}", output.status, output.content.is_some());
                }
                // Skip web_search_call and other types
                other => {
                    info!("[OPENAI_RESPONSES_API] Skipping unknown output type: {}", other);
                    continue;
                }
            }
        }

        // Return collected function calls if any were found
        if !collected_tool_calls.is_empty() {
            info!(
                "[OPENAI_RESPONSES_API] Returning {} function_call(s)",
                collected_tool_calls.len()
            );
            return Ok(ParsedResponse::Text {
                content: None,
                tool_calls: collected_tool_calls,
                events_str: None,
            });
        }

        info!("[OPENAI_RESPONSES_API] No completed output found, returning error");
        Err(Error::internal_err(
            "No completed output received from OpenAI Responses API".to_string(),
        ))
    }

    fn get_endpoint(
        &self,
        base_url: &str,
        _model: &str,
        _output_type: &OutputType,
        _stream: bool,
    ) -> String {
        // Always use responses endpoint for OpenAI/Azure
        if self.provider_kind.is_azure_openai(base_url) {
            AIProvider::build_azure_openai_url(base_url, "responses")
        } else {
            format!("{}/responses", base_url)
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
