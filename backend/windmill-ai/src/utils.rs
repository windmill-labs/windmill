use crate::{
    ai_providers::AIProvider,
    ai_types::{ContentPart, OpenAIContent},
};
use windmill_common::utils::configure_client;

lazy_static::lazy_static! {
    /// HTTP client for requests to user-configured AI provider endpoints. Must be
    /// used instead of the shared `HTTP_CLIENT` for anything targeting a provider
    /// `base_url`. SSRF validation on `base_url` is single-shot (see
    /// `AIProvider::get_base_url`), so redirects MUST stay disabled: otherwise a
    /// validated public host could 3xx the worker into a private/internal address.
    /// AI APIs respond directly, so this holds even for ALLOW_PRIVATE_AI_BASE_URLS.
    /// Mirrors the API proxy client (windmill-api/src/ai.rs, GHSA-5q4v-c4v3-v7wr).
    pub static ref AI_HTTP_CLIENT: reqwest::Client = configure_client(reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .connect_timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none()))
        .build()
        .expect("Failed to build AI HTTP client - check system TLS configuration");

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
