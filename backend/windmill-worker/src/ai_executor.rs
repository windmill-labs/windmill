use async_recursion::async_recursion;
use base64::Engine;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{collections::HashMap, sync::Arc};
#[cfg(feature = "benchmark")]
use windmill_common::bench::BenchmarkIter;
use windmill_common::{
    ai_providers::AIProvider,
    auth::get_job_perms,
    cache,
    client::AuthedClient,
    db::DB,
    error::{self, to_anyhow, Error},
    flow_status::AgentAction,
    flows::{FlowModule, FlowModuleValue, Step},
    get_latest_hash_for_path,
    jobs::JobKind,
    s3_helpers::S3Object,
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    utils::{StripPath, HTTP_CLIENT},
    worker::{to_raw_value, Connection},
};
use windmill_parser::Typ;
use windmill_queue::{
    flow_status::get_step_of_flow_status, get_mini_pulled_job, push, CanceledBy, JobCompleted,
    MiniPulledJob, PushArgs, PushIsolationLevel,
};

use crate::{
    common::{build_args_map, error_to_value, OccupancyMetrics},
    create_job_dir,
    handle_child::run_future_with_polling_update_job_poller,
    handle_queued_job, parse_sig_of_lang,
    result_processor::handle_non_flow_job_error,
    worker_flow::{raw_script_to_payload, script_to_payload},
    JobCompletedSender, SendResult, SendResultPayload,
};

const MAX_AGENT_ITERATIONS: usize = 10;

lazy_static::lazy_static! {
    static ref TOOL_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct OpenAIFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct OpenAIToolCall {
    id: String,
    function: OpenAIFunction,
    r#type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentPart {
    Text {
        text: String,
    },
    #[serde(rename = "image_url")]
    ImageUrl {
        image_url: ImageUrlData,
    },
    #[serde(rename = "s3_object")]
    S3Object {
        s3_object: S3Object,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ImageUrlData {
    url: String, // data:image/png;base64,... or https://...
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
enum OpenAIContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
struct OpenAIMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<OpenAIContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing)]
    agent_action: Option<AgentAction>,
}

/// same as OpenAIMessage but with agent_action field included in the serialization
#[derive(Serialize)]
struct Message<'a> {
    #[serde(flatten)]
    message: &'a OpenAIMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_action: Option<&'a AgentAction>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Serialize)]
struct ImageGenerationTool {
    r#type: String,
    quality: Option<String>,
    background: Option<String>,
}

// Input content for image generation - supports both text and images
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ImageGenerationContent {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_image")]
    InputImage { image_url: String },
}

#[derive(Serialize)]
struct ImageGenerationMessage {
    role: String,
    content: Vec<ImageGenerationContent>,
}

#[derive(Serialize)]
struct ImageGenerationRequest<'a> {
    model: &'a str,
    input: Vec<ImageGenerationMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<&'a str>,
    tools: Vec<ImageGenerationTool>,
}

#[derive(Deserialize)]
struct OpenAIImageResponse {
    output: Vec<OpenAIImageOutput>,
}

#[derive(Deserialize)]
struct OpenAIImageOutput {
    r#type: String, // Expected to be "image_generation_call"
    result: String, // Base64 encoded image
}

// Gemini API structures
#[derive(Serialize, Deserialize, Clone, Debug)]
struct GeminiInlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
#[serde(untagged)]
enum GeminiPart {
    Text { text: String },
    InlineData { inline_data: GeminiInlineData },
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiImageRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    contents: Option<Vec<GeminiContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instances: Option<Vec<GeminiPredictContent>>,
}

#[derive(Serialize)]
struct GeminiPredictContent {
    prompt: String,
}

#[derive(Deserialize)]
struct GeminiImageResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    candidates: Option<Vec<GeminiCandidate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    predictions: Option<Vec<GeminiPredictCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}

#[derive(Deserialize)]
struct GeminiPredictCandidate {
    #[serde(rename = "bytesBase64Encoded")]
    bytes_base64_encoded: String, // base64 encoded image
}

#[derive(Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Deserialize)]
struct GeminiResponsePart {
    #[serde(rename = "inlineData")]
    inline_data: Option<GeminiInlineData>,
}

// OpenRouter image generation structures
#[derive(Serialize)]
struct OpenRouterImageRequest<'a> {
    model: &'a str,
    messages: Vec<OpenRouterImageMessage>,
    modalities: Vec<&'a str>,
}

#[derive(Serialize)]
struct OpenRouterImageMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenRouterImageResponse {
    choices: Vec<OpenRouterImageChoice>,
}

#[derive(Deserialize)]
struct OpenRouterImageChoice {
    message: OpenRouterImageResponseMessage,
}

#[derive(Deserialize)]
struct OpenRouterImageResponseMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<OpenRouterImageData>>,
}

#[derive(Deserialize)]
struct OpenRouterImageData {
    image_url: OpenRouterImageUrl,
}

#[derive(Deserialize)]
struct OpenRouterImageUrl {
    url: String, // data:image/png;base64,... format
}

#[derive(Serialize)]
struct OpenAIRequest<'a> {
    model: &'a str,
    messages: &'a Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a Vec<ToolDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize, Clone, Debug)]
struct ResponseFormat {
    r#type: String,
    json_schema: JsonSchemaFormat,
}

#[derive(Serialize, Clone, Debug)]
struct JsonSchemaFormat {
    name: String,
    schema: OpenAPISchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    strict: Option<bool>,
}

#[derive(Serialize, Clone, Debug)]
struct ToolDefFunction {
    name: String,
    description: Option<String>,
    parameters: Box<RawValue>,
}

#[derive(Serialize, Clone, Debug)]
struct ToolDef {
    r#type: String,
    function: ToolDefFunction,
}

struct Tool {
    module: FlowModule,
    def: ToolDef,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
enum OutputType {
    Text,
    Image,
}

impl Default for OutputType {
    fn default() -> Self {
        OutputType::Text
    }
}

#[derive(Deserialize, Debug)]
struct AIAgentArgs {
    provider: ProviderWithResource,
    system_prompt: Option<String>,
    user_message: String,
    temperature: Option<f32>,
    max_completion_tokens: Option<u32>,
    output_schema: Option<OpenAPISchema>,
    output_type: Option<OutputType>,
    image: Option<S3Object>,
}

#[derive(Deserialize, Debug)]
struct ProviderResource {
    #[serde(alias = "apiKey")]
    api_key: String,
    #[serde(alias = "baseUrl")]
    base_url: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ProviderWithResource {
    kind: AIProvider,
    resource: ProviderResource,
    model: String,
}

impl ProviderWithResource {
    fn get_api_key(&self) -> &str {
        &self.resource.api_key
    }

    fn get_model(&self) -> &str {
        &self.model
    }

    async fn get_base_url(&self, db: &DB) -> Result<String, Error> {
        self.kind
            .get_base_url(self.resource.base_url.clone(), db)
            .await
    }
}

#[derive(Serialize)]
struct AIAgentResult<'a> {
    output: Box<RawValue>,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
enum SchemaType {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for SchemaType {
    fn default() -> Self {
        SchemaType::Single("object".to_string())
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
struct OpenAPISchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<SchemaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Box<OpenAPISchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<HashMap<String, Box<OpenAPISchema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "oneOf")]
    one_of: Option<Vec<Box<OpenAPISchema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#enum: Option<Vec<String>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "additionalProperties"
    )]
    additional_properties: Option<bool>,
}

impl OpenAPISchema {
    fn from_str(typ: &str) -> Self {
        OpenAPISchema { r#type: Some(SchemaType::Single(typ.to_string())), ..Default::default() }
    }

    fn from_str_with_enum(typ: &str, enu: &Option<Vec<String>>) -> Self {
        OpenAPISchema {
            r#type: Some(SchemaType::Single(typ.to_string())),
            r#enum: enu.clone(),
            ..Default::default()
        }
    }

    fn datetime() -> Self {
        Self {
            r#type: Some(SchemaType::Single("string".to_string())),
            format: Some("date-time".to_string()),
            ..Default::default()
        }
    }

    fn from_typ(typ: &Typ) -> Self {
        match typ {
            Typ::Str(enu) => Self::from_str_with_enum("string", enu),
            Typ::Int => Self::from_str("integer"),
            Typ::Float => Self::from_str("number"),
            Typ::Bool => Self::from_str("boolean"),
            Typ::Bytes => Self::from_str("string"),
            Typ::Datetime => Self::datetime(),
            Typ::Resource(_) => Self::from_str("string"),
            Typ::Email => Self::from_str("string"),
            Typ::Sql => Self::from_str("string"),
            Typ::DynSelect(_) => Self::from_str("string"),
            Typ::DynMultiselect(_) => Self::from_str("string"),
            Typ::List(typ) => OpenAPISchema {
                r#type: Some(SchemaType::Single("array".to_string())),
                items: Some(Box::new(Self::from_typ(typ))),
                ..Default::default()
            },
            Typ::Object(typ) => OpenAPISchema {
                r#type: Some(SchemaType::Single("object".to_string())),
                items: None,
                properties: typ.props.as_ref().map(|props| {
                    props
                        .iter()
                        .map(|prop| (prop.key.clone(), Box::new(Self::from_typ(&prop.typ))))
                        .collect()
                }),
                required: typ
                    .props
                    .as_ref()
                    .map(|props| props.iter().map(|prop| prop.key.clone()).collect()),
                ..Default::default()
            },
            Typ::OneOf(variants) => OpenAPISchema {
                r#type: Some(SchemaType::Single("object".to_string())),
                one_of: Some(
                    variants
                        .iter()
                        .map(|variant| {
                            let schema = OpenAPISchema {
                                r#type: Some(SchemaType::Single("object".to_string())),
                                properties: Some(
                                    variant
                                        .properties
                                        .iter()
                                        .map(|prop| {
                                            (
                                                prop.key.clone(),
                                                Box::new(
                                                    if prop.key == "label" || prop.key == "kind" {
                                                        Self::from_str_with_enum(
                                                            "string",
                                                            &Some(vec![variant.label.clone()]),
                                                        )
                                                    } else {
                                                        Self::from_typ(&prop.typ)
                                                    },
                                                ),
                                            )
                                        })
                                        .collect(),
                                ),
                                required: Some(
                                    variant
                                        .properties
                                        .iter()
                                        .map(|prop| prop.key.clone())
                                        .collect(),
                                ),
                                ..Default::default()
                            };
                            Box::new(schema)
                        })
                        .collect(),
                ),
                ..Default::default()
            },
            Typ::Unknown => Self::from_str("object"),
        }
    }

    /// Makes this schema compatible with OpenAI's strict mode by:
    /// - Adding additionalProperties: false to all object types
    /// - Making non-required properties nullable
    /// - Ensuring all properties are in the required array
    fn make_strict(mut self) -> Self {
        // Handle this schema if it's an object type
        if let Some(SchemaType::Single(ref type_str)) = self.r#type {
            if type_str == "object" {
                // Set additionalProperties to false
                self.additional_properties = Some(false);

                if let Some(properties) = self.properties.as_mut() {
                    // Get original required fields
                    let original_required = self.required.as_ref();

                    if let Some(required) = original_required {
                        // Update properties to make non-required fields nullable
                        for (key, prop) in properties.iter_mut() {
                            let mut new_prop = (**prop).clone();
                            // Make non-required fields nullable
                            if !required.contains(key) {
                                new_prop = new_prop.make_nullable();
                            }
                            // Recursively make nested schemas strict
                            new_prop = new_prop.make_strict();
                            *prop = Box::new(new_prop);
                        }
                    }

                    // All properties must be in required array for strict mode
                    self.required = Some(properties.keys().cloned().collect());
                }
            }
        }

        // Recursively process nested schemas
        if let Some(ref mut items) = self.items {
            **items = items.as_ref().clone().make_strict();
        }

        if let Some(ref mut one_of) = self.one_of {
            *one_of = one_of
                .iter()
                .map(|schema| Box::new(schema.as_ref().clone().make_strict()))
                .collect();
        }

        self
    }

    /// Makes this property nullable by converting its type to a union with null
    fn make_nullable(mut self) -> Self {
        match self.r#type.take() {
            Some(SchemaType::Single(type_str)) => {
                if type_str != "null" {
                    self.r#type = Some(SchemaType::Multiple(vec![type_str, "null".into()]));
                } else {
                    self.r#type = Some(SchemaType::Single("null".into()));
                }
            }
            Some(SchemaType::Multiple(mut types)) => {
                if !types.iter().any(|t| t == "null") {
                    types.push("null".into());
                }
                self.r#type = Some(SchemaType::Multiple(types));
            }
            None => {
                self.r#type = Some(SchemaType::Single("null".into()));
            }
        }
        self
    }
}

/// Find a unique tool name to avoid collisions with user-provided tools
fn find_unique_tool_name(base_name: &str, existing_tools: Option<&[ToolDef]>) -> String {
    let Some(tools) = existing_tools else {
        return base_name.to_string();
    };

    if !tools.iter().any(|t| t.function.name == base_name) {
        return base_name.to_string();
    }

    for i in 1..100 {
        let candidate = format!("{}_{}", base_name, i);
        if !tools.iter().any(|t| t.function.name == candidate) {
            return candidate;
        }
    }

    // Fallback with process id if somehow we can't find a unique name
    format!("{}_{}_fallback", base_name, std::process::id())
}

/// Helper function to download an S3 image and convert it to a base64 data URL
async fn download_and_encode_s3_image(
    image: &S3Object,
    client: &AuthedClient,
    workspace_id: &str,
) -> error::Result<String> {
    // Download the image from S3
    let image_bytes = client
        .download_s3_file(workspace_id, &image.s3, image.storage.clone())
        .await
        .map_err(|e| Error::internal_err(format!("Failed to download S3 image: {}", e)))?;

    // Encode as base64 data URL
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&image_bytes);

    // Determine MIME type based on file extension or default to PNG
    let mime_type = if image.s3.ends_with(".jpg") || image.s3.ends_with(".jpeg") {
        "image/jpeg"
    } else if image.s3.ends_with(".gif") {
        "image/gif"
    } else if image.s3.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png" // default
    };

    Ok(format!("data:{};base64,{}", mime_type, base64_data))
}

/// Convert messages with S3Objects to messages with base64 image URLs for API calls
async fn prepare_messages_for_api(
    messages: &[OpenAIMessage],
    client: &AuthedClient,
    workspace_id: &str,
) -> error::Result<Vec<OpenAIMessage>> {
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
                                let image_data_url =
                                    download_and_encode_s3_image(s3_object, client, workspace_id)
                                        .await?;
                                prepared_content.push(ContentPart::ImageUrl {
                                    image_url: ImageUrlData { url: image_data_url },
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

/// Generate image from provider and extract base64 data
async fn generate_image_from_provider(
    provider: &ProviderWithResource,
    user_message: &str,
    system_prompt: Option<&str>,
    base_url: &str,
    api_key: &str,
    image: Option<&S3Object>,
    client: &AuthedClient,
    workspace_id: &str,
) -> error::Result<String> {
    match provider.kind {
        AIProvider::OpenAI => {
            // Build content array with text and optional image
            let mut content =
                vec![ImageGenerationContent::InputText { text: user_message.to_string() }];

            // Add image if provided
            if let Some(image) = image {
                if !image.s3.is_empty() {
                    // Download and encode S3 image to base64
                    let image_bytes = client
                        .download_s3_file(workspace_id, &image.s3, image.storage.clone())
                        .await
                        .map_err(|e| {
                            Error::internal_err(format!("Failed to download S3 image: {}", e))
                        })?;

                    let base64_image =
                        base64::engine::general_purpose::STANDARD.encode(&image_bytes);
                    let image_data_url = format!("data:image/jpeg;base64,{}", base64_image);

                    content.push(ImageGenerationContent::InputImage { image_url: image_data_url });
                }
            }

            let image_request = ImageGenerationRequest {
                model: provider.get_model(),
                input: vec![ImageGenerationMessage { role: "user".to_string(), content }],
                instructions: system_prompt,
                tools: vec![ImageGenerationTool {
                    r#type: "image_generation".to_string(),
                    quality: Some("low".to_string()),
                    background: None,
                }],
            };

            let resp = HTTP_CLIENT
                .post(format!("{}/responses", base_url))
                .timeout(std::time::Duration::from_secs(120))
                .bearer_auth(api_key)
                .json(&image_request)
                .send()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to call OpenAI API: {}", e)))?;

            match resp.error_for_status_ref() {
                Ok(_) => {
                    let image_response = resp.json::<OpenAIImageResponse>().await.map_err(|e| {
                        Error::internal_err(format!("Failed to parse OpenAI response: {}", e))
                    })?;

                    // Find the first image generation output
                    let image_generation_call = image_response
                        .output
                        .iter()
                        .find(|output| output.r#type == "image_generation_call")
                        .map(|output| &output.result);

                    if let Some(base64_image) = image_generation_call {
                        Ok(base64_image.clone())
                    } else {
                        Err(Error::internal_err(
                            "No image output received from OpenAI".to_string(),
                        ))
                    }
                }
                Err(e) => {
                    let _status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    Err(Error::internal_err(format!(
                        "OpenAI API error: {} - {}",
                        e, text
                    )))
                }
            }
        }
        AIProvider::GoogleAI => {
            let is_imagen = provider.get_model().contains("imagen");

            let gemini_request = if is_imagen {
                // For Imagen models, we keep the simple prompt format (no image support)
                GeminiImageRequest {
                    instances: Some(vec![GeminiPredictContent {
                        prompt: user_message.trim().to_string(),
                    }]),
                    contents: None,
                }
            } else {
                // For Gemini models, build parts array with text and optional image
                let mut parts = vec![GeminiPart::Text { text: user_message.trim().to_string() }];

                if let Some(system_prompt) = system_prompt {
                    parts.insert(
                        0,
                        GeminiPart::Text {
                            text: format!("SYSTEM PROMPT: {}", system_prompt.trim().to_string()),
                        },
                    );
                }

                // Add image if provided
                if let Some(image) = image {
                    if !image.s3.is_empty() {
                        // Download and encode S3 image to base64
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
                    contents: Some(vec![GeminiContent { parts }]),
                }
            };

            let url_suffix = if is_imagen {
                "predict"
            } else {
                "generateContent"
            };
            let gemini_url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:{}",
                provider.get_model(),
                url_suffix
            );

            let resp = HTTP_CLIENT
                .post(&gemini_url)
                .timeout(std::time::Duration::from_secs(120))
                .header("x-goog-api-key", api_key)
                .header("Content-Type", "application/json")
                .json(&gemini_request)
                .send()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to call Gemini API: {}", e)))?;

            match resp.error_for_status_ref() {
                Ok(_) => {
                    let response_text = resp.text().await.map_err(|e| {
                        Error::internal_err(format!("Failed to read response text: {}", e))
                    })?;

                    let gemini_response: GeminiImageResponse = serde_json::from_str(&response_text)
                        .map_err(|e| {
                            Error::internal_err(format!(
                                "Failed to parse Gemini response: {}. Raw response: {}",
                                e, response_text
                            ))
                        })?;

                    // Find the first candidate with inline image data
                    let mut image_data =
                        gemini_response.candidates.as_ref().and_then(|candidates| {
                            candidates.iter().find_map(|candidate| {
                                candidate.content.parts.iter().find_map(|part| {
                                    part.inline_data.as_ref().map(|data| &data.data)
                                })
                            })
                        });

                    if image_data.is_none() {
                        image_data = gemini_response
                            .predictions
                            .as_ref()
                            .and_then(|predictions| {
                                predictions
                                    .iter()
                                    .find_map(|prediction| Some(&prediction.bytes_base64_encoded))
                            });
                    }

                    if let Some(base64_image) = image_data {
                        Ok(base64_image.clone())
                    } else {
                        Err(Error::internal_err(
                            "No image data received from Gemini".to_string(),
                        ))
                    }
                }
                Err(e) => {
                    let _status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    Err(Error::internal_err(format!(
                        "Gemini API error: {} - {}",
                        e, text
                    )))
                }
            }
        }
        AIProvider::OpenRouter => {
            let mut messages = Vec::new();

            // Add system message if provided
            if let Some(system_prompt) = system_prompt {
                messages.push(OpenRouterImageMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                });
            }

            // Add user message
            messages.push(OpenRouterImageMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            });

            let openrouter_request = OpenRouterImageRequest {
                model: provider.get_model(),
                messages,
                modalities: vec!["image", "text"],
            };

            let resp = HTTP_CLIENT
                .post(format!("{}/chat/completions", base_url))
                .timeout(std::time::Duration::from_secs(120))
                .bearer_auth(api_key)
                .json(&openrouter_request)
                .send()
                .await
                .map_err(|e| {
                    Error::internal_err(format!("Failed to call OpenRouter API: {}", e))
                })?;

            match resp.error_for_status_ref() {
                Ok(_) => {
                    let openrouter_response =
                        resp.json::<OpenRouterImageResponse>().await.map_err(|e| {
                            Error::internal_err(format!(
                                "Failed to parse OpenRouter response: {}",
                                e
                            ))
                        })?;

                    // Extract base64 image from the first choice
                    let image_url = openrouter_response
                        .choices
                        .get(0)
                        .and_then(|choice| choice.message.images.as_ref())
                        .and_then(|images| images.get(0))
                        .map(|image| &image.image_url.url);

                    if let Some(data_url) = image_url {
                        // Extract base64 data from data URL format: data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA...
                        if let Some(base64_start) = data_url.find("base64,") {
                            let base64_data = &data_url[base64_start + 7..]; // Skip "base64," prefix
                            Ok(base64_data.to_string())
                        } else {
                            Err(Error::internal_err(
                                "Invalid data URL format received from OpenRouter".to_string(),
                            ))
                        }
                    } else {
                        Err(Error::internal_err(
                            "No image data received from OpenRouter".to_string(),
                        ))
                    }
                }
                Err(e) => {
                    let _status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    Err(Error::internal_err(format!(
                        "OpenRouter API error: {} - {}",
                        e, text
                    )))
                }
            }
        }
        _ => Err(Error::BadRequest(format!(
            "Image generation is not supported for provider: {:?}",
            provider.kind
        ))),
    }
}

/// Upload image to S3 and return S3Object
async fn upload_image_to_s3(
    base64_image: &str,
    job: &MiniPulledJob,
    client: &AuthedClient,
) -> error::Result<S3Object> {
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_image)
        .map_err(|e| Error::internal_err(format!("Failed to decode base64 image: {}", e)))?;

    // Generate unique S3 key
    let unique_id = ulid::Ulid::new().to_string();
    let s3_key = format!("ai_images/{}/{}.png", job.id, unique_id);

    // Create byte stream
    let byte_stream = futures::stream::once(async move {
        Ok::<_, std::convert::Infallible>(bytes::Bytes::from(image_bytes))
    });

    // Upload to S3
    client
        .upload_s3_file(
            &job.workspace_id,
            s3_key.clone(),
            None, // storage - use default
            byte_stream,
        )
        .await
        .map_err(|e| Error::internal_err(format!("Failed to upload image to S3: {}", e)))?;

    Ok(S3Object {
        s3: s3_key,
        storage: None,
        filename: Some("generated_image.png".to_string()),
        presigned: None,
    })
}

/// Handle image output generation and return S3 object and messages
async fn handle_image_output(
    args: &AIAgentArgs,
    job: &MiniPulledJob,
    client: &AuthedClient,
    db: &DB,
) -> error::Result<(Option<S3Object>, Vec<OpenAIMessage>)> {
    let base_url = args.provider.get_base_url(db).await?;
    let api_key = args.provider.get_api_key();

    let mut messages =
        if let Some(system_prompt) = args.system_prompt.clone().filter(|s| !s.is_empty()) {
            vec![OpenAIMessage {
                role: "system".to_string(),
                content: Some(OpenAIContent::Text(system_prompt)),
                ..Default::default()
            }]
        } else {
            vec![]
        };

    // Generate image from provider
    let base64_image = generate_image_from_provider(
        &args.provider,
        &args.user_message,
        args.system_prompt.as_deref(),
        &base_url,
        api_key,
        args.image.as_ref(),
        client,
        &job.workspace_id,
    )
    .await?;

    // Add assistant success message
    messages.push(OpenAIMessage {
        role: "assistant".to_string(),
        content: Some(OpenAIContent::Text(
            "Image created successfully".to_string(),
        )),
        ..Default::default()
    });

    // Upload to S3
    let s3_object = upload_image_to_s3(&base64_image, job, client).await?;

    Ok((Some(s3_object), messages))
}

/// Handle text output generation with tools and structured output support
#[async_recursion]
async fn handle_text_output(
    // connection
    db: &DB,
    conn: &Connection,

    // agent job and flow data
    job: &MiniPulledJob,
    parent_job: &uuid::Uuid,
    args: AIAgentArgs,
    tools: Vec<Tool>,

    // job execution context
    client: &AuthedClient,
    occupancy_metrics: &mut OccupancyMetrics,
    job_completed_tx: &JobCompletedSender,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
) -> error::Result<Box<RawValue>> {
    let base_url = args.provider.get_base_url(db).await?;
    let api_key = args.provider.get_api_key();

    let mut messages =
        if let Some(system_prompt) = args.system_prompt.clone().filter(|s| !s.is_empty()) {
            vec![OpenAIMessage {
                role: "system".to_string(),
                content: Some(OpenAIContent::Text(system_prompt)),
                ..Default::default()
            }]
        } else {
            vec![]
        };

    // Create user message with optional image
    let user_content = if let Some(image) = &args.image {
        if !image.s3.is_empty() {
            OpenAIContent::Parts(vec![
                ContentPart::Text { text: args.user_message.clone() },
                ContentPart::S3Object { s3_object: image.clone() },
            ])
        } else {
            OpenAIContent::Text(args.user_message.clone())
        }
    } else {
        OpenAIContent::Text(args.user_message.clone())
    };

    messages.push(OpenAIMessage {
        role: "user".to_string(),
        content: Some(user_content),
        ..Default::default()
    });

    let mut actions = vec![];
    let mut content = None;

    let mut tool_defs: Option<Vec<ToolDef>> = if tools.is_empty() {
        None
    } else {
        Some(tools.iter().map(|t| t.def.clone()).collect())
    };

    let has_output_properties = args
        .output_schema
        .as_ref()
        .and_then(|schema| schema.properties.as_ref())
        .map(|props| !props.is_empty())
        .unwrap_or(false);
    let provider_is_anthropic = args.provider.kind.is_anthropic();
    let is_openrouter_anthropic = args.provider.kind == AIProvider::OpenRouter
        && args.provider.model.starts_with("anthropic/");
    let is_anthropic = provider_is_anthropic || is_openrouter_anthropic;
    let mut response_format: Option<ResponseFormat> = None;
    let mut used_structured_output_tool = false;
    let mut structured_output_tool_name: Option<String> = None;

    if has_output_properties {
        let schema = args.output_schema.as_ref().unwrap(); // we know it's some because of the check above
        if is_anthropic {
            // if output schema is provided, and provider is anthropic, add a structured_output tool in the list of tools
            let unique_tool_name = find_unique_tool_name("structured_output", tool_defs.as_deref());
            structured_output_tool_name = Some(unique_tool_name.clone());

            let output_tool = ToolDef {
                r#type: "function".to_string(),
                function: ToolDefFunction {
                    name: unique_tool_name,
                    description: Some(
                        "This tool MUST be used last to return a structured JSON object as the final output."
                            .to_string(),
                    ),
                    parameters: to_raw_value(&schema),
                },
            };
            if let Some(ref mut existing_tools) = tool_defs {
                existing_tools.push(output_tool);
            } else {
                tool_defs = Some(vec![output_tool]);
            }
        } else {
            // if output schema is provided, and provider is openai, add a response_format with json_schema
            let strict_schema = schema.clone().make_strict();
            response_format = Some(ResponseFormat {
                r#type: "json_schema".to_string(),
                json_schema: JsonSchemaFormat {
                    name: "structured_output".to_string(),
                    schema: strict_schema,
                    strict: Some(true),
                },
            });
        }
    }

    for i in 0..MAX_AGENT_ITERATIONS {
        if used_structured_output_tool {
            break;
        }

        let response = {
            // Convert messages with S3Objects to base64 image URLs for API request
            let prepared_messages =
                prepare_messages_for_api(&messages, client, &job.workspace_id).await?;

            let resp = HTTP_CLIENT
                .post(format!("{}/chat/completions", base_url))
                .timeout(std::time::Duration::from_secs(120))
                .bearer_auth(api_key)
                .json(&OpenAIRequest {
                    model: args.provider.get_model(),
                    messages: &prepared_messages,
                    tools: tool_defs.as_ref(),
                    temperature: args.temperature,
                    max_completion_tokens: args.max_completion_tokens,
                    response_format: if has_output_properties && !is_anthropic {
                        response_format.clone()
                    } else {
                        None
                    },
                })
                .send()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to call API: {}", e)))?;

            match resp.error_for_status_ref() {
                Ok(_) => resp,
                Err(e) => {
                    let status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    tracing::error!(
                        "Non 200 response from API: status: {}, body: {}",
                        status,
                        text
                    );
                    return Err(Error::internal_err(format!(
                        "Non 200 response from API: {} - {}",
                        e, text
                    )));
                }
            }
        };

        let mut response = response
            .json::<OpenAIResponse>()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to parse API response: {}", e)))?;

        let first_choice = response
            .choices
            .pop()
            .ok_or_else(|| Error::internal_err("No response from API"))?;

        content = first_choice.message.content;
        let tool_calls = first_choice.message.tool_calls.unwrap_or_default();

        if let Some(ref response_content) = content {
            actions.push(AgentAction::Message {});
            messages.push(OpenAIMessage {
                role: "assistant".to_string(),
                content: Some(response_content.clone()),
                agent_action: Some(AgentAction::Message {}),
                ..Default::default()
            });

            update_flow_status_module_with_actions(db, parent_job, &actions).await?;
            update_flow_status_module_with_actions_success(db, parent_job, true).await?;
        }

        if tool_calls.is_empty() {
            break;
        } else if i == MAX_AGENT_ITERATIONS - 1 {
            return Err(Error::internal_err(
                "AI agent reached max iterations, but there are still tool calls".to_string(),
            ));
        }

        messages.push(OpenAIMessage {
            role: "assistant".to_string(),
            tool_calls: Some(tool_calls.clone()),
            ..Default::default()
        });

        for tool_call in tool_calls.iter() {
            // Structured output tool is used, we stop here as this will be the final output
            if structured_output_tool_name
                .as_ref()
                .map_or(false, |name| tool_call.function.name == *name)
            {
                used_structured_output_tool = true;
                messages.push(OpenAIMessage {
                    role: "tool".to_string(),
                    content: Some(OpenAIContent::Text(
                        "Successfully ran structured_output tool".to_string(),
                    )),
                    tool_call_id: Some(tool_call.id.clone()),
                    ..Default::default()
                });
                messages.push(OpenAIMessage {
                    role: "assistant".to_string(),
                    content: Some(OpenAIContent::Text(tool_call.function.arguments.clone())),
                    agent_action: Some(AgentAction::Message {}),
                    ..Default::default()
                });
                content = Some(OpenAIContent::Text(tool_call.function.arguments.clone()));
                break;
            }

            let tool = tools
                .iter()
                .find(|t| t.def.function.name == tool_call.function.name);
            if let Some(tool) = tool {
                let job_id = ulid::Ulid::new().into();
                actions.push(AgentAction::ToolCall {
                    job_id,
                    function_name: tool_call.function.name.clone(),
                    module_id: tool.module.id.clone(),
                });

                update_flow_status_module_with_actions(db, parent_job, &actions).await?;

                match call_tool(
                    db,
                    conn,
                    job,
                    &tool.module,
                    &tool_call,
                    job_id,
                    client,
                    occupancy_metrics,
                    base_internal_url,
                    worker_dir,
                    worker_name,
                    hostname,
                    job_completed_tx,
                    killpill_rx,
                )
                .await
                {
                    Ok((success, result)) => {
                        messages.push(OpenAIMessage {
                            role: "tool".to_string(),
                            content: Some(OpenAIContent::Text(result.get().to_string())),
                            tool_call_id: Some(tool_call.id.clone()),
                            agent_action: Some(AgentAction::ToolCall {
                                job_id,
                                function_name: tool_call.function.name.clone(),
                                module_id: tool.module.id.clone(),
                            }),
                            ..Default::default()
                        });
                        update_flow_status_module_with_actions_success(db, parent_job, success)
                            .await?;
                    }
                    Err(err) => {
                        let err_string = format!("{}: {}", err.name(), err.to_string());
                        messages.push(OpenAIMessage {
                            role: "tool".to_string(),
                            content: Some(OpenAIContent::Text(format!(
                                "Error running tool: {}",
                                err_string
                            ))),
                            tool_call_id: Some(tool_call.id.clone()),
                            agent_action: Some(AgentAction::ToolCall {
                                job_id,
                                function_name: tool_call.function.name.clone(),
                                module_id: tool.module.id.clone(),
                            }),
                            ..Default::default()
                        });
                        update_flow_status_module_with_actions_success(db, parent_job, false)
                            .await?;
                    }
                }
            } else {
                return Err(Error::internal_err(format!(
                    "Tool not found: {}",
                    tool_call.function.name
                )));
            }
        }
    }

    let final_messages: Vec<Message> = messages
        .iter()
        .map(|m| Message { message: m, agent_action: m.agent_action.as_ref() })
        .collect();

    // Parse content as JSON, fallback to string if it fails
    let output_value = match content {
        Some(content_str) => match has_output_properties {
            true => match content_str {
                OpenAIContent::Text(text) => {
                    serde_json::from_str::<Box<RawValue>>(&text).map_err(|_e| {
                        Error::internal_err(format!("Failed to parse structured output: {}", text))
                    })
                }
                // No need to handle this, it will always be a text string
                OpenAIContent::Parts(_parts) => Err(Error::internal_err(
                    "Failed to parse structured output".to_string(),
                )),
            },
            false => Ok(match content_str {
                OpenAIContent::Text(text) => to_raw_value(&text),
                OpenAIContent::Parts(parts) => to_raw_value(&parts),
            }),
        }?,
        None => to_raw_value(&""),
    };

    Ok(to_raw_value(&AIAgentResult {
        output: output_value,
        messages: final_messages,
    }))
}

async fn update_flow_status_module_with_actions(
    db: &DB,
    parent_job: &uuid::Uuid,
    actions: &[AgentAction],
) -> Result<(), Error> {
    let step = get_step_of_flow_status(db, parent_job.to_owned()).await?;
    match step {
        Step::Step(step) => {
            sqlx::query!(
                r#"
                UPDATE v2_job_status SET
                    flow_status = jsonb_set(
                        flow_status,
                        array['modules', $3::TEXT, 'agent_actions'],
                        $2
                    )
                WHERE id = $1
                "#,
                parent_job,
                sqlx::types::Json(actions) as _,
                step as i32
            )
            .execute(db)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

async fn update_flow_status_module_with_actions_success(
    db: &DB,
    parent_job: &uuid::Uuid,
    action_success: bool,
) -> Result<(), Error> {
    let step = get_step_of_flow_status(db, parent_job.to_owned()).await?;
    match step {
        Step::Step(step) => {
            // Append the new bool to the existing array, or create a new array if it doesn't exist
            sqlx::query!(
                r#"
                UPDATE v2_job_status SET
                    flow_status = jsonb_set(
                        flow_status,
                        array['modules', $2::TEXT, 'agent_actions_success'],
                        COALESCE(
                            flow_status->'modules'->$2->'agent_actions_success',
                            to_jsonb(ARRAY[]::bool[])
                        ) || to_jsonb(ARRAY[$3::bool])
                    )
                WHERE id = $1
                "#,
                parent_job,
                step as i32,
                action_success
            )
            .execute(db)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

fn parse_raw_script_schema(content: &str, language: &ScriptLang) -> Result<Box<RawValue>, Error> {
    let main_arg_signature = parse_sig_of_lang(content, Some(&language), None)?.unwrap(); // safe to unwrap as langauge is some

    let schema = OpenAPISchema {
        r#type: Some(SchemaType::default()),
        properties: Some(
            main_arg_signature
                .args
                .iter()
                .map(|arg| {
                    let name = arg.name.clone();
                    let typ = OpenAPISchema::from_typ(&arg.typ);
                    (name, Box::new(typ))
                })
                .collect(),
        ),
        required: Some(
            main_arg_signature
                .args
                .iter()
                .map(|arg| arg.name.clone())
                .collect(),
        ),
        ..Default::default()
    };

    Ok(to_raw_value(&schema))
}

async fn call_tool(
    // connection
    db: &DB,
    conn: &Connection,

    // agent job and flow step id
    agent_job: &MiniPulledJob,

    // tool
    tool_module: &FlowModule,
    tool_call: &OpenAIToolCall,
    job_id: uuid::Uuid,

    // execution context
    client: &AuthedClient,
    occupancy_metrics: &mut OccupancyMetrics,
    base_internal_url: &str,
    worker_dir: &str,
    worker_name: &str,
    hostname: &str,
    job_completed_tx: &JobCompletedSender,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
) -> error::Result<(bool, Arc<Box<RawValue>>)> {
    let tool_call_args =
        serde_json::from_str::<HashMap<String, Box<RawValue>>>(&tool_call.function.arguments)?;

    let job_payload = match tool_module.get_value()? {
        FlowModuleValue::Script { path: script_path, hash: script_hash, tag_override, .. } => {
            let payload = script_to_payload(
                script_hash,
                script_path,
                db,
                agent_job,
                tool_module,
                tag_override,
                tool_module.apply_preprocessor,
            )
            .await?;
            payload
        }
        FlowModuleValue::RawScript {
            path,
            content,
            language,
            lock,
            tag,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            ..
        } => {
            let path = path.unwrap_or_else(|| {
                format!("{}/tools/{}", agent_job.runnable_path(), tool_module.id)
            });

            let payload = raw_script_to_payload(
                path,
                content,
                language,
                lock,
                custom_concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                tool_module,
                tag,
                tool_module.delete_after_use.unwrap_or(false),
            );
            payload
        }
        _ => {
            return Err(Error::internal_err(format!(
                "Unsupported tool: {}",
                tool_call.function.name
            )));
        }
    };

    let mut tx = db.begin().await?;

    let job_perms = get_job_perms(&mut *tx, &agent_job.id, &agent_job.workspace_id)
        .await?
        .map(|x| x.into());

    let (email, permissioned_as) = if let Some(on_behalf_of) = job_payload.on_behalf_of.as_ref() {
        (&on_behalf_of.email, on_behalf_of.permissioned_as.clone())
    } else {
        (
            &agent_job.permissioned_as_email,
            agent_job.permissioned_as.to_owned(),
        )
    };

    let job_priority = tool_module.priority.or(agent_job.priority);

    let tx = PushIsolationLevel::Transaction(tx);
    let (uuid, tx) = push(
        db,
        tx,
        &agent_job.workspace_id,
        job_payload.payload,
        PushArgs { args: &tool_call_args, extra: None },
        &agent_job.created_by,
        email,
        permissioned_as,
        Some(&format!("job-span-{}", agent_job.id)),
        None,
        agent_job.schedule_path(),
        Some(agent_job.id),
        None,
        None,
        Some(job_id),
        false,
        false,
        None,
        agent_job.visible_to_owner,
        Some(agent_job.tag.clone()), // we reuse the same tag as the agent job because it's run on the same worker
        job_payload.timeout,
        None,
        job_priority,
        job_perms.as_ref(),
        true,
    )
    .await?;

    tx.commit().await?;

    let tool_job = get_mini_pulled_job(db, &uuid).await?;

    let Some(tool_job) = tool_job else {
        return Err(Error::internal_err("Tool job not found".to_string()));
    };

    let tool_job = Arc::new(tool_job);

    let job_dir = create_job_dir(&worker_dir, agent_job.id).await;

    let (inner_job_completed_tx, inner_job_completed_rx) = JobCompletedSender::new(&conn, 1);

    let inner_job_completed_rx = inner_job_completed_rx.expect(
        "inner_job_completed_tx should be set as agent jobs are not supported on agent workers",
    );

    #[cfg(feature = "benchmark")]
    let mut bench = BenchmarkIter::new();

    match handle_queued_job(
        tool_job.clone(),
        None,
        None,
        None,
        None,
        conn,
        client,
        hostname,
        worker_name,
        worker_dir,
        &job_dir,
        None,
        base_internal_url,
        inner_job_completed_tx,
        occupancy_metrics,
        killpill_rx,
        None,
        #[cfg(feature = "benchmark")]
        &mut bench,
    )
    .await
    {
        Err(err) => {
            let err_string = format!("{}: {}", err.name(), err.to_string());
            let err_json = error_to_value(&err);
            let _ = handle_non_flow_job_error(
                db,
                &tool_job,
                0,
                None,
                err_string,
                err_json,
                worker_name,
            )
            .await;
            Err(err)
        }
        Ok(success) => {
            let send_result = inner_job_completed_rx.bounded_rx.try_recv().ok();

            let result = if let Some(SendResult {
                result: SendResultPayload::JobCompleted(JobCompleted { result, .. }),
                ..
            }) = send_result.as_ref()
            {
                job_completed_tx
                    .send(send_result.as_ref().unwrap().result.clone(), true)
                    .await
                    .map_err(to_anyhow)?;
                result
            } else {
                if let Some(send_result) = send_result {
                    job_completed_tx
                        .send(send_result.result, true)
                        .await
                        .map_err(to_anyhow)?;
                }
                return Err(Error::internal_err(
                    "Tool job completed but no result".to_string(),
                ));
            };

            Ok((success, result.clone()))
        }
    }
}

async fn run_agent(
    // connection
    db: &DB,
    conn: &Connection,

    // agent job and flow data
    job: &MiniPulledJob,
    parent_job: &uuid::Uuid,
    args: AIAgentArgs,
    tools: Vec<Tool>,

    // job execution context
    client: &AuthedClient,
    occupancy_metrics: &mut OccupancyMetrics,
    job_completed_tx: &JobCompletedSender,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
) -> error::Result<Box<RawValue>> {
    let output_type = args.output_type.as_ref().unwrap_or(&OutputType::Text);

    match *output_type {
        OutputType::Image => {
            let (s3_result, messages) = handle_image_output(&args, job, client, db).await?;

            let final_messages: Vec<Message> = messages
                .iter()
                .map(|m| Message { message: m, agent_action: m.agent_action.as_ref() })
                .collect();

            if let Some(s3_output) = s3_result {
                Ok(to_raw_value(&s3_output))
            } else {
                Ok(to_raw_value(&AIAgentResult {
                    output: to_raw_value(&None::<String>),
                    messages: final_messages,
                }))
            }
        }
        OutputType::Text => {
            handle_text_output(
                db,
                conn,
                job,
                parent_job,
                args,
                tools,
                client,
                occupancy_metrics,
                job_completed_tx,
                worker_dir,
                base_internal_url,
                worker_name,
                hostname,
                killpill_rx,
            )
            .await
        }
    }
}

pub struct FlowJobRunnableIdAndRawFlow {
    pub runnable_id: Option<ScriptHash>,
    pub raw_flow: Option<sqlx::types::Json<Box<RawValue>>>,
    pub kind: JobKind,
}

pub async fn get_flow_job_runnable_and_raw_flow(
    db: &DB,
    job_id: &uuid::Uuid,
) -> windmill_common::error::Result<FlowJobRunnableIdAndRawFlow> {
    let job = sqlx::query_as!(
        FlowJobRunnableIdAndRawFlow,
        "SELECT runnable_id as \"runnable_id: ScriptHash\", raw_flow as \"raw_flow: _\", kind as \"kind: _\" FROM v2_job WHERE id = $1",
        job_id
    )
    .fetch_one(db)
    .await?;
    Ok(job)
}

pub async fn handle_ai_agent_job(
    // connection
    conn: &Connection,
    db: &DB,

    // agent job
    job: &MiniPulledJob,

    // job execution context
    client: &AuthedClient,
    canceled_by: &mut Option<CanceledBy>,
    mem_peak: &mut i32,
    occupancy_metrics: &mut OccupancyMetrics,
    job_completed_tx: &JobCompletedSender,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
) -> Result<Box<RawValue>, Error> {
    let args = build_args_map(job, client, conn).await?;

    let args = serde_json::from_str::<AIAgentArgs>(&serde_json::to_string(&args)?)?;

    let Some(flow_step_id) = &job.flow_step_id else {
        return Err(Error::internal_err(
            "AI agent job has no flow step id".to_string(),
        ));
    };

    let Some(parent_job) = &job.parent_job else {
        return Err(Error::internal_err(
            "AI agent job has no parent job".to_string(),
        ));
    };

    let flow_job = get_flow_job_runnable_and_raw_flow(db, &parent_job).await?;

    let flow_data = match flow_job.kind {
        JobKind::Flow | JobKind::FlowNode => {
            cache::job::fetch_flow(db, &flow_job.kind, flow_job.runnable_id).await?
        }
        JobKind::FlowPreview => {
            cache::job::fetch_preview_flow(db, &parent_job, flow_job.raw_flow).await?
        }
        _ => {
            return Err(Error::internal_err(
                "expected parent flow, flow preview or flow node for ai agent job".to_string(),
            ));
        }
    };

    let value = flow_data.value();

    let module = value.modules.iter().find(|m| m.id == *flow_step_id);

    let Some(module) = module else {
        return Err(Error::internal_err(
            "AI agent module not found in flow".to_string(),
        ));
    };

    let FlowModuleValue::AIAgent { tools, .. } = module.get_value()? else {
        return Err(Error::internal_err(
            "AI agent module is not an AI agent".to_string(),
        ));
    };

    let tools = futures::future::try_join_all(tools.into_iter().map(|mut t| {
        let conn = conn;
        let db = db;
        let job = job;
        async move {
            let Some(summary) = t.summary.as_ref().filter(|s| TOOL_NAME_REGEX.is_match(s)) else {
                return Err(Error::internal_err(format!(
                    "Invalid tool name: {:?}",
                    t.summary
                )));
            };

            let schema = match &t.get_value() {
                Ok(FlowModuleValue::Script {
                    hash,
                    path,
                    tag_override,
                    input_transforms,
                    is_trigger,
                }) => match hash {
                    Some(hash) => {
                        let (_, metadata) = cache::script::fetch(conn, hash.clone()).await?;
                        Ok::<_, Error>(
                            metadata
                                .schema
                                .clone()
                                .map(|s| RawValue::from_string(s).ok())
                                .flatten(),
                        )
                    }
                    None => {
                        if path.starts_with("hub/") {
                            let hub_script = get_full_hub_script_by_path(
                                StripPath(path.to_string()),
                                &HTTP_CLIENT,
                                None,
                            )
                            .await?;
                            Ok(Some(hub_script.schema))
                        } else {
                            let hash = get_latest_hash_for_path(db, &job.workspace_id, path, true)
                                .await?
                                .0;
                            // update module definition to use a fixed hash so all tool calls match the same schema
                            t.value = to_raw_value(&FlowModuleValue::Script {
                                hash: Some(hash),
                                path: path.clone(),
                                tag_override: tag_override.clone(),
                                input_transforms: input_transforms.clone(),
                                is_trigger: *is_trigger,
                            });
                            let (_, metadata) = cache::script::fetch(conn, hash).await?;
                            Ok(metadata
                                .schema
                                .clone()
                                .map(|s| RawValue::from_string(s).ok())
                                .flatten())
                        }
                    }
                },
                Ok(FlowModuleValue::RawScript { content, language, .. }) => {
                    Ok(Some(parse_raw_script_schema(&content, &language)?))
                }
                Err(e) => {
                    return Err(Error::internal_err(format!(
                        "Invalid tool {}: {}",
                        summary,
                        e.to_string()
                    )));
                }
                _ => {
                    return Err(Error::internal_err(format!(
                        "Unsupported tool: {}",
                        summary
                    )));
                }
            }?;

            Ok(Tool {
                def: ToolDef {
                    r#type: "function".to_string(),
                    function: ToolDefFunction {
                        name: summary.clone(),
                        description: None,
                        parameters: schema.unwrap_or_else(|| {
                            to_raw_value(&serde_json::json!({
                                "type": "object",
                                "properties": {},
                                "required": [],
                            }))
                        }),
                    },
                },
                module: t,
            })
        }
    }))
    .await?;

    let mut inner_occupancy_metrics = occupancy_metrics.clone();

    let agent_fut = run_agent(
        db,
        conn,
        job,
        parent_job,
        args,
        tools,
        client,
        &mut inner_occupancy_metrics,
        job_completed_tx,
        worker_dir,
        base_internal_url,
        worker_name,
        hostname,
        killpill_rx,
    );

    let result = run_future_with_polling_update_job_poller(
        job.id,
        job.timeout,
        conn,
        mem_peak,
        canceled_by,
        agent_fut,
        worker_name,
        &job.workspace_id,
        &mut Some(occupancy_metrics),
        Box::pin(futures::stream::once(async { 0 })),
    )
    .await?;

    Ok(result)
}
