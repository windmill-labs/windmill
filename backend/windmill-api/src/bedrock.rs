use axum::body::Bytes;
use bytes;
use futures;
use uuid;
use windmill_common::error::{Error, Result};

/// Transform OpenAI format request to AWS Bedrock Converse format
/// Returns: (model_id, transformed_body, is_streaming)
pub fn transform_openai_to_bedrock(body: &[u8]) -> Result<(String, Bytes, bool)> {
    use serde_json::Value;

    // Parse the OpenAI request
    let openai_req: Value = serde_json::from_slice(body)
        .map_err(|e| Error::internal_err(format!("Failed to parse OpenAI request: {}", e)))?;

    // Extract model and streaming flag
    let model = openai_req["model"]
        .as_str()
        .ok_or_else(|| Error::BadRequest("Missing 'model' field in request".to_string()))?
        .to_string();

    let is_streaming = openai_req["stream"].as_bool().unwrap_or(false);

    // Build Bedrock request
    let mut bedrock_req = serde_json::json!({});

    // Transform messages
    if let Some(messages) = openai_req["messages"].as_array() {
        let mut system_messages = Vec::new();
        let mut conversation_messages = Vec::new();

        for msg in messages {
            let role = msg["role"].as_str().unwrap_or("");

            match role {
                "system" => {
                    // Extract system messages to separate array
                    if let Some(content) = msg["content"].as_str() {
                        system_messages.push(serde_json::json!({"text": content}));
                    }
                }
                "user" | "assistant" => {
                    // Normalize content to array format
                    let mut content = if let Some(text) = msg["content"].as_str() {
                        // Simple string → array of content blocks
                        vec![serde_json::json!({"text": text})]
                    } else if let Some(content_array) = msg["content"].as_array() {
                        // Already an array - transform each item
                        content_array
                            .iter()
                            .filter_map(|item| {
                                if let Some(text) = item["text"].as_str() {
                                    Some(serde_json::json!({"text": text}))
                                } else if item["type"].as_str() == Some("text") {
                                    Some(serde_json::json!({"text": item["text"]}))
                                } else if item["type"].as_str() == Some("image_url") {
                                    // Transform image_url format if needed
                                    // For now, pass through - may need more sophisticated handling
                                    Some(item.clone())
                                } else {
                                    None
                                }
                            })
                            .collect()
                    } else {
                        vec![]
                    };

                    // Handle tool_calls for assistant messages (OpenAI → Bedrock toolUse)
                    if role == "assistant" {
                        if let Some(tool_calls) = msg["tool_calls"].as_array() {
                            for tool_call in tool_calls {
                                if tool_call["type"].as_str() == Some("function") {
                                    let tool_use_id = tool_call["id"].as_str().unwrap_or("");
                                    let function_name =
                                        tool_call["function"]["name"].as_str().unwrap_or("");
                                    let arguments_str =
                                        tool_call["function"]["arguments"].as_str().unwrap_or("{}");

                                    // Parse arguments JSON string to object
                                    let input = serde_json::from_str::<Value>(arguments_str)
                                        .map_err(|e| {
                                            Error::internal_err(format!(
                                                "Failed to parse tool call arguments: {}",
                                                e
                                            ))
                                        })?;

                                    content.push(serde_json::json!({
                                        "toolUse": {
                                            "toolUseId": tool_use_id,
                                            "name": function_name,
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
                    let tool_call_id = msg["tool_call_id"].as_str().unwrap_or("");
                    let content = msg["content"].as_str().unwrap_or("");

                    // Try to parse content as JSON
                    // Bedrock requires json field to be an object, not a primitive or array
                    let tool_result_content =
                        if let Ok(json_content) = serde_json::from_str::<Value>(content) {
                            if json_content.is_object() {
                                vec![serde_json::json!({"json": json_content})]
                            } else {
                                // Wrap primitives and arrays in an object
                                vec![serde_json::json!({"json": {"result": json_content}})]
                            }
                        } else {
                            vec![serde_json::json!({"text": content})]
                        };

                    conversation_messages.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "toolResult": {
                                "toolUseId": tool_call_id,
                                "content": tool_result_content
                            }
                        }]
                    }));
                }
                _ => {}
            }
        }

        if !system_messages.is_empty() {
            bedrock_req["system"] = Value::Array(system_messages);
        }
        bedrock_req["messages"] = Value::Array(conversation_messages);
    }

    // Transform inference parameters
    let mut inference_config = serde_json::json!({});
    if let Some(max_tokens) = openai_req["max_tokens"].as_i64() {
        inference_config["maxTokens"] = Value::Number(max_tokens.into());
    }
    if let Some(temperature) = openai_req["temperature"].as_f64() {
        inference_config["temperature"] = serde_json::json!(temperature);
    }
    if let Some(top_p) = openai_req["top_p"].as_f64() {
        inference_config["topP"] = serde_json::json!(top_p);
    }
    if let Some(stop) = openai_req["stop"].as_array() {
        let stop_sequences: Vec<String> = stop
            .iter()
            .filter_map(|s| s.as_str().map(|s| s.to_string()))
            .collect();
        if !stop_sequences.is_empty() {
            inference_config["stopSequences"] =
                Value::Array(stop_sequences.into_iter().map(Value::String).collect());
        }
    }
    if !inference_config.as_object().unwrap().is_empty() {
        bedrock_req["inferenceConfig"] = inference_config;
    }

    // Transform tools if present
    if let Some(tools) = openai_req["tools"].as_array() {
        let mut bedrock_tools = Vec::new();

        for tool in tools {
            if tool["type"].as_str() == Some("function") {
                if let Some(function) = tool["function"].as_object() {
                    bedrock_tools.push(serde_json::json!({
                        "toolSpec": {
                            "name": function.get("name"),
                            "description": function.get("description")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                                .unwrap_or("Tool function"),
                            "inputSchema": {
                                "json": function.get("parameters")
                            }
                        }
                    }));
                }
            }
        }

        if !bedrock_tools.is_empty() {
            let mut tool_config = serde_json::json!({
                "tools": bedrock_tools
            });

            // Transform tool_choice
            if let Some(tool_choice) = openai_req.get("tool_choice") {
                if tool_choice == "auto" {
                    tool_config["toolChoice"] = serde_json::json!({"auto": {}});
                } else if tool_choice == "required" {
                    tool_config["toolChoice"] = serde_json::json!({"any": {}});
                } else if let Some(obj) = tool_choice.as_object() {
                    if obj.get("type").and_then(|v| v.as_str()) == Some("function") {
                        if let Some(function) = obj.get("function").and_then(|v| v.as_object()) {
                            if let Some(name) = function.get("name").and_then(|v| v.as_str()) {
                                tool_config["toolChoice"] = serde_json::json!({
                                    "tool": {"name": name}
                                });
                            }
                        }
                    }
                }
            }

            bedrock_req["toolConfig"] = tool_config;
        }
    }

    let transformed_body = serde_json::to_vec(&bedrock_req)
        .map_err(|e| Error::internal_err(format!("Failed to serialize Bedrock request: {}", e)))?
        .into();

    Ok((model, transformed_body, is_streaming))
}

/// Transform AWS Bedrock Converse response to OpenAI format
pub async fn transform_bedrock_to_openai(
    response: reqwest::Response,
    model: String,
) -> Result<Bytes> {
    use serde_json::Value;

    let bedrock_resp: Value = response
        .json()
        .await
        .map_err(|e| Error::internal_err(format!("Failed to parse Bedrock response: {}", e)))?;

    // Generate unique ID and timestamp
    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Extract stop reason and map to finish_reason
    let stop_reason = bedrock_resp["stopReason"].as_str().unwrap_or("end_turn");
    let finish_reason = match stop_reason {
        "end_turn" => "stop",
        "max_tokens" => "length",
        "tool_use" => "tool_calls",
        "stop_sequence" => "stop",
        "guardrail_intervened" | "content_filtered" => "content_filter",
        _ => "stop",
    };

    // Extract message content
    let message_content = &bedrock_resp["output"]["message"]["content"];
    let mut text_content = String::new();
    let mut tool_calls = Vec::new();

    if let Some(content_array) = message_content.as_array() {
        for (_index, block) in content_array.iter().enumerate() {
            if let Some(text) = block["text"].as_str() {
                text_content.push_str(text);
            } else if let Some(tool_use) = block.get("toolUse") {
                // Transform tool use to OpenAI tool_calls format
                let tool_call_id = tool_use["toolUseId"].as_str().unwrap_or("");
                let name = tool_use["name"].as_str().unwrap_or("");
                let input = &tool_use["input"];

                tool_calls.push(serde_json::json!({
                    "id": tool_call_id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": serde_json::to_string(input).unwrap_or_default()
                    }
                }));
            }
        }
    }

    // Build the message
    let message = if !tool_calls.is_empty() {
        serde_json::json!({
            "role": "assistant",
            "content": if text_content.is_empty() { Value::Null } else { Value::String(text_content) },
            "tool_calls": tool_calls
        })
    } else {
        serde_json::json!({
            "role": "assistant",
            "content": text_content
        })
    };

    // Extract usage information
    let usage = if let Some(usage_data) = bedrock_resp.get("usage") {
        serde_json::json!({
            "prompt_tokens": usage_data["inputTokens"].as_i64().unwrap_or(0),
            "completion_tokens": usage_data["outputTokens"].as_i64().unwrap_or(0),
            "total_tokens": usage_data["totalTokens"].as_i64().unwrap_or(0)
        })
    } else {
        serde_json::json!({
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0
        })
    };

    // Build OpenAI-format response
    let openai_resp = serde_json::json!({
        "id": id,
        "object": "chat.completion",
        "created": created,
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason
        }],
        "usage": usage
    });

    let response_body = serde_json::to_vec(&openai_resp)
        .map_err(|e| Error::internal_err(format!("Failed to serialize OpenAI response: {}", e)))?
        .into();

    Ok(response_body)
}

/// Transform AWS Bedrock streaming response to OpenAI SSE format
/// Bedrock uses AWS event stream binary format, not SSE
pub fn transform_bedrock_stream_to_openai(
    stream: impl futures::Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>>
        + Send
        + 'static,
    model: String,
) -> impl futures::Stream<Item = std::result::Result<bytes::Bytes, std::io::Error>> + Send {
    use futures::stream::StreamExt;
    use serde_json::Value;
    use std::collections::HashMap;

    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // State to track partial tool calls and binary buffer
    struct StreamState {
        id: String,
        model: String,
        created: u64,
        tool_calls: HashMap<usize, (String, String, String)>, // index -> (id, name, args)
        buffer: Vec<u8>,                                      // Binary buffer for AWS event stream
    }

    let state = std::sync::Arc::new(tokio::sync::Mutex::new(StreamState {
        id: id.clone(),
        model: model.clone(),
        created,
        tool_calls: HashMap::new(),
        buffer: Vec::new(),
    }));

    stream
        .then(move |chunk_result| {
            let state = state.clone();
            async move {
                match chunk_result {
                    Ok(chunk) => {
                        let mut state = state.lock().await;
                        state.buffer.extend_from_slice(&chunk);

                        let mut events = Vec::new();

                        // Parse AWS event stream messages from buffer
                        loop {
                            // Need at least 12 bytes for prelude (8) + prelude CRC (4)
                            if state.buffer.len() < 12 {
                                break;
                            }

                            // Read prelude: total_length (4 bytes) + headers_length (4 bytes)
                            let total_length = u32::from_be_bytes([
                                state.buffer[0],
                                state.buffer[1],
                                state.buffer[2],
                                state.buffer[3],
                            ]) as usize;

                            // Check if we have the complete message
                            if state.buffer.len() < total_length {
                                break;
                            }

                            let headers_length = u32::from_be_bytes([
                                state.buffer[4],
                                state.buffer[5],
                                state.buffer[6],
                                state.buffer[7],
                            ]) as usize;

                            // Skip prelude CRC (4 bytes after prelude)
                            let headers_start = 12;
                            let payload_start = headers_start + headers_length;
                            let payload_end = total_length - 4; // Exclude message CRC

                            // Parse headers to extract event type
                            let mut event_type = None;
                            let mut pos = headers_start;
                            while pos < payload_start {
                                if pos + 1 > state.buffer.len() {
                                    break;
                                }
                                let name_len = state.buffer[pos] as usize;
                                pos += 1;

                                if pos + name_len > state.buffer.len() {
                                    break;
                                }
                                let name = String::from_utf8_lossy(&state.buffer[pos..pos + name_len]).to_string();
                                pos += name_len;

                                if pos + 3 > state.buffer.len() {
                                    break;
                                }
                                let value_type = state.buffer[pos];
                                pos += 1;
                                let value_len = u16::from_be_bytes([state.buffer[pos], state.buffer[pos + 1]]) as usize;
                                pos += 2;

                                if pos + value_len > state.buffer.len() {
                                    break;
                                }

                                if value_type == 7 && name == ":event-type" {
                                    event_type = Some(String::from_utf8_lossy(&state.buffer[pos..pos + value_len]).to_string());
                                }
                                pos += value_len;
                            }

                            // Extract JSON payload (copy to avoid borrow issues)
                            let payload = state.buffer[payload_start..payload_end].to_vec();

                            // Remove processed message from buffer
                            state.buffer.drain(0..total_length);

                            // Process the event
                            if let Some(evt_type) = event_type {
                                if let Ok(payload_str) = std::str::from_utf8(&payload) {
                                    if let Ok(parsed_data) = serde_json::from_str::<Value>(payload_str) {
                                    // Transform based on event type
                                    match evt_type.as_str() {
                                        "messageStart" => {
                                            // No output for messageStart
                                        }
                                        "contentBlockStart" => {
                                            let index = parsed_data["contentBlockIndex"].as_u64().unwrap_or(0) as usize;

                                            if let Some(tool_use) = parsed_data["start"].get("toolUse") {
                                                let tool_id = tool_use["toolUseId"].as_str().unwrap_or("").to_string();
                                                let name = tool_use["name"].as_str().unwrap_or("").to_string();

                                                state.tool_calls.insert(index, (tool_id.clone(), name.clone(), String::new()));

                                                // Send initial tool call chunk
                                                let chunk = serde_json::json!({
                                                    "id": state.id,
                                                    "object": "chat.completion.chunk",
                                                    "created": state.created,
                                                    "model": state.model,
                                                    "choices": [{
                                                        "index": 0,
                                                        "delta": {
                                                            "tool_calls": [{
                                                                "index": index,
                                                                "id": tool_id,
                                                                "type": "function",
                                                                "function": {
                                                                    "name": name,
                                                                    "arguments": ""
                                                                }
                                                            }]
                                                        },
                                                        "finish_reason": Value::Null
                                                    }]
                                                });

                                                events.push(Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk))));
                                            }
                                        }
                                        "contentBlockDelta" => {
                                            let index = parsed_data["contentBlockIndex"].as_u64().unwrap_or(0) as usize;

                                            if let Some(text) = parsed_data["delta"]["text"].as_str() {
                                                // Text content delta
                                                let chunk = serde_json::json!({
                                                    "id": state.id,
                                                    "object": "chat.completion.chunk",
                                                    "created": state.created,
                                                    "model": state.model,
                                                    "choices": [{
                                                        "index": 0,
                                                        "delta": {
                                                            "content": text
                                                        },
                                                        "finish_reason": Value::Null
                                                    }]
                                                });

                                                events.push(Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk))));
                                            } else if let Some(tool_use_input) = parsed_data["delta"]["toolUse"]["input"].as_str() {
                                                // Tool use arguments delta
                                                if let Some((_tool_id, _name, ref mut args)) = state.tool_calls.get_mut(&index) {
                                                    args.push_str(tool_use_input);

                                                    let chunk = serde_json::json!({
                                                        "id": state.id,
                                                        "object": "chat.completion.chunk",
                                                        "created": state.created,
                                                        "model": state.model,
                                                        "choices": [{
                                                            "index": 0,
                                                            "delta": {
                                                                "tool_calls": [{
                                                                    "index": index,
                                                                    "function": {
                                                                        "arguments": tool_use_input
                                                                    }
                                                                }]
                                                            },
                                                            "finish_reason": Value::Null
                                                        }]
                                                    });

                                                    events.push(Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk))));
                                                }
                                            }
                                        }
                                        "contentBlockStop" => {
                                            // No output needed
                                        }
                                        "messageStop" => {
                                            let stop_reason = parsed_data["stopReason"].as_str().unwrap_or("end_turn");
                                            let finish_reason = match stop_reason {
                                                "end_turn" => "stop",
                                                "max_tokens" => "length",
                                                "tool_use" => "tool_calls",
                                                "stop_sequence" => "stop",
                                                "guardrail_intervened" | "content_filtered" => "content_filter",
                                                _ => "stop",
                                            };

                                            let chunk = serde_json::json!({
                                                "id": state.id,
                                                "object": "chat.completion.chunk",
                                                "created": state.created,
                                                "model": state.model,
                                                "choices": [{
                                                    "index": 0,
                                                    "delta": {},
                                                    "finish_reason": finish_reason
                                                }]
                                            });

                                            events.push(Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk))));
                                        }
                                        "metadata" => {
                                            // Could include usage info here if needed
                                        }
                                        _ => {}
                                    }
                                    }
                                }
                            }
                        } // end loop

                        events
                    }
                    Err(e) => {
                        vec![Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string(),
                        ))]
                    }
                }
            }
        })
        .flat_map(|events| futures::stream::iter(events))
        .chain(futures::stream::iter(vec![
            // Send [DONE] at the end
            Ok(bytes::Bytes::from("data: [DONE]\n\n"))
        ]))
}
