use async_trait::async_trait;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json;
use windmill_common::{
    client::AuthedClient,
    error::Error,
};

use crate::ai::{
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder},
    types::*,
};

// Gemini-specific structures that include role
#[derive(Serialize)]
struct GeminiContentWithRole {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContentWithRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_config: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxOutputTokens")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "responseMimeType")]
    response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "responseSchema")]
    response_schema: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct GeminiTool {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "promptFeedback")]
    prompt_feedback: Option<serde_json::Value>,
}

pub struct GoogleAIQueryBuilder;

impl GoogleAIQueryBuilder {
    pub fn new() -> Self {
        Self
    }

    async fn prepare_contents_for_api(
        &self,
        messages: &[OpenAIMessage],
        system_prompt: Option<&str>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<Vec<GeminiContentWithRole>, Error> {
        let mut contents = Vec::new();
        let mut current_role: Option<&str> = None;
        let mut current_parts = Vec::new();

        // Add system prompt if provided
        if let Some(system_prompt) = system_prompt {
            current_parts.push(GeminiPart::Text {
                text: format!("SYSTEM PROMPT: {}", system_prompt),
            });
        }

        for message in messages {
            let role = match message.role.as_str() {
                "system" => "user", // Gemini doesn't have system role, convert to user
                "assistant" => "model",
                _ => "user",
            };

            // If role changes, save current content and start new one
            if let Some(prev_role) = current_role {
                if prev_role != role {
                    if !current_parts.is_empty() {
                        contents.push(GeminiContentWithRole {
                            role: prev_role.to_string(),
                            parts: current_parts.clone(),
                        });
                        current_parts.clear();
                    }
                }
            }
            current_role = Some(role);

            // Add message content to parts
            if let Some(content) = &message.content {
                match content {
                    OpenAIContent::Text(text) => {
                        current_parts.push(GeminiPart::Text { text: text.clone() });
                    }
                    OpenAIContent::Parts(parts) => {
                        for part in parts {
                            match part {
                                ContentPart::Text { text } => {
                                    current_parts.push(GeminiPart::Text { text: text.clone() });
                                }
                                ContentPart::ImageUrl { image_url } => {
                                    // Extract base64 data from data URL
                                    if let Some(base64_start) = image_url.url.find("base64,") {
                                        let base64_data = &image_url.url[base64_start + 7..];
                                        let mime_type = if image_url.url.starts_with("data:image/jpeg") {
                                            "image/jpeg"
                                        } else if image_url.url.starts_with("data:image/png") {
                                            "image/png"
                                        } else if image_url.url.starts_with("data:image/gif") {
                                            "image/gif"
                                        } else if image_url.url.starts_with("data:image/webp") {
                                            "image/webp"
                                        } else {
                                            "image/png"
                                        };

                                        current_parts.push(GeminiPart::InlineData {
                                            inline_data: GeminiInlineData {
                                                mime_type: mime_type.to_string(),
                                                data: base64_data.to_string(),
                                            },
                                        });
                                    }
                                }
                                ContentPart::S3Object { s3_object } => {
                                    // Download and convert S3 image
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

                                    current_parts.push(GeminiPart::InlineData {
                                        inline_data: GeminiInlineData {
                                            mime_type: mime_type.to_string(),
                                            data: base64_data,
                                        },
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // Handle tool calls
            if let Some(tool_calls) = &message.tool_calls {
                for tool_call in tool_calls {
                    current_parts.push(GeminiPart::FunctionCall {
                        function_call: GeminiFunctionCall {
                            name: tool_call.function.name.clone(),
                            args: serde_json::from_str(&tool_call.function.arguments)
                                .unwrap_or(serde_json::json!({})),
                        },
                    });
                }
            }

            // Handle tool results
            if message.role == "tool" {
                if let (Some(OpenAIContent::Text(text)), Some(tool_call_id)) = 
                    (&message.content, &message.tool_call_id) {
                    current_parts.push(GeminiPart::FunctionResponse {
                        function_response: GeminiFunctionResponse {
                            name: tool_call_id.clone(), // Using ID as name for matching
                            response: serde_json::json!({ "result": text }),
                        },
                    });
                }
            }
        }

        // Add final content
        if let Some(role) = current_role {
            if !current_parts.is_empty() {
                contents.push(GeminiContentWithRole {
                    role: role.to_string(),
                    parts: current_parts,
                });
            }
        }

        Ok(contents)
    }

    async fn build_text_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        let contents = self.prepare_contents_for_api(
            args.messages, 
            args.system_prompt,
            client,
            workspace_id
        ).await?;

        let mut request = GeminiRequest {
            contents,
            generation_config: Some(GeminiGenerationConfig {
                temperature: args.temperature,
                max_output_tokens: args.max_tokens,
                response_mime_type: None,
                response_schema: None,
            }),
            safety_settings: None,
            tools: args.tools.map(|tools| {
                vec![GeminiTool {
                    function_declarations: tools.iter().map(|t| {
                        GeminiFunctionDeclaration {
                            name: t.function.name.clone(),
                            description: t.function.description.clone().unwrap_or_default(),
                            parameters: serde_json::from_str(t.function.parameters.get())
                                .unwrap_or(serde_json::json!({})),
                        }
                    }).collect(),
                }]
            }),
            tool_config: None,
        };

        // Handle structured output via response schema
        let has_output_properties = args
            .output_schema
            .and_then(|schema| schema.properties.as_ref())
            .map(|props| !props.is_empty())
            .unwrap_or(false);

        if has_output_properties && args.output_schema.is_some() {
            let schema = args.output_schema.unwrap();
            let strict_schema = schema.clone().make_strict();
            
            if let Some(ref mut config) = request.generation_config {
                config.response_mime_type = Some("application/json".to_string());
                config.response_schema = Some(serde_json::to_value(&strict_schema)
                    .unwrap_or(serde_json::json!({})));
            }
        }

        serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
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
            OutputType::Text => self.build_text_request(args, client, workspace_id).await,
            OutputType::Image => self.build_image_request(args, client, workspace_id).await,
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        let url = response.url().path();
        
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
            // Parse text/chat response
            let gemini_response: GeminiResponse = response
                .json()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to parse response: {}", e)))?;

            let candidate = gemini_response
                .candidates
                .into_iter()
                .next()
                .ok_or_else(|| Error::internal_err("No response candidate from Gemini"))?;

            let mut content = None;
            let mut tool_calls = vec![];

            for part in candidate.content.parts {
                // Parse the part based on its content
                if let Some(text) = part.text.as_ref() {
                    content = Some(text.clone());
                } else if let Some(function_call) = part.function_call.as_ref() {
                    tool_calls.push(OpenAIToolCall {
                        id: format!("call_{}", ulid::Ulid::new()),
                        r#type: "function".to_string(),
                        function: OpenAIFunction {
                            name: function_call.name.clone(),
                            arguments: function_call.args.to_string(),
                        },
                    });
                }
            }

            // If response has structured JSON content, extract it
            if let Some(ref text) = content {
                if text.trim().starts_with('{') && text.trim().ends_with('}') {
                    // Content is already JSON, keep as is
                } else if gemini_response.prompt_feedback.is_some() {
                    // Check if there's structured data in usage metadata or elsewhere
                }
            }

            Ok(ParsedResponse::Text {
                content,
                tool_calls,
            })
        }
    }

    fn get_endpoint(&self, base_url: &str, output_type: &OutputType) -> String {
        match output_type {
            OutputType::Text => base_url.to_string(), // Base URL already includes the full path
            OutputType::Image => {
                // For image generation, the model name determines the endpoint
                base_url.to_string()
            }
        }
    }
}