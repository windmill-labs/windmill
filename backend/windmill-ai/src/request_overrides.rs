use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Map, Value};
use windmill_common::error::{Error, Result};

/// Changes a Custom AI resource makes to the chat completions bodies Windmill builds, keyed by
/// model id, with `*` covering every model without an entry of its own. Each patch is a JSON
/// Merge Patch (RFC 7396): a `null` removes the field.
pub type RequestOverrides = HashMap<String, ModelRequestOverride>;

#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ModelRequestOverride {
    /// Merged into every request for the model.
    #[serde(default)]
    pub body: Option<Value>,
    /// Patch per reasoning effort level. When set, it replaces the `reasoning_effort` Windmill
    /// would send: a level without an entry sends nothing about reasoning.
    #[serde(default)]
    pub reasoning: Option<HashMap<String, Value>>,
}

const ALL_MODELS: &str = "*";

/// Fields the agent loop and the chat read back from the response, or that carry the
/// conversation itself: a patch changing them would break the run rather than tune it.
const RESERVED_FIELDS: [&str; 4] = ["model", "messages", "stream", "tools"];

/// Reads a resource's `request_overrides`. Resources hold it as raw JSON so a malformed value
/// fails here with its own message, rather than failing the whole resource's deserialization.
pub fn parse_request_overrides(value: Option<&Value>) -> Result<RequestOverrides> {
    match value {
        None | Some(Value::Null) => Ok(RequestOverrides::new()),
        Some(value) => RequestOverrides::deserialize(value).map_err(|e| {
            Error::BadRequest(format!("Invalid request_overrides in the AI resource: {e}"))
        }),
    }
}

/// Applies the overrides for the body's model to a chat completions body.
pub fn apply_request_overrides(body: &mut Value, overrides: &RequestOverrides) -> Result<()> {
    let Some(model) = body.get("model").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(entry) = overrides.get(model).or_else(|| overrides.get(ALL_MODELS)) else {
        return Ok(());
    };
    let model = model.to_string();
    let Some(fields) = body.as_object_mut() else {
        return Ok(());
    };

    let level_patch = match &entry.reasoning {
        Some(levels) => fields.remove("reasoning_effort").and_then(|effort| {
            effort
                .as_str()
                .and_then(|effort| levels.get(effort))
                .cloned()
        }),
        None => None,
    };

    for patch in entry.body.iter().chain(level_patch.iter()) {
        let Some(patch) = patch.as_object() else {
            return Err(Error::BadRequest(format!(
                "The request overrides for model {model} must be JSON objects"
            )));
        };
        if let Some(field) = RESERVED_FIELDS.iter().find(|f| patch.contains_key(**f)) {
            return Err(Error::BadRequest(format!(
                "The request overrides for model {model} cannot change `{field}`"
            )));
        }
        merge_patch(fields, patch);
    }
    Ok(())
}

/// Applies the overrides to a serialized body, returning it unchanged when there are none.
pub fn apply_request_overrides_to_str(
    body: String,
    overrides: &RequestOverrides,
) -> Result<String> {
    if overrides.is_empty() {
        return Ok(body);
    }
    let mut value: Value = serde_json::from_str(&body)
        .map_err(|e| Error::internal_err(format!("Failed to parse request body: {e}")))?;
    apply_request_overrides(&mut value, overrides)?;
    serde_json::to_string(&value)
        .map_err(|e| Error::internal_err(format!("Failed to serialize request body: {e}")))
}

fn merge_patch(target: &mut Map<String, Value>, patch: &Map<String, Value>) {
    for (key, value) in patch {
        match value {
            Value::Null => {
                target.remove(key);
            }
            Value::Object(patch) => {
                let entry = target.entry(key.clone()).or_insert(Value::Null);
                if !entry.is_object() {
                    *entry = Value::Object(Map::new());
                }
                if let Value::Object(target) = entry {
                    merge_patch(target, patch);
                }
            }
            value => {
                target.insert(key.clone(), value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn overrides(value: Value) -> RequestOverrides {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn a_reasoning_level_replaces_the_effort_windmill_sends() {
        let overrides = overrides(json!({
            "qwen": {
                "body": { "chat_template_kwargs": { "enable_thinking": true }, "top_k": 20 },
                "reasoning": {
                    "none": { "chat_template_kwargs": { "enable_thinking": false } },
                    "high": { "chat_template_kwargs": { "reasoning_effort": "high" } }
                }
            }
        }));
        let request = |effort: Option<&str>| {
            let mut body =
                json!({ "model": "qwen", "messages": [], "stream": true, "temperature": 0.2 });
            if let Some(effort) = effort {
                body["reasoning_effort"] = json!(effort);
            }
            apply_request_overrides(&mut body, &overrides).unwrap();
            body
        };

        assert_eq!(
            request(Some("none")),
            json!({ "model": "qwen", "messages": [], "stream": true, "temperature": 0.2, "top_k": 20,
                    "chat_template_kwargs": { "enable_thinking": false } })
        );
        assert_eq!(
            request(Some("high"))["chat_template_kwargs"],
            json!({ "enable_thinking": true, "reasoning_effort": "high" })
        );
        // A level the model has no entry for sends nothing about reasoning.
        let low = request(Some("low"));
        assert!(low.get("reasoning_effort").is_none());
        assert_eq!(
            low["chat_template_kwargs"],
            json!({ "enable_thinking": true })
        );
    }

    #[test]
    fn null_removes_a_field_and_the_wildcard_covers_unlisted_models() {
        let overrides = overrides(json!({
            "*": { "body": { "reasoning_effort": null, "temperature": null } },
            "other": { "body": { "seed": 1 } }
        }));
        let mut body = json!({ "model": "qwen", "messages": [], "reasoning_effort": "low", "temperature": 0.2 });
        apply_request_overrides(&mut body, &overrides).unwrap();
        assert_eq!(body, json!({ "model": "qwen", "messages": [] }));

        let mut body = json!({ "model": "other", "messages": [], "reasoning_effort": "low" });
        apply_request_overrides(&mut body, &overrides).unwrap();
        assert_eq!(
            body,
            json!({ "model": "other", "messages": [], "reasoning_effort": "low", "seed": 1 })
        );
    }

    #[test]
    fn refuses_to_change_the_fields_the_run_depends_on() {
        let overrides = overrides(json!({
            "qwen": { "reasoning": { "high": { "stream": false } } }
        }));
        let mut body =
            json!({ "model": "qwen", "messages": [], "stream": true, "reasoning_effort": "high" });
        assert!(apply_request_overrides(&mut body, &overrides).is_err());
    }
}
