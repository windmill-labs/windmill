use serde_json::Value;
use windmill_common::error::{Error, Result};

pub(crate) const CODEX_RESPONSES_PATH: &str = "codex/responses";
pub(crate) const CODEX_MODELS_PATH: &str = "codex/models?client_version=1.0.0";
pub(crate) const CODEX_ORIGINATOR: &str = "codex_cli_rs";
pub(crate) const CODEX_USER_AGENT: &str = "codex_cli_rs/0.0.0 (windmill)";

#[derive(Clone)]
pub(crate) struct CodexRequestContext {
    client_request_id: String,
    window_id: String,
    installation_id: String,
}

impl CodexRequestContext {
    pub(crate) fn new() -> Self {
        Self {
            client_request_id: uuid::Uuid::new_v4().to_string(),
            window_id: uuid::Uuid::new_v4().to_string(),
            installation_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub(crate) fn headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("OpenAI-Beta", "responses=experimental".to_string()),
            ("originator", CODEX_ORIGINATOR.to_string()),
            ("User-Agent", CODEX_USER_AGENT.to_string()),
            ("x-client-request-id", self.client_request_id.clone()),
            ("x-codex-window-id", self.window_id.clone()),
            ("x-codex-installation-id", self.installation_id.clone()),
        ]
    }

    fn client_metadata(&self) -> Value {
        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "x-codex-installation-id".to_string(),
            Value::String(self.installation_id.clone()),
        );
        metadata.insert(
            "x-codex-window-id".to_string(),
            Value::String(self.window_id.clone()),
        );
        Value::Object(metadata)
    }
}

pub(crate) fn add_codex_responses_defaults(
    mut value: Value,
    context: &CodexRequestContext,
    stream: Option<bool>,
) -> Result<Value> {
    let Value::Object(ref mut object) = value else {
        return Err(Error::BadRequest(
            "OpenAI ChatGPT Account responses body must be a JSON object".to_string(),
        ));
    };

    object.remove("max_output_tokens");
    object.entry("store").or_insert(Value::Bool(false));
    if let Some(stream) = stream {
        object.entry("stream").or_insert(Value::Bool(stream));
    }
    object
        .entry("tool_choice")
        .or_insert(Value::String("auto".to_string()));
    if object.get("tools").and_then(Value::as_array).is_some_and(|tools| !tools.is_empty()) {
        object
            .entry("parallel_tool_calls")
            .or_insert(Value::Bool(true));
    }
    object
        .entry("include")
        .or_insert(Value::Array(vec![Value::String(
            "reasoning.encrypted_content".to_string(),
        )]));
    object
        .entry("client_metadata")
        .or_insert_with(|| context.client_metadata());

    Ok(value)
}
