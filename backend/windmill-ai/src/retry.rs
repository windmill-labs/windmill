use std::time::Duration;

use reqwest::{header::HeaderMap, RequestBuilder, Response, StatusCode};
use windmill_common::error::{to_anyhow, Error};

/// How many times a provider call is retried after its first attempt.
pub const MAX_RETRIES: u32 = 3;
const BASE_DELAY: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(10);
/// A provider that asks for a longer wait is out of capacity for longer than anyone should
/// block on (a spent daily quota, say), so its error is returned instead of waited out.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

/// A provider call that failed for a reason a later attempt may not meet: the provider was
/// overloaded, throttled or unreachable, or its response broke off part way.
#[derive(Debug)]
pub struct TransientAIError {
    pub message: String,
    /// The wait the provider asked for, when it named one.
    pub retry_after: Option<Duration>,
}

impl std::fmt::Display for TransientAIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TransientAIError {}

pub fn transient_error(message: impl Into<String>, retry_after: Option<Duration>) -> Error {
    to_anyhow(TransientAIError { message: message.into(), retry_after }).into()
}

pub fn as_transient(error: &Error) -> Option<&TransientAIError> {
    match error {
        Error::Anyhow { error, .. } => error.downcast_ref::<TransientAIError>(),
        _ => None,
    }
}

/// The error to report once a transient failure is no longer retried, without the source
/// location `Error::Anyhow` renders.
pub fn into_final_error(error: Error) -> Error {
    match as_transient(&error) {
        Some(transient) => Error::AIError(transient.message.clone()),
        None => error,
    }
}

/// Whether an error status from a provider may clear on another attempt.
pub fn is_transient_status(status: StatusCode, body: &str) -> bool {
    let transient = matches!(
        status,
        StatusCode::REQUEST_TIMEOUT | StatusCode::CONFLICT | StatusCode::TOO_MANY_REQUESTS
    ) || (status.is_server_error()
        && !matches!(
            status,
            StatusCode::NOT_IMPLEMENTED | StatusCode::HTTP_VERSION_NOT_SUPPORTED
        ));
    // OpenAI answers a spent balance with the 429 it also uses for rate limits.
    transient && !body.contains("insufficient_quota")
}

/// The wait a provider asked for in its `retry-after-ms` or `retry-after` header. An
/// HTTP-date `retry-after` is ignored, which falls back to the computed backoff.
pub fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let seconds = |name: &str, scale: f64| {
        headers
            .get(name)?
            .to_str()
            .ok()?
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v >= 0.0)
            .map(|v| Duration::try_from_secs_f64(v / scale).unwrap_or(Duration::MAX))
    };
    seconds("retry-after-ms", 1000.0).or_else(|| seconds("retry-after", 1.0))
}

/// The error for a failure the provider reported inside a stream it had already opened
/// with a 200. The request was accepted, so the failure is taken to be the provider's own
/// unless its status or kind names the request as the cause.
pub fn stream_error(status: Option<u16>, kind: Option<&str>, message: String) -> Error {
    let transient = match (status.and_then(|s| StatusCode::from_u16(s).ok()), kind) {
        (Some(status), _) => is_transient_status(status, kind.unwrap_or_default()),
        (None, Some(kind)) => {
            let kind = kind.to_ascii_lowercase();
            ![
                "invalid",
                "authentication",
                "permission",
                "not_found",
                "quota",
                "content_filter",
                "context_length",
                "too_large",
            ]
            .iter()
            .any(|marker| kind.contains(marker))
        }
        (None, None) => true,
    };
    if transient {
        transient_error(message, None)
    } else {
        Error::AIError(message)
    }
}

/// The error object a provider sends inside a stream: OpenAI-compatible chunks carry
/// `type` and a string or status `code`, Anthropic `type`, Gemini a status `code` and
/// `status`.
#[derive(Debug, Default, serde::Deserialize)]
pub struct StreamErrorBody {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub code: Option<serde_json::Value>,
    #[serde(default)]
    pub status: Option<String>,
}

impl StreamErrorBody {
    pub fn into_error(self, provider: &str) -> Error {
        let status = self
            .code
            .as_ref()
            .and_then(|code| code.as_u64())
            .and_then(|code| u16::try_from(code).ok());
        let kind = [
            self.kind.as_deref(),
            self.code.as_ref().and_then(|code| code.as_str()),
            self.status.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("/");
        let kind = (!kind.is_empty()).then_some(kind);
        let message = format!(
            "{provider} stream error{}: {}",
            kind.as_ref().map(|k| format!(" ({k})")).unwrap_or_default(),
            self.message.as_deref().unwrap_or("no details")
        );
        stream_error(status, kind.as_deref(), message)
    }
}

/// The error for a stream that closed before the provider sent the event ending a
/// complete response: cut off by the provider or by a proxy in between.
pub fn truncated_stream_error() -> Error {
    transient_error(
        "The AI provider's response stream ended before the response was complete",
        None,
    )
}

/// An OpenAI-format stream chunk reporting `error`, which ends a response converted to that
/// format after it started streaming: the chat's OpenAI SDK throws on a chunk carrying
/// `error`, where a bare end of stream would read as a finished answer.
pub fn openai_stream_error_chunk(error: &Error) -> String {
    let message = as_transient(error).map_or_else(|| error.to_string(), |t| t.message.clone());
    format!(
        "data: {}\n\n",
        serde_json::json!({ "error": { "message": message } })
    )
}

/// Sends a provider request, retrying a transient failure while nothing of the response
/// has been read: a failed connection or an error status. Once retries are spent the error
/// carries the provider's answer, as a single attempt's would.
pub async fn send_with_retries(request: RequestBuilder) -> Result<Response, Error> {
    let mut backoff = Backoff::default();
    loop {
        // A streamed body cannot be replayed, so such a request is sent once.
        let Some(attempt) = request.try_clone() else {
            return send_once(request).await.map_err(into_final_error);
        };
        let error = match send_once(attempt).await {
            Ok(response) => return Ok(response),
            Err(error) => error,
        };
        let Some(delay) = as_transient(&error).and_then(|t| backoff.next_delay(t)) else {
            return Err(into_final_error(error));
        };
        tracing::warn!(
            "AI provider call failed ({error}); retrying in {:.1}s ({}/{MAX_RETRIES})",
            delay.as_secs_f64(),
            backoff.retries()
        );
        tokio::time::sleep(delay).await;
    }
}

async fn send_once(request: RequestBuilder) -> Result<Response, Error> {
    let response = request.send().await.map_err(|e| {
        let message = format!("Failed to reach the AI provider: {e}");
        if e.is_builder() || (e.is_timeout() && !e.is_connect()) {
            Error::AIError(message)
        } else {
            transient_error(message, None)
        }
    })?;
    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        let wait = retry_after(response.headers());
        let body = response.text().await.unwrap_or_default();
        return Err(if is_transient_status(status, &body) {
            transient_error(body, wait)
        } else {
            Error::AIError(body)
        });
    }
    Ok(response)
}

/// Counts the retries of one provider call and spaces them out.
#[derive(Debug, Default)]
pub struct Backoff {
    retries: u32,
}

impl Backoff {
    /// The wait before retrying a call that failed with `error`, or `None` once it should
    /// fail instead.
    pub fn next_delay(&mut self, error: &TransientAIError) -> Option<Duration> {
        if self.retries >= MAX_RETRIES {
            return None;
        }
        let delay = match error.retry_after {
            Some(wait) if wait > MAX_RETRY_AFTER => return None,
            // A zero wait would retry back to back, so it gets the computed backoff.
            Some(wait) if !wait.is_zero() => wait,
            // Jittered, so callers throttled together do not come back together.
            _ => (BASE_DELAY * 2u32.pow(self.retries))
                .min(MAX_BACKOFF)
                .mul_f64(0.5 + rand::random::<f64>() * 0.5),
        };
        self.retries += 1;
        Some(delay)
    }

    /// Retries granted so far.
    pub fn retries(&self) -> u32 {
        self.retries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_provider_statuses() {
        for status in [408, 409, 429, 500, 502, 503, 504, 529] {
            let status = StatusCode::from_u16(status).unwrap();
            assert!(is_transient_status(status, ""), "{status}");
        }
        for status in [400, 401, 403, 404, 413, 422, 501] {
            let status = StatusCode::from_u16(status).unwrap();
            assert!(!is_transient_status(status, ""), "{status}");
        }
        assert!(!is_transient_status(
            StatusCode::TOO_MANY_REQUESTS,
            r#"{"error":{"type":"insufficient_quota","code":"insufficient_quota"}}"#
        ));
    }

    #[test]
    fn classifies_mid_stream_errors() {
        let transient = |e: Error| as_transient(&e).is_some();
        assert!(transient(stream_error(
            None,
            Some("overloaded_error"),
            "Overloaded".into()
        )));
        assert!(transient(stream_error(
            None,
            Some("server_error"),
            "x".into()
        )));
        assert!(transient(stream_error(
            Some(503),
            Some("UNAVAILABLE"),
            "x".into()
        )));
        assert!(transient(stream_error(None, None, "x".into())));
        assert!(!transient(stream_error(
            None,
            Some("invalid_request_error"),
            "x".into()
        )));
        assert!(!transient(stream_error(
            Some(400),
            Some("INVALID_ARGUMENT"),
            "x".into()
        )));
    }

    #[test]
    fn honours_retry_after_up_to_a_bound() {
        let mut headers = HeaderMap::new();
        headers.insert("retry-after", "2".parse().unwrap());
        assert_eq!(retry_after(&headers), Some(Duration::from_secs(2)));
        headers.insert("retry-after-ms", "1500".parse().unwrap());
        assert_eq!(retry_after(&headers), Some(Duration::from_millis(1500)));
        headers.remove("retry-after-ms");
        headers.insert("retry-after", "1e30".parse().unwrap());
        assert_eq!(retry_after(&headers), Some(Duration::MAX));

        let mut backoff = Backoff::default();
        let asked = |secs| TransientAIError {
            message: String::new(),
            retry_after: Some(Duration::from_secs(secs)),
        };
        assert_eq!(backoff.next_delay(&asked(5)), Some(Duration::from_secs(5)));
        assert_eq!(backoff.next_delay(&asked(3600)), None);
    }

    #[test]
    fn stops_after_max_retries() {
        let mut backoff = Backoff::default();
        let error = TransientAIError { message: String::new(), retry_after: None };
        for _ in 0..MAX_RETRIES {
            let delay = backoff.next_delay(&error).expect("retry");
            assert!(delay <= MAX_BACKOFF);
        }
        assert_eq!(backoff.next_delay(&error), None);
    }
}
