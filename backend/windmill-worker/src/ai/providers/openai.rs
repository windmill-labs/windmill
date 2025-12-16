use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::download_and_encode_s3_image,
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder},
    types::*,
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAIFunction {
    pub name: String,
    pub arguments: String,
}

/// Google-specific extra content for thought signatures (Gemini 3 Pro / 2.5)
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct GoogleExtraContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

/// Extra content for provider-specific metadata (e.g., Google thought signatures)
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct ExtraContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google: Option<GoogleExtraContent>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAIToolCall {
    pub id: String,
    pub function: OpenAIFunction,
    pub r#type: String,
    /// Extra content for provider-specific metadata (e.g., Google Gemini thought signatures)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_content: Option<ExtraContent>,
}

// Responses API structures
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ResponsesMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<OpenAIContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Deserialize)]
pub struct ResponsesOutput {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>, // Base64 encoded image for image_generation_call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<ResponsesMessage>, // Message for message_call
}

#[derive(Deserialize)]
pub struct ResponsesApiResponse {
    pub output: Vec<ResponsesOutput>,
}

#[derive(Serialize)]
#[allow(dead_code)]
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
pub struct ResponsesApiRequest<'a> {
    pub model: &'a str,
    pub input: Vec<ImageGenerationMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<&'a str>,
    pub tools: Vec<serde_json::Value>, // Can be ImageGenerationTool, WebSearchTool, or ToolDef
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum ToolChoice {
    Auto,
    Required,
}

/// Query builder for OpenAI and Azure OpenAI using the Responses API
pub struct OpenAIQueryBuilder {
    provider_kind: AIProvider,
}

impl OpenAIQueryBuilder {
    pub fn new(provider_kind: AIProvider) -> Self {
        Self { provider_kind }
    }

    async fn build_text_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
        _stream: bool,
    ) -> Result<String, Error> {
        // Build input messages for responses API
        let mut input_messages = Vec::new();

        // Add system prompt if present
        if let Some(system_prompt) = args.system_prompt {
            if !system_prompt.is_empty() {
                input_messages.push(ImageGenerationMessage {
                    role: "system".to_string(),
                    content: vec![ImageGenerationContent::InputText { text: system_prompt.to_string() }],
                });
            }
        }

        // Add user message
        input_messages.push(ImageGenerationMessage {
            role: "user".to_string(),
            content: vec![ImageGenerationContent::InputText { text: args.user_message.to_string() }],
        });

        // Add images if provided
        if let Some(images) = args.images {
            for image in images.iter() {
                if !image.s3.is_empty() {
                    let (mime_type, image_bytes) =
                        download_and_encode_s3_image(image, client, workspace_id).await?;
                    if let Some(last_msg) = input_messages.last_mut() {
                        last_msg.content.push(ImageGenerationContent::InputImage {
                            image_url: format!("data:{};base64,{}", mime_type, image_bytes),
                        });
                    }
                }
            }
        }

        // Convert tools to JSON values
        let mut tools: Vec<serde_json::Value> = Vec::new();

        // Add websearch tool if enabled
        if args.has_websearch {
            tools.push(serde_json::json!({
                "type": "web_search"
            }));
        }

        if let Some(tool_defs) = args.tools {
            for tool in tool_defs {
                tools.push(serde_json::to_value(tool).map_err(|e| {
                    Error::internal_err(format!("Failed to serialize tool: {}", e))
                })?);
            }
        }

        let request = ResponsesApiRequest {
            model: args.model,
            input: input_messages,
            instructions: None, // System prompt is in messages now
            tools,
            temperature: args.temperature,
            max_completion_tokens: args.max_tokens,
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
        let tools: Vec<serde_json::Value> = vec![serde_json::json!({
            "type": "image_generation",
            "quality": "low"
        })];

        let request = ResponsesApiRequest {
            model: args.model,
            input: vec![ImageGenerationMessage { role: "user".to_string(), content }],
            instructions: args.system_prompt,
            tools,
            temperature: args.temperature,
            max_completion_tokens: args.max_tokens,
        };

        serde_json::to_string(&request)
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
        // Responses API doesn't support streaming yet (or has different streaming mechanism)
        false
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
                self.build_text_request(args, client, workspace_id, stream)
                    .await
            }
            OutputType::Image => self.build_image_request(args, client, workspace_id).await,
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // Both text and image use the same responses API format
        let response_text = response
            .text()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to read response text: {}", e)))?;

        let responses_response: ResponsesApiResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                Error::internal_err(format!(
                    "Failed to parse OpenAI responses API response: {}. Raw response: {}",
                    e, response_text
                ))
            })?;

        // Find the first completed output
        for output in responses_response.output {
            match output.r#type.as_str() {
                "image_generation_call" => {
                    if output.status.as_deref() == Some("completed") {
                        if let Some(base64_image) = output.result {
                            return Ok(ParsedResponse::Image { base64_data: base64_image });
                        }
                    }
                }
                "message_call" => {
                    if let Some(message) = output.message {
                        return Ok(ParsedResponse::Text {
                            content: message.content.map(|c| match c {
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
                            tool_calls: message.tool_calls.unwrap_or_default(),
                            events_str: None,
                        });
                    }
                }
                _ => continue,
            }
        }

        Err(Error::internal_err(
            "No completed output received from OpenAI Responses API".to_string(),
        ))
    }

    fn get_endpoint(
        &self,
        base_url: &str,
        _model: &str,
        _output_type: &OutputType,
        _stream: bool,
    ) -> String {
        // Always use responses endpoint for OpenAI/Azure
        if self.provider_kind.is_azure_openai(base_url) {
            AIProvider::build_azure_openai_url(base_url, "responses")
        } else {
            format!("{}/responses", base_url)
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
