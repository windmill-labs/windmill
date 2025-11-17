use crate::ai::{
    image_handler::prepare_messages_for_api,
    providers::openai::{OpenAIFunction, OpenAIToolCall},
    query_builder::{ParsedResponse, StreamEventProcessor},
    types::StreamingEvent,
    types::{ContentPart, OpenAIContent, OpenAIMessage, ToolDef},
};
use aws_config::BehaviorVersion;
use aws_credential_types::provider::token::ProvideToken;
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, ConverseStreamOutput, ImageBlock, ImageFormat, ImageSource,
    InferenceConfiguration, Message, SystemContentBlock, Tool, ToolInputSchema, ToolSpecification,
};
use aws_sdk_bedrockruntime::Client as BedrockRuntimeClient;
use std::collections::HashMap;
use windmill_common::{client::AuthedClient, error::Error};

/// Constants for commonly used strings to avoid allocations
const FUNCTION_TYPE: &str = "function";
const EMPTY_JSON: &str = "{}";

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

    pub fn client(&self) -> &BedrockRuntimeClient {
        &self.client
    }
}

/// Format AWS SDK errors with detailed information
fn format_bedrock_error<E, R>(error: &aws_sdk_bedrockruntime::error::SdkError<E, R>) -> String
where
    E: std::fmt::Debug + std::fmt::Display,
    R: std::fmt::Debug,
{
    use aws_sdk_bedrockruntime::error::SdkError;

    match error {
        SdkError::ServiceError(err) => {
            // Include both the display and debug representations for maximum detail
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

/// Convert AWS Smithy Document to serde_json::Value
fn document_to_json(doc: &aws_smithy_types::Document) -> serde_json::Value {
    use aws_smithy_types::Document;

    match doc {
        Document::Object(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k.clone(), document_to_json(v));
            }
            serde_json::Value::Object(obj)
        }
        Document::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(document_to_json).collect())
        }
        Document::Number(num) => {
            // Try to parse as different number types
            serde_json::Value::Number(
                serde_json::Number::from_f64(num.to_f64_lossy())
                    .unwrap_or(serde_json::Number::from(0)),
            )
        }
        Document::String(s) => serde_json::Value::String(s.clone()),
        Document::Bool(b) => serde_json::Value::Bool(*b),
        Document::Null => serde_json::Value::Null,
    }
}

/// Convert serde_json::Value to AWS Smithy Document
fn json_to_document(value: serde_json::Value) -> aws_smithy_types::Document {
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

/// Convert OpenAI-style messages to Bedrock format
///
/// Separates system messages from conversation messages as required by Bedrock API.
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
                // Extract system messages separately
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
                // Tool results are handled as user messages with ToolResult content
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
            // S3Objects are already converted to ImageUrl by prepare_messages_for_api
            // If we somehow get here, skip it
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

    // Handle tool calls (for assistant messages)
    if let Some(ref tool_calls) = msg.tool_calls {
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

/// Extract text content and tool calls from Bedrock Converse response
pub fn bedrock_response_to_openai(
    output: &aws_sdk_bedrockruntime::operation::converse::ConverseOutput,
) -> Result<(Option<String>, Vec<OpenAIToolCall>), Error> {
    let mut text_content = String::new();
    let mut tool_calls = Vec::new();

    if let Some(message) = output.output().and_then(|o| o.as_message().ok()) {
        let content_blocks = message.content();
        if !content_blocks.is_empty() {
            for block in content_blocks {
                match block {
                    ContentBlock::Text(text) => {
                        text_content.push_str(&text);
                    }
                    ContentBlock::ToolUse(tool_use) => {
                        // Convert to OpenAI tool call format
                        // Convert aws_smithy_types::Document to serde_json::Value
                        let input_value = document_to_json(tool_use.input());
                        let arguments = serde_json::to_string(&input_value)
                            .unwrap_or_else(|_| EMPTY_JSON.to_string());

                        tool_calls.push(OpenAIToolCall {
                            id: tool_use.tool_use_id().to_string(),
                            r#type: FUNCTION_TYPE.to_string(),
                            function: OpenAIFunction {
                                name: tool_use.name().to_string(),
                                arguments,
                            },
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    let content = if text_content.is_empty() {
        None
    } else {
        Some(text_content)
    };

    Ok((content, tool_calls))
}

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
        })
        .collect()
}

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
        should_stream: bool,
        stream_event_processor: Option<StreamEventProcessor>,
        client: &AuthedClient,
        workspace_id: &str,
        structured_output_tool_name: Option<&str>,
    ) -> Result<ParsedResponse, Error> {
        // Create Bedrock client with bearer token authentication
        let bedrock_client = BedrockClient::from_bearer_token(api_key.to_string(), region).await?;

        // Prepare messages: convert S3Objects to ImageUrls by downloading from S3
        let prepared_messages = prepare_messages_for_api(messages, client, workspace_id).await?;

        // Convert messages to Bedrock format (separates system prompts)
        let (bedrock_messages, system_prompts) = openai_messages_to_bedrock(&prepared_messages)?;

        // Build inference configuration
        let inference_config = create_inference_config(temperature, max_tokens.map(|t| t as i32));

        // Build tool configuration with optional ToolChoice
        let tool_config = self.build_tool_config(tools, structured_output_tool_name.is_some())?;

        if should_stream {
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
        } else {
            self.execute_converse(
                &bedrock_client,
                model,
                bedrock_messages,
                system_prompts,
                inference_config,
                tool_config,
            )
            .await
        }
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

            // For structured output, force the model to use the tool
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

    /// Execute non-streaming Bedrock request
    async fn execute_converse(
        &self,
        bedrock_client: &BedrockClient,
        model: &str,
        bedrock_messages: Vec<aws_sdk_bedrockruntime::types::Message>,
        system_prompts: Vec<aws_sdk_bedrockruntime::types::SystemContentBlock>,
        inference_config: Option<aws_sdk_bedrockruntime::types::InferenceConfiguration>,
        tool_config: Option<aws_sdk_bedrockruntime::types::ToolConfiguration>,
    ) -> Result<ParsedResponse, Error> {
        let mut request_builder = bedrock_client
            .client()
            .converse()
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

        // Execute the request
        let response = request_builder.send().await.map_err(|e| {
            let error_msg = format!("Bedrock API error: {}", format_bedrock_error(&e));
            Error::internal_err(error_msg)
        })?;

        // Convert response back to OpenAI format
        let (content, tool_calls) = bedrock_response_to_openai(&response)?;

        Ok(ParsedResponse::Text { content, tool_calls, events_str: None })
    }

    /// Execute streaming Bedrock request
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
        // Build streaming request
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

        // Execute streaming request
        let mut stream = request_builder
            .send()
            .await
            .map_err(|e| {
                let error_msg =
                    format!("Bedrock streaming API error: {}", format_bedrock_error(&e));
                Error::internal_err(error_msg)
            })?
            .stream;

        let mut accumulated_text = String::new();
        let mut events_str = String::new();
        let mut accumulated_tool_calls: HashMap<String, StreamingToolCall> = HashMap::new();
        let mut current_tool_use_id: Option<String> = None;

        // Process stream events
        loop {
            match stream.recv().await {
                Ok(Some(event)) => {
                    // Handle tool use start
                    if let Some(tool_call) = bedrock_stream_event_to_tool_start(&event) {
                        current_tool_use_id = Some(tool_call.id.clone());
                        accumulated_tool_calls.insert(tool_call.id.clone(), tool_call);
                    }

                    // Handle text delta
                    if let Some(text_delta) = bedrock_stream_event_to_text(&event) {
                        accumulated_text.push_str(&text_delta);
                        if let Some(processor) = stream_event_processor.as_ref() {
                            processor
                                .send(
                                    StreamingEvent::TokenDelta { content: text_delta },
                                    &mut events_str,
                                )
                                .await?;
                        }
                    }

                    // Handle tool use input delta
                    if let Some(input_delta) = bedrock_stream_event_to_tool_delta(&event) {
                        if let Some(tool_id) = &current_tool_use_id {
                            if let Some(tool_call) = accumulated_tool_calls.get_mut(tool_id) {
                                tool_call.arguments.push_str(&input_delta);
                            }
                        }
                    }

                    // Handle content block stop
                    if bedrock_stream_event_is_block_stop(&event) {
                        current_tool_use_id = None;
                    }
                }
                Ok(None) => break, // Stream ended
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

        Ok(ParsedResponse::Text {
            content,
            tool_calls,
            events_str: if events_str.is_empty() {
                None
            } else {
                Some(events_str)
            },
        })
    }
}
