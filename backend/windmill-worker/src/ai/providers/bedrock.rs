use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use windmill_common::{client::AuthedClient, error::Error};

use crate::ai::{
    providers::openai::{OpenAIFunction, OpenAIToolCall},
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    types::*,
};

// Bedrock-specific response types
#[derive(Deserialize)]
struct BedrockResponse {
    output: BedrockOutput,
    #[serde(rename = "stopReason")]
    stop_reason: Option<String>,
    usage: Option<BedrockUsage>,
}

#[derive(Deserialize)]
struct BedrockOutput {
    message: BedrockMessage,
}

#[derive(Deserialize)]
struct BedrockMessage {
    role: String,
    content: Vec<BedrockContent>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BedrockContent {
    Text { text: String },
    ToolUse { #[serde(rename = "toolUse")] tool_use: ToolUse },
}

#[derive(Deserialize)]
struct ToolUse {
    #[serde(rename = "toolUseId")]
    tool_use_id: String,
    name: String,
    input: Value,
}

#[derive(Deserialize)]
struct BedrockUsage {
    #[serde(rename = "inputTokens")]
    input_tokens: Option<u32>,
    #[serde(rename = "outputTokens")]
    output_tokens: Option<u32>,
}

pub struct BedrockQueryBuilder;

impl BedrockQueryBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Transform OpenAI format messages to Bedrock Converse format
    fn transform_messages_to_bedrock(
        messages: &[OpenAIMessage],
    ) -> Result<(Vec<Value>, Vec<Value>), Error> {
        let mut system_messages = Vec::new();
        let mut conversation_messages = Vec::new();

        for msg in messages {
            let role = &msg.role;

            match role.as_str() {
                "system" => {
                    // Extract system messages
                    if let Some(content) = &msg.content {
                        let text = match content {
                            OpenAIContent::Text(t) => t.clone(),
                            OpenAIContent::Parts(parts) => {
                                // Extract text from parts
                                parts
                                    .iter()
                                    .filter_map(|part| match part {
                                        ContentPart::Text { text } => Some(text.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            }
                        };
                        system_messages.push(serde_json::json!({"text": text}));
                    }
                }
                "user" | "assistant" => {
                    let mut content = Vec::new();

                    // Handle message content
                    if let Some(msg_content) = &msg.content {
                        match msg_content {
                            OpenAIContent::Text(text) => {
                                content.push(serde_json::json!({"text": text}));
                            }
                            OpenAIContent::Parts(parts) => {
                                for part in parts {
                                    match part {
                                        ContentPart::Text { text } => {
                                            content.push(serde_json::json!({"text": text}));
                                        }
                                        ContentPart::ImageUrl { image_url } => {
                                            // Bedrock image format - extract base64 from data URL
                                            let url = &image_url.url;
                                            if url.starts_with("data:") {
                                                // Parse data:image/png;base64,<data>
                                                if let Some(base64_start) = url.find("base64,") {
                                                    let base64_data =
                                                        &url[base64_start + 7..];
                                                    let mime_type = url
                                                        .split(';')
                                                        .next()
                                                        .and_then(|s| s.strip_prefix("data:"))
                                                        .unwrap_or("image/png");

                                                    content.push(serde_json::json!({
                                                        "image": {
                                                            "format": mime_type.split('/').last().unwrap_or("png"),
                                                            "source": {
                                                                "bytes": base64_data
                                                            }
                                                        }
                                                    }));
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }

                    // Handle tool_calls for assistant messages
                    if role == "assistant" {
                        if let Some(tool_calls) = &msg.tool_calls {
                            for tool_call in tool_calls {
                                if tool_call.r#type == "function" {
                                    // Parse arguments JSON string to object
                                    let input = serde_json::from_str::<Value>(
                                        &tool_call.function.arguments,
                                    )
                                    .unwrap_or(serde_json::json!({}));

                                    content.push(serde_json::json!({
                                        "toolUse": {
                                            "toolUseId": tool_call.id,
                                            "name": tool_call.function.name,
                                            "input": input
                                        }
                                    }));
                                }
                            }
                        }
                    }

                    // Only add message if it has content
                    if !content.is_empty() {
                        conversation_messages.push(serde_json::json!({
                            "role": role,
                            "content": content
                        }));
                    }
                }
                "tool" => {
                    // Transform tool response to Bedrock format
                    let tool_call_id = msg
                        .tool_call_id
                        .as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let content_text = match &msg.content {
                        Some(OpenAIContent::Text(t)) => t.clone(),
                        Some(OpenAIContent::Parts(parts)) => parts
                            .iter()
                            .filter_map(|part| match part {
                                ContentPart::Text { text } => Some(text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(" "),
                        None => String::new(),
                    };

                    // Parse content as JSON if possible, otherwise use as text
                    // Bedrock requires json field to be an object, not a primitive or array
                    let tool_result_content = if let Ok(parsed) =
                        serde_json::from_str::<Value>(&content_text)
                    {
                        if parsed.is_object() {
                            parsed
                        } else {
                            // Wrap primitives and arrays in an object
                            serde_json::json!({"result": parsed})
                        }
                    } else {
                        serde_json::json!({"result": content_text})
                    };

                    // Bedrock requires toolResult to be in a user message
                    conversation_messages.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "toolResult": {
                                "toolUseId": tool_call_id,
                                "content": [{"json": tool_result_content}]
                            }
                        }]
                    }));
                }
                _ => {
                    // Skip unknown roles
                }
            }
        }

        Ok((system_messages, conversation_messages))
    }

    /// Transform Bedrock response to OpenAI format
    fn transform_bedrock_response_to_openai(
        bedrock_response: BedrockResponse,
    ) -> Result<ParsedResponse, Error> {
        let mut content_text = String::new();
        let mut tool_calls = Vec::new();

        for content_item in bedrock_response.output.message.content {
            match content_item {
                BedrockContent::Text { text } => {
                    if !content_text.is_empty() {
                        content_text.push(' ');
                    }
                    content_text.push_str(&text);
                }
                BedrockContent::ToolUse { tool_use } => {
                    // Convert Bedrock toolUse to OpenAI tool_call
                    let arguments = serde_json::to_string(&tool_use.input)
                        .unwrap_or_else(|_| "{}".to_string());

                    tool_calls.push(OpenAIToolCall {
                        id: tool_use.tool_use_id,
                        function: OpenAIFunction {
                            name: tool_use.name,
                            arguments,
                        },
                        r#type: "function".to_string(),
                    });
                }
            }
        }

        Ok(ParsedResponse::Text {
            content: if content_text.is_empty() {
                None
            } else {
                Some(content_text)
            },
            tool_calls,
            events_str: None,
        })
    }
}

#[async_trait]
impl QueryBuilder for BedrockQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        // Bedrock supports tools for text output
        matches!(output_type, OutputType::Text)
    }

    fn supports_streaming(&self) -> bool {
        // Bedrock supports streaming
        true
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        _client: &AuthedClient,
        _workspace_id: &str,
        stream: bool,
    ) -> Result<String, Error> {
        // Only support text output for now
        if !matches!(args.output_type, OutputType::Text) {
            return Err(Error::internal_err(
                "Bedrock only supports text output type".to_string(),
            ));
        }

        // Transform messages
        let (system_messages, conversation_messages) =
            Self::transform_messages_to_bedrock(args.messages)?;

        // Build Bedrock request
        let mut bedrock_req = serde_json::json!({
            "messages": conversation_messages,
        });

        // Add system messages if any
        if !system_messages.is_empty() {
            bedrock_req["system"] = serde_json::json!(system_messages);
        }

        // Add inference configuration
        let mut inference_config = serde_json::json!({});
        if let Some(temp) = args.temperature {
            inference_config["temperature"] = serde_json::json!(temp);
        }
        if let Some(max_tokens) = args.max_tokens {
            inference_config["maxTokens"] = serde_json::json!(max_tokens);
        }

        if !inference_config.as_object().unwrap().is_empty() {
            bedrock_req["inferenceConfig"] = inference_config;
        }

        // Add tools if provided
        if let Some(tools) = args.tools {
            let bedrock_tools: Vec<Value> = tools
                .iter()
                .map(|tool| {
                    // Parse the parameters from RawValue
                    let params: Value = serde_json::from_str(tool.function.parameters.get())
                        .unwrap_or(serde_json::json!({}));

                    serde_json::json!({
                        "toolSpec": {
                            "name": tool.function.name,
                            "description": tool.function.description.as_ref().map(|s| s.as_str()).unwrap_or("Tool function"),
                            "inputSchema": {
                                "json": params
                            }
                        }
                    })
                })
                .collect();

            bedrock_req["toolConfig"] = serde_json::json!({
                "tools": bedrock_tools
            });
        }

        serde_json::to_string(&bedrock_req)
            .map_err(|e| Error::internal_err(format!("Failed to serialize Bedrock request: {}", e)))
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        let response_text = response
            .text()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to read response text: {}", e)))?;

        let bedrock_response: BedrockResponse = serde_json::from_str(&response_text).map_err(
            |e| {
                Error::internal_err(format!(
                    "Failed to parse Bedrock response: {}. Raw response: {}",
                    e, response_text
                ))
            },
        )?;

        Self::transform_bedrock_response_to_openai(bedrock_response)
    }

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        let mut stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();
        let mut accumulated_content = String::new();
        let mut accumulated_tool_calls: std::collections::HashMap<String, OpenAIToolCall> =
            std::collections::HashMap::new();
        let mut events_str = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk
                .map_err(|e| Error::internal_err(format!("Stream error: {}", e)))?;
            buffer.extend_from_slice(&chunk);

            // Parse AWS event stream binary format
            while buffer.len() >= 12 {
                // Need at least prelude + CRC
                // Read prelude
                let total_length = u32::from_be_bytes([
                    buffer[0], buffer[1], buffer[2], buffer[3],
                ]) as usize;

                // Check if we have the complete message
                if buffer.len() < total_length {
                    break;
                }

                let headers_length = u32::from_be_bytes([
                    buffer[4], buffer[5], buffer[6], buffer[7],
                ]) as usize;

                let headers_start = 12; // After prelude (8 bytes) + prelude CRC (4 bytes)
                let payload_start = headers_start + headers_length;
                let payload_end = total_length - 4; // Before message CRC (4 bytes)

                // Parse headers to extract event type
                let mut event_type = None;
                let mut pos = headers_start;
                while pos < payload_start && pos < buffer.len() {
                    if pos + 1 > buffer.len() {
                        break;
                    }
                    let name_len = buffer[pos] as usize;
                    pos += 1;

                    if pos + name_len > buffer.len() {
                        break;
                    }
                    let name =
                        String::from_utf8_lossy(&buffer[pos..pos + name_len]).to_string();
                    pos += name_len;

                    if pos + 3 > buffer.len() {
                        break;
                    }
                    let value_type = buffer[pos];
                    pos += 1;
                    let value_len =
                        u16::from_be_bytes([buffer[pos], buffer[pos + 1]]) as usize;
                    pos += 2;

                    if pos + value_len > buffer.len() {
                        break;
                    }

                    if value_type == 7 && name == ":event-type" {
                        event_type = Some(
                            String::from_utf8_lossy(&buffer[pos..pos + value_len]).to_string(),
                        );
                    }
                    pos += value_len;
                }

                // Extract and parse JSON payload
                if payload_start < payload_end && payload_end <= buffer.len() {
                    let payload = &buffer[payload_start..payload_end];

                    if let Ok(event_data) = serde_json::from_slice::<Value>(payload) {
                        // Handle different event types
                        match event_type.as_deref() {
                            Some("contentBlockStart") => {
                                // Tool use started
                                if let Some(tool_use) = event_data
                                    .get("start")
                                    .and_then(|s| s.get("toolUse"))
                                {
                                    if let Some(tool_use_id) = tool_use.get("toolUseId").and_then(|id| id.as_str()) {
                                        let name = tool_use.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();

                                        accumulated_tool_calls.insert(
                                            tool_use_id.to_string(),
                                            OpenAIToolCall {
                                                id: tool_use_id.to_string(),
                                                function: OpenAIFunction {
                                                    name,
                                                    arguments: String::new(),
                                                },
                                                r#type: "function".to_string(),
                                            },
                                        );
                                    }
                                }
                            }
                            Some("contentBlockDelta") => {
                                if let Some(delta) = event_data.get("delta") {
                                    // Text delta
                                    if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                        accumulated_content.push_str(text);

                                        let event = StreamingEvent::TokenDelta {
                                            content: text.to_string(),
                                        };
                                        stream_event_processor.send(event, &mut events_str).await?;
                                    }

                                    // Tool use delta (input accumulation)
                                    if let Some(tool_use) = delta.get("toolUse") {
                                        if let Some(input_str) = tool_use.get("input").and_then(|i| i.as_str()) {
                                            // Find the tool call being updated (last one added)
                                            if let Some(last_tool_call) = accumulated_tool_calls.values_mut().last() {
                                                last_tool_call.function.arguments.push_str(input_str);
                                            }
                                        }
                                    }
                                }
                            }
                            Some("contentBlockStop") => {
                                // Block completed - nothing to do
                            }
                            Some("messageStop") => {
                                // Message completed
                                break;
                            }
                            Some("metadata") => {
                                // Usage information - ignore for now
                            }
                            _ => {
                                // Unknown event type - ignore
                            }
                        }
                    }
                }

                // Remove processed message from buffer
                buffer.drain(0..total_length);
            }
        }

        // Send tool call events
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
        model: &str,
        output_type: &OutputType,
        stream: bool,
    ) -> String {
        // Bedrock uses different URL structure: /model/{model-id}/converse[-stream]
        if !matches!(output_type, OutputType::Text) {
            // Image generation not supported yet
            return format!("{}/model/{}/converse", base_url, model);
        }

        // Use -stream suffix for streaming requests
        let endpoint = if stream { "converse-stream" } else { "converse" };
        format!("{}/model/{}/{}", base_url, model, endpoint)
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        _base_url: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        // Bedrock uses Bearer token authentication
        vec![("Authorization", format!("Bearer {}", api_key))]
    }
}
