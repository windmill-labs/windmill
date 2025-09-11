use async_trait::async_trait;
use base64::Engine;
use serde_json;
use windmill_common::{
    client::AuthedClient,
    error::Error,
};

use crate::ai::{
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder},
    types::*,
};

pub struct OpenAIQueryBuilder;

impl OpenAIQueryBuilder {
    pub fn new() -> Self {
        Self
    }

    async fn prepare_messages_for_api(
        &self,
        messages: &[OpenAIMessage],
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<Vec<OpenAIMessage>, Error> {
        let mut prepared_messages = Vec::new();

        for message in messages {
            let mut prepared_message = message.clone();

            if let Some(content) = &message.content {
                match content {
                    OpenAIContent::Text(text) => {
                        prepared_message.content = Some(OpenAIContent::Text(text.clone()));
                    }
                    OpenAIContent::Parts(parts) => {
                        let mut prepared_content = Vec::new();

                        for part in parts {
                            match part {
                                ContentPart::S3Object { s3_object } => {
                                    // Convert S3Object to base64 image URL
                                    let image_bytes = client
                                        .download_s3_file(
                                            workspace_id,
                                            &s3_object.s3,
                                            s3_object.storage.clone(),
                                        )
                                        .await
                                        .map_err(|e| {
                                            Error::internal_err(format!(
                                                "Failed to download S3 image: {}",
                                                e
                                            ))
                                        })?;

                                    let base64_data = base64::engine::general_purpose::STANDARD
                                        .encode(&image_bytes);

                                    // Determine MIME type based on file extension
                                    let mime_type = if s3_object.s3.ends_with(".jpg")
                                        || s3_object.s3.ends_with(".jpeg")
                                    {
                                        "image/jpeg"
                                    } else if s3_object.s3.ends_with(".gif") {
                                        "image/gif"
                                    } else if s3_object.s3.ends_with(".webp") {
                                        "image/webp"
                                    } else {
                                        "image/png"
                                    };

                                    prepared_content.push(ContentPart::ImageUrl {
                                        image_url: ImageUrlData {
                                            url: format!("data:{};base64,{}", mime_type, base64_data),
                                        },
                                    });
                                }
                                other => {
                                    // Keep Text and ImageUrl as-is
                                    prepared_content.push(other.clone());
                                }
                            }
                        }

                        prepared_message.content = Some(OpenAIContent::Parts(prepared_content));
                    }
                }
            }

            prepared_messages.push(prepared_message);
        }

        Ok(prepared_messages)
    }

    async fn build_text_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        let prepared_messages = self.prepare_messages_for_api(args.messages, client, workspace_id).await?;

        // Check if we need to add response_format for structured output
        let has_output_properties = args
            .output_schema
            .and_then(|schema| schema.properties.as_ref())
            .map(|props| !props.is_empty())
            .unwrap_or(false);

        let response_format = if has_output_properties && args.output_schema.is_some() {
            let schema = args.output_schema.unwrap();
            let strict_schema = schema.clone().make_strict();
            Some(ResponseFormat {
                r#type: "json_schema".to_string(),
                json_schema: JsonSchemaFormat {
                    name: "structured_output".to_string(),
                    schema: strict_schema,
                    strict: Some(true),
                },
            })
        } else {
            None
        };

        let request = OpenAIRequest {
            model: args.model,
            messages: &prepared_messages,
            tools: args.tools,
            temperature: args.temperature,
            max_completion_tokens: args.max_tokens,
            response_format,
        };

        serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
    }

    async fn build_image_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // Build content array with text and optional image
        let mut content = vec![ImageGenerationContent::InputText {
            text: args.user_message.to_string(),
        }];

        // Add image if provided
        if let Some(image) = args.image {
            if !image.s3.is_empty() {
                let image_bytes = client
                    .download_s3_file(workspace_id, &image.s3, image.storage.clone())
                    .await
                    .map_err(|e| {
                        Error::internal_err(format!("Failed to download S3 image: {}", e))
                    })?;

                let base64_image =
                    base64::engine::general_purpose::STANDARD.encode(&image_bytes);
                let image_data_url = format!("data:image/jpeg;base64,{}", base64_image);

                content.push(ImageGenerationContent::InputImage {
                    image_url: image_data_url,
                });
            }
        }

        // Build the request with tools if provided
        let tools = vec![ImageGenerationTool {
            r#type: "image_generation".to_string(),
            quality: Some("low".to_string()),
            background: None,
        }];

        // TODO: OpenAI's image generation API doesn't support custom tools in the same way as chat completions
        // This would require a different approach, potentially using chat completions with image output
        // For now, we'll use the standard image generation without custom tools

        let image_request = ImageGenerationRequest {
            model: args.model,
            input: vec![ImageGenerationMessage {
                role: "user".to_string(),
                content,
            }],
            instructions: args.system_prompt,
            tools,
        };

        serde_json::to_string(&image_request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
    }
}

#[async_trait]
impl QueryBuilder for OpenAIQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        // OpenAI supports tools for both text and image output
        true
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

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // Check if this is an image response
        let url = response.url().path();
        if url.contains("/responses") {
            // Parse image generation response
            let image_response: OpenAIImageResponse = response
                .json()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to parse image response: {}", e)))?;

            // Find the first image generation output
            let image_generation_call = image_response
                .output
                .iter()
                .find(|output| output.r#type == "image_generation_call")
                .map(|output| &output.result);

            if let Some(base64_image) = image_generation_call {
                Ok(ParsedResponse::Image {
                    base64_data: base64_image.clone(),
                })
            } else {
                Err(Error::internal_err(
                    "No image output received from OpenAI".to_string(),
                ))
            }
        } else {
            // Parse text/chat completion response
            let openai_response: OpenAIResponse = response
                .json()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to parse response: {}", e)))?;

            let first_choice = openai_response
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| Error::internal_err("No response from API"))?;

            Ok(ParsedResponse::Text {
                content: first_choice.message.content.map(|c| match c {
                    OpenAIContent::Text(text) => text,
                    OpenAIContent::Parts(parts) => {
                        // Extract text from parts
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
        }
    }

    fn get_endpoint(&self, base_url: &str, output_type: &OutputType) -> String {
        match output_type {
            OutputType::Text => format!("{}/chat/completions", base_url),
            OutputType::Image => format!("{}/responses", base_url),
        }
    }
}