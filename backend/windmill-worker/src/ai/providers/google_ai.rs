use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::download_and_encode_s3_image,
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    types::*,
};

// Google AI/Gemini-specific types
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeminiInlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum GeminiPart {
    Text { text: String },
    InlineData { inline_data: GeminiInlineData },
    FunctionCall { function_call: GeminiFunctionCall },
    FunctionResponse { function_response: GeminiFunctionResponse },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GeminiFunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GeminiFunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

#[derive(Serialize)]
pub struct GeminiContent {
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
pub struct GeminiImageRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<GeminiContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instances: Option<Vec<GeminiPredictContent>>,
}

#[derive(Serialize)]
pub struct GeminiPredictContent {
    pub prompt: String,
}

#[derive(Deserialize)]
pub struct GeminiImageResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<GeminiCandidate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predictions: Option<Vec<GeminiPredictCandidate>>,
}

#[derive(Deserialize)]
pub struct GeminiCandidate {
    pub content: GeminiResponseContent,
}

#[derive(Deserialize)]
pub struct GeminiPredictCandidate {
    #[serde(rename = "bytesBase64Encoded")]
    pub bytes_base64_encoded: String, // base64 encoded image
}

#[derive(Deserialize)]
pub struct GeminiResponseContent {
    pub parts: Vec<GeminiResponsePart>,
}

#[derive(Deserialize)]
pub struct GeminiResponsePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    pub text: Option<String>,
    #[serde(rename = "inlineData", skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<GeminiInlineData>,
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    pub function_call: Option<GeminiFunctionCall>,
}

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
            let mut parts = vec![GeminiPart::Text { text: args.user_message.trim().to_string() }];

            if let Some(system_prompt) = args.system_prompt {
                parts.insert(
                    0,
                    GeminiPart::Text { text: format!("SYSTEM PROMPT: {}", system_prompt.trim()) },
                );
            }

            // Add input images if provided
            if let Some(images) = args.images {
                for image in images.iter() {
                    if !image.s3.is_empty() {
                        let (mime_type, image_bytes) =
                            download_and_encode_s3_image(image, client, workspace_id).await?;
                        parts.push(GeminiPart::InlineData {
                            inline_data: GeminiInlineData {
                                mime_type: mime_type,
                                data: image_bytes,
                            },
                        });
                    }
                }
            }

            GeminiImageRequest { instances: None, contents: Some(vec![GeminiContent { parts }]) }
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

    fn supports_streaming(&self) -> bool {
        // Google AI supports streaming for text output
        true
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
        stream: bool,
    ) -> Result<String, Error> {
        match args.output_type {
            OutputType::Text => {
                // For text output, use OpenAI-compatible format
                let openai_builder = super::openai::OpenAIQueryBuilder::new(AIProvider::GoogleAI);
                openai_builder
                    .build_request(args, client, workspace_id, stream)
                    .await
            }
            OutputType::Image => self.build_image_request(args, client, workspace_id).await,
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        let url = response.url().path();

        // For chat completions (text), use OpenAI parser
        if url.contains("/chat/completions") {
            let openai_builder = super::openai::OpenAIQueryBuilder::new(AIProvider::GoogleAI);
            return openai_builder.parse_response(response).await;
        }

        // Check if this is an image generation response
        if url.contains(":predict") || url.contains(":generateContent") {
            let response_text = response
                .text()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to read response text: {}", e)))?;

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
                        candidate
                            .content
                            .parts
                            .iter()
                            .find_map(|part| part.inline_data.as_ref().map(|data| &data.data))
                    })
                })
                .or_else(|| {
                    gemini_response
                        .predictions
                        .as_ref()
                        .and_then(|predictions| {
                            predictions
                                .iter()
                                .find_map(|prediction| Some(&prediction.bytes_base64_encoded))
                        })
                });

            if let Some(base64_image) = image_data {
                Ok(ParsedResponse::Image { base64_data: base64_image.clone() })
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

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        let openai_builder = super::openai::OpenAIQueryBuilder::new(AIProvider::GoogleAI);
        openai_builder
            .parse_streaming_response(response, stream_event_processor)
            .await
    }

    fn get_endpoint(
        &self,
        base_url: &str,
        model: &str,
        output_type: &OutputType,
        _stream: bool,
    ) -> String {
        match output_type {
            OutputType::Text => format!("{}/chat/completions", base_url), // Use OpenAI-compatible endpoint
            OutputType::Image => {
                // For image generation, build the full URL with model name
                let url_suffix = if model.contains("imagen") {
                    "predict"
                } else {
                    "generateContent"
                };
                format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:{}",
                    model, url_suffix
                )
            }
        }
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        _base_url: &str,
        output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        match output_type {
            OutputType::Text => {
                // For text output, use Bearer token (OpenAI-compatible)
                vec![("Authorization", format!("Bearer {}", api_key))]
            }
            OutputType::Image => {
                // For image generation, use Google API key header
                vec![("x-goog-api-key", api_key.to_string())]
            }
        }
    }
}
