use async_trait::async_trait;
use serde_json;
use windmill_common::{
    client::AuthedClient,
    error::Error,
};

use crate::ai::{
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder},
    types::*,
};

pub struct AnthropicQueryBuilder;

impl AnthropicQueryBuilder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl QueryBuilder for AnthropicQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        // Anthropic only supports tools for text output
        matches!(output_type, OutputType::Text)
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        _client: &AuthedClient,
        _workspace_id: &str,
    ) -> Result<String, Error> {
        match args.output_type {
            OutputType::Text => {
                // Build Anthropic-style request
                let mut messages = vec![];
                let mut system_content = String::new();

                // Extract system messages and convert to Anthropic format
                for msg in args.messages {
                    match msg.role.as_str() {
                        "system" => {
                            if let Some(OpenAIContent::Text(text)) = &msg.content {
                                if !system_content.is_empty() {
                                    system_content.push_str("\n\n");
                                }
                                system_content.push_str(text);
                            }
                        }
                        "user" => {
                            let mut content = vec![];
                            if let Some(msg_content) = &msg.content {
                                match msg_content {
                                    OpenAIContent::Text(text) => {
                                        content.push(AnthropicContent::Text {
                                            text: text.clone(),
                                        });
                                    }
                                    OpenAIContent::Parts(parts) => {
                                        for part in parts {
                                            match part {
                                                ContentPart::Text { text } => {
                                                    content.push(AnthropicContent::Text {
                                                        text: text.clone(),
                                                    });
                                                }
                                                ContentPart::ImageUrl { image_url } => {
                                                    // Extract base64 data from data URL
                                                    if let Some(base64_start) = image_url.url.find("base64,") {
                                                        let base64_data = &image_url.url[base64_start + 7..];
                                                        let media_type = if image_url.url.starts_with("data:image/jpeg") {
                                                            "image/jpeg"
                                                        } else if image_url.url.starts_with("data:image/png") {
                                                            "image/png"
                                                        } else if image_url.url.starts_with("data:image/gif") {
                                                            "image/gif"
                                                        } else if image_url.url.starts_with("data:image/webp") {
                                                            "image/webp"
                                                        } else {
                                                            "image/jpeg"
                                                        };

                                                        content.push(AnthropicContent::Image {
                                                            source: AnthropicImageSource {
                                                                r#type: "base64".to_string(),
                                                                media_type: media_type.to_string(),
                                                                data: base64_data.to_string(),
                                                            },
                                                        });
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                            messages.push(AnthropicMessage {
                                role: "user".to_string(),
                                content,
                            });
                        }
                        "assistant" => {
                            if let Some(OpenAIContent::Text(text)) = &msg.content {
                                messages.push(AnthropicMessage {
                                    role: "assistant".to_string(),
                                    content: vec![AnthropicContent::Text {
                                        text: text.clone(),
                                    }],
                                });
                            } else if let Some(tool_calls) = &msg.tool_calls {
                                // Convert tool calls to Anthropic format
                                let mut content = vec![];
                                for tool_call in tool_calls {
                                    content.push(AnthropicContent::ToolUse {
                                        id: tool_call.id.clone(),
                                        name: tool_call.function.name.clone(),
                                        input: serde_json::from_str(&tool_call.function.arguments)
                                            .unwrap_or(serde_json::json!({})),
                                    });
                                }
                                messages.push(AnthropicMessage {
                                    role: "assistant".to_string(),
                                    content,
                                });
                            }
                        }
                        "tool" => {
                            // Tool results in Anthropic format
                            if let (Some(OpenAIContent::Text(text)), Some(tool_call_id)) = 
                                (&msg.content, &msg.tool_call_id) {
                                messages.push(AnthropicMessage {
                                    role: "user".to_string(),
                                    content: vec![AnthropicContent::ToolResult {
                                        tool_use_id: tool_call_id.clone(),
                                        content: text.clone(),
                                    }],
                                });
                            }
                        }
                        _ => {}
                    }
                }

                let mut request = AnthropicRequest {
                    model: args.model.to_string(),
                    messages,
                    max_tokens: args.max_tokens.unwrap_or(4096),
                    temperature: args.temperature,
                    system: if system_content.is_empty() { None } else { Some(system_content) },
                    tools: args.tools.map(|tools| {
                        tools.iter().map(|t| AnthropicTool {
                            name: t.function.name.clone(),
                            description: t.function.description.clone(),
                            input_schema: t.function.parameters.get().into(),
                        }).collect()
                    }),
                    tool_choice: None,
                };

                // Handle forced tool use for structured output
                if let Some(tools) = &request.tools {
                    if let Some(structured_tool) = tools.iter()
                        .find(|t| t.name.starts_with("structured_output")) {
                        request.tool_choice = Some(AnthropicToolChoice::Tool {
                            name: structured_tool.name.clone(),
                        });
                    }
                }

                serde_json::to_string(&request)
                    .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
            }
            OutputType::Image => {
                Err(Error::internal_err(
                    "Anthropic does not support image generation".to_string(),
                ))
            }
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to parse response: {}", e)))?;

        let mut content = None;
        let mut tool_calls = vec![];

        for content_item in anthropic_response.content {
            match content_item {
                AnthropicContent::Text { text } => {
                    content = Some(text);
                }
                AnthropicContent::ToolUse { id, name, input } => {
                    tool_calls.push(OpenAIToolCall {
                        id,
                        r#type: "function".to_string(),
                        function: OpenAIFunction {
                            name,
                            arguments: input.to_string(),
                        },
                    });
                }
                _ => {}
            }
        }

        Ok(ParsedResponse::Text {
            content,
            tool_calls,
        })
    }

    fn get_endpoint(&self, base_url: &str, _output_type: &OutputType) -> String {
        format!("{}/messages", base_url)
    }
}