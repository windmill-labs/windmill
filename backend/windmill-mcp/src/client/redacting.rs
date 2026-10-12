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
use tracing::{instrument::WithSubscriber, subscriber::NoSubscriber};

type Error = StreamableHttpError<reqwest::Error>;

/// reqwest names the URL it failed on in its errors, and an MCP server URL can
/// carry the server's key. rmcp logs those errors as well as returning them, so
/// the URL is dropped before the transport sees one, and rmcp's reqwest client,
/// which logs the errors it swallows, runs without a subscriber.
#[derive(Clone)]
pub(super) struct UrlRedactingClient(pub(super) reqwest::Client);

async fn redacted<T>(request: impl Future<Output = Result<T, Error>>) -> Result<T, Error> {
    match request.with_subscriber(NoSubscriber::default()).await {
        Err(StreamableHttpError::Client(e)) => Err(StreamableHttpError::Client(e.without_url())),
        r => r,
    }
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
        redacted(self.0.post_message_with_max_sse_event_size(
            uri,
            message,
            session_id,
            auth_header,
            custom_headers,
            max_sse_event_size,
        ))
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
        redacted(self.0.get_stream_with_max_sse_event_size(
            uri,
            session_id,
            last_event_id,
            auth_header,
            custom_headers,
            max_sse_event_size,
        ))
        .await
    }
}
