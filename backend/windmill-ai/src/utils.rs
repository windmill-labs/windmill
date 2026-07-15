use crate::{
    ai_providers::AIProvider,
    ai_types::{ContentPart, OpenAIContent, OpenAIMessage},
};
use windmill_common::utils::configure_client;

lazy_static::lazy_static! {
    /// Debug escape hatch: when set, `AI_HTTP_CLIENT` follows redirects again. Off by
    /// default because SSRF validation on `base_url` is single-shot, so a redirect can
    /// bounce a validated public host into a private/internal one (GHSA-5q4v-c4v3-v7wr).
    /// Only enable to debug a non-standard/self-hosted gateway you control.
    pub static ref ALLOW_AI_BASE_URL_REDIRECTS: bool = std::env::var("ALLOW_AI_BASE_URL_REDIRECTS")
        .ok()
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    /// HTTP client for anything targeting a user-configured AI provider `base_url`;
    /// use it instead of the shared `HTTP_CLIENT`. Redirects are governed by
    /// `ALLOW_AI_BASE_URL_REDIRECTS` (disabled by default). Mirrors the API proxy
    /// client (windmill-api/src/ai.rs).
    pub static ref AI_HTTP_CLIENT: reqwest::Client = {
        let redirect = if *ALLOW_AI_BASE_URL_REDIRECTS {
            tracing::warn!(
                "ALLOW_AI_BASE_URL_REDIRECTS is enabled - the AI HTTP client will follow \
                 redirects, weakening SSRF protection on provider base URLs"
            );
            reqwest::redirect::Policy::default()
        } else {
            reqwest::redirect::Policy::none()
        };
        configure_client(reqwest::ClientBuilder::new()
            .user_agent("windmill/beta")
            .connect_timeout(std::time::Duration::from_secs(10))
            .redirect(redirect))
            .build()
            .expect("Failed to build AI HTTP client - check system TLS configuration")
    };

    /// Parse AI_HTTP_HEADERS environment variable into a vector of (header_name, header_value) tuples
    /// Format: "header1: value1, header2: value2"
    pub static ref AI_HTTP_HEADERS: Vec<(String, String)> = {
        std::env::var("AI_HTTP_HEADERS")
            .ok()
            .map(|headers_str| {
                headers_str
                    .split(',')
                    .filter_map(|header| {
                        let parts: Vec<&str> = header.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let name = parts[0].trim().to_string();
                            let value = parts[1].trim().to_string();
                            if !name.is_empty() && !value.is_empty() {
                                Some((name, value))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
}

/// AWS Bedrock do not handle structured output query param, so we use a tool for structured output. Same for every Claude models.
pub fn should_use_structured_output_tool(provider: &AIProvider, model: &str) -> bool {
    model.contains("claude") || provider == &AIProvider::AWSBedrock
}

/// Collect the system prompt for providers that take it in a dedicated top-level field
/// (Anthropic's `system`, OpenAI's `instructions`) instead of inline in the message list.
///
/// Every system message in `messages` is joined, since manual-memory conversations can carry
/// system messages of their own alongside the one the caller prepends from `system_prompt`.
/// `system_prompt` is a fallback used only when `messages` holds no system message, for callers
/// that pass it without prepending it. Only text content survives, so pass just the messages the
/// provider cannot render inline: Anthropic's API takes no system role at all and hands over
/// everything, while OpenAI's accepts system messages inside `input` and hands over only the
/// leading ones. Whatever is passed here must be left out of the message list the provider
/// sends, or the same prompt goes over the wire twice.
pub fn collect_system_prompt(
    messages: &[OpenAIMessage],
    system_prompt: Option<&str>,
) -> Option<String> {
    let from_messages = messages
        .iter()
        .filter(|message| message.role == "system")
        .filter_map(|message| message.content.as_ref().map(extract_text_content))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>();

    if from_messages.is_empty() {
        system_prompt
            .filter(|prompt| !prompt.is_empty())
            .map(str::to_string)
    } else {
        Some(from_messages.join("\n\n"))
    }
}

/// Extract text content from OpenAIContent, joining parts with space if multiple
pub fn extract_text_content(content: &OpenAIContent) -> String {
    match content {
        OpenAIContent::Text(text) => text.clone(),
        OpenAIContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| {
                if let ContentPart::Text { text } = p {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn message(role: &str, text: &str) -> OpenAIMessage {
        OpenAIMessage {
            role: role.to_string(),
            content: Some(OpenAIContent::Text(text.to_string())),
            ..Default::default()
        }
    }

    #[test]
    fn joins_every_system_message() {
        let messages = vec![
            message("system", "be helpful"),
            message("user", "hi"),
            message("system", "be terse"),
        ];

        assert_eq!(
            collect_system_prompt(&messages, Some("be helpful")),
            Some("be helpful\n\nbe terse".to_string())
        );
    }

    /// A dedicated system field is text-only, so non-text parts cannot be carried over.
    #[test]
    fn keeps_only_text_parts_of_a_system_message() {
        let messages = vec![OpenAIMessage {
            role: "system".to_string(),
            content: Some(OpenAIContent::Parts(vec![
                ContentPart::Text { text: "be terse".to_string() },
                ContentPart::ImageUrl {
                    image_url: crate::ai_types::ImageUrlData {
                        url: "data:image/png;base64,x".to_string(),
                    },
                },
            ])),
            ..Default::default()
        }];

        assert_eq!(
            collect_system_prompt(&messages, None),
            Some("be terse".to_string())
        );
    }

    /// The argument is a fallback, not an extra source: system messages win outright.
    #[test]
    fn prefers_system_messages_over_the_argument() {
        let messages = vec![message("system", "be terse"), message("user", "hi")];

        assert_eq!(
            collect_system_prompt(&messages, Some("unused fallback")),
            Some("be terse".to_string())
        );
    }

    #[test]
    fn falls_back_to_the_system_prompt_argument() {
        let messages = vec![message("user", "hi")];

        assert_eq!(
            collect_system_prompt(&messages, Some("be helpful")),
            Some("be helpful".to_string())
        );
    }

    #[test]
    fn treats_empty_prompts_as_absent() {
        assert_eq!(
            collect_system_prompt(&[message("user", "hi")], Some("")),
            None
        );
        assert_eq!(collect_system_prompt(&[message("user", "hi")], None), None);
        assert_eq!(collect_system_prompt(&[message("system", "")], None), None);
    }

    /// Regression test for GHSA-5q4v-c4v3-v7wr: `AI_HTTP_CLIENT` must not follow redirects.
    #[tokio::test]
    async fn ai_http_client_does_not_follow_redirects() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Serve a single 302 to the link-local metadata address; the client must
        // surface the 302 rather than connect onward to it.
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let response = "HTTP/1.1 302 Found\r\n\
                    Location: http://169.254.169.254/latest/meta-data\r\n\
                    Content-Length: 0\r\n\r\n";
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            }
        });

        let resp = AI_HTTP_CLIENT
            .get(format!("http://{}/", addr))
            .send()
            .await
            .expect("request should succeed without following the redirect");

        assert_eq!(
            resp.status().as_u16(),
            302,
            "AI HTTP client must surface the redirect response, not follow it"
        );
    }
}
