use async_trait::async_trait;
use serde_json;
use windmill_common::{
    client::AuthedClient,
    error::Error,
};

use crate::ai::{
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder},
    types::*,
    providers::openai::OpenAIQueryBuilder,
};

pub struct OpenRouterQueryBuilder {
    // OpenRouter uses OpenAI-compatible API, so we delegate most work to OpenAI builder
    openai_builder: OpenAIQueryBuilder,
}

impl OpenRouterQueryBuilder {
    pub fn new() -> Self {
        Self {
            openai_builder: OpenAIQueryBuilder::new(),
        }
    }
}

#[async_trait]
impl QueryBuilder for OpenRouterQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        // OpenRouter supports tools for both text and image output (via OpenAI-compatible API)
        true
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        match args.output_type {
            OutputType::Text => {
                // Use OpenAI builder for text requests
                self.openai_builder.build_request(args, client, workspace_id).await
            }
            OutputType::Image => {
                // For image generation, OpenRouter uses a specific format
                let mut messages = Vec::new();

                // Add system message if provided
                if let Some(system_prompt) = args.system_prompt {
                    messages.push(OpenRouterImageMessage {
                        role: "system".to_string(),
                        content: system_prompt.to_string(),
                    });
                }

                // Add user message
                messages.push(OpenRouterImageMessage {
                    role: "user".to_string(),
                    content: args.user_message.to_string(),
                });

                let request = OpenRouterImageRequest {
                    model: args.model,
                    messages,
                    modalities: vec!["image", "text"],
                };

                serde_json::to_string(&request)
                    .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
            }
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        let url = response.url().path();
        
        // Check if this might be an image response based on the request
        // OpenRouter returns images in chat completion format
        if url.contains("/chat/completions") {
            let openrouter_response: serde_json::Value = response
                .json()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to parse response: {}", e)))?;

            // Check if response contains images
            if let Some(choices) = openrouter_response.get("choices").and_then(|v| v.as_array()) {
                if let Some(first_choice) = choices.first() {
                    if let Some(message) = first_choice.get("message") {
                        if let Some(images) = message.get("images").and_then(|v| v.as_array()) {
                            if let Some(first_image) = images.first() {
                                if let Some(image_url) = first_image.get("image_url").and_then(|v| v.get("url")).and_then(|v| v.as_str()) {
                                    // Extract base64 data from data URL
                                    if let Some(base64_start) = image_url.find("base64,") {
                                        let base64_data = &image_url[base64_start + 7..];
                                        return Ok(ParsedResponse::Image {
                                            base64_data: base64_data.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // If no images found, parse as regular OpenAI response
            let openai_response: OpenAIResponse = serde_json::from_value(openrouter_response)
                .map_err(|e| Error::internal_err(format!("Failed to parse as OpenAI response: {}", e)))?;

            let first_choice = openai_response
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| Error::internal_err("No response from API"))?;

            Ok(ParsedResponse::Text {
                content: first_choice.message.content.map(|c| match c {
                    OpenAIContent::Text(text) => text,
                    OpenAIContent::Parts(parts) => {
                        parts
                            .into_iter()
                            .filter_map(|part| match part {
                                ContentPart::Text { text } => Some(text),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                }),
                tool_calls: first_choice.message.tool_calls.unwrap_or_default(),
            })
        } else {
            // Fallback to OpenAI parser
            self.openai_builder.parse_response(response).await
        }
    }

    fn get_endpoint(&self, base_url: &str, output_type: &OutputType) -> String {
        // OpenRouter uses the same endpoint for both text and image generation
        format!("{}/chat/completions", base_url)
    }
}