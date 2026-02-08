use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::{download_and_encode_s3_image, prepare_messages_for_api},
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    sse::{OpenAIResponsesSSEParser, SSEParser},
    types::*,
    utils::extract_text_content,
};

use windmill_common::ai_types::OpenAIToolCall;

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

/// URL citation annotation from OpenAI API response (includes type field for deserialization)
#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct OpenAIUrlCitation {
    pub r#type: String,
    pub start_index: usize,
    pub end_index: usize,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl From<OpenAIUrlCitation> for UrlCitation {
    fn from(c: OpenAIUrlCitation) -> Self {
        UrlCitation {
            start_index: c.start_index,
            end_index: c.end_index,
            url: c.url,
            title: c.title,
        }
    }
}

/// Output text content item (used in "message" type outputs)
#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct OutputTextContent {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default)]
    pub annotations: Vec<OpenAIUrlCitation>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
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

// Input content for image generation and user/system messages - supports both text and images
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageGenerationContent {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_image")]
    InputImage { image_url: String },
}

/// Output content for assistant messages in Responses API
/// OpenAI requires assistant content to use "output_text" type, not "input_text"
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantContent {
    #[serde(rename = "output_text")]
    OutputText { text: String },
}

/// Input items for OpenAI Responses API - supports messages, function calls, and function outputs
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponsesApiInputItem {
    /// User/system message with input content
    #[serde(rename = "message")]
    InputMessage { role: String, content: Vec<ImageGenerationContent> },
    /// Assistant message with output content (uses output_text instead of input_text)
    #[serde(rename = "message")]
    OutputMessage { role: String, content: Vec<AssistantContent> },
    /// Function call from model (must echo back from previous response)
    FunctionCall { call_id: String, name: String, arguments: String },
    /// Tool result output linked by call_id
    FunctionCallOutput { call_id: String, output: String },
}

#[derive(Serialize)]
#[allow(dead_code)]
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

/// Text format configuration for structured output in Responses API
#[derive(Serialize)]
pub struct ResponsesApiTextFormatConfig {
    pub r#type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    pub schema: serde_json::Value,
}

/// Text configuration for Responses API (used for structured output)
#[derive(Serialize)]
pub struct ResponsesApiTextFormat {
    pub format: ResponsesApiTextFormatConfig,
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
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<ResponsesApiTextFormat>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum ToolChoice {
    Auto,
    Required,
}

pub struct OpenAIQueryBuilder {
    #[allow(dead_code)]
    provider_kind: AIProvider,
}

/// Convert OpenAIContent to ImageGenerationContent array
fn convert_content_to_responses_format(
    content: &Option<OpenAIContent>,
) -> Vec<ImageGenerationContent> {
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
                        Some(ImageGenerationContent::InputImage {
                            image_url: image_url.url.clone(),
                        })
                    }
                    // S3 objects should have been resolved earlier, but handle gracefully
                    ContentPart::S3Object { .. } => None,
                })
                .collect()
        }
        None => Vec::new(),
    }
}

/// Extract text content as a string from Option<OpenAIContent>
fn extract_text_content_opt(content: &Option<OpenAIContent>) -> String {
    match content {
        Some(c) => extract_text_content(c),
        None => String::new(),
    }
}

/// Convert OpenAIContent to AssistantContent array (uses output_text for assistant messages)
fn convert_content_to_assistant_format(content: &Option<OpenAIContent>) -> Vec<AssistantContent> {
    match content {
        Some(OpenAIContent::Text(text)) => {
            vec![AssistantContent::OutputText { text: text.clone() }]
        }
        Some(OpenAIContent::Parts(parts)) => parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => {
                    Some(AssistantContent::OutputText { text: text.clone() })
                }
                // Images not supported in assistant output
                _ => None,
            })
            .collect(),
        None => Vec::new(),
    }
}

/// Convert OpenAIMessage array to Responses API input items
/// Following the same pattern as frontend openai-responses.ts:convertMessagesToResponsesInput
fn convert_messages_to_responses_input(messages: &[OpenAIMessage]) -> Vec<ResponsesApiInputItem> {
    let mut input = Vec::new();

    for m in messages {
        match m.role.as_str() {
            "system" | "user" => {
                // User/system messages use input_text content type
                let content = convert_content_to_responses_format(&m.content);
                if !content.is_empty() {
                    input.push(ResponsesApiInputItem::InputMessage {
                        role: m.role.clone(),
                        content,
                    });
                }
            }
            "assistant" => {
                // Assistant messages use output_text content type
                if m.content.is_some() {
                    let content = convert_content_to_assistant_format(&m.content);
                    if !content.is_empty() {
                        input.push(ResponsesApiInputItem::OutputMessage {
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
                    let output = extract_text_content_opt(&m.content);
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
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // First prepare messages (handles S3 object to base64 conversion)
        let prepared_messages =
            prepare_messages_for_api(args.messages, client, workspace_id).await?;

        // Convert full message history to Responses API input format
        // (following frontend pattern from openai-responses.ts)
        let input_items = convert_messages_to_responses_input(&prepared_messages);

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
                tools.push(ResponsesApiTool::Function(ResponsesApiFunctionTool::from(
                    tool,
                )));
            }
        }

        // Build text format for structured output if output_schema is provided
        let text = args
            .output_schema
            .and_then(|schema| schema.properties.as_ref())
            .filter(|props| !props.is_empty())
            .map(|_| {
                let mut strict_schema = args.output_schema.unwrap().clone();
                strict_schema.make_strict();
                ResponsesApiTextFormat {
                    format: ResponsesApiTextFormatConfig {
                        r#type: "json_schema".to_string(),
                        name: "structured_output".to_string(),
                        strict: Some(true),
                        schema: serde_json::to_value(&strict_schema).unwrap_or_default(),
                    },
                }
            });

        let request = ResponsesApiRequest {
            model: args.model,
            input: input_items,
            instructions: args.system_prompt, // System prompt goes to instructions field
            tools,
            stream: Some(true),
            temperature: args.temperature,
            max_output_tokens: args.max_tokens,
            text,
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

        // Build the request with image generation tool
        let tools: Vec<ResponsesApiTool> =
            vec![ResponsesApiTool::BuiltIn(ResponsesApiBuiltInTool {
                r#type: "image_generation".to_string(),
                quality: Some("low".to_string()),
            })];

        let request = ResponsesApiRequest {
            model: args.model,
            input: vec![ResponsesApiInputItem::InputMessage { role: "user".to_string(), content }],
            instructions: args.system_prompt,
            tools,
            stream: None, // Image generation doesn't use streaming
            temperature: args.temperature,
            max_output_tokens: args.max_tokens,
            text: None, // No structured output for image generation
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

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        let mut parser = OpenAIResponsesSSEParser::new(stream_event_processor);
        parser.parse_events(response).await?;

        // Convert OpenAI Responses usage to TokenUsage
        let usage = parser
            .usage
            .map(|u| TokenUsage::new(u.input_tokens, u.output_tokens, u.total_tokens));

        Ok(ParsedResponse::Text {
            content: if parser.accumulated_content.is_empty() {
                None
            } else {
                Some(parser.accumulated_content)
            },
            tool_calls: parser.accumulated_tool_calls.into_values().collect(),
            events_str: Some(parser.events_str),
            annotations: parser.annotations,
            used_websearch: parser.used_websearch,
            usage,
        })
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        match args.output_type {
            OutputType::Text => self.build_text_request(args, client, workspace_id).await,
            OutputType::Image => self.build_image_request(args, client, workspace_id).await,
        }
    }

    async fn parse_image_response(
        &self,
        response: reqwest::Response,
    ) -> Result<ParsedResponse, Error> {
        let responses_response: ResponsesApiResponse = response.json().await.map_err(|e| {
            Error::internal_err(format!(
                "Failed to parse OpenAI responses API response: {}",
                e
            ))
        })?;

        for output in responses_response.output.iter() {
            match output.r#type.as_str() {
                "image_generation_call" => {
                    if output.status.as_deref() == Some("completed") {
                        if let Some(ref base64_image) = output.result {
                            return Ok(ParsedResponse::Image { base64_data: base64_image.clone() });
                        }
                    }
                }
                _ => {
                    continue;
                }
            }
        }

        Err(Error::internal_err(
            "No completed output received from OpenAI Responses API".to_string(),
        ))
    }

    fn get_endpoint(&self, base_url: &str, _model: &str, _output_type: &OutputType) -> String {
        format!("{}/responses", base_url)
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
