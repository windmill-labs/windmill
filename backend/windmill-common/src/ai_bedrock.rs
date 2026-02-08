//! Shared AWS Bedrock SDK code for AI chat proxy and worker.
//!
//! This module provides:
//! - BedrockClient: SDK wrapper with bearer token and IAM credentials auth
//! - Message/tool conversion: OpenAI format <-> Bedrock Converse API format
//! - Stream event parsing: Extract text/tool deltas from Bedrock stream events
//!
//! Used by both windmill-api (chat proxy) and windmill-worker (AI agent).

use aws_config::BehaviorVersion;
use aws_credential_types::provider::token::ProvideToken;
use aws_credential_types::provider::ProvideCredentials;
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, ConverseStreamOutput, ImageBlock, ImageFormat, ImageSource,
    InferenceConfiguration, Message, SystemContentBlock, Tool, ToolInputSchema, ToolSpecification,
};
use aws_sdk_bedrockruntime::Client as BedrockRuntimeClient;
use serde::{Deserialize, Serialize};

use crate::error::Error;

use crate::ai_types::{
    ContentPart, OpenAIContent, OpenAIFunction, OpenAIMessage, OpenAIToolCall, ToolDef,
};

// ============================================================================
// Cached AWS SDK Config
// ============================================================================

/// Cached AWS SDK config loaded from environment
/// Avoids repeated I/O for environment variable lookups and file reads
static AWS_SDK_CONFIG: tokio::sync::OnceCell<aws_config::SdkConfig> =
    tokio::sync::OnceCell::const_new();

/// Get or initialize the cached AWS SDK config
async fn get_aws_sdk_config() -> &'static aws_config::SdkConfig {
    AWS_SDK_CONFIG
        .get_or_init(|| async { aws_config::load_defaults(BehaviorVersion::latest()).await })
        .await
}

// ============================================================================
// Bedrock Client
// ============================================================================

/// Result of checking AWS Bedrock credentials availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockCredentialsCheck {
    pub available: bool,
    pub access_key_id_prefix: Option<String>,
    pub region: Option<String>,
    pub error: Option<String>,
}

/// Check if AWS credentials are available from the environment
pub async fn check_env_credentials() -> BedrockCredentialsCheck {
    let config = get_aws_sdk_config().await;

    if let Some(creds_provider) = config.credentials_provider() {
        match creds_provider.provide_credentials().await {
            Ok(creds) => {
                let access_key_id = creds.access_key_id();
                let prefix = if access_key_id.len() >= 8 {
                    format!("{}...", &access_key_id[..8])
                } else {
                    access_key_id.to_string()
                };

                BedrockCredentialsCheck {
                    available: true,
                    access_key_id_prefix: Some(prefix),
                    region: config.region().map(|r| r.to_string()),
                    error: None,
                }
            }
            Err(e) => BedrockCredentialsCheck {
                available: false,
                access_key_id_prefix: None,
                region: None,
                error: Some(format!("Failed to retrieve credentials: {}", e)),
            },
        }
    } else {
        BedrockCredentialsCheck {
            available: false,
            access_key_id_prefix: None,
            region: None,
            error: Some("No credentials provider configured".to_string()),
        }
    }
}

/// Constants for commonly used strings to avoid allocations
pub const FUNCTION_TYPE: &str = "function";

#[derive(Debug, Clone)]
pub struct BearerTokenProvider {
    token: String,
}

impl BearerTokenProvider {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

impl ProvideToken for BearerTokenProvider {
    fn provide_token<'a>(&'a self) -> aws_credential_types::provider::future::ProvideToken<'a>
    where
        Self: 'a,
    {
        aws_credential_types::provider::future::ProvideToken::ready(Ok(
            aws_credential_types::Token::new(self.token.clone(), None),
        ))
    }
}

pub struct BedrockClient {
    client: BedrockRuntimeClient,
}

impl BedrockClient {
    pub async fn from_bearer_token(bearer_token: String, region: &str) -> Result<Self, Error> {
        let config = aws_sdk_bedrockruntime::config::Builder::new()
            .region(aws_config::Region::new(region.to_string()))
            .behavior_version(BehaviorVersion::latest())
            .token_provider(BearerTokenProvider::new(bearer_token))
            .build();

        Ok(Self { client: BedrockRuntimeClient::from_conf(config) })
    }

    pub async fn from_credentials(
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
        region: &str,
    ) -> Result<Self, Error> {
        let credentials = aws_credential_types::Credentials::new(
            access_key_id,
            secret_access_key,
            session_token,
            None, // expiration
            "windmill",
        );

        let config = aws_sdk_bedrockruntime::config::Builder::new()
            .region(aws_config::Region::new(region.to_string()))
            .behavior_version(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .build();

        Ok(Self { client: BedrockRuntimeClient::from_conf(config) })
    }

    pub async fn from_env(region: &str) -> Result<Self, Error> {
        let config = get_aws_sdk_config().await;

        // Verify that credentials are actually available
        if let Some(creds_provider) = config.credentials_provider() {
            match creds_provider.provide_credentials().await {
                Ok(creds) => {
                    tracing::debug!(
                        "Bedrock: using env credentials, access_key={}...",
                        &creds.access_key_id().get(..8).unwrap_or("N/A"),
                    );
                }
                Err(e) => {
                    return Err(Error::internal_err(format!(
                        "AWS credentials not available from environment: {}",
                        e
                    )));
                }
            }
        } else {
            return Err(Error::internal_err(
                "No AWS credentials provider configured in environment".to_string(),
            ));
        }

        // Build client, only override region if explicitly provided
        let mut builder = aws_sdk_bedrockruntime::config::Builder::from(config);
        if !region.is_empty() {
            builder = builder.region(aws_config::Region::new(region.to_string()));
        }
        let bedrock_config = builder.build();

        let client = aws_sdk_bedrockruntime::Client::from_conf(bedrock_config);
        Ok(Self { client })
    }

    pub fn client(&self) -> &BedrockRuntimeClient {
        &self.client
    }
}

// ============================================================================
// Error Formatting
// ============================================================================

/// Format AWS SDK errors with detailed information
pub fn format_bedrock_error<E, R>(error: &aws_sdk_bedrockruntime::error::SdkError<E, R>) -> String
where
    E: std::fmt::Debug + std::fmt::Display,
    R: std::fmt::Debug,
{
    use aws_sdk_bedrockruntime::error::SdkError;

    match error {
        SdkError::ServiceError(err) => {
            format!("Service error: {} (details: {:?})", err.err(), err)
        }
        SdkError::ConstructionFailure(err) => {
            format!("Request construction failed: {:?}", err)
        }
        SdkError::DispatchFailure(err) => {
            format!("Request dispatch failed: {:?}", err)
        }
        SdkError::ResponseError(err) => {
            format!("Response error: {:?}", err)
        }
        SdkError::TimeoutError(err) => {
            format!("Request timeout: {:?}", err)
        }
        _ => format!("{:?}", error),
    }
}

// ============================================================================
// Type Conversion Utilities
// ============================================================================

/// Convert serde_json::Value to AWS Smithy Document
pub fn json_to_document(value: serde_json::Value) -> aws_smithy_types::Document {
    use aws_smithy_types::Document;
    use serde_json::Value;

    match value {
        Value::Object(map) => {
            let mut doc_map = std::collections::HashMap::new();
            for (k, v) in map {
                doc_map.insert(k, json_to_document(v));
            }
            Document::Object(doc_map)
        }
        Value::Array(arr) => Document::Array(arr.into_iter().map(json_to_document).collect()),
        Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                Document::Number(aws_smithy_types::Number::PosInt(i as u64))
            } else if let Some(f) = num.as_f64() {
                Document::Number(aws_smithy_types::Number::Float(f))
            } else {
                Document::Number(aws_smithy_types::Number::PosInt(0))
            }
        }
        Value::String(s) => Document::String(s),
        Value::Bool(b) => Document::Bool(b),
        Value::Null => Document::Null,
    }
}

// ============================================================================
// Message Conversion (OpenAI -> Bedrock)
// ============================================================================

/// Convert OpenAI-style messages to Bedrock format
///
/// Separates system messages from conversation messages as required by Bedrock API.
///
/// Important: Bedrock requires messages to alternate between user and assistant roles.
/// When an assistant message has tool_use blocks, the next user message must contain
/// ALL corresponding tool_result blocks. This function groups consecutive tool messages
/// into a single user message.
///
/// # Returns
/// Tuple of (conversation_messages, system_prompts)
pub fn openai_messages_to_bedrock(
    messages: &[OpenAIMessage],
) -> Result<(Vec<Message>, Vec<SystemContentBlock>), Error> {
    let mut bedrock_messages = Vec::new();
    let mut system_prompts = Vec::new();
    let mut pending_tool_results: Vec<ContentBlock> = Vec::new();

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                // Extract system messages separately
                if let Some(content) = &msg.content {
                    let text = content_to_text(content);
                    if !text.is_empty() {
                        system_prompts.push(SystemContentBlock::Text(text));
                    }
                }
            }
            "user" | "assistant" => {
                // Before adding a user/assistant message, flush any pending tool results
                if !pending_tool_results.is_empty() {
                    let tool_result_message = Message::builder()
                        .role(ConversationRole::User)
                        .set_content(Some(pending_tool_results.drain(..).collect()))
                        .build()
                        .map_err(|e| {
                            Error::internal_err(format!(
                                "Failed to build tool results message: {}",
                                e
                            ))
                        })?;
                    bedrock_messages.push(tool_result_message);
                }
                bedrock_messages.push(convert_message(msg)?);
            }
            "tool" => {
                // Accumulate tool results - they will be flushed as a single message
                // when we encounter a non-tool message or at the end
                let tool_result = convert_tool_result_content(msg)?;
                pending_tool_results.push(tool_result);
            }
            _ => {
                return Err(Error::BadRequest(format!("Unsupported role: {}", msg.role)));
            }
        }
    }

    // Flush any remaining tool results at the end
    if !pending_tool_results.is_empty() {
        let tool_result_message = Message::builder()
            .role(ConversationRole::User)
            .set_content(Some(pending_tool_results))
            .build()
            .map_err(|e| {
                Error::internal_err(format!("Failed to build tool results message: {}", e))
            })?;
        bedrock_messages.push(tool_result_message);
    }

    Ok((bedrock_messages, system_prompts))
}

/// Helper to extract text from OpenAIContent (ignoring images)
///
/// This is public so it can be reused by the worker module.
pub fn content_to_text(content: &OpenAIContent) -> String {
    match content {
        OpenAIContent::Text(text) => text.to_string(),
        OpenAIContent::Parts(parts) => {
            // Extract only text parts and join them
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

    // Parse data:image/png;base64,<data>
    let base64_start = url
        .find("base64,")
        .ok_or_else(|| Error::internal_err("Invalid data URL format"))?;

    let base64_data = &url[base64_start + 7..];
    let mime_type = url
        .split(';')
        .next()
        .and_then(|s| s.strip_prefix("data:"))
        .unwrap_or("image/png");

    // Extract format from MIME type (e.g., "image/png" -> "png")
    let format_str = mime_type
        .rsplit_once('/')
        .map(|(_, format)| format)
        .unwrap_or("png");

    // Map to ImageFormat enum
    let format = match format_str {
        "png" => ImageFormat::Png,
        "jpeg" | "jpg" => ImageFormat::Jpeg,
        "gif" => ImageFormat::Gif,
        "webp" => ImageFormat::Webp,
        _ => ImageFormat::Png, // Default to PNG
    };

    // Decode base64
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
            // S3Objects should be converted to ImageUrl before calling this function
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

    // Handle content (text and/or images)
    if let Some(content) = &msg.content {
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

    // Handle tool calls (for assistant messages)
    if let Some(tool_calls) = &msg.tool_calls {
        for tc in tool_calls {
            content_blocks.push(convert_tool_call_to_content(tc)?);
        }
    }

    // Bedrock requires at least one content block
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

/// Convert tool result message to Bedrock ToolResult ContentBlock
///
/// Returns just the ContentBlock (not a full Message) so multiple tool results
/// can be combined into a single user message.
fn convert_tool_result_content(msg: &OpenAIMessage) -> Result<ContentBlock, Error> {
    let tool_call_id = msg
        .tool_call_id
        .as_ref()
        .ok_or_else(|| Error::internal_err("Tool message missing tool_call_id"))?;

    let content_str = msg
        .content
        .as_ref()
        .map(|c| content_to_text(c))
        .unwrap_or_default();

    // Try to parse as JSON, otherwise use text
    let tool_result_content =
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content_str) {
            if json_val.is_object() {
                vec![aws_sdk_bedrockruntime::types::ToolResultContentBlock::Json(
                    json_to_document(json_val),
                )]
            } else {
                // Wrap primitives and arrays in an object
                vec![aws_sdk_bedrockruntime::types::ToolResultContentBlock::Json(
                    json_to_document(serde_json::json!({"result": json_val})),
                )]
            }
        } else {
            vec![aws_sdk_bedrockruntime::types::ToolResultContentBlock::Text(
                content_str.to_string(),
            )]
        };

    Ok(ContentBlock::ToolResult(
        aws_sdk_bedrockruntime::types::ToolResultBlock::builder()
            .tool_use_id(tool_call_id)
            .set_content(Some(tool_result_content))
            .build()
            .map_err(|e| Error::internal_err(format!("Failed to build tool result: {}", e)))?,
    ))
}

// ============================================================================
// Tool Conversion (OpenAI -> Bedrock)
// ============================================================================

/// Convert OpenAI tool definitions to Bedrock format
pub fn openai_tools_to_bedrock(tools: &[ToolDef]) -> Result<Vec<Tool>, Error> {
    tools
        .iter()
        .map(|tool_def| {
            let spec = &tool_def.function;

            // Convert parameters (RawValue) to Document via serde_json::Value
            let param_value: serde_json::Value = serde_json::from_str(spec.parameters.get())
                .map_err(|e| Error::internal_err(format!("Invalid tool schema: {}", e)))?;
            let input_schema = ToolInputSchema::Json(json_to_document(param_value));

            let tool_spec = ToolSpecification::builder()
                .name(&spec.name)
                .set_description(spec.description.clone())
                .input_schema(input_schema)
                .build()
                .map_err(|e| Error::internal_err(format!("Failed to build tool spec: {}", e)))?;

            Ok(Tool::ToolSpec(tool_spec))
        })
        .collect()
}

// ============================================================================
// Inference Configuration
// ============================================================================

/// Create inference configuration from parameters
pub fn create_inference_config(
    temperature: Option<f32>,
    max_tokens: Option<i32>,
) -> Option<InferenceConfiguration> {
    if temperature.is_none() && max_tokens.is_none() {
        return None;
    }

    let mut builder = InferenceConfiguration::builder();

    if let Some(temp) = temperature {
        builder = builder.temperature(temp);
    }

    if let Some(max_tok) = max_tokens {
        builder = builder.max_tokens(max_tok);
    }

    Some(builder.build())
}

// ============================================================================
// Stream Event Parsing
// ============================================================================

/// Extract text delta from Bedrock stream event
pub fn bedrock_stream_event_to_text(event: &ConverseStreamOutput) -> Option<String> {
    match event {
        ConverseStreamOutput::ContentBlockDelta(delta) => delta
            .delta()
            .and_then(|d| d.as_text().ok())
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// Represents a streaming tool call being accumulated
#[derive(Debug, Clone)]
pub struct StreamingToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Extract tool use start event from stream
pub fn bedrock_stream_event_to_tool_start(
    event: &ConverseStreamOutput,
) -> Option<StreamingToolCall> {
    match event {
        ConverseStreamOutput::ContentBlockStart(start) => {
            if let Some(tool_use) = start.start().and_then(|s| s.as_tool_use().ok()) {
                Some(StreamingToolCall {
                    id: tool_use.tool_use_id().to_string(),
                    name: tool_use.name().to_string(),
                    arguments: String::new(),
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extract tool use input delta from stream
pub fn bedrock_stream_event_to_tool_delta(event: &ConverseStreamOutput) -> Option<String> {
    match event {
        ConverseStreamOutput::ContentBlockDelta(delta) => delta
            .delta()
            .and_then(|d| d.as_tool_use().ok())
            .map(|tool_use| tool_use.input().to_string()),
        _ => None,
    }
}

/// Check if stream event indicates content block stop
pub fn bedrock_stream_event_is_block_stop(event: &ConverseStreamOutput) -> bool {
    matches!(event, ConverseStreamOutput::ContentBlockStop(_))
}

/// Convert accumulated streaming tool calls to OpenAI format
pub fn streaming_tool_calls_to_openai(tool_calls: Vec<StreamingToolCall>) -> Vec<OpenAIToolCall> {
    tool_calls
        .into_iter()
        .map(|tc| OpenAIToolCall {
            id: tc.id,
            function: OpenAIFunction { name: tc.name, arguments: tc.arguments },
            r#type: FUNCTION_TYPE.to_string(),
            extra_content: None, // Bedrock doesn't use thought signatures
        })
        .collect()
}

// ============================================================================
// Tool Configuration Builder
// ============================================================================

/// Build tool configuration with optional ToolChoice for structured output
pub fn build_tool_config(
    tools: Option<&[ToolDef]>,
    force_tool_use: bool,
) -> Result<Option<aws_sdk_bedrockruntime::types::ToolConfiguration>, Error> {
    if let Some(tools) = tools {
        let bedrock_tools = openai_tools_to_bedrock(tools)?;
        let mut tool_config_builder = aws_sdk_bedrockruntime::types::ToolConfiguration::builder()
            .set_tools(Some(bedrock_tools));

        // For structured output, force the model to use the tool
        if force_tool_use {
            tool_config_builder =
                tool_config_builder.tool_choice(aws_sdk_bedrockruntime::types::ToolChoice::Any(
                    aws_sdk_bedrockruntime::types::AnyToolChoice::builder().build(),
                ));
        }

        Ok(Some(tool_config_builder.build().map_err(|e| {
            Error::internal_err(format!("Failed to build tool configuration: {}", e))
        })?))
    } else {
        Ok(None)
    }
}
