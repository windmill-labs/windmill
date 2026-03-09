use async_trait::async_trait;
use windmill_common::{
    ai_google::{
        openai_messages_to_gemini, openai_tools_to_gemini, GeminiGenerationConfig,
        GeminiImageContent, GeminiImageRequest, GeminiImageResponse, GeminiInlineData, GeminiPart,
        GeminiPredictContent, GeminiTextRequest, GeminiTool,
    },
    client::AuthedClient,
    error::Error,
};

use crate::ai::{
    image_handler::{download_and_encode_s3_image, prepare_messages_for_api},
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    sse::{GeminiSSEParser, SSEParser},
    types::*,
};

// ============================================================================
// Query Builder Implementation
// ============================================================================

pub struct GoogleAIQueryBuilder;

impl GoogleAIQueryBuilder {
    pub fn new() -> Self {
        Self
    }

    async fn build_text_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        let prepared_messages =
            prepare_messages_for_api(args.messages, client, workspace_id).await?;
        let (contents, system_instruction) = openai_messages_to_gemini(&prepared_messages);

        let tools = self.convert_tools_to_gemini(args.tools, args.has_websearch);

        let generation_config = self.build_generation_config(args);

        let request = GeminiTextRequest {
            contents,
            tools,
            tool_config: None,
            system_instruction,
            generation_config,
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
        let is_imagen = args.model.contains("imagen");

        let request = if is_imagen {
            GeminiImageRequest {
                instances: Some(vec![GeminiPredictContent {
                    prompt: args.user_message.trim().to_string(),
                }]),
                contents: None,
            }
        } else {
            let mut parts = vec![GeminiPart::Text { text: args.user_message.trim().to_string() }];

            if let Some(system_prompt) = args.system_prompt {
                parts.insert(
                    0,
                    GeminiPart::Text { text: format!("SYSTEM PROMPT: {}", system_prompt.trim()) },
                );
            }

            if let Some(images) = args.images {
                for image in images.iter() {
                    if !image.s3.is_empty() {
                        let (mime_type, image_bytes) =
                            download_and_encode_s3_image(image, client, workspace_id).await?;
                        parts.push(GeminiPart::InlineData {
                            inline_data: GeminiInlineData { mime_type, data: image_bytes },
                        });
                    }
                }
            }

            GeminiImageRequest {
                instances: None,
                contents: Some(vec![GeminiImageContent { parts }]),
            }
        };

        serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
    }

    /// Convert OpenAI tool definitions to Gemini format.
    ///
    /// Sanitizes each tool's JSON schema for Google compatibility before delegating
    /// to the shared [`openai_tools_to_gemini`] function.
    fn convert_tools_to_gemini(
        &self,
        tools: Option<&[ToolDef]>,
        has_websearch: bool,
    ) -> Option<Vec<GeminiTool>> {
        let Some(tool_defs) = tools else {
            if has_websearch {
                return Some(vec![GeminiTool {
                    function_declarations: None,
                    google_search: Some(serde_json::json!({})),
                }]);
            }
            return None;
        };

        let tool_params: Vec<serde_json::Value> = tool_defs
            .iter()
            .map(|t| {
                let mut schema: OpenAPISchema =
                    serde_json::from_str(t.function.parameters.get()).unwrap_or_default();
                schema.sanitize_for_google();
                serde_json::to_value(&schema).unwrap_or_default()
            })
            .collect();

        openai_tools_to_gemini(tool_defs, &tool_params, has_websearch)
    }

    fn build_generation_config(&self, args: &BuildRequestArgs<'_>) -> Option<GeminiGenerationConfig> {
        let has_output_schema = args
            .output_schema
            .and_then(|s| s.properties.as_ref())
            .map(|p| !p.is_empty())
            .unwrap_or(false);

        let (response_mime_type, response_schema) = if has_output_schema {
            let mut schema = args.output_schema.unwrap().clone();
            schema.sanitize_for_google();
            (Some("application/json".to_string()), serde_json::to_value(&schema).ok())
        } else {
            (None, None)
        };

        if args.temperature.is_some() || args.max_tokens.is_some() || response_mime_type.is_some() {
            Some(GeminiGenerationConfig {
                temperature: args.temperature,
                max_output_tokens: args.max_tokens,
                response_mime_type,
                response_schema,
            })
        } else {
            None
        }
    }
}

#[async_trait]
impl QueryBuilder for GoogleAIQueryBuilder {
    fn supports_tools_with_output_type(&self, output_type: &OutputType) -> bool {
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

    async fn parse_image_response(
        &self,
        response: reqwest::Response,
    ) -> Result<ParsedResponse, Error> {
        let gemini_response: GeminiImageResponse = response.json().await.map_err(|e| {
            Error::internal_err(format!("Failed to parse Gemini image response: {}", e))
        })?;

        let image_data_from_gemini = gemini_response.candidates.as_ref().and_then(|candidates| {
            candidates.iter().find_map(|candidate| {
                candidate
                    .content
                    .parts
                    .iter()
                    .find_map(|part| part.inline_data.as_ref().map(|data| &data.data))
            })
        });

        let image_data_from_imagen = gemini_response
            .predictions
            .as_ref()
            .and_then(|predictions| predictions.first().map(|p| &p.bytes_base64_encoded));

        let image_data = image_data_from_gemini.or(image_data_from_imagen);

        match image_data {
            Some(base64_data) if !base64_data.is_empty() => {
                Ok(ParsedResponse::Image { base64_data: base64_data.clone() })
            }
            _ => Err(Error::internal_err(
                "No image data received from Google Gemini/Imagen API".to_string(),
            )),
        }
    }

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        let mut gemini_sse_parser = GeminiSSEParser::new(stream_event_processor);
        gemini_sse_parser.parse_events(response).await?;

        let GeminiSSEParser {
            accumulated_content,
            accumulated_tool_calls,
            mut events_str,
            stream_event_processor,
            annotations,
            used_websearch,
            usage: gemini_usage,
            ..
        } = gemini_sse_parser;

        for tool_call in accumulated_tool_calls.values() {
            let event = StreamingEvent::ToolCallArguments {
                call_id: tool_call.id.clone(),
                function_name: tool_call.function.name.clone(),
                arguments: tool_call.function.arguments.clone(),
            };
            stream_event_processor.send(event, &mut events_str).await?;
        }

        let usage = gemini_usage.map(|u| {
            TokenUsage::new(
                u.prompt_token_count,
                u.candidates_token_count,
                u.total_token_count,
            )
        });

        Ok(ParsedResponse::Text {
            content: if accumulated_content.is_empty() { None } else { Some(accumulated_content) },
            tool_calls: accumulated_tool_calls.into_values().collect(),
            events_str: Some(events_str),
            annotations,
            used_websearch,
            usage,
        })
    }

    fn get_endpoint(&self, base_url: &str, model: &str, output_type: &OutputType) -> String {
        match output_type {
            OutputType::Text => {
                format!("{}/models/{}:streamGenerateContent?alt=sse", base_url, model)
            }
            OutputType::Image => {
                let url_suffix =
                    if model.contains("imagen") { "predict" } else { "generateContent" };
                format!("{}/models/{}:{}", base_url, model, url_suffix)
            }
        }
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        _base_url: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        vec![("x-goog-api-key", api_key.to_string())]
    }
}
