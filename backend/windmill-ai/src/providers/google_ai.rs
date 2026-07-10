use crate::{
    ai_google::{
        gemini_event_to_openai_sse_chunks, gemini_response_to_openai, openai_messages_to_gemini,
        openai_tools_to_gemini, parse_gemini_response, parse_gemini_sse_event,
        sanitize_schema_for_google, GeminiFunctionDeclaration, GeminiGenerationConfig,
        GeminiImageContent, GeminiImageRequest, GeminiImageResponse, GeminiInlineData, GeminiPart,
        GeminiPredictContent, GeminiTextRequest, GeminiThinkingConfig, GeminiTool,
    },
    image_handler::{download_and_encode_s3_image, prepare_messages_for_api},
    proxy::{ProxyBuildArgs, ProxyRequest},
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventSink},
    sse::{GeminiSSEParser, SSEParser},
    types::*,
};
use async_trait::async_trait;
use bytes::Bytes;
use eventsource_stream::Eventsource;
use futures::{stream::BoxStream, StreamExt};
use http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use serde::Deserialize;
use serde_json::json;
use windmill_common::{client::AuthedClient, error::Error};

// ============================================================================
// Query Builder Implementation
// ============================================================================

pub struct GoogleAIQueryBuilder {
    platform: AIPlatform,
}

impl GoogleAIQueryBuilder {
    pub fn new(platform: AIPlatform) -> Self {
        Self { platform }
    }

    fn is_vertex(&self) -> bool {
        self.platform == AIPlatform::GoogleVertexAi
    }

    async fn build_text_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        let prepared_messages =
            prepare_messages_for_api(args.messages, client, workspace_id).await?;
        build_gemini_text_request_body(
            &prepared_messages,
            self.convert_tools_to_gemini(args.tools, args.has_websearch),
            self.build_generation_config(args),
        )
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

            if let Some(attachments) = args.attachments {
                for attachment in attachments.iter() {
                    if !attachment.s3.is_empty() {
                        let (mime_type, file_bytes) =
                            download_and_encode_s3_image(attachment, client, workspace_id).await?;
                        parts.push(GeminiPart::InlineData {
                            inline_data: GeminiInlineData { mime_type, data: file_bytes },
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

    fn build_generation_config(
        &self,
        args: &BuildRequestArgs<'_>,
    ) -> Option<GeminiGenerationConfig> {
        let has_output_schema = args
            .output_schema
            .and_then(|s| s.properties.as_ref())
            .map(|p| !p.is_empty())
            .unwrap_or(false);

        let (response_mime_type, response_schema) = if has_output_schema {
            let mut schema = args.output_schema.unwrap().clone();
            schema.sanitize_for_google();
            (
                Some("application/json".to_string()),
                serde_json::to_value(&schema).ok(),
            )
        } else {
            (None, None)
        };

        // Map the effort token onto Gemini's native thinking controls; without
        // one, leave provider defaults untouched.
        let thinking_config = args
            .reasoning_effort
            .map(|effort| gemini_thinking_config(args.model, effort));

        build_gemini_generation_config(
            args.temperature,
            args.max_tokens,
            response_mime_type,
            response_schema,
            thinking_config,
        )
    }
}

fn build_gemini_text_request_body(
    messages: &[OpenAIMessage],
    tools: Option<Vec<GeminiTool>>,
    generation_config: Option<GeminiGenerationConfig>,
) -> Result<String, Error> {
    let request = build_gemini_text_request(messages, tools, generation_config);
    serde_json::to_string(&request)
        .map_err(|e| Error::internal_err(format!("Failed to serialize Gemini request: {}", e)))
}

fn build_gemini_text_request(
    messages: &[OpenAIMessage],
    tools: Option<Vec<GeminiTool>>,
    generation_config: Option<GeminiGenerationConfig>,
) -> GeminiTextRequest {
    let (contents, system_instruction) = openai_messages_to_gemini(messages);

    GeminiTextRequest { contents, tools, tool_config: None, system_instruction, generation_config }
}

fn build_gemini_generation_config(
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    response_mime_type: Option<String>,
    response_schema: Option<serde_json::Value>,
    thinking_config: Option<GeminiThinkingConfig>,
) -> Option<GeminiGenerationConfig> {
    if temperature.is_some()
        || max_tokens.is_some()
        || response_mime_type.is_some()
        || response_schema.is_some()
        || thinking_config.is_some()
    {
        Some(GeminiGenerationConfig {
            temperature,
            max_output_tokens: max_tokens,
            response_mime_type,
            response_schema,
            thinking_config,
        })
    } else {
        None
    }
}

/// Map an OpenAI-style `reasoning_effort` token onto Gemini's native thinking
/// controls. Gemini models think by default, so this is what makes the chat
/// effort setting (including explicit `none`) actually change model behavior.
///
/// - Gemini 3+: `thinkingLevel` takes the token verbatim (provider-native open
///   vocabulary: `low`/`medium`/`high`, plus `minimal` on Flash). `none` maps to
///   the lowest supported level — thinking cannot be fully disabled on Pro.
/// - Gemini 2.5: `thinkingBudget` in tokens, using the same tiering as Google's
///   own OpenAI-compatibility layer. 2.5 Pro cannot disable thinking and has a
///   minimum budget of 128; Flash accepts 0 (off). Unknown tokens fall back to
///   `-1` (dynamic, the provider default).
fn gemini_thinking_config(model: &str, effort: &str) -> GeminiThinkingConfig {
    let model = model.to_lowercase();
    let is_flash = model.contains("flash");
    let is_gemini_2_5 = model.contains("gemini-2.5");
    // Thought summaries are free to return (thinking tokens are billed either
    // way); skip them only when the user explicitly turned reasoning off.
    let include_thoughts = (effort != "none").then_some(true);

    if is_gemini_2_5 {
        let budget = match effort {
            "none" if is_flash => 0,
            "none" => 128,
            "low" => 1024,
            "medium" => 8192,
            "high" => 24576,
            _ => -1,
        };
        GeminiThinkingConfig {
            thinking_level: None,
            thinking_budget: Some(budget),
            include_thoughts,
        }
    } else {
        let level = match effort {
            "none" if is_flash => "minimal".to_string(),
            "none" => "low".to_string(),
            other => other.to_string(),
        };
        GeminiThinkingConfig {
            thinking_level: Some(level),
            thinking_budget: None,
            include_thoughts,
        }
    }
}

#[derive(Deserialize, Debug)]
struct GoogleAIProxyChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    max_tokens: Option<u32>,
    /// OpenAI-style effort token from the chat reasoning setting (e.g. `low`,
    /// `high`, or `none` for an explicit off). Mapped to Gemini's native
    /// `thinkingConfig` — see [`gemini_thinking_config`].
    #[serde(default)]
    reasoning_effort: Option<String>,
    #[serde(default)]
    tools: Option<Vec<GoogleAIProxyChatTool>>,
}

#[derive(Deserialize, Debug)]
struct GoogleAIProxyChatTool {
    function: GoogleAIProxyChatToolFunction,
}

#[derive(Deserialize, Debug)]
struct GoogleAIProxyChatToolFunction {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    parameters: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GeminiModel {
    name: String,
    #[serde(rename = "displayName", default)]
    display_name: String,
}

#[derive(Deserialize)]
struct GeminiModelsResponse {
    #[serde(default)]
    models: Vec<GeminiModel>,
}

struct GoogleAIProxyRequest {
    request: ProxyRequest,
    model: String,
    stream: bool,
}

pub enum GoogleAIProxyResponseBody {
    Fixed(Bytes),
    Stream(BoxStream<'static, std::result::Result<Bytes, reqwest::Error>>),
}

pub struct GoogleAIProxyResponse {
    pub status_code: StatusCode,
    pub headers: HeaderMap,
    pub body: GoogleAIProxyResponseBody,
}

/// Handle a workspace Google AI chat proxy request.
///
/// The API still owns credential resolution, auditing, and keepalive injection.
/// Callers must verify the user can use the supplied credentials before calling.
/// This helper owns the provider-specific OpenAI <-> Gemini transformations.
pub async fn handle_google_ai_chat_proxy(
    client: &reqwest::Client,
    args: &ProxyBuildArgs<'_>,
) -> Result<GoogleAIProxyResponse, Error> {
    let GoogleAIProxyRequest { request, model, stream } = build_google_ai_chat_proxy_request(args)?;

    let response =
        send_google_ai_proxy_request(client, request, "Failed to send request to Gemini API")
            .await?;

    if stream {
        Ok(convert_streaming_response(response, &model))
    } else {
        convert_non_streaming_response(response, &model).await
    }
}

/// Handle a workspace Google AI model-list proxy request.
///
/// The API still owns credential resolution and auditing. Callers must verify
/// the user can use the supplied credentials before calling.
pub async fn handle_google_ai_models_proxy(
    client: &reqwest::Client,
    args: &ProxyBuildArgs<'_>,
) -> Result<GoogleAIProxyResponse, Error> {
    let request = build_google_ai_models_proxy_request(args);
    let response =
        send_google_ai_proxy_request(client, request, "Failed to fetch Gemini models").await?;

    let gemini_resp: GeminiModelsResponse = response.json().await.map_err(|e| {
        Error::internal_err(format!("Failed to parse Gemini models response: {}", e))
    })?;

    let data: Vec<serde_json::Value> = gemini_resp
        .models
        .into_iter()
        .map(|m| {
            json!({
                "id": m.name,
                "object": "model",
                "display_name": m.display_name,
            })
        })
        .collect();

    let body = serde_json::to_vec(&json!({ "data": data }))
        .map_err(|e| Error::internal_err(format!("Failed to serialize models: {}", e)))?;

    Ok(GoogleAIProxyResponse {
        status_code: StatusCode::OK,
        headers: json_response_headers(),
        body: GoogleAIProxyResponseBody::Fixed(Bytes::from(body)),
    })
}

fn build_google_ai_chat_proxy_request(
    args: &ProxyBuildArgs<'_>,
) -> Result<GoogleAIProxyRequest, Error> {
    let request: GoogleAIProxyChatRequest = serde_json::from_slice(args.body)
        .map_err(|e| Error::BadRequest(format!("Failed to parse request body: {}", e)))?;

    let gemini_tools = request.tools.as_ref().map(|tools| {
        let declarations: Vec<GeminiFunctionDeclaration> = tools
            .iter()
            .map(|t| {
                let mut params = t.function.parameters.clone().unwrap_or(json!({}));
                sanitize_schema_for_google(&mut params);
                GeminiFunctionDeclaration {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    parameters: params,
                }
            })
            .collect();
        vec![GeminiTool { function_declarations: Some(declarations), google_search: None }]
    });

    let thinking_config = request
        .reasoning_effort
        .as_deref()
        .map(|effort| gemini_thinking_config(&request.model, effort));
    let body = build_gemini_text_request_body(
        &request.messages,
        gemini_tools,
        build_gemini_generation_config(
            request.temperature,
            request.max_tokens,
            None,
            None,
            thinking_config,
        ),
    )?
    .into_bytes();

    let credentials = args.credentials;
    let base_url = credentials.base_url.trim_end_matches('/');
    let is_vertex = credentials.platform == AIPlatform::GoogleVertexAi;
    let endpoint = if request.stream {
        format!(
            "{}?alt=sse",
            build_google_ai_model_endpoint(
                base_url,
                &request.model,
                "streamGenerateContent",
                is_vertex,
            )
        )
    } else {
        build_google_ai_model_endpoint(base_url, &request.model, "generateContent", is_vertex)
    };

    let mut headers = vec![("content-type".to_string(), "application/json".to_string())];
    add_google_ai_auth_header(
        &mut headers,
        credentials.api_key.as_deref().unwrap_or(""),
        is_vertex,
    );

    Ok(GoogleAIProxyRequest {
        request: ProxyRequest { method: Method::POST, url: endpoint, headers, body },
        model: request.model,
        stream: request.stream,
    })
}

fn build_google_ai_models_proxy_request(args: &ProxyBuildArgs<'_>) -> ProxyRequest {
    let credentials = args.credentials;
    let base_url = credentials.base_url.trim_end_matches('/');
    let is_vertex = credentials.platform == AIPlatform::GoogleVertexAi;
    let url = if is_vertex {
        base_url.to_string()
    } else {
        format!("{}/models", base_url)
    };

    let mut headers = Vec::new();
    add_google_ai_auth_header(
        &mut headers,
        credentials.api_key.as_deref().unwrap_or(""),
        is_vertex,
    );

    ProxyRequest { method: Method::GET, url, headers, body: Vec::new() }
}

fn build_google_ai_model_endpoint(
    base_url: &str,
    model: &str,
    action: &str,
    is_vertex: bool,
) -> String {
    let model = model.strip_prefix("models/").unwrap_or(model);

    if is_vertex {
        format!("{}/{}:{}", base_url, model, action)
    } else {
        format!("{}/models/{}:{}", base_url, model, action)
    }
}

fn add_google_ai_auth_header(headers: &mut Vec<(String, String)>, api_key: &str, is_vertex: bool) {
    // Native Google AI proxy intentionally does not apply AI_HTTP_HEADERS or
    // resource custom headers yet. Gemini/Vertex header semantics are
    // provider-specific; keep this limited to required auth headers until
    // explicit custom-header support is designed.
    if is_vertex {
        headers.push(("Authorization".to_string(), format!("Bearer {}", api_key)));
    } else {
        headers.push(("x-goog-api-key".to_string(), api_key.to_string()));
    }
}

async fn send_google_ai_proxy_request(
    client: &reqwest::Client,
    proxy_request: ProxyRequest,
    send_error_message: &str,
) -> Result<reqwest::Response, Error> {
    let mut request = client.request(proxy_request.method.clone(), &proxy_request.url);
    for (header_name, header_value) in &proxy_request.headers {
        request = request.header(header_name.as_str(), header_value.as_str());
    }

    let response = request
        .body(proxy_request.body)
        .send()
        .await
        .map_err(|e| Error::internal_err(format!("{}: {}", send_error_message, e)))?;

    if let Err(e) = response.error_for_status_ref() {
        let status = e.status().map(|s| s.to_string()).unwrap_or_default();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::AIError(format!("{}: {}", status, body)));
    }

    Ok(response)
}

fn convert_streaming_response(response: reqwest::Response, model: &str) -> GoogleAIProxyResponse {
    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
    let model = model.to_string();

    let gemini_sse_stream = response.bytes_stream().eventsource();
    let openai_sse_stream = async_stream::stream! {
        tokio::pin!(gemini_sse_stream);
        let mut tool_call_index: usize = 0;
        while let Some(event) = gemini_sse_stream.next().await {
            match event {
                Ok(event) => match parse_gemini_sse_event(&event.data) {
                    Ok(Some(parsed)) => {
                        for chunk in gemini_event_to_openai_sse_chunks(
                            &parsed, &id, &model, &mut tool_call_index,
                        ) {
                            yield Ok::<Bytes, reqwest::Error>(Bytes::from(chunk));
                        }
                    }
                    Ok(None) => {}
                    Err(e) => tracing::error!("Error parsing Gemini SSE event: {}", e),
                },
                Err(e) => tracing::error!("Error reading Gemini SSE stream: {}", e),
            }
        }
        yield Ok::<Bytes, reqwest::Error>(Bytes::from("data: [DONE]\n\n"));
    }
    .boxed();

    GoogleAIProxyResponse {
        status_code: StatusCode::OK,
        headers: event_stream_response_headers(),
        body: GoogleAIProxyResponseBody::Stream(openai_sse_stream),
    }
}

async fn convert_non_streaming_response(
    response: reqwest::Response,
    model: &str,
) -> Result<GoogleAIProxyResponse, Error> {
    let body = response
        .bytes()
        .await
        .map_err(|e| Error::internal_err(format!("Failed to read Gemini response body: {}", e)))?;

    let parsed = parse_gemini_response(&body)?;
    let openai_response = gemini_response_to_openai(&parsed, model);

    let body = serde_json::to_vec(&openai_response)
        .map_err(|e| Error::internal_err(format!("Failed to serialize response: {}", e)))?;

    Ok(GoogleAIProxyResponse {
        status_code: StatusCode::OK,
        headers: json_response_headers(),
        body: GoogleAIProxyResponseBody::Fixed(Bytes::from(body)),
    })
}

fn json_response_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers
}

fn event_stream_response_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    headers.insert(header::CONNECTION, HeaderValue::from_static("keep-alive"));
    headers
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
        stream_event_sink: Box<dyn StreamEventSink>,
    ) -> Result<ParsedResponse, Error> {
        let mut gemini_sse_parser = GeminiSSEParser::new(stream_event_sink);
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
            content: if accumulated_content.is_empty() {
                None
            } else {
                Some(accumulated_content)
            },
            tool_calls: accumulated_tool_calls.into_values().collect(),
            events_str: Some(events_str),
            annotations,
            used_websearch,
            usage,
        })
    }

    fn get_endpoint(&self, base_url: &str, model: &str, output_type: &OutputType) -> String {
        let base_url = base_url.trim_end_matches('/');
        if self.is_vertex() {
            // Vertex AI: base_url is .../publishers/google/models
            match output_type {
                OutputType::Text => {
                    format!("{}/{}:streamGenerateContent?alt=sse", base_url, model)
                }
                OutputType::Image => {
                    let url_suffix = if model.contains("imagen") {
                        "predict"
                    } else {
                        "generateContent"
                    };
                    format!("{}/{}:{}", base_url, model, url_suffix)
                }
            }
        } else {
            // Standard Google AI: base_url is generativelanguage.googleapis.com/v1beta
            match output_type {
                OutputType::Text => {
                    format!(
                        "{}/models/{}:streamGenerateContent?alt=sse",
                        base_url, model
                    )
                }
                OutputType::Image => {
                    let url_suffix = if model.contains("imagen") {
                        "predict"
                    } else {
                        "generateContent"
                    };
                    format!("{}/models/{}:{}", base_url, model, url_suffix)
                }
            }
        }
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        _base_url: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        if self.is_vertex() {
            // Vertex AI uses OAuth2 Bearer token
            vec![("Authorization", format!("Bearer {}", api_key))]
        } else {
            // Standard Google AI uses API key
            vec![("x-goog-api-key", api_key.to_string())]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ai_providers::AIProvider, credentials::ProviderCredentials};
    use std::collections::HashMap;

    fn credentials(base_url: &str, platform: AIPlatform) -> ProviderCredentials {
        ProviderCredentials {
            provider: AIProvider::GoogleAI,
            base_url: base_url.to_string(),
            api_key: Some("api-key".to_string()),
            access_token: None,
            organization_id: None,
            user: None,
            region: None,
            aws_access_key_id: None,
            aws_secret_access_key: None,
            aws_session_token: None,
            platform,
            custom_headers: HashMap::new(),
        }
    }

    #[test]
    fn builds_standard_google_ai_chat_proxy_request() {
        let credentials = credentials(
            "https://generativelanguage.googleapis.com/v1beta/",
            AIPlatform::Standard,
        );
        let method = Method::POST;
        let headers = HeaderMap::new();
        let body = br#"{
            "model": "gemini-2.0-flash",
            "messages": [{"role": "user", "content": "hello"}],
            "temperature": 0.2,
            "max_tokens": 123,
            "stream": false
        }"#;

        let request = build_google_ai_chat_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "chat/completions",
            headers: &headers,
            body,
            credentials: &credentials,
        })
        .unwrap();

        assert_eq!(request.request.method, Method::POST);
        assert_eq!(
            request.request.url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent"
        );
        assert!(!request.stream);
        assert_eq!(request.model, "gemini-2.0-flash");
        assert!(request
            .request
            .headers
            .contains(&("x-goog-api-key".to_string(), "api-key".to_string())));

        let body: serde_json::Value = serde_json::from_slice(&request.request.body).unwrap();
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 123);
        assert_eq!(body["generationConfig"]["temperature"], 0.2);
        // No reasoning_effort in the request -> no thinkingConfig (provider defaults).
        assert!(body["generationConfig"].get("thinkingConfig").is_none());
        assert!(body["contents"].is_array());
    }

    fn proxy_body_for(model: &str, reasoning_effort: &str) -> serde_json::Value {
        let credentials = credentials(
            "https://generativelanguage.googleapis.com/v1beta/",
            AIPlatform::Standard,
        );
        let method = Method::POST;
        let headers = HeaderMap::new();
        let body = format!(
            r#"{{
                "model": "{model}",
                "messages": [{{"role": "user", "content": "hello"}}],
                "reasoning_effort": "{reasoning_effort}",
                "stream": false
            }}"#
        );

        let request = build_google_ai_chat_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "chat/completions",
            headers: &headers,
            body: body.as_bytes(),
            credentials: &credentials,
        })
        .unwrap();

        serde_json::from_slice(&request.request.body).unwrap()
    }

    #[test]
    fn maps_reasoning_effort_to_thinking_level_on_gemini_3() {
        let body = proxy_body_for("gemini-3-pro-preview", "low");
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
            "low"
        );
        // Thought summaries are requested whenever reasoning is on...
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["includeThoughts"],
            true
        );
        // ...but not when the user explicitly turned it off.
        let body = proxy_body_for("gemini-3-pro-preview", "none");
        assert!(body["generationConfig"]["thinkingConfig"]
            .get("includeThoughts")
            .is_none());
        assert!(body["generationConfig"]["thinkingConfig"]
            .get("thinkingBudget")
            .is_none());
    }

    #[test]
    fn maps_reasoning_effort_none_per_gemini_3_model() {
        // Pro cannot disable thinking -> lowest level.
        let body = proxy_body_for("gemini-3-pro-preview", "none");
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
            "low"
        );
        // Flash supports `minimal`.
        let body = proxy_body_for("gemini-3-flash-preview", "none");
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
            "minimal"
        );
    }

    #[test]
    fn maps_reasoning_effort_to_thinking_budget_on_gemini_2_5() {
        let body = proxy_body_for("gemini-2.5-flash", "medium");
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingBudget"],
            8192
        );
        assert!(body["generationConfig"]["thinkingConfig"]
            .get("thinkingLevel")
            .is_none());

        // Off: Flash can fully disable; Pro has a 128-token minimum.
        let body = proxy_body_for("gemini-2.5-flash", "none");
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingBudget"],
            0
        );
        let body = proxy_body_for("gemini-2.5-pro", "none");
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingBudget"],
            128
        );

        // Unknown token -> dynamic budget (provider default behavior).
        let body = proxy_body_for("gemini-2.5-pro", "custom-level");
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingBudget"],
            -1
        );
    }

    #[test]
    fn builds_standard_google_ai_endpoint_from_model_resource_name() {
        assert_eq!(
            build_google_ai_model_endpoint(
                "https://generativelanguage.googleapis.com/v1beta",
                "models/gemini-2.0-flash",
                "generateContent",
                false,
            ),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent"
        );
    }

    #[test]
    fn builds_vertex_google_ai_endpoint_from_model_resource_name() {
        assert_eq!(
            build_google_ai_model_endpoint(
                "https://us-central1-aiplatform.googleapis.com/v1/projects/p/locations/us-central1/publishers/google/models",
                "models/gemini-2.0-flash",
                "streamGenerateContent",
                true,
            ),
            "https://us-central1-aiplatform.googleapis.com/v1/projects/p/locations/us-central1/publishers/google/models/gemini-2.0-flash:streamGenerateContent"
        );
    }

    #[test]
    fn builds_vertex_google_ai_streaming_proxy_request() {
        let credentials = credentials(
            "https://us-central1-aiplatform.googleapis.com/v1/projects/p/locations/us-central1/publishers/google/models/",
            AIPlatform::GoogleVertexAi,
        );
        let method = Method::POST;
        let headers = HeaderMap::new();
        let body = br#"{
            "model": "gemini-2.0-flash",
            "messages": [{"role": "user", "content": "hello"}],
            "stream": true
        }"#;

        let request = build_google_ai_chat_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "chat/completions",
            headers: &headers,
            body,
            credentials: &credentials,
        })
        .unwrap();

        assert_eq!(
            request.request.url,
            "https://us-central1-aiplatform.googleapis.com/v1/projects/p/locations/us-central1/publishers/google/models/gemini-2.0-flash:streamGenerateContent?alt=sse"
        );
        assert!(request.stream);
        assert!(request
            .request
            .headers
            .contains(&("Authorization".to_string(), "Bearer api-key".to_string())));
    }

    #[test]
    fn builds_google_ai_models_proxy_request() {
        let credentials = credentials(
            "https://generativelanguage.googleapis.com/v1beta/",
            AIPlatform::Standard,
        );
        let method = Method::GET;
        let headers = HeaderMap::new();

        let request = build_google_ai_models_proxy_request(&ProxyBuildArgs {
            method: &method,
            path: "models",
            headers: &headers,
            body: &[],
            credentials: &credentials,
        });

        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.url,
            "https://generativelanguage.googleapis.com/v1beta/models"
        );
        assert!(request
            .headers
            .contains(&("x-goog-api-key".to_string(), "api-key".to_string())));
    }
}
