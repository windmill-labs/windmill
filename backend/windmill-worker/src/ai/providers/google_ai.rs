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

pub struct GoogleAIQueryBuilder;

impl GoogleAIQueryBuilder {
    pub fn new() -> Self {
        Self
    }


    async fn build_image_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        let is_imagen = args.model.contains("imagen");

        let request = if is_imagen {
            // For Imagen models, use simple prompt format
            GeminiImageRequest {
                instances: Some(vec![GeminiPredictContent {
                    prompt: args.user_message.trim().to_string(),
                }]),
                contents: None,
            }
        } else {
            // For Gemini models with image generation, build parts
            let mut parts = vec![GeminiPart::Text {
                text: args.user_message.trim().to_string(),
            }];

            if let Some(system_prompt) = args.system_prompt {
                parts.insert(
                    0,
                    GeminiPart::Text {
                        text: format!("SYSTEM PROMPT: {}", system_prompt.trim()),
                    },
                );
            }

            // Add input image if provided
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

                    parts.push(GeminiPart::InlineData {
                        inline_data: GeminiInlineData {
                            mime_type: "image/jpeg".to_string(),
                            data: base64_image,
                        },
                    });
                }
            }

            GeminiImageRequest {
                instances: None,
                contents: Some(vec![GeminiContent {
                    parts,
                }]),
            }
        };

        serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
    }
}

#[async_trait]
impl QueryBuilder for GoogleAIQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
        // Google AI supports tools only for text output
        matches!(output_type, OutputType::Text)
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        match args.output_type {
            OutputType::Text => {
                // For text output, use OpenAI-compatible format
                let openai_builder = super::openai::OpenAIQueryBuilder::new();
                openai_builder.build_request(args, client, workspace_id).await
            },
            OutputType::Image => self.build_image_request(args, client, workspace_id).await,
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        let url = response.url().path();
        
        // For chat completions (text), use OpenAI parser
        if url.contains("/chat/completions") {
            let openai_builder = super::openai::OpenAIQueryBuilder::new();
            return openai_builder.parse_response(response).await;
        }
        
        // Check if this is an image generation response
        if url.contains(":predict") || url.contains("imagen") {
            let response_text = response.text().await.map_err(|e| {
                Error::internal_err(format!("Failed to read response text: {}", e))
            })?;

            let gemini_response: GeminiImageResponse = serde_json::from_str(&response_text)
                .map_err(|e| {
                    Error::internal_err(format!(
                        "Failed to parse Gemini response: {}. Raw response: {}",
                        e, response_text
                    ))
                })?;

            // Find image data in response
            let image_data = gemini_response
                .candidates
                .as_ref()
                .and_then(|candidates| {
                    candidates.iter().find_map(|candidate| {
                        candidate.content.parts.iter().find_map(|part| {
                            part.inline_data.as_ref().map(|data| &data.data)
                        })
                    })
                })
                .or_else(|| {
                    gemini_response.predictions.as_ref().and_then(|predictions| {
                        predictions
                            .iter()
                            .find_map(|prediction| Some(&prediction.bytes_base64_encoded))
                    })
                });

            if let Some(base64_image) = image_data {
                Ok(ParsedResponse::Image {
                    base64_data: base64_image.clone(),
                })
            } else {
                Err(Error::internal_err(
                    "No image data received from Gemini".to_string(),
                ))
            }
        } else {
            // This should not happen as we use OpenAI format for text
            Err(Error::internal_err(
                "Unexpected text response in Google AI parser".to_string(),
            ))
        }
    }

    fn get_endpoint(&self, base_url: &str, output_type: &OutputType) -> String {
        match output_type {
            OutputType::Text => format!("{}/chat/completions", base_url), // Use OpenAI-compatible endpoint
            OutputType::Image => {
                // For image generation, the model name determines the endpoint
                base_url.to_string()
            }
        }
    }
}