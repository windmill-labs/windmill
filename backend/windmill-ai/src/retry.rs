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

/// The statuses the OpenAI and Anthropic SDKs and the Vercel AI SDK retry.
fn is_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 409 | 429 | 500..=599)
}

/// The error for a provider's error response. The rules follow the OpenAI and Anthropic
/// SDKs (`x-should-retry`, then the status) and gemini-cli (a daily quota is spent until
/// tomorrow, and the wait Gemini asks for is in its body).
pub fn response_error(
    status: StatusCode,
    headers: &HeaderMap,
    body: &str,
    message: String,
) -> Error {
    let retryable = match headers.get("x-should-retry").and_then(|v| v.to_str().ok()) {
        Some("true") => true,
        Some("false") => false,
        _ => {
            is_retryable_status(status.as_u16())
                // OpenAI answers a spent balance with the 429 it also uses for rate limits.
                && !body.contains("insufficient_quota")
                && !google_quota_spent_for_the_day(body)
        }
    };
    if retryable {
        transient_error(
            message,
            retry_after(headers).or_else(|| google_retry_delay(body)),
        )
    } else {
        Error::AIError(message)
    }
}

/// The `details` of a Google API error body, where Gemini puts its quota and retry hints.
fn google_error_details(body: &str) -> Vec<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("error")?.get("details")?.as_array().cloned())
        .unwrap_or_default()
}

fn google_quota_spent_for_the_day(body: &str) -> bool {
    google_error_details(body).iter().any(|detail| {
        detail
            .get("violations")
            .and_then(|v| v.as_array())
            .is_some_and(|violations| {
                violations.iter().any(|violation| {
                    violation
                        .get("quotaId")
                        .and_then(|id| id.as_str())
                        .is_some_and(|id| id.contains("PerDay") || id.contains("Daily"))
                })
            })
    })
}

/// The `RetryInfo.retryDelay` of a Google API error, a protobuf duration such as `"53s"`.
fn google_retry_delay(body: &str) -> Option<Duration> {
    google_error_details(body).iter().find_map(|detail| {
        let delay = detail.get("retryDelay")?.as_str()?.strip_suffix('s')?;
        let seconds = delay
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v >= 0.0)?;
        Some(Duration::try_from_secs_f64(seconds).unwrap_or(Duration::MAX))
    })
}

/// The wait a provider asked for in its `retry-after-ms` or `retry-after` header. An
/// HTTP-date `retry-after` is ignored, which falls back to the computed backoff.
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
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
    /// The error for a failure an OpenAI-compatible or Gemini stream reported after opening
    /// with a 200, classified as the Vercel AI SDK classifies OpenAI stream errors
    /// (`openai-stream-error.ts`): the status `code` holds, else the status its code and type
    /// name, else a server error.
    pub fn into_error(self, provider: &str) -> Error {
        let code = self.code_str();
        let discriminator = [code.as_deref(), self.kind.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let has = |terms: &[&str]| terms.iter().any(|term| discriminator.contains(term));
        let status = match code.as_deref().and_then(|code| code.parse::<u16>().ok()) {
            Some(status @ 400..=599) => status,
            _ if has(&["insufficient_quota", "rate_limit"]) => 429,
            _ if has(&["authentication"]) => 401,
            _ if has(&["permission"]) => 403,
            _ if has(&["not_found"]) => 404,
            _ if has(&["invalid", "bad_request", "context_length"]) => 400,
            _ if has(&["overload"]) => 503,
            _ if has(&["timeout"]) => 504,
            _ => 500,
        };
        let retryable = !has(&["insufficient_quota"]) && is_retryable_status(status);
        self.into_classified_error(provider, retryable)
    }

    /// The error for a failure an Anthropic stream reported after opening with a 200. The
    /// kinds retried are those the Vercel AI SDK retries, plus `timeout_error`, which
    /// Anthropic documents as its 504; any other kind is a fault in the request or account.
    pub fn into_anthropic_error(self) -> Error {
        let retryable = matches!(
            self.kind.as_deref(),
            Some("api_error" | "overloaded_error" | "rate_limit_error" | "timeout_error")
        );
        self.into_classified_error("Anthropic", retryable)
    }

    fn code_str(&self) -> Option<String> {
        match self.code.as_ref()? {
            serde_json::Value::String(code) => Some(code.clone()),
            serde_json::Value::Number(code) => Some(code.to_string()),
            _ => None,
        }
    }

    fn into_classified_error(self, provider: &str, retryable: bool) -> Error {
        let kind = [self.kind.clone(), self.code_str(), self.status]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("/");
        let message = format!(
            "{provider} stream error{}: {}",
            (!kind.is_empty())
                .then(|| format!(" ({kind})"))
                .unwrap_or_default(),
            self.message.as_deref().unwrap_or("no details")
        );
        if retryable {
            transient_error(message, None)
        } else {
            Error::AIError(message)
        }
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

/// The error for a provider request that failed before any response arrived. The request
/// timeout is the caller's whole budget, so a timed-out call leaves no time for another
/// attempt; a connect timeout has its own, shorter one.
pub fn send_error(e: reqwest::Error, context: &str) -> Error {
    let message = format!("{context}: {e}");
    if e.is_builder() || (e.is_timeout() && !e.is_connect()) {
        Error::AIError(message)
    } else {
        transient_error(message, None)
    }
}

async fn send_once(request: RequestBuilder) -> Result<Response, Error> {
    let response = request
        .send()
        .await
        .map_err(|e| send_error(e, "Failed to reach the AI provider"))?;
    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        let headers = response.headers().clone();
        let body = response.text().await.unwrap_or_default();
        return Err(response_error(status, &headers, &body, body.clone()));
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

    /// Error responses as providers send them, each with the retry decision of the client
    /// the rule is taken from.
    #[test]
    fn classifies_error_responses() {
        const GEMINI_QUOTA: &str = r#"{"error":{"code":429,"status":"RESOURCE_EXHAUSTED","details":[{"@type":"type.googleapis.com/google.rpc.QuotaFailure","violations":[{"quotaId":"QUOTA_ID"}]},{"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"53s"}]}}"#;
        let cases: &[(u16, &[(&str, &str)], &str, Option<Option<u64>>)] = &[
            (
                429,
                &[],
                r#"{"error":{"code":"rate_limit_exceeded"}}"#,
                Some(None),
            ),
            (
                429,
                &[],
                r#"{"error":{"type":"insufficient_quota","code":"insufficient_quota"}}"#,
                None,
            ),
            (
                529,
                &[],
                r#"{"type":"error","error":{"type":"overloaded_error"}}"#,
                Some(None),
            ),
            (501, &[], "", Some(None)),
            (400, &[], "", None),
            (401, &[], "", None),
            (400, &[("x-should-retry", "true")], "", Some(None)),
            (503, &[("x-should-retry", "false")], "", None),
            (429, &[("retry-after", "2")], "", Some(Some(2))),
            (
                429,
                &[],
                &GEMINI_QUOTA.replace("QUOTA_ID", "GenerateRequestsPerMinutePerProjectPerModel"),
                Some(Some(53)),
            ),
            (
                429,
                &[],
                &GEMINI_QUOTA.replace(
                    "QUOTA_ID",
                    "GenerateRequestsPerDayPerProjectPerModel-FreeTier",
                ),
                None,
            ),
        ];
        for (status, headers, body, expected) in cases {
            let headers = headers
                .iter()
                .map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap()))
                .collect::<HeaderMap>();
            let error = response_error(
                StatusCode::from_u16(*status).unwrap(),
                &headers,
                body,
                String::new(),
            );
            let got = as_transient(&error).map(|t| t.retry_after.map(|d| d.as_secs()));
            assert_eq!(&got, expected, "{status} {headers:?} {body}");
        }
    }

    /// In-stream error objects as providers send them, after the 200 opened the stream.
    #[test]
    fn classifies_stream_errors() {
        let retried = |json: &str, anthropic: bool| {
            let body: StreamErrorBody = serde_json::from_str(json).unwrap();
            let error = if anthropic {
                body.into_anthropic_error()
            } else {
                body.into_error("AI provider")
            };
            as_transient(&error).is_some()
        };
        for (json, expected) in [
            (r#"{"message":"m","type":"server_error"}"#, true),
            (
                r#"{"message":"m","type":"requests","code":"rate_limit_exceeded"}"#,
                true,
            ),
            (
                r#"{"message":"m","type":"insufficient_quota","code":"insufficient_quota"}"#,
                false,
            ),
            (
                r#"{"message":"m","type":"invalid_request_error","code":"context_length_exceeded"}"#,
                false,
            ),
            // LiteLLM's proxy puts the upstream status in `code` as a string.
            (r#"{"message":"m","type":"None","code":"400"}"#, false),
            (r#"{"message":"m","code":"503"}"#, true),
            (r#"{"code":503,"message":"m","status":"UNAVAILABLE"}"#, true),
            (
                r#"{"code":400,"message":"m","status":"INVALID_ARGUMENT"}"#,
                false,
            ),
            (r#"{"message":"m"}"#, true),
        ] {
            assert_eq!(retried(json, false), expected, "{json}");
        }
        for (kind, expected) in [
            ("overloaded_error", true),
            ("api_error", true),
            ("rate_limit_error", true),
            ("timeout_error", true),
            ("billing_error", false),
            ("invalid_request_error", false),
            ("some_new_error", false),
        ] {
            let json = format!(r#"{{"type":"{kind}","message":"m"}}"#);
            assert_eq!(retried(&json, true), expected, "{kind}");
        }
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
