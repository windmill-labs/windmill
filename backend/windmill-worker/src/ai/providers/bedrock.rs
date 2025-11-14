use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use windmill_common::{
    bedrock_client::{extract_region_from_url, BedrockClient},
    bedrock_converters::{
        bedrock_response_to_openai, create_inference_config, openai_messages_to_bedrock,
        openai_tools_to_bedrock, SimpleOpenAIMessage, SimpleToolCall, SimpleToolDef,
    },
    client::AuthedClient,
    error::Error,
};

use crate::ai::{
    image_handler::prepare_messages_for_api,
    providers::openai::{OpenAIFunction, OpenAIToolCall},
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    types::*,
};

// Bedrock-specific response types
#[derive(Deserialize)]
struct BedrockResponse {
    output: BedrockOutput,
    #[allow(unused)]
    #[serde(rename = "stopReason", default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct BedrockOutput {
    message: BedrockMessage,
}

#[derive(Deserialize)]
struct BedrockMessage {
    #[allow(unused)]
    role: String,
    content: Vec<BedrockContent>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BedrockContent {
    Text {
        text: String,
    },
    ToolUse {
        #[serde(rename = "toolUse")]
        tool_use: ToolUse,
    },
}

#[derive(Deserialize)]
struct ToolUse {
    #[serde(rename = "toolUseId")]
    tool_use_id: String,
    name: String,
    input: Value,
}

pub struct BedrockQueryBuilder;

impl BedrockQueryBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Convert Windmill OpenAIMessage to SimpleOpenAIMessage for SDK converters
    fn convert_to_simple_messages(messages: &[OpenAIMessage]) -> Vec<SimpleOpenAIMessage> {
        messages
            .iter()
            .map(|msg| {
                let content = msg.content.as_ref().map(|c| match c {
                    OpenAIContent::Text(t) => t.clone(),
                    OpenAIContent::Parts(parts) => {
                        // Extract text from parts and join them
                        parts
                            .iter()
                            .filter_map(|part| {
                                if let ContentPart::Text { text } = part {
                                    Some(text.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                });

                let tool_calls = msg.tool_calls.as_ref().map(|tcs| {
                    tcs.iter()
                        .map(|tc| SimpleToolCall {
                            id: tc.id.clone(),
                            r#type: tc.r#type.clone(),
                            function: windmill_common::bedrock_converters::SimpleFunction {
                                name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                            },
                        })
                        .collect()
                });

                SimpleOpenAIMessage {
                    role: msg.role.clone(),
                    content,
                    tool_calls,
                    tool_call_id: msg.tool_call_id.clone(),
                }
            })
            .collect()
    }

    /// Convert ToolDef to SimpleToolDef for SDK converters
    fn convert_to_simple_tools(tools: &[ToolDef]) -> Vec<SimpleToolDef> {
        tools
            .iter()
            .map(|tool| SimpleToolDef {
                r#type: tool.r#type.clone(),
                function: windmill_common::bedrock_converters::SimpleToolDefFunction {
                    name: tool.function.name.clone(),
                    description: tool.function.description.clone(),
                    parameters: tool.function.parameters.clone(),
                },
            })
            .collect()
    }

    /// Transform OpenAI format messages to Bedrock Converse format
    fn transform_messages_to_bedrock(
        messages: &[OpenAIMessage],
    ) -> Result<(Vec<Value>, Vec<Value>), Error> {
        let mut system_messages = Vec::with_capacity(messages.len() / 4);
        let mut conversation_messages = Vec::with_capacity(messages.len());

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
                                parts.iter().fold(String::new(), |mut acc, part| {
                                    if let ContentPart::Text { text } = part {
                                        if !acc.is_empty() {
                                            acc.push(' ');
                                        }
                                        acc.push_str(text);
                                    }
                                    acc
                                })
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
                                                    let base64_data = &url[base64_start + 7..];
                                                    let mime_type = url
                                                        .split(';')
                                                        .next()
                                                        .and_then(|s| s.strip_prefix("data:"))
                                                        .unwrap_or("image/png");

                                                    // Extract format from MIME type (e.g., "image/png" -> "png")
                                                    let format = mime_type
                                                        .rsplit_once('/')
                                                        .map(|(_, format)| format)
                                                        .unwrap_or("png");

                                                    content.push(serde_json::json!({
                                                        "image": {
                                                            "format": format,
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
                                    .map_err(|e| {
                                        Error::internal_err(format!(
                                            "Failed to parse tool arguments: {}",
                                            e
                                        ))
                                    })?;

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
                    let tool_call_id = msg.tool_call_id.as_ref().map(|s| s.as_str()).unwrap_or("");
                    let content_text = match &msg.content {
                        Some(OpenAIContent::Text(t)) => t.clone(),
                        Some(OpenAIContent::Parts(parts)) => {
                            parts.iter().fold(String::new(), |mut acc, part| {
                                if let ContentPart::Text { text } = part {
                                    if !acc.is_empty() {
                                        acc.push(' ');
                                    }
                                    acc.push_str(text);
                                }
                                acc
                            })
                        }
                        None => String::new(),
                    };

                    // Parse content as JSON if possible, otherwise use as text
                    // Bedrock requires json field to be an object, not a primitive or array
                    let tool_result_content =
                        if let Ok(parsed) = serde_json::from_str::<Value>(&content_text) {
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
                    let arguments =
                        serde_json::to_string(&tool_use.input).unwrap_or_else(|_| "{}".to_string());

                    tool_calls.push(OpenAIToolCall {
                        id: tool_use.tool_use_id,
                        function: OpenAIFunction { name: tool_use.name, arguments },
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
        client: &AuthedClient,
        workspace_id: &str,
        _stream: bool,
    ) -> Result<String, Error> {
        // Only support text output for now
        if !matches!(args.output_type, OutputType::Text) {
            return Err(Error::internal_err(
                "Bedrock only supports text output type".to_string(),
            ));
        }

        // Prepare messages first (converts S3Objects to ImageUrls)
        let prepared_messages =
            prepare_messages_for_api(args.messages, client, workspace_id).await?;

        // Transform messages
        let (system_messages, conversation_messages) =
            Self::transform_messages_to_bedrock(&prepared_messages)?;

        // Build Bedrock request
        let mut bedrock_req = serde_json::json!({
            "messages": conversation_messages,
        });

        // Add system messages if any
        if !system_messages.is_empty() {
            bedrock_req["system"] = serde_json::json!(system_messages);
        }

        // Add inference configuration
        let has_inference_params = args.temperature.is_some() || args.max_tokens.is_some();
        if has_inference_params {
            let mut inference_config = serde_json::Map::new();
            if let Some(temp) = args.temperature {
                inference_config.insert("temperature".to_string(), serde_json::json!(temp));
            }
            if let Some(max_tokens) = args.max_tokens {
                inference_config.insert("maxTokens".to_string(), serde_json::json!(max_tokens));
            }
            bedrock_req["inferenceConfig"] = serde_json::Value::Object(inference_config);
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
                "tools": bedrock_tools,
            });

            // Handle structured output schema
            let has_output_properties = args
                .output_schema
                .as_ref()
                .and_then(|schema| schema.properties.as_ref())
                .map(|props| !props.is_empty())
                .unwrap_or(false);

            if has_output_properties {
                bedrock_req["toolConfig"]["toolChoice"] = serde_json::json!({
                    "any": {}
                });
            }
        }

        serde_json::to_string(&bedrock_req)
            .map_err(|e| Error::internal_err(format!("Failed to serialize Bedrock request: {}", e)))
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        let response_text = response
            .text()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to read response text: {}", e)))?;

        let bedrock_response: BedrockResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                Error::internal_err(format!(
                    "Failed to parse Bedrock response: {}. Raw response: {}",
                    e, response_text
                ))
            })?;

        Self::transform_bedrock_response_to_openai(bedrock_response)
    }

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        let mut stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::with_capacity(8192);
        let mut accumulated_content = String::new();
        let mut accumulated_tool_calls: std::collections::HashMap<String, OpenAIToolCall> =
            std::collections::HashMap::new();
        let mut current_tool_use_id: Option<String> = None;
        let mut events_str = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| Error::internal_err(format!("Stream error: {}", e)))?;
            buffer.extend_from_slice(&chunk);

            // Parse AWS event stream binary format
            while buffer.len() >= 12 {
                // Need at least prelude + CRC
                // Read prelude using slice operations
                let total_length = u32::from_be_bytes(
                    buffer[0..4].try_into().expect("slice with correct length"),
                ) as usize;

                // Check if we have the complete message
                if buffer.len() < total_length {
                    break;
                }

                let headers_length = u32::from_be_bytes(
                    buffer[4..8].try_into().expect("slice with correct length"),
                ) as usize;

                let headers_start = 12; // After prelude (8 bytes) + prelude CRC (4 bytes)
                let payload_start = headers_start + headers_length;
                let payload_end = total_length - 4; // Before message CRC (4 bytes)

                // Parse headers to extract event type
                let mut event_type = None;
                let mut pos = headers_start;
                while pos < payload_start && pos < buffer.len() {
                    // Check if we can read name_len
                    let name_len = match buffer.get(pos) {
                        Some(&len) => len as usize,
                        None => break,
                    };
                    pos += 1;

                    // Check if we can read the name
                    if pos + name_len > buffer.len() {
                        break;
                    }
                    let name = String::from_utf8_lossy(&buffer[pos..pos + name_len]).into_owned();
                    pos += name_len;

                    // Check if we can read value_type and value_len
                    if pos + 3 > buffer.len() {
                        break;
                    }
                    let value_type = buffer[pos];
                    pos += 1;
                    let value_len = u16::from_be_bytes(
                        buffer[pos..pos + 2]
                            .try_into()
                            .expect("slice with correct length"),
                    ) as usize;
                    pos += 2;

                    // Check if we can read the value
                    if pos + value_len > buffer.len() {
                        break;
                    }

                    if value_type == 7 && name == ":event-type" {
                        event_type = Some(
                            String::from_utf8_lossy(&buffer[pos..pos + value_len]).into_owned(),
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
                                if let Some(tool_use) =
                                    event_data.get("start").and_then(|s| s.get("toolUse"))
                                {
                                    if let Some(tool_use_id) =
                                        tool_use.get("toolUseId").and_then(|id| id.as_str())
                                    {
                                        let name = tool_use
                                            .get("name")
                                            .and_then(|n| n.as_str())
                                            .unwrap_or("")
                                            .to_string();

                                        let tool_use_id_owned = tool_use_id.to_string();
                                        current_tool_use_id = Some(tool_use_id_owned.clone());

                                        accumulated_tool_calls.insert(
                                            tool_use_id_owned,
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
                                        if let Some(input_str) =
                                            tool_use.get("input").and_then(|i| i.as_str())
                                        {
                                            // Append to the current tool call being built
                                            if let Some(tool_id) = &current_tool_use_id {
                                                if let Some(tool_call) =
                                                    accumulated_tool_calls.get_mut(tool_id)
                                                {
                                                    tool_call.function.arguments.push_str(input_str);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Some("contentBlockStop") => {
                                // Block completed - clear current tool ID
                                current_tool_use_id = None;
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
        let endpoint = if stream {
            "converse-stream"
        } else {
            "converse"
        };
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
