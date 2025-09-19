use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::download_and_encode_s3_image,
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamingResponse},
    types::*,
};

// OpenAI-specific types
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAIFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAIToolCall {
    pub id: String,
    pub function: OpenAIFunction,
    pub r#type: String,
}

#[derive(Deserialize)]
pub struct OpenAIChoice {
    pub message: OpenAIMessage,
}

#[derive(Deserialize)]
pub struct OpenAIResponse {
    pub choices: Vec<OpenAIChoice>,
}

#[derive(Serialize)]
pub struct ImageGenerationTool {
    pub r#type: String,
    pub quality: Option<String>,
    pub background: Option<String>,
}

// Input content for image generation - supports both text and images
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageGenerationContent {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_image")]
    InputImage { image_url: String },
}

#[derive(Serialize)]
pub struct ImageGenerationMessage {
    pub role: String,
    pub content: Vec<ImageGenerationContent>,
}

#[derive(Serialize)]
pub struct ImageGenerationRequest<'a> {
    pub model: &'a str,
    pub input: Vec<ImageGenerationMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<&'a str>,
    pub tools: Vec<ImageGenerationTool>,
}

#[derive(Deserialize)]
pub struct OpenAIImageResponse {
    pub output: Vec<OpenAIImageOutput>,
}

#[derive(Deserialize)]
pub struct OpenAIImageOutput {
    pub r#type: String, // Expected to be "image_generation_call"
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>, // Base64 encoded image, None if not completed
}

#[derive(Serialize)]
pub struct OpenAIRequest<'a> {
    pub model: &'a str,
    pub messages: &'a [OpenAIMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<&'a [ToolDef]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

#[derive(Serialize)]
pub struct OpenAIStreamingRequest<'a> {
    pub model: &'a str,
    pub messages: &'a [OpenAIMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<&'a [ToolDef]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    pub stream: bool,
}

pub struct OpenAIQueryBuilder {
    provider_kind: AIProvider,
}

impl OpenAIQueryBuilder {
    pub fn new(provider_kind: AIProvider) -> Self {
        Self { provider_kind }
    }

    pub async fn prepare_messages_for_api(
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
                                    let (mime_type, image_bytes) = download_and_encode_s3_image(
                                        s3_object,
                                        client,
                                        workspace_id,
                                    )
                                    .await?;
                                    prepared_content.push(ContentPart::ImageUrl {
                                        image_url: ImageUrlData {
                                            url: format!(
                                                "data:{};base64,{}",
                                                mime_type, image_bytes
                                            ),
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
        let prepared_messages = self
            .prepare_messages_for_api(args.messages, client, workspace_id)
            .await?;

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

    async fn build_text_streaming_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        let prepared_messages = self
            .prepare_messages_for_api(args.messages, client, workspace_id)
            .await?;

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

        let request = OpenAIStreamingRequest {
            model: args.model,
            messages: &prepared_messages,
            tools: args.tools,
            temperature: args.temperature,
            max_completion_tokens: args.max_tokens,
            response_format,
            stream: true,
        };

        serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize streaming request: {}", e)))
    }

    async fn build_image_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // Build content array with text and optional image
        let mut content =
            vec![ImageGenerationContent::InputText { text: args.user_message.to_string() }];

        // Add images if provided
        if let Some(images) = args.images {
            for image in images.iter() {
                if !image.s3.is_empty() {
                    let (mime_type, image_bytes) =
                        download_and_encode_s3_image(image, client, workspace_id).await?;
                    content.push(ImageGenerationContent::InputImage {
                        image_url: format!("data:{};base64,{}", mime_type, image_bytes),
                    });
                }
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
            input: vec![ImageGenerationMessage { role: "user".to_string(), content }],
            instructions: args.system_prompt,
            tools,
        };

        serde_json::to_string(&image_request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
    }
}

#[async_trait]
impl QueryBuilder for OpenAIQueryBuilder {
    fn supports_tools_with_output_type(&self, _output_type: &OutputType) -> bool {
        // OpenAI supports tools for both text and image output
        true
    }

    fn supports_streaming(&self) -> bool {
        // OpenAI supports streaming for text output
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

    async fn build_streaming_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        match args.output_type {
            OutputType::Text => self.build_text_streaming_request(args, client, workspace_id).await,
            OutputType::Image => {
                // Image generation doesn't support streaming, fall back to regular request
                self.build_image_request(args, client, workspace_id).await
            }
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // Check if this is an image response
        let url = response.url().path();
        if url.contains("/responses") {
            // Parse image generation response
            let response_text = response
                .text()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to read response text: {}", e)))?;

            let image_response: OpenAIImageResponse = serde_json::from_str(&response_text)
                .map_err(|e| {
                    Error::internal_err(format!(
                        "Failed to parse OpenAI image response: {}. Raw response: {}",
                        e, response_text
                    ))
                })?;

            // Find the first completed image generation output
            let image_generation_call = image_response
                .output
                .iter()
                .find(|output| {
                    output.r#type == "image_generation_call" && output.status == "completed"
                })
                .and_then(|output| output.result.as_ref());

            if let Some(base64_image) = image_generation_call {
                Ok(ParsedResponse::Image { base64_data: base64_image.clone() })
            } else {
                Err(Error::internal_err(
                    "No completed image output received from OpenAI".to_string(),
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

    fn parse_streaming_response(&self, response: reqwest::Response) -> Result<StreamingResponse, Error> {
        // Use the default implementation which just wraps the response
        Ok(StreamingResponse { response })
    }

    fn get_endpoint(&self, base_url: &str, model: &str, output_type: &OutputType) -> String {
        let path = match output_type {
            OutputType::Text => "chat/completions",
            OutputType::Image => "responses",
        };

        if self.provider_kind.is_azure_openai(base_url) {
            AIProvider::build_azure_openai_url(base_url, model, path)
        } else {
            format!("{}/{}", base_url, path)
        }
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        base_url: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        if self.provider_kind.is_azure_openai(base_url) {
            vec![("api-key", api_key.to_string())]
        } else {
            vec![("Authorization", format!("Bearer {}", api_key))]
        }
    }
}
