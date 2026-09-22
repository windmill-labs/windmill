//! AI decision steps: one call to TypeSafe's System One endpoint, which answers typed questions
//! (`choice`, `score`, `noul`) about a state with calibrated probabilities instead of text.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{value::RawValue, Map, Value};
use windmill_common::error::{Error, Result};

use crate::{
    credentials::ProviderCredentials,
    proxy::{common_outbound_headers, retain_effective_credentials},
    types::{ProviderWithResource, TokenUsage},
    utils::pinned_ai_client_for,
};

/// The inputs of an AI decision step or tool, as its `input_transforms` resolve them.
#[derive(Deserialize, Debug)]
pub struct AIDecisionArgs {
    pub provider: ProviderWithResource,
    #[serde(default)]
    pub state: Option<Value>,
    #[serde(default)]
    pub questions: Option<Value>,
}

/// The alias TypeSafe keeps on its current model, sent when the step names none.
pub const TYPESAFE_DEFAULT_MODEL: &str = "jev-latest";

#[derive(Serialize)]
struct SystemOneRequest<'a> {
    model: &'a str,
    state: &'a Value,
    questions: &'a Map<String, Value>,
}

#[derive(Deserialize)]
struct SystemOneResponse {
    #[serde(default)]
    model: Option<String>,
    answers: Box<RawValue>,
    #[serde(default)]
    usage: Option<SystemOneUsage>,
}

#[derive(Deserialize)]
struct SystemOneUsage {
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
}

/// A decision's result. `output` holds the answers keyed by question name, where an agent's
/// holds its answer, so a later step reads both as `results.<step>.output`.
#[derive(Serialize)]
pub struct DecisionResult {
    pub output: Box<RawValue>,
    /// The version that answered, which `jev-latest` resolves to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

/// The state and questions of a decision, refused here rather than by the endpoint when
/// either is missing or of a shape TypeSafe rejects: a flow that passes an empty expression result
/// should read what it forgot, without a request going out.
pub fn decision_inputs<'a>(
    state: Option<&'a Value>,
    questions: Option<&'a Value>,
) -> Result<(&'a Value, &'a Map<String, Value>)> {
    let state = match state {
        None | Some(Value::Null) => {
            return Err(Error::BadRequest(
                "'state' must be provided for an AI decision".to_string(),
            ))
        }
        Some(Value::String(s)) if s.trim().is_empty() => {
            return Err(Error::BadRequest(
                "'state' must be provided for an AI decision".to_string(),
            ))
        }
        Some(state @ (Value::String(_) | Value::Object(_) | Value::Array(_))) => state,
        Some(_) => {
            return Err(Error::BadRequest(
                "'state' must be a text, an object or an array for an AI decision".to_string(),
            ))
        }
    };
    let questions =
        match questions {
            Some(Value::Object(questions)) if !questions.is_empty() => questions,
            _ => return Err(Error::BadRequest(
                "'questions' must be an object naming at least one question for an AI decision"
                    .to_string(),
            )),
        };
    Ok((state, questions))
}

pub async fn run_systemone(
    credentials: &ProviderCredentials,
    model: &str,
    state: &Value,
    questions: &Map<String, Value>,
    timeout: Duration,
) -> Result<DecisionResult> {
    let model = if model.trim().is_empty() {
        TYPESAFE_DEFAULT_MODEL
    } else {
        model
    };
    let endpoint = format!("{}/systemone", credentials.base_url.trim_end_matches('/'));
    let auth_headers = retain_effective_credentials(
        credentials,
        vec![(
            "Authorization",
            format!(
                "Bearer {}",
                credentials.api_key.as_deref().unwrap_or_default()
            ),
        )],
    );

    // The endpoint derives from the resource's base URL, so the connect is pinned to the
    // address the SSRF check validated.
    let client = pinned_ai_client_for(&credentials.base_url).await?;
    let mut request = client.post(&endpoint).timeout(timeout);
    for (name, value) in auth_headers {
        request = request.header(name, value);
    }
    for (name, value) in common_outbound_headers(credentials) {
        request = request.header(name, value);
    }

    let response = request
        .json(&SystemOneRequest { model, state, questions })
        .send()
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to call TypeSafe: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        return Err(Error::ExecutionErr(format!(
            "TypeSafe error calling {endpoint}: {status} - {body}"
        )));
    }

    let body = response
        .text()
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to read the TypeSafe response: {e}")))?;
    let parsed = serde_json::from_str::<SystemOneResponse>(&body)
        .map_err(|e| Error::ExecutionErr(format!("Unexpected TypeSafe response ({e}): {body}")))?;

    Ok(DecisionResult {
        output: parsed.answers,
        model: parsed.model,
        // Dropped when empty, as an agent's is, so `usage` is either counted or absent.
        usage: parsed
            .usage
            .map(|u| TokenUsage::from_input_output(u.input_tokens, u.output_tokens))
            .filter(|u| !u.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Refused are the values a flow produces when it forgot the input (no key, `null`, a blank
    /// string from an empty expression) and the scalars TypeSafe rejects. A text, an object or an
    /// array is a state, whatever it holds.
    #[test]
    fn decision_inputs_refuse_what_typesafe_would() {
        let questions = json!({"urgent": {"type": "noul", "instructions": "Is it urgent?"}});
        for state in [None, Some(json!(null)), Some(json!("  ")), Some(json!(false)), Some(json!(0))] {
            let err = decision_inputs(state.as_ref(), Some(&questions)).unwrap_err();
            assert!(err.to_string().contains("'state'"), "{err}");
        }
        for state in [json!("0"), json!({}), json!([]), json!({"m": "hi"})] {
            assert!(decision_inputs(Some(&state), Some(&questions)).is_ok(), "{state}");
        }
        for questions in [None, Some(json!({})), Some(json!([])), Some(json!("urgent"))] {
            let err = decision_inputs(Some(&json!("hi")), questions.as_ref()).unwrap_err();
            assert!(err.to_string().contains("'questions'"), "{err}");
        }
    }
}
