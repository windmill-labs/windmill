//! AI decision steps: one call to a decision endpoint, which answers typed questions (`choice`,
//! `score`, `noul`) about a state with calibrated probabilities instead of text. The questions and
//! answers are System One's (TypeSafe's Jev, Cloudflare's Jev-compatible Clef on Workers AI);
//! OpenAI's and Gemini's Decisions APIs are asked and read through a translation to that shape.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{value::RawValue, Map, Value};
use windmill_common::error::{Error, Result};

use crate::{
    ai_providers::{AIPlatform, AIProvider},
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
/// Cloudflare's larger decision model, run when the step names none.
pub const CLOUDFLARE_DEFAULT_MODEL: &str = "clef";
/// The model OpenAI's Decisions API serves, run when the step names none.
pub const OPENAI_DEFAULT_MODEL: &str = "gpt-6-luna";
/// The model Gemini's Decisions API serves, run when the step names none.
pub const GOOGLE_DEFAULT_MODEL: &str = "gemini-decision-1";

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

/// Workers AI wraps the System One response in its API envelope.
#[derive(Deserialize)]
struct CloudflareEnvelope {
    result: SystemOneResponse,
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
    let questions = match questions {
        Some(Value::Object(questions)) if !questions.is_empty() => questions,
        _ => {
            return Err(Error::BadRequest(
                "'questions' must be an object naming at least one question for an AI decision"
                    .to_string(),
            ))
        }
    };
    Ok((state, questions))
}

/// Posts a decision request with the resource's key and returns the response body.
async fn post_decision<T: Serialize>(
    credentials: &ProviderCredentials,
    endpoint: &str,
    vendor: &str,
    payload: &T,
    timeout: Duration,
) -> Result<String> {
    let api_key = credentials.api_key.as_deref().unwrap_or_default();
    let auth_header = if credentials.provider == AIProvider::GoogleAI {
        ("x-goog-api-key", api_key.to_string())
    } else {
        ("Authorization", format!("Bearer {api_key}"))
    };
    let auth_headers = retain_effective_credentials(credentials, vec![auth_header]);

    // The endpoint derives from the resource's base URL, so the connect is pinned to the
    // address the SSRF check validated.
    let client = pinned_ai_client_for(&credentials.base_url).await?;
    let mut request = client.post(endpoint).timeout(timeout);
    for (name, value) in auth_headers {
        request = request.header(name, value);
    }
    for (name, value) in common_outbound_headers(credentials) {
        request = request.header(name, value);
    }

    let response = request
        .json(payload)
        .send()
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to call {vendor}: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        return Err(Error::ExecutionErr(format!(
            "{vendor} error calling {endpoint}: {status} - {body}"
        )));
    }

    response
        .text()
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to read the {vendor} response: {e}")))
}

/// Answers a decision on the endpoint its provider serves.
pub async fn run_decision(
    credentials: &ProviderCredentials,
    model: &str,
    state: &Value,
    questions: &Map<String, Value>,
    timeout: Duration,
) -> Result<DecisionResult> {
    match credentials.provider {
        AIProvider::OpenAI => {
            run_openai_decisions(credentials, model, state, questions, timeout).await
        }
        AIProvider::GoogleAI => {
            run_gemini_decisions(credentials, model, state, questions, timeout).await
        }
        _ => run_systemone(credentials, model, state, questions, timeout).await,
    }
}

#[derive(Serialize)]
struct GeminiDecisionsRequest<'a> {
    model: &'a str,
    input: Value,
    questions: Map<String, Value>,
}

#[derive(Deserialize)]
struct GeminiDecisionsResponse {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    answers: Option<Box<RawValue>>,
    #[serde(default)]
    usage: Option<GeminiUsage>,
    #[serde(default)]
    prompt_feedback: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
struct GeminiUsage {
    total_input_tokens: Option<i32>,
    total_output_tokens: Option<i32>,
}

/// The content types a Gemini Decisions input block has.
const GEMINI_BLOCK_TYPES: [&str; 5] = ["text", "image", "audio", "video", "document"];

/// The state as a Gemini Decisions `input`: a text, or a content block or list of them (the way to
/// pass an image or a document) as it is, and any other object or array as its JSON text.
fn gemini_input(state: &Value) -> Value {
    let is_block = |block: &Value| match block.get("type").and_then(Value::as_str) {
        Some("text") => block.get("text").is_some_and(Value::is_string),
        Some(kind) if GEMINI_BLOCK_TYPES.contains(&kind) => {
            block.get("mime_type").is_some_and(Value::is_string)
                && (block.get("data").is_some_and(Value::is_string)
                    || block.get("uri").is_some_and(Value::is_string))
        }
        _ => false,
    };
    match state {
        Value::String(_) => state.clone(),
        Value::Object(_) if is_block(state) => state.clone(),
        Value::Array(blocks) if !blocks.is_empty() && blocks.iter().all(is_block) => state.clone(),
        _ => Value::String(state.to_string()),
    }
}

/// The questions as Gemini takes them. They are keyed and typed as System One's are; a `noul`'s
/// meanings go as `true_description`/`false_description`, and a structured description, option
/// or level as its JSON text, since Gemini reads only texts there.
fn gemini_questions(questions: &Map<String, Value>) -> Map<String, Value> {
    let text = |v: &Value| match v {
        Value::String(s) => Value::String(s.clone()),
        v => Value::String(v.to_string()),
    };
    questions
        .iter()
        .map(|(name, question)| {
            let mut asked = question.as_object().cloned().unwrap_or_default();
            let kind = asked.get("type").and_then(Value::as_str).map(str::to_owned);
            let criteria = match (kind.as_deref(), asked.remove("criteria")) {
                (Some("noul"), Some(Value::Object(meanings))) => Some(Value::Object(
                    [("true", "true_description"), ("false", "false_description")]
                        .into_iter()
                        .filter_map(|(key, field)| match meanings.get(key) {
                            None | Some(Value::Null) => None,
                            Some(d) => Some((field.to_string(), text(d))),
                        })
                        .collect(),
                )),
                (Some("choice"), Some(Value::Object(options))) => Some(Value::Object(
                    options
                        .into_iter()
                        .map(|(option, d)| {
                            let d = match d {
                                Value::Null => Value::Null,
                                d => text(&d),
                            };
                            (option, d)
                        })
                        .collect(),
                )),
                (Some("score"), Some(Value::Array(levels))) => {
                    Some(Value::Array(levels.iter().map(text).collect()))
                }
                (_, criteria) => criteria,
            };
            if let Some(criteria) = criteria {
                asked.insert("criteria".to_string(), criteria);
            }
            if let Some(instructions) = asked.get_mut("instructions") {
                if !instructions.is_string() {
                    *instructions = text(instructions);
                }
            }
            (name.clone(), Value::Object(asked))
        })
        .collect()
}

async fn run_gemini_decisions(
    credentials: &ProviderCredentials,
    model: &str,
    state: &Value,
    questions: &Map<String, Value>,
    timeout: Duration,
) -> Result<DecisionResult> {
    if credentials.platform == AIPlatform::GoogleVertexAi {
        return Err(Error::BadRequest(
            "Gemini's Decisions API is served by the Gemini API, not Vertex AI: use a Google AI \
             resource on the standard platform"
                .to_string(),
        ));
    }
    let model = match model.trim() {
        "" => GOOGLE_DEFAULT_MODEL,
        model => model.strip_prefix("models/").unwrap_or(model),
    };
    let endpoint = format!("{}/decisions", credentials.base_url.trim_end_matches('/'));
    let body = post_decision(
        credentials,
        &endpoint,
        "Gemini",
        &GeminiDecisionsRequest {
            model,
            input: gemini_input(state),
            questions: gemini_questions(questions),
        },
        timeout,
    )
    .await?;
    let parsed = serde_json::from_str::<GeminiDecisionsResponse>(&body)
        .map_err(|e| Error::ExecutionErr(format!("Unexpected Gemini response ({e}): {body}")))?;
    // A blocked prompt comes back with no answers; left as an empty output, a branch on it would
    // quietly take its default arm.
    if let Some(feedback) = parsed
        .prompt_feedback
        .filter(|f| f.get("block_reason").is_some_and(|r| !r.is_null()))
    {
        return Err(Error::ExecutionErr(format!(
            "Gemini blocked the decision: {}",
            Value::Object(feedback)
        )));
    }
    let output = parsed.answers.ok_or_else(|| {
        Error::ExecutionErr(format!("Unexpected Gemini response (no answers): {body}"))
    })?;

    Ok(DecisionResult {
        output,
        model: parsed.model,
        usage: parsed
            .usage
            .map(|u| TokenUsage::from_input_output(u.total_input_tokens, u.total_output_tokens))
            .filter(|u| !u.is_empty()),
    })
}

#[derive(Serialize)]
struct OpenAIDecisionsRequest<'a> {
    model: &'a str,
    input: Value,
    questions: Vec<Value>,
}

#[derive(Deserialize)]
struct OpenAIDecisionsResponse {
    #[serde(default)]
    model: Option<String>,
    answers: Vec<Map<String, Value>>,
    #[serde(default)]
    usage: Option<SystemOneUsage>,
}

/// The state as a Decisions `input`. OpenAI reads a text or a list of user messages, so a list of
/// user messages (the only way to pass an image) goes as it is and any other object or array,
/// a chat history included, as its JSON text.
fn openai_input(state: &Value) -> Value {
    match state {
        Value::String(_) => state.clone(),
        Value::Array(items) if !items.is_empty() && items.iter().all(is_openai_user_message) => {
            state.clone()
        }
        _ => Value::String(state.to_string()),
    }
}

/// Exactly a message the Decisions API reads: `role: "user"` with a text, or with parts that are
/// each an `input_text` or an `input_image`.
fn is_openai_user_message(message: &Value) -> bool {
    let is_part = |part: &Value| match part.get("type").and_then(Value::as_str) {
        Some("input_text") => part.get("text").is_some_and(Value::is_string),
        Some("input_image") => part.get("image_url").is_some_and(Value::is_string),
        _ => false,
    };
    message.get("role").and_then(Value::as_str) == Some("user")
        && match message.get("content") {
            Some(Value::String(_)) => true,
            Some(Value::Array(parts)) => !parts.is_empty() && parts.iter().all(is_part),
            _ => false,
        }
}

/// The questions as OpenAI's Decisions API takes them: a list naming each question, with a
/// `noul` asked as its `predicate` and each score level as a label.
fn openai_questions(questions: &Map<String, Value>) -> Result<Vec<Value>> {
    let text = |v: &Value| match v {
        Value::String(s) => s.clone(),
        v => v.to_string(),
    };
    // A structured description goes as its JSON text, as structured instructions do.
    let stated = |description: Option<&Value>| match description {
        None | Some(Value::Null) => None,
        Some(Value::String(d)) if d.trim().is_empty() => None,
        Some(d) => Some(text(d)),
    };
    questions
        .iter()
        .map(|(name, question)| {
            let criteria = question.get("criteria");
            let mut instructions = question.get("instructions").map(text).unwrap_or_default();
            let mut asked = Map::new();
            let kind = match question.get("type").and_then(Value::as_str) {
                Some("choice") => {
                    let choices = criteria
                        .and_then(Value::as_object)
                        .filter(|c| !c.is_empty())
                        .ok_or_else(|| {
                            Error::BadRequest(format!(
                                "The choice question '{name}' needs its options as 'criteria'"
                            ))
                        })?
                        .iter()
                        .map(|(value, d)| match stated(Some(d)) {
                            Some(d) => serde_json::json!({"value": value, "description": d}),
                            None => serde_json::json!({"value": value}),
                        })
                        .collect();
                    asked.insert("choices".to_string(), Value::Array(choices));
                    "choice"
                }
                Some("score") => {
                    let levels = criteria
                        .and_then(Value::as_array)
                        .filter(|c| !c.is_empty())
                        .ok_or_else(|| {
                            Error::BadRequest(format!(
                                "The score question '{name}' needs its levels as 'criteria'"
                            ))
                        })?
                        .iter()
                        .map(|level| serde_json::json!({"label": text(level)}))
                        .collect();
                    asked.insert("levels".to_string(), Value::Array(levels));
                    "score"
                }
                Some("noul") => {
                    for (key, meaning) in [("true", "Yes"), ("false", "No")] {
                        if let Some(d) = stated(criteria.and_then(|c| c.get(key))) {
                            instructions.push_str(&format!("\n{meaning}: {d}"));
                        }
                    }
                    "predicate"
                }
                other => {
                    return Err(Error::BadRequest(format!(
                    "The question '{name}' has type {}, which is not 'choice', 'score' or 'noul'",
                    other.map_or("none".to_string(), |t| format!("'{t}'"))
                )))
                }
            };
            asked.insert("type".to_string(), kind.into());
            asked.insert("name".to_string(), name.as_str().into());
            asked.insert("instructions".to_string(), instructions.into());
            Ok(Value::Object(asked))
        })
        .collect()
}

/// OpenAI's answers keyed by question name, as every decision's are. A predicate's probability is
/// also given as `noul`, so a flow reads a yes/no answer the same way whichever provider gave it.
/// A refused question fails the decision: left in the output it has no `choice` or `noul`, so a
/// branch on it would quietly take its "no" or default arm.
fn openai_answers(answers: Vec<Map<String, Value>>) -> Result<Map<String, Value>> {
    let mut by_name = Map::new();
    for mut answer in answers {
        let Some(Value::String(name)) = answer.remove("name") else {
            continue;
        };
        match answer.get("type").and_then(Value::as_str) {
            Some("refusal") => {
                return Err(Error::ExecutionErr(format!(
                    "OpenAI refused to answer the question '{name}': {}",
                    Value::Object(answer)
                )))
            }
            Some("predicate") => {
                if let Some(probability) = answer.get("probability").cloned() {
                    answer.insert("noul".to_string(), probability);
                }
            }
            _ => {}
        }
        by_name.insert(name, Value::Object(answer));
    }
    Ok(by_name)
}

async fn run_openai_decisions(
    credentials: &ProviderCredentials,
    model: &str,
    state: &Value,
    questions: &Map<String, Value>,
    timeout: Duration,
) -> Result<DecisionResult> {
    let model = match model.trim() {
        "" => OPENAI_DEFAULT_MODEL,
        model => model,
    };
    let endpoint = format!("{}/decisions", credentials.base_url.trim_end_matches('/'));
    let body = post_decision(
        credentials,
        &endpoint,
        "OpenAI",
        &OpenAIDecisionsRequest {
            model,
            input: openai_input(state),
            questions: openai_questions(questions)?,
        },
        timeout,
    )
    .await?;
    let parsed = serde_json::from_str::<OpenAIDecisionsResponse>(&body)
        .map_err(|e| Error::ExecutionErr(format!("Unexpected OpenAI response ({e}): {body}")))?;

    Ok(DecisionResult {
        output: serde_json::value::to_raw_value(&openai_answers(parsed.answers)?)
            .map_err(|e| Error::internal_err(format!("serializing decision answers: {e}")))?,
        model: parsed.model,
        usage: parsed
            .usage
            .map(|u| TokenUsage::from_input_output(u.input_tokens, u.output_tokens))
            .filter(|u| !u.is_empty()),
    })
}

async fn run_systemone(
    credentials: &ProviderCredentials,
    model: &str,
    state: &Value,
    questions: &Map<String, Value>,
    timeout: Duration,
) -> Result<DecisionResult> {
    let cloudflare = credentials.provider == AIProvider::Cloudflare;
    let model = match model.trim() {
        "" if cloudflare => CLOUDFLARE_DEFAULT_MODEL,
        "" => TYPESAFE_DEFAULT_MODEL,
        model => model,
    };
    let base = credentials.base_url.trim_end_matches('/');
    // Workers AI serves each model at its own URL; the model goes into the path, so it is held to
    // the characters a model id has.
    let endpoint = if cloudflare {
        if !model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(Error::BadRequest(format!(
                "Invalid Cloudflare decision model: {model}"
            )));
        }
        format!("{base}/{model}")
    } else {
        format!("{base}/systemone")
    };
    let vendor = if cloudflare { "Cloudflare" } else { "TypeSafe" };
    let body = post_decision(
        credentials,
        &endpoint,
        vendor,
        &SystemOneRequest { model, state, questions },
        timeout,
    )
    .await?;
    // Tried in turn rather than as an untagged enum, which cannot hold the raw `answers`.
    let parsed = match serde_json::from_str::<SystemOneResponse>(&body) {
        Ok(parsed) => parsed,
        Err(e) => match serde_json::from_str::<CloudflareEnvelope>(&body) {
            Ok(envelope) => envelope.result,
            Err(_) => {
                return Err(Error::ExecutionErr(format!(
                    "Unexpected {vendor} response ({e}): {body}"
                )))
            }
        },
    };

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

    /// The account id goes into the URL path, so anything that could leave its segment is refused.
    #[test]
    fn cloudflare_account_id_stays_in_its_segment() {
        use crate::ai_providers::cloudflare_workers_ai_base_url;
        assert_eq!(
            cloudflare_workers_ai_base_url(Some("abc123")).unwrap(),
            "https://api.cloudflare.com/client/v4/accounts/abc123/ai/run/@cf/cloudflare"
        );
        for id in [None, Some("a/b"), Some("../x"), Some("a?b"), Some("")] {
            assert!(cloudflare_workers_ai_base_url(id).is_err(), "{id:?}");
        }
    }

    /// A `noul` goes out as a predicate with its yes/no meanings in the instructions, and comes
    /// back readable as `noul`; choices and levels go out as lists, answers come back by name.
    #[test]
    fn openai_questions_and_answers_translate() {
        let questions = json!({
            "intent": {"type": "choice", "instructions": "Why?", "criteria": {"refund": {"when": "paid"}, "bug": ""}},
            "urgency": {"type": "score", "instructions": "How urgent?", "criteria": ["Can wait", "Now"]},
            "angry": {"type": "noul", "instructions": "Angry?", "criteria": {"true": "Insults"}}
        });
        let asked = Value::Array(openai_questions(questions.as_object().unwrap()).unwrap());
        assert_eq!(
            asked,
            json!([
                {"choices": [
                    {"value": "refund", "description": "{\"when\":\"paid\"}"},
                    {"value": "bug"}
                ], "type": "choice", "name": "intent", "instructions": "Why?"},
                {"levels": [
                    {"label": "Can wait"},
                    {"label": "Now"}
                ], "type": "score", "name": "urgency", "instructions": "How urgent?"},
                {"type": "predicate", "name": "angry", "instructions": "Angry?\nYes: Insults"}
            ])
        );
        for bad in [
            json!({"q": {"type": "choice", "instructions": "?"}}),
            json!({"q": {"type": "score", "instructions": "?", "criteria": []}}),
            json!({"q": {"type": "predicate", "instructions": "?"}}),
        ] {
            assert!(openai_questions(bad.as_object().unwrap()).is_err(), "{bad}");
        }

        let answers = serde_json::from_value(json!([
            {"type": "predicate", "name": "angry", "probability": 0.92},
            {"type": "choice", "name": "intent", "choice": "refund", "confidence": 0.9}
        ]))
        .unwrap();
        assert_eq!(
            Value::Object(openai_answers(answers).unwrap()),
            json!({
                "angry": {"type": "predicate", "probability": 0.92, "noul": 0.92},
                "intent": {"type": "choice", "choice": "refund", "confidence": 0.9}
            })
        );

        let messages = json!([{"role": "user", "content": [{"type": "input_text", "text": "hi"}]}]);
        assert_eq!(openai_input(&messages), messages);
        assert_eq!(openai_input(&json!({"m": "hi"})), json!("{\"m\":\"hi\"}"));
        assert_eq!(
            openai_input(&json!([{"role": "admin"}])),
            json!("[{\"role\":\"admin\"}]")
        );
        for records in [
            json!([{"role": "user", "content": {"ticket": "charged twice"}}]),
            json!([{"role": "user", "content": null}]),
            json!([{"role": "user", "content": ["a"]}]),
            json!([{"role": "user", "content": [{"type": "input_text", "text": {"t": 1}}]}]),
            json!([{"role": "user", "content": [{"type": "input_image"}]}]),
        ] {
            assert_eq!(openai_input(&records), json!(records.to_string()));
        }
        assert_eq!(
            openai_input(&json!([{"role": "user", "content": "Hi"}])),
            json!([{"role": "user", "content": "Hi"}])
        );
        // A chat history is evaluated as text: OpenAI takes user messages only.
        let history =
            json!([{"role": "user", "content": "Hi"}, {"role": "assistant", "content": "Yo"}]);
        assert_eq!(openai_input(&history), json!(history.to_string()));
        let refused =
            serde_json::from_value(json!([{"type": "refusal", "name": "angry"}])).unwrap();
        assert!(openai_answers(refused).is_err());
    }

    /// A `noul`'s meanings go out under Gemini's field names and structured texts as JSON text;
    /// content blocks pass through as the input, anything else structured goes as its JSON text.
    #[test]
    fn gemini_questions_and_input_translate() {
        let questions = json!({
            "intent": {"type": "choice", "instructions": "Why?", "criteria": {"refund": {"when": "paid"}, "bug": null}},
            "urgency": {"type": "score", "instructions": "How urgent?", "criteria": ["Can wait", 2]},
            "angry": {"type": "noul", "instructions": {"t": 1}, "criteria": {"true": "Insults", "false": null}}
        });
        assert_eq!(
            Value::Object(gemini_questions(questions.as_object().unwrap())),
            json!({
                "intent": {"type": "choice", "instructions": "Why?", "criteria": {"refund": "{\"when\":\"paid\"}", "bug": null}},
                "urgency": {"type": "score", "instructions": "How urgent?", "criteria": ["Can wait", "2"]},
                "angry": {"type": "noul", "instructions": "{\"t\":1}", "criteria": {"true_description": "Insults"}}
            })
        );

        let image = json!({"type": "image", "mime_type": "image/png", "data": "iVBOR"});
        let blocks = json!([{"type": "text", "text": "hi"}, image]);
        assert_eq!(gemini_input(&json!("hi")), json!("hi"));
        assert_eq!(gemini_input(&image), image);
        assert_eq!(gemini_input(&blocks), blocks);
        for other in [
            json!({"m": "hi"}),
            json!([]),
            json!([{"type": "text", "text": "hi"}, "plain"]),
            json!([{"type": "image", "data": "iVBOR"}]),
            json!([{"type": "input_text", "text": "hi"}]),
        ] {
            assert_eq!(gemini_input(&other), json!(other.to_string()), "{other}");
        }
    }

    /// Refused are the values a flow produces when it forgot the input (no key, `null`, a blank
    /// string from an empty expression) and the scalars TypeSafe rejects. A text, an object or an
    /// array is a state, whatever it holds.
    #[test]
    fn decision_inputs_refuse_what_typesafe_would() {
        let questions = json!({"urgent": {"type": "noul", "instructions": "Is it urgent?"}});
        for state in [
            None,
            Some(json!(null)),
            Some(json!("  ")),
            Some(json!(false)),
            Some(json!(0)),
        ] {
            let err = decision_inputs(state.as_ref(), Some(&questions)).unwrap_err();
            assert!(err.to_string().contains("'state'"), "{err}");
        }
        for state in [json!("0"), json!({}), json!([]), json!({"m": "hi"})] {
            assert!(
                decision_inputs(Some(&state), Some(&questions)).is_ok(),
                "{state}"
            );
        }
        for questions in [
            None,
            Some(json!({})),
            Some(json!([])),
            Some(json!("urgent")),
        ] {
            let err = decision_inputs(Some(&json!("hi")), questions.as_ref()).unwrap_err();
            assert!(err.to_string().contains("'questions'"), "{err}");
        }
    }
}
