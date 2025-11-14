// Copyright Windmill Labs. All rights reserved.

//! OpenAI to AWS Bedrock message format converters
//!
//! This module provides bidirectional conversion between OpenAI-compatible
//! message formats (used throughout Windmill) and AWS Bedrock Converse API formats.

use crate::error::{Error, Result};
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, InferenceConfiguration, Message, SystemContentBlock, Tool,
    ToolInputSchema, ToolSpecification,
};
use serde_json::value::RawValue;

// Re-export types from worker AI module when they become available in common
// For now, we'll define minimal types here and convert when integrated
use serde::{Deserialize, Serialize};

/// Simplified OpenAI message for conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleOpenAIMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<SimpleToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleToolCall {
    pub id: String,
    pub r#type: String,
    pub function: SimpleFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimpleToolDef {
    pub r#type: String,
    pub function: SimpleToolDefFunction,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimpleToolDefFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: Box<RawValue>,
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
                serde_json::Number::from_f64(num.to_f64_lossy()).unwrap_or(serde_json::Number::from(0)),
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
    messages: &[SimpleOpenAIMessage],
) -> Result<(Vec<Message>, Vec<SystemContentBlock>)> {
    let mut bedrock_messages = Vec::new();
    let mut system_prompts = Vec::new();

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                // Extract system messages separately
                if let Some(ref content) = msg.content {
                    system_prompts.push(SystemContentBlock::Text(content.clone()));
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

/// Convert a single OpenAI message to Bedrock Message
fn convert_message(msg: &SimpleOpenAIMessage) -> Result<Message> {
    let role = match msg.role.as_str() {
        "user" => ConversationRole::User,
        "assistant" => ConversationRole::Assistant,
        _ => {
            return Err(Error::internal_err(format!("Unsupported role: {}", msg.role)));
        }
    };

    let mut content_blocks = Vec::new();

    // Handle text content
    if let Some(ref content) = msg.content {
        if !content.is_empty() {
            content_blocks.push(ContentBlock::Text(content.clone()));
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
fn convert_tool_call_to_content(tool_call: &SimpleToolCall) -> Result<ContentBlock> {
    // Parse arguments string to JSON value, then convert to Document
    let input: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
        .map_err(|e| Error::internal_err(format!("Invalid tool arguments: {}", e)))?;

    let input_doc = json_to_document(input);

    Ok(ContentBlock::ToolUse(
        aws_sdk_bedrockruntime::types::ToolUseBlock::builder()
            .tool_use_id(&tool_call.id)
            .name(&tool_call.function.name)
            .input(input_doc)
            .build()
            .map_err(|e| Error::internal_err(format!("Failed to build tool use: {}", e)))?,
    ))
}

/// Convert tool result message to Bedrock format
fn convert_tool_message(msg: &SimpleOpenAIMessage) -> Result<Message> {
    let tool_call_id = msg
        .tool_call_id
        .as_ref()
        .ok_or_else(|| Error::internal_err("Tool message missing tool_call_id"))?;

    let content_str = msg.content.as_ref().map(|s| s.as_str()).unwrap_or("");

    // Try to parse as JSON, otherwise use text
    let tool_result_content =
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(content_str) {
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
pub fn openai_tools_to_bedrock(tools: &[SimpleToolDef]) -> Result<Vec<Tool>> {
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
) -> Result<(Option<String>, Vec<SimpleToolCall>)> {
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
                        let arguments =
                            serde_json::to_string(&input_value).unwrap_or_else(|_| "{}".to_string());

                        tool_calls.push(SimpleToolCall {
                            id: tool_use.tool_use_id().to_string(),
                            r#type: "function".to_string(),
                            function: SimpleFunction {
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

    let content = if text_content.is_empty() { None } else { Some(text_content) };

    Ok((content, tool_calls))
}

/// Extract text delta from Bedrock stream event
pub fn bedrock_stream_event_to_text(
    event: &aws_sdk_bedrockruntime::types::ConverseStreamOutput,
) -> Option<String> {
    use aws_sdk_bedrockruntime::types::ConverseStreamOutput;

    match event {
        ConverseStreamOutput::ContentBlockDelta(delta) => {
            delta.delta().and_then(|d| d.as_text().ok()).map(|s| s.to_string())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_to_bedrock_simple_user_message() {
        let messages = vec![SimpleOpenAIMessage {
            role: "user".to_string(),
            content: Some("Hello, world!".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        let (bedrock_messages, system_prompts) = openai_messages_to_bedrock(&messages).unwrap();

        assert_eq!(bedrock_messages.len(), 1);
        assert_eq!(system_prompts.len(), 0);
        assert!(matches!(
            bedrock_messages[0].role(),
            &ConversationRole::User
        ));
    }

    #[test]
    fn test_openai_to_bedrock_with_system_prompt() {
        let messages = vec![
            SimpleOpenAIMessage {
                role: "system".to_string(),
                content: Some("You are a helpful assistant.".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            SimpleOpenAIMessage {
                role: "user".to_string(),
                content: Some("Hello!".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let (bedrock_messages, system_prompts) = openai_messages_to_bedrock(&messages).unwrap();

        assert_eq!(bedrock_messages.len(), 1);
        assert_eq!(system_prompts.len(), 1);
    }

    #[test]
    fn test_create_inference_config_temperature_only() {
        let config = create_inference_config(Some(0.7), None);
        assert!(config.is_some());
    }

    #[test]
    fn test_create_inference_config_both_params() {
        let config = create_inference_config(Some(0.7), Some(1000));
        assert!(config.is_some());
    }

    #[test]
    fn test_create_inference_config_none() {
        let config = create_inference_config(None, None);
        assert!(config.is_none());
    }
}
