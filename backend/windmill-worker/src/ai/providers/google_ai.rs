use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use windmill_common::{client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::download_and_encode_s3_image,
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    sse::{GeminiSSEParser, SSEParser},
    types::*,
    utils::parse_data_url,
};

// ============================================================================
// Gemini API Types - Shared between text and image
// ============================================================================

/// Inline data for binary content (images)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeminiInlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

/// A part of content - can be text, inline data, function call, or function response
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum GeminiPart {
    Text {
        text: String,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: GeminiInlineData,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: GeminiFunctionCall,
        /// Thought signature for Gemini 3+ models - required for function calling
        #[serde(rename = "thoughtSignature", skip_serializing_if = "Option::is_none")]
        thought_signature: Option<String>,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: GeminiFunctionResponse,
    },
}

/// A function call from the model
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeminiFunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// A function response to send back to the model
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeminiFunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

// ============================================================================
// Gemini Text API Request Types
// ============================================================================

/// Main request structure for Gemini generateContent
#[derive(Serialize)]
pub struct GeminiTextRequest {
    pub contents: Vec<GeminiContentMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GeminiTool>>,
    #[serde(rename = "toolConfig", skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<GeminiToolConfig>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiContentMessage>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

/// Content message with role and parts
#[derive(Serialize)]
pub struct GeminiContentMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<GeminiPart>,
}

/// Tool definition - either function declarations or Google Search
#[derive(Serialize)]
pub struct GeminiTool {
    #[serde(
        rename = "functionDeclarations",
        skip_serializing_if = "Option::is_none"
    )]
    pub function_declarations: Option<Vec<GeminiFunctionDeclaration>>,
    #[serde(rename = "googleSearch", skip_serializing_if = "Option::is_none")]
    pub google_search: Option<serde_json::Value>,
}

/// Function declaration for tool use
#[derive(Serialize)]
pub struct GeminiFunctionDeclaration {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: OpenAPISchema,
}

/// Tool configuration for controlling function calling behavior
#[derive(Serialize)]
pub struct GeminiToolConfig {
    #[serde(rename = "functionCallingConfig")]
    pub function_calling_config: GeminiFunctionCallingConfig,
}

/// Function calling configuration
#[derive(Serialize)]
pub struct GeminiFunctionCallingConfig {
    pub mode: String,
    #[serde(
        rename = "allowedFunctionNames",
        skip_serializing_if = "Option::is_none"
    )]
    pub allowed_function_names: Option<Vec<String>>,
}

/// Generation configuration for output format
#[derive(Serialize)]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(rename = "responseMimeType", skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    #[serde(rename = "responseSchema", skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
}

// ============================================================================
// Gemini API Response Types
// ============================================================================

/// Grounding metadata from Google Search
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct GeminiGroundingMetadata {
    #[serde(rename = "webSearchQueries")]
    pub web_search_queries: Option<Vec<String>>,
    #[serde(rename = "groundingChunks")]
    pub grounding_chunks: Option<Vec<serde_json::Value>>,
}

// ============================================================================
// Gemini Image API Types (for Imagen models)
// ============================================================================

/// Request for image generation (Imagen models)
#[derive(Serialize)]
pub struct GeminiImageRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<GeminiImageContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instances: Option<Vec<GeminiPredictContent>>,
}

/// Content for image generation
#[derive(Serialize)]
pub struct GeminiImageContent {
    pub parts: Vec<GeminiPart>,
}

/// Content for Imagen predict endpoint
#[derive(Serialize)]
pub struct GeminiPredictContent {
    pub prompt: String,
}

/// Response for image generation
#[derive(Deserialize)]
pub struct GeminiImageResponse {
    pub candidates: Option<Vec<GeminiImageCandidate>>,
    pub predictions: Option<Vec<GeminiPredictCandidate>>,
}

/// Image candidate from generateContent
#[derive(Deserialize)]
pub struct GeminiImageCandidate {
    pub content: GeminiImageCandidateContent,
}

/// Content in image candidate
#[derive(Deserialize)]
pub struct GeminiImageCandidateContent {
    pub parts: Vec<GeminiImageCandidatePart>,
}

/// Part of image candidate
#[derive(Deserialize)]
pub struct GeminiImageCandidatePart {
    #[serde(rename = "inlineData", skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<GeminiInlineData>,
}

/// Prediction candidate from Imagen
#[derive(Deserialize)]
pub struct GeminiPredictCandidate {
    #[serde(rename = "bytesBase64Encoded")]
    pub bytes_base64_encoded: String,
}

// ============================================================================
// Query Builder Implementation
// ============================================================================

pub struct GoogleAIQueryBuilder;

impl GoogleAIQueryBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Build a text request using the native Gemini API format
    async fn build_text_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // Convert messages to Gemini format
        let contents = self
            .convert_messages_to_gemini(args.messages, client, workspace_id)
            .await?;

        // Build tools array
        let tools = self.convert_tools_to_gemini(args.tools, args.has_websearch);

        // Build generation config
        let generation_config = self.build_generation_config(args);

        // Build system instruction from system_prompt
        let system_instruction = args.system_prompt.map(|s| GeminiContentMessage {
            role: None,
            parts: vec![GeminiPart::Text { text: s.to_string() }],
        });

        let request = GeminiTextRequest {
            contents,
            tools,
            tool_config: None, // Use AUTO mode by default
            system_instruction,
            generation_config,
        };

        serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
    }

    /// Build an image generation request
    async fn build_image_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        let is_imagen = args.model.contains("imagen");

        let request = if is_imagen {
            // For Imagen models, use simple prompt format
            GeminiImageRequest {
                instances: Some(vec![GeminiPredictContent {
                    prompt: args.user_message.trim().to_string(),
                }]),
                contents: None,
            }
        } else {
            // For Gemini models with image generation, build parts
            let mut parts = vec![GeminiPart::Text { text: args.user_message.trim().to_string() }];

            if let Some(system_prompt) = args.system_prompt {
                parts.insert(
                    0,
                    GeminiPart::Text { text: format!("SYSTEM PROMPT: {}", system_prompt.trim()) },
                );
            }

            // Add input images if provided
            if let Some(images) = args.images {
                for image in images.iter() {
                    if !image.s3.is_empty() {
                        let (mime_type, image_bytes) =
                            download_and_encode_s3_image(image, client, workspace_id).await?;
                        parts.push(GeminiPart::InlineData {
                            inline_data: GeminiInlineData { mime_type, data: image_bytes },
                        });
                    }
                }
            }

            GeminiImageRequest {
                instances: None,
                contents: Some(vec![GeminiImageContent { parts }]),
            }
        };

        serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
    }

    /// Convert OpenAI-format messages to Gemini format
    async fn convert_messages_to_gemini(
        &self,
        messages: &[OpenAIMessage],
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<Vec<GeminiContentMessage>, Error> {
        let mut gemini_messages = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    // Skip - handled via args.system_prompt in build_text_request
                }
                "tool" => {
                    // Handle tool responses
                    if let (Some(tool_call_id), Some(content)) = (&msg.tool_call_id, &msg.content) {
                        let func_name = self.find_function_name_by_id(messages, tool_call_id);
                        let response_text = match content {
                            OpenAIContent::Text(text) => text.clone(),
                            OpenAIContent::Parts(parts) => parts
                                .iter()
                                .filter_map(|p| match p {
                                    ContentPart::Text { text } => Some(text.clone()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join(" "),
                        };

                        gemini_messages.push(GeminiContentMessage {
                            role: Some("user".to_string()),
                            parts: vec![GeminiPart::FunctionResponse {
                                function_response: GeminiFunctionResponse {
                                    name: func_name,
                                    response: serde_json::json!({ "result": response_text }),
                                },
                            }],
                        });
                    }
                }
                _ => {
                    // Handle user/assistant messages
                    let role = match msg.role.as_str() {
                        "assistant" => "model",
                        _ => "user",
                    };

                    let mut parts = Vec::new();

                    // Handle regular content
                    if let Some(content) = &msg.content {
                        let content_parts = self
                            .convert_content_to_parts(&Some(content.clone()), client, workspace_id)
                            .await?;
                        parts.extend(content_parts);
                    }

                    // Handle tool calls from assistant
                    if let Some(tool_calls) = &msg.tool_calls {
                        for tc in tool_calls {
                            let args: serde_json::Value =
                                serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                            // Extract thought_signature from extra_content if present
                            let thought_signature = tc
                                .extra_content
                                .as_ref()
                                .and_then(|ec| ec.google.as_ref())
                                .and_then(|g| g.thought_signature.clone());
                            parts.push(GeminiPart::FunctionCall {
                                function_call: GeminiFunctionCall {
                                    name: tc.function.name.clone(),
                                    args,
                                },
                                thought_signature,
                            });
                        }
                    }

                    if !parts.is_empty() {
                        gemini_messages
                            .push(GeminiContentMessage { role: Some(role.to_string()), parts });
                    }
                }
            }
        }

        Ok(gemini_messages)
    }

    /// Convert OpenAI content to Gemini parts
    async fn convert_content_to_parts(
        &self,
        content: &Option<OpenAIContent>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<Vec<GeminiPart>, Error> {
        let mut parts = Vec::new();

        if let Some(content) = content {
            match content {
                OpenAIContent::Text(text) => {
                    if !text.is_empty() {
                        parts.push(GeminiPart::Text { text: text.clone() });
                    }
                }
                OpenAIContent::Parts(content_parts) => {
                    for part in content_parts {
                        match part {
                            ContentPart::Text { text } => {
                                if !text.is_empty() {
                                    parts.push(GeminiPart::Text { text: text.clone() });
                                }
                            }
                            ContentPart::ImageUrl { image_url } => {
                                // Parse data URL format: data:mime_type;base64,data
                                if let Some((mime_type, data)) = parse_data_url(&image_url.url) {
                                    parts.push(GeminiPart::InlineData {
                                        inline_data: GeminiInlineData { mime_type, data },
                                    });
                                }
                            }
                            ContentPart::S3Object { s3_object } => {
                                if !s3_object.s3.is_empty() {
                                    let (mime_type, data) = download_and_encode_s3_image(
                                        s3_object,
                                        client,
                                        workspace_id,
                                    )
                                    .await?;
                                    parts.push(GeminiPart::InlineData {
                                        inline_data: GeminiInlineData { mime_type, data },
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(parts)
    }

    /// Find function name by tool call ID from previous messages
    fn find_function_name_by_id(&self, messages: &[OpenAIMessage], tool_call_id: &str) -> String {
        for msg in messages {
            if let Some(tool_calls) = &msg.tool_calls {
                for tc in tool_calls {
                    if tc.id == tool_call_id {
                        return tc.function.name.clone();
                    }
                }
            }
        }
        "unknown_function".to_string()
    }

    /// Convert OpenAI tools to Gemini format
    fn convert_tools_to_gemini(
        &self,
        tools: Option<&[ToolDef]>,
        has_websearch: bool,
    ) -> Option<Vec<GeminiTool>> {
        let mut gemini_tools = Vec::new();

        // Add function declarations
        if let Some(tool_defs) = tools {
            let declarations: Vec<GeminiFunctionDeclaration> = tool_defs
                .iter()
                .filter_map(|t| {
                    // Deserialize RawValue into OpenAPISchema, sanitize, then use
                    let mut schema: OpenAPISchema =
                        serde_json::from_str(t.function.parameters.get()).ok()?;
                    schema.sanitize_for_google();

                    Some(GeminiFunctionDeclaration {
                        name: t.function.name.clone(),
                        description: t.function.description.clone(),
                        parameters: schema,
                    })
                })
                .collect();

            if !declarations.is_empty() {
                gemini_tools.push(GeminiTool {
                    function_declarations: Some(declarations),
                    google_search: None,
                });
            }
        }

        // Add Google Search tool if enabled
        if has_websearch {
            gemini_tools.push(GeminiTool {
                function_declarations: None,
                google_search: Some(serde_json::json!({})),
            });
        }

        if gemini_tools.is_empty() {
            None
        } else {
            Some(gemini_tools)
        }
    }

    /// Build generation config for structured output and other settings
    fn build_generation_config(
        &self,
        args: &BuildRequestArgs<'_>,
    ) -> Option<GeminiGenerationConfig> {
        let has_output_schema = args
            .output_schema
            .and_then(|s| s.properties.as_ref())
            .map(|p| !p.is_empty())
            .unwrap_or(false);

        let (response_mime_type, response_schema) = if has_output_schema {
            let mut schema = args.output_schema.unwrap().clone();
            schema.sanitize_for_google();
            (
                Some("application/json".to_string()),
                serde_json::to_value(&schema).ok(),
            )
        } else {
            (None, None)
        };

        // Only create config if there's something to configure
        if args.temperature.is_some() || args.max_tokens.is_some() || response_mime_type.is_some() {
            Some(GeminiGenerationConfig {
                temperature: args.temperature,
                max_output_tokens: args.max_tokens,
                response_mime_type,
                response_schema,
            })
        } else {
            None
        }
    }
}

#[async_trait]
impl QueryBuilder for GoogleAIQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        // Google AI supports tools only for text output
        matches!(output_type, OutputType::Text)
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
        let gemini_response: GeminiImageResponse = response.json().await.map_err(|e| {
            Error::internal_err(format!("Failed to parse Gemini image response: {}", e))
        })?;

        // First, check Gemini models (candidates -> content -> parts -> inline_data)
        let image_data_from_gemini = gemini_response.candidates.as_ref().and_then(|candidates| {
            candidates.iter().find_map(|candidate| {
                candidate
                    .content
                    .parts
                    .iter()
                    .find_map(|part| part.inline_data.as_ref().map(|data| &data.data))
            })
        });

        // Then, check Imagen models (predictions -> bytes_base64_encoded)
        let image_data_from_imagen = gemini_response
            .predictions
            .as_ref()
            .and_then(|predictions| predictions.first().map(|p| &p.bytes_base64_encoded));

        // Image data, preferring Gemini first then Imagen models
        let image_data = image_data_from_gemini.or(image_data_from_imagen);

        match image_data {
            Some(base64_data) if !base64_data.is_empty() => {
                Ok(ParsedResponse::Image { base64_data: base64_data.clone() })
            }
            _ => Err(Error::internal_err(
                "No image data received from Google Gemini/Imagen API".to_string(),
            )),
        }
    }

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        let mut gemini_sse_parser = GeminiSSEParser::new(stream_event_processor);
        gemini_sse_parser.parse_events(response).await?;

        let GeminiSSEParser {
            accumulated_content,
            accumulated_tool_calls,
            mut events_str,
            stream_event_processor,
            annotations,
            used_websearch,
            usage: gemini_usage,
            ..
        } = gemini_sse_parser;

        // Send tool call arguments events for accumulated tool calls
        for tool_call in accumulated_tool_calls.values() {
            let event = StreamingEvent::ToolCallArguments {
                call_id: tool_call.id.clone(),
                function_name: tool_call.function.name.clone(),
                arguments: tool_call.function.arguments.clone(),
            };
            stream_event_processor.send(event, &mut events_str).await?;
        }

        // Convert Gemini usage metadata to TokenUsage
        let usage = gemini_usage.map(|u| {
            TokenUsage::new(
                u.prompt_token_count,
                u.candidates_token_count,
                u.total_token_count,
            )
        });

        Ok(ParsedResponse::Text {
            content: if accumulated_content.is_empty() {
                None
            } else {
                Some(accumulated_content)
            },
            tool_calls: accumulated_tool_calls.into_values().collect(),
            events_str: Some(events_str),
            annotations,
            used_websearch,
            usage,
        })
    }

    fn get_endpoint(&self, base_url: &str, model: &str, output_type: &OutputType) -> String {
        match output_type {
            OutputType::Text => {
                format!(
                    "{}/models/{}:streamGenerateContent?alt=sse",
                    base_url, model
                )
            }
            OutputType::Image => {
                let url_suffix = if model.contains("imagen") {
                    "predict"
                } else {
                    "generateContent"
                };
                format!("{}/models/{}:{}", base_url, model, url_suffix)
            }
        }
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        _base_url: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        // Native Gemini API always uses x-goog-api-key
        vec![("x-goog-api-key", api_key.to_string())]
    }
}
