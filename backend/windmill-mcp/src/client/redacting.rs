use std::{collections::HashMap, future::Future, sync::Arc};

use futures::stream::BoxStream;
use reqwest::header::{HeaderName, HeaderValue};
use rmcp::{
    model::ClientJsonRpcMessage,
    transport::streamable_http_client::{
        SseError, StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
    },
};
use sse_stream::Sse;
use tracing::{instrument::WithSubscriber, Dispatch};

type Error = StreamableHttpError<reqwest::Error>;

/// reqwest names the URL it failed on in its errors, and an MCP server URL can
/// carry the server's key. rmcp logs those errors as well as returning them, so
/// the URL is dropped before the transport sees one, and rmcp's reqwest client,
/// which logs the errors it swallows, runs without a subscriber.
#[derive(Clone)]
pub(super) struct UrlRedactingClient(pub(super) reqwest::Client);

async fn redacted<T>(
    uri: Arc<str>,
    request: impl Future<Output = Result<T, Error>>,
) -> Result<T, Error> {
    match request.with_subscriber(Dispatch::none()).await {
        Err(StreamableHttpError::Client(e)) => Err(StreamableHttpError::Client(e.without_url())),
        Err(StreamableHttpError::UnexpectedServerResponse(body)) => {
            Err(StreamableHttpError::UnexpectedServerResponse(
                without_request_target(&uri, &body).into(),
            ))
        }
        r => r,
    }
}

/// The body of an error response is the server's to write, and a common one
/// echoes the request target (`Cannot POST /mcp/<key>`).
fn without_request_target(uri: &str, body: &str) -> String {
    let mut body = body.replace(uri, "<url>");
    if let Ok(url) = reqwest::Url::parse(uri) {
        for part in [Some(url.path()).filter(|p| p.len() > 1), url.query()] {
            if let Some(part) = part.filter(|p| !p.is_empty()) {
                body = body.replace(part, "<redacted>");
            }
        }
    }
    body
}

impl StreamableHttpClient for UrlRedactingClient {
    type Error = reqwest::Error;

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<StreamableHttpPostResponse, Error> {
        redacted(
            uri.clone(),
            self.0
                .post_message(uri, message, session_id, auth_header, custom_headers),
        )
        .await
    }

    async fn post_message_with_max_sse_event_size(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
        max_sse_event_size: usize,
    ) -> Result<StreamableHttpPostResponse, Error> {
        redacted(
            uri.clone(),
            self.0.post_message_with_max_sse_event_size(
                uri,
                message,
                session_id,
                auth_header,
                custom_headers,
                max_sse_event_size,
            ),
        )
        .await
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<(), Error> {
        redacted(
            uri.clone(),
            self.0
                .delete_session(uri, session_id, auth_header, custom_headers),
        )
        .await
    }

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Option<Arc<str>>,
        last_event_id: Option<String>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<BoxStream<'static, Result<Sse, SseError>>, Error> {
        redacted(
            uri.clone(),
            self.0
                .get_stream(uri, session_id, last_event_id, auth_header, custom_headers),
        )
        .await
    }

    async fn get_stream_with_max_sse_event_size(
        &self,
        uri: Arc<str>,
        session_id: Option<Arc<str>>,
        last_event_id: Option<String>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
        max_sse_event_size: usize,
    ) -> Result<BoxStream<'static, Result<Sse, SseError>>, Error> {
        redacted(
            uri.clone(),
            self.0.get_stream_with_max_sse_event_size(
                uri,
                session_id,
                last_event_id,
                auth_header,
                custom_headers,
                max_sse_event_size,
            ),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::without_request_target;

    #[test]
    fn error_body_loses_the_request_target() {
        let uri = "https://mcp.example.com/mcp/KEY?token=T0K";
        for body in [
            "Cannot POST /mcp/KEY",
            "no route for /mcp/KEY?token=T0K",
            uri,
        ] {
            let scrubbed = without_request_target(uri, &format!("HTTP 404: {body}"));
            assert!(
                !scrubbed.contains("KEY") && !scrubbed.contains("T0K"),
                "{scrubbed}"
            );
        }
        assert_eq!(
            without_request_target("https://mcp.example.com/", "a/b"),
            "a/b"
        );
    }
}
