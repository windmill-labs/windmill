//! A single text completion with no tools and no streaming reader: agent-step history
//! compaction and the `/ai/complete` endpoint both send one through the provider's query
//! builder, so they share its request shape, credential headers and error handling.

use async_trait::async_trait;
use windmill_common::{client::AuthedClient, error::Error};

use crate::{
    ai_providers::AIProvider,
    credentials::ProviderCredentials,
    proxy::{common_outbound_headers, retain_effective_credentials},
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventSink},
    types::{OpenAIMessage, OutputType, StreamingEvent, TokenUsage},
    utils::pinned_ai_client_for,
};

pub struct OneShotRequest<'a> {
    pub query_builder: &'a dyn QueryBuilder,
    /// For Bedrock, the keys an assumed OIDC role yielded are expected here already.
    pub credentials: &'a ProviderCredentials,
    pub model: &'a str,
    pub max_tokens: Option<u32>,
    pub reasoning_effort: Option<&'a str>,
    pub timeout: std::time::Duration,
    /// Only reached for attachments, which a one-shot request never carries.
    pub client: &'a AuthedClient,
    pub workspace_id: &'a str,
    /// An already exchanged Anthropic federation token, sent in place of the api key.
    pub federated_token: Option<&'a str>,
    /// Whether to send the usage-tracking request shape. A gateway that rejects it is
    /// retried once without it either way.
    pub include_usage: bool,
}

struct DiscardEvents;

#[async_trait]
impl StreamEventSink for DiscardEvents {
    async fn send(&self, _event: StreamingEvent, _events_str: &mut String) -> Result<(), Error> {
        Ok(())
    }
}

pub async fn complete_text(
    messages: &[OpenAIMessage],
    request: &OneShotRequest<'_>,
) -> Result<(String, Option<TokenUsage>), Error> {
    let build_args = BuildRequestArgs {
        messages,
        tools: None,
        model: request.model,
        // OpenAI's reasoning models reject `temperature` alongside any effort but their
        // own default, so it is left to the provider rather than tracked per model.
        temperature: None,
        reasoning_effort: request.reasoning_effort,
        // Nothing reads this call's reasoning, and a summary of it would be billed.
        reasoning_summary: false,
        max_tokens: request.max_tokens,
        output_schema: None,
        output_type: &OutputType::Text,
        system_prompt: None,
        user_message: "",
        attachments: None,
        has_websearch: false,
        prompt_cache_key: None,
    };

    let parsed = if request.credentials.provider == AIProvider::AWSBedrock {
        #[cfg(feature = "bedrock")]
        {
            let credentials = request.credentials;
            crate::providers::bedrock::BedrockQueryBuilder::default()
                .execute_request(
                    messages,
                    None,
                    request.model,
                    None,
                    build_args.reasoning_effort,
                    build_args.max_tokens,
                    credentials.api_key.as_deref().unwrap_or(""),
                    credentials
                        .region
                        .as_deref()
                        .unwrap_or(crate::ai_providers::USE_ENV_REGION),
                    Some(Box::new(DiscardEvents)),
                    request.client,
                    request.workspace_id,
                    None,
                    credentials.aws_access_key_id.as_deref(),
                    credentials.aws_secret_access_key.as_deref(),
                    credentials.aws_session_token.as_deref(),
                )
                .await?
        }
        #[cfg(not(feature = "bedrock"))]
        {
            return Err(Error::internal_err(
                "AWS Bedrock support is not enabled. Build with 'bedrock' feature.".to_string(),
            ));
        }
    } else {
        send_http(&build_args, request).await?
    };

    match parsed.without_nul() {
        ParsedResponse::Text { content, usage, .. } => Ok((content.unwrap_or_default(), usage)),
        ParsedResponse::Image { .. } => Err(Error::internal_err(
            "The completion came back as an image".to_string(),
        )),
    }
}

async fn send_http(
    build_args: &BuildRequestArgs<'_>,
    request: &OneShotRequest<'_>,
) -> Result<ParsedResponse, Error> {
    let credentials = request.credentials;
    let query_builder = request.query_builder;
    let base_url = &credentials.base_url;
    let api_key = credentials.api_key.as_deref().unwrap_or("");
    let endpoint = query_builder.get_endpoint(base_url, request.model, &OutputType::Text);
    let auth_headers = match request.federated_token {
        Some(token) => crate::providers::anthropic::federated_auth_headers(token),
        None => retain_effective_credentials(
            credentials,
            query_builder.get_auth_headers(api_key, base_url, &OutputType::Text),
        ),
    };
    // `endpoint` derives from the user-controlled provider base_url, so DNS is pinned to
    // the SSRF-validated address.
    let client = pinned_ai_client_for(base_url).await?;

    let mut include_usage = request.include_usage;
    loop {
        let body = if include_usage {
            query_builder
                .build_request(build_args, request.client, request.workspace_id)
                .await?
        } else {
            query_builder
                .build_request_without_usage(build_args, request.client, request.workspace_id)
                .await?
        };
        let mut http_request = client
            .post(&endpoint)
            .timeout(request.timeout)
            .header("Content-Type", "application/json");
        for (header_name, header_value) in &auth_headers {
            http_request = http_request.header(*header_name, header_value.clone());
        }
        for (header_name, header_value) in common_outbound_headers(credentials) {
            http_request = http_request.header(header_name.as_str(), header_value.as_str());
        }
        let resp = http_request
            .body(body)
            .send()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to call API: {e}")))?;

        if let Err(e) = resp.error_for_status_ref() {
            let status = resp.status();
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read body>".to_string());
            // Not every OpenAI-compatible gateway accepts `stream_options`.
            if include_usage
                && query_builder.supports_retry_without_usage()
                && status.as_u16() == 400
                && (text.contains("stream_options")
                    || text.contains("include_usage")
                    || text.contains("Additional properties are not allowed"))
            {
                include_usage = false;
                continue;
            }
            return Err(Error::internal_err(format!(
                "API error calling {endpoint}: {e} - {text}"
            )));
        }

        return query_builder
            .parse_streaming_response(resp, Box::new(DiscardEvents))
            .await;
    }
}
