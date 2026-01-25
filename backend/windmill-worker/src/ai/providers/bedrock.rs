//! AWS Bedrock provider for the AI agent.
//!
//! Uses shared SDK code from windmill_common::ai_bedrock for:
//! - BedrockClient (SDK wrapper with auth)
//! - Stream event parsing
//! - Helper utilities
//!
//! Keeps conversion functions local since they depend on worker-specific types.

use crate::ai::{
    image_handler::prepare_messages_for_api,
    providers::openai::{OpenAIFunction, OpenAIToolCall},
    query_builder::{ParsedResponse, StreamEventProcessor},
    types::StreamingEvent,
    types::{ContentPart, OpenAIContent, OpenAIMessage, ToolDef},
};
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, ImageBlock, ImageFormat, ImageSource, Message,
    SystemContentBlock,
};
use std::collections::HashMap;
use windmill_common::{client::AuthedClient, error::Error};

// Re-export from shared module for use by other parts of the worker
pub use windmill_common::ai_bedrock::{check_env_credentials, BedrockClient};
use windmill_common::ai_bedrock::{
    bedrock_stream_event_is_block_stop, bedrock_stream_event_to_text,
    bedrock_stream_event_to_tool_delta, bedrock_stream_event_to_tool_start,
    create_inference_config, format_bedrock_error, json_to_document,
    openai_tools_to_bedrock, StreamingToolCall,
};

/// Constants for commonly used strings to avoid allocations
const FUNCTION_TYPE: &str = "function";

// ============================================================================
// Message Conversion (Worker-specific - uses worker's OpenAIMessage type)
// ============================================================================

/// Convert OpenAI-style messages to Bedrock format
///
/// Separates system messages from conversation messages as required by Bedrock API.
/// This is kept in the worker because it uses worker-specific types (OpenAIMessage with agent_action).
///
/// # Returns
/// Tuple of (conversation_messages, system_prompts)
pub fn openai_messages_to_bedrock(
    messages: &[OpenAIMessage],
) -> Result<(Vec<Message>, Vec<SystemContentBlock>), Error> {
    let mut bedrock_messages = Vec::new();
    let mut system_prompts = Vec::new();

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                if let Some(ref content) = msg.content {
                    let text = content_to_text(content);
                    if !text.is_empty() {
                        system_prompts.push(SystemContentBlock::Text(text));
                    }
                }
            }
            "user" | "assistant" => {
                bedrock_messages.push(convert_message(msg)?);
            }
            "tool" => {
                bedrock_messages.push(convert_tool_message(msg)?);
            }
            _ => {
                return Err(Error::BadRequest(format!("Unsupported role: {}", msg.role)));
            }
        }
    }

    Ok((bedrock_messages, system_prompts))
}

/// Helper to extract text from OpenAIContent (ignoring images)
fn content_to_text(content: &OpenAIContent) -> String {
    match content {
        OpenAIContent::Text(text) => text.to_string(),
        OpenAIContent::Parts(parts) => {
            let text_parts: Vec<&str> = parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            text_parts.join(" ")
        }
    }
}

/// Parse image data URL and extract format and base64 data
fn parse_image_data_url(url: &str) -> Result<(ImageFormat, Vec<u8>), Error> {
    if !url.starts_with("data:") {
        return Err(Error::internal_err("Image URL must be a data URL"));
    }

    let base64_start = url
        .find("base64,")
        .ok_or_else(|| Error::internal_err("Invalid data URL format"))?;

    let base64_data = &url[base64_start + 7..];
    let mime_type = url
        .split(';')
        .next()
        .and_then(|s| s.strip_prefix("data:"))
        .unwrap_or("image/png");

    let format_str = mime_type
        .rsplit_once('/')
        .map(|(_, format)| format)
        .unwrap_or("png");

    let format = match format_str {
        "png" => ImageFormat::Png,
        "jpeg" | "jpg" => ImageFormat::Jpeg,
        "gif" => ImageFormat::Gif,
        "webp" => ImageFormat::Webp,
        _ => ImageFormat::Png,
    };

    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data)
        .map_err(|e| Error::internal_err(format!("Failed to decode base64 image: {}", e)))?;

    Ok((format, bytes))
}

/// Convert a ContentPart to Bedrock ContentBlock
fn content_part_to_block(part: &ContentPart) -> Result<Option<ContentBlock>, Error> {
    match part {
        ContentPart::Text { text } => {
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(ContentBlock::Text(text.clone())))
            }
        }
        ContentPart::ImageUrl { image_url } => {
            let (format, bytes) = parse_image_data_url(&image_url.url)?;

            let image_source = ImageSource::Bytes(bytes.into());
            let image_block = ImageBlock::builder()
                .format(format)
                .source(image_source)
                .build()
                .map_err(|e| Error::internal_err(format!("Failed to build image block: {}", e)))?;

            Ok(Some(ContentBlock::Image(image_block)))
        }
        ContentPart::S3Object { .. } => {
            // S3Objects are already converted to ImageUrl by prepare_messages_for_api
            Ok(None)
        }
    }
}

/// Convert a single OpenAI message to Bedrock Message
fn convert_message(msg: &OpenAIMessage) -> Result<Message, Error> {
    let role = match msg.role.as_str() {
        "user" => ConversationRole::User,
        "assistant" => ConversationRole::Assistant,
        _ => {
            return Err(Error::internal_err(format!(
                "Unsupported role: {}",
                msg.role
            )));
        }
    };

    let mut content_blocks = Vec::new();

    if let Some(ref content) = msg.content {
        match content {
            OpenAIContent::Text(text) => {
                if !text.is_empty() {
                    content_blocks.push(ContentBlock::Text(text.clone()));
                }
            }
            OpenAIContent::Parts(parts) => {
                for part in parts {
                    if let Some(block) = content_part_to_block(part)? {
                        content_blocks.push(block);
                    }
                }
            }
        }
    }

    if let Some(ref tool_calls) = msg.tool_calls {
        for tc in tool_calls {
            content_blocks.push(convert_tool_call_to_content(tc)?);
        }
    }

    if content_blocks.is_empty() {
        content_blocks.push(ContentBlock::Text(String::new()));
    }

    Message::builder()
        .role(role)
        .set_content(Some(content_blocks))
        .build()
        .map_err(|e| Error::internal_err(format!("Failed to build message: {}", e)))
}

/// Convert OpenAI tool call to Bedrock ToolUse content block
fn convert_tool_call_to_content(tool_call: &OpenAIToolCall) -> Result<ContentBlock, Error> {
    let input = json_to_document(
        serde_json::from_str(&tool_call.function.arguments)
            .unwrap_or_else(|_| serde_json::json!({})),
    );
    Ok(ContentBlock::ToolUse(
        aws_sdk_bedrockruntime::types::ToolUseBlock::builder()
            .tool_use_id(&tool_call.id)
            .name(&tool_call.function.name)
            .input(input)
            .build()
            .map_err(|e| Error::internal_err(format!("Failed to build tool use: {}", e)))?,
    ))
}

/// Convert tool result message to Bedrock format
fn convert_tool_message(msg: &OpenAIMessage) -> Result<Message, Error> {
    let tool_call_id = msg
        .tool_call_id
        .as_ref()
        .ok_or_else(|| Error::internal_err("Tool message missing tool_call_id"))?;

    let content_str = msg
        .content
        .as_ref()
        .map(|c| content_to_text(c))
        .unwrap_or_default();

    let tool_result_content =
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content_str) {
            if json_val.is_object() {
                vec![aws_sdk_bedrockruntime::types::ToolResultContentBlock::Json(
                    json_to_document(json_val),
                )]
            } else {
                vec![aws_sdk_bedrockruntime::types::ToolResultContentBlock::Json(
                    json_to_document(serde_json::json!({"result": json_val})),
                )]
            }
        } else {
            vec![aws_sdk_bedrockruntime::types::ToolResultContentBlock::Text(
                content_str.to_string(),
            )]
        };

    let tool_result = ContentBlock::ToolResult(
        aws_sdk_bedrockruntime::types::ToolResultBlock::builder()
            .tool_use_id(tool_call_id)
            .set_content(Some(tool_result_content))
            .build()
            .map_err(|e| Error::internal_err(format!("Failed to build tool result: {}", e)))?,
    );

    Message::builder()
        .role(ConversationRole::User)
        .content(tool_result)
        .build()
        .map_err(|e| Error::internal_err(format!("Failed to build tool result message: {}", e)))
}

// ============================================================================
// Tool Call Conversion (Worker-specific types)
// ============================================================================

/// Convert accumulated streaming tool calls to OpenAI format (worker types)
fn streaming_tool_calls_to_openai(tool_calls: Vec<StreamingToolCall>) -> Vec<OpenAIToolCall> {
    tool_calls
        .into_iter()
        .map(|tc| OpenAIToolCall {
            id: tc.id,
            function: OpenAIFunction {
                name: tc.name,
                arguments: tc.arguments,
            },
            r#type: FUNCTION_TYPE.to_string(),
            extra_content: None,
        })
        .collect()
}

// ============================================================================
// Query Builder (Worker-specific orchestration)
// ============================================================================

#[derive(Default)]
pub struct BedrockQueryBuilder;

impl BedrockQueryBuilder {
    /// Execute Bedrock request (streaming or non-streaming)
    pub async fn execute_request(
        &self,
        messages: &[OpenAIMessage],
        tools: Option<&[ToolDef]>,
        model: &str,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        api_key: &str,
        region: &str,
        stream_event_processor: Option<StreamEventProcessor>,
        client: &AuthedClient,
        workspace_id: &str,
        structured_output_tool_name: Option<&str>,
        aws_access_key_id: Option<&str>,
        aws_secret_access_key: Option<&str>,
    ) -> Result<ParsedResponse, Error> {
        tracing::info!(
            "Worker Bedrock: model={}, region={}, messages={}, tools={}, structured_output={}",
            model,
            region,
            messages.len(),
            tools.map(|t| t.len()).unwrap_or(0),
            structured_output_tool_name.is_some()
        );

        // Create Bedrock client with priority: bearer token → IAM credentials → environment
        let bedrock_client = if !api_key.is_empty() {
            // 1. Bearer token if API key provided
            tracing::info!("Worker Bedrock: auth=bearer token");
            BedrockClient::from_bearer_token(api_key.to_string(), region).await?
        } else if let (Some(access_key_id), Some(secret_access_key)) = (
            aws_access_key_id.filter(|s| !s.is_empty()),
            aws_secret_access_key.filter(|s| !s.is_empty()),
        ) {
            // 2. IAM credentials if provided and not empty
            tracing::info!(
                "Worker Bedrock: auth=IAM credentials, access_key_id={}...",
                &access_key_id.get(..8).unwrap_or("N/A")
            );
            BedrockClient::from_credentials(
                access_key_id.to_string(),
                secret_access_key.to_string(),
                None, // session token - could be added to resource config in future
                region,
            )
            .await?
        } else {
            // 3. Environment credentials as fallback
            tracing::info!("Worker Bedrock: auth=environment credentials");
            BedrockClient::from_env(region).await?
        };

        // Prepare messages: convert S3Objects to ImageUrls by downloading from S3
        let prepared_messages = prepare_messages_for_api(messages, client, workspace_id).await?;

        // Convert messages to Bedrock format (separates system prompts)
        let (bedrock_messages, system_prompts) = openai_messages_to_bedrock(&prepared_messages)?;

        // Build inference configuration using shared helper
        let inference_config = create_inference_config(temperature, max_tokens.map(|t| t as i32));

        // Build tool configuration with optional ToolChoice
        let tool_config = self.build_tool_config(tools, structured_output_tool_name.is_some())?;

        self.execute_converse_stream(
            &bedrock_client,
            model,
            bedrock_messages,
            system_prompts,
            inference_config,
            tool_config,
            stream_event_processor,
        )
        .await
    }

    /// Build tool configuration with optional ToolChoice for structured output
    fn build_tool_config(
        &self,
        tools: Option<&[ToolDef]>,
        force_tool_use: bool,
    ) -> Result<Option<aws_sdk_bedrockruntime::types::ToolConfiguration>, Error> {
        if let Some(tools) = tools {
            let bedrock_tools = openai_tools_to_bedrock(tools)?;
            let mut tool_config_builder =
                aws_sdk_bedrockruntime::types::ToolConfiguration::builder()
                    .set_tools(Some(bedrock_tools));

            if force_tool_use {
                tool_config_builder = tool_config_builder.tool_choice(
                    aws_sdk_bedrockruntime::types::ToolChoice::Any(
                        aws_sdk_bedrockruntime::types::AnyToolChoice::builder().build(),
                    ),
                );
            }

            Ok(Some(tool_config_builder.build().map_err(|e| {
                Error::internal_err(format!("Failed to build tool configuration: {}", e))
            })?))
        } else {
            Ok(None)
        }
    }

    /// Execute streaming Bedrock request using shared stream parsing functions
    async fn execute_converse_stream(
        &self,
        bedrock_client: &BedrockClient,
        model: &str,
        bedrock_messages: Vec<aws_sdk_bedrockruntime::types::Message>,
        system_prompts: Vec<aws_sdk_bedrockruntime::types::SystemContentBlock>,
        inference_config: Option<aws_sdk_bedrockruntime::types::InferenceConfiguration>,
        tool_config: Option<aws_sdk_bedrockruntime::types::ToolConfiguration>,
        stream_event_processor: Option<StreamEventProcessor>,
    ) -> Result<ParsedResponse, Error> {
        tracing::debug!(
            "Worker Bedrock: executing converse_stream, messages={}, system_prompts={}, has_tools={}",
            bedrock_messages.len(),
            system_prompts.len(),
            tool_config.is_some()
        );

        let mut request_builder = bedrock_client
            .client()
            .converse_stream()
            .model_id(model)
            .set_messages(Some(bedrock_messages));

        if !system_prompts.is_empty() {
            request_builder = request_builder.set_system(Some(system_prompts));
        }

        if let Some(config) = inference_config {
            request_builder = request_builder.inference_config(config);
        }

        if let Some(config) = tool_config {
            request_builder = request_builder.set_tool_config(Some(config));
        }

        let mut stream = request_builder
            .send()
            .await
            .map_err(|e| {
                let error_msg =
                    format!("Bedrock streaming API error: {}", format_bedrock_error(&e));
                tracing::error!("Worker Bedrock: {}", error_msg);
                Error::internal_err(error_msg)
            })?
            .stream;

        tracing::debug!("Worker Bedrock: stream established, processing events");

        let mut accumulated_text = String::new();
        let mut events_str = String::new();
        let mut accumulated_tool_calls: HashMap<String, StreamingToolCall> = HashMap::new();
        let mut current_tool_use_id: Option<String> = None;

        // Process stream events using shared parsing functions
        loop {
            match stream.recv().await {
                Ok(Some(event)) => {
                    // Handle tool use start using shared parser
                    if let Some(tool_call) = bedrock_stream_event_to_tool_start(&event) {
                        current_tool_use_id = Some(tool_call.id.clone());
                        accumulated_tool_calls.insert(tool_call.id.clone(), tool_call);
                    }

                    // Handle text delta using shared parser
                    if let Some(text_delta) = bedrock_stream_event_to_text(&event) {
                        accumulated_text.push_str(&text_delta);
                        if let Some(processor) = stream_event_processor.as_ref() {
                            processor
                                .send(
                                    StreamingEvent::TokenDelta {
                                        content: text_delta,
                                    },
                                    &mut events_str,
                                )
                                .await?;
                        }
                    }

                    // Handle tool use input delta using shared parser
                    if let Some(input_delta) = bedrock_stream_event_to_tool_delta(&event) {
                        if let Some(tool_id) = &current_tool_use_id {
                            if let Some(tool_call) = accumulated_tool_calls.get_mut(tool_id) {
                                tool_call.arguments.push_str(&input_delta);
                            }
                        }
                    }

                    // Handle content block stop using shared parser
                    if bedrock_stream_event_is_block_stop(&event) {
                        current_tool_use_id = None;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    return Err(Error::internal_err(format!("Bedrock stream error: {}", e)));
                }
            }
        }

        // Send tool call events to stream processor
        if let Some(processor) = stream_event_processor.as_ref() {
            for tool_call in accumulated_tool_calls.values() {
                processor
                    .send(
                        StreamingEvent::ToolCallArguments {
                            call_id: tool_call.id.clone(),
                            function_name: tool_call.name.clone(),
                            arguments: tool_call.arguments.clone(),
                        },
                        &mut events_str,
                    )
                    .await?;
            }
        }

        let content = if accumulated_text.is_empty() {
            None
        } else {
            Some(accumulated_text)
        };

        let tool_calls =
            streaming_tool_calls_to_openai(accumulated_tool_calls.into_values().collect());

        tracing::info!(
            "Worker Bedrock: completed, text_len={}, tool_calls={}",
            content.as_ref().map(|s| s.len()).unwrap_or(0),
            tool_calls.len()
        );

        Ok(ParsedResponse::Text {
            content,
            tool_calls,
            events_str: if events_str.is_empty() {
                None
            } else {
                Some(events_str)
            },
            annotations: Vec::new(),
            used_websearch: false,
        })
    }
}
