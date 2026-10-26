use std::collections::HashMap;

/// Recursively merges `patch` into `target`.
///
/// Follows RFC 7396 JSON Merge Patch semantics for deletion:
/// - If a key in `patch` has a value of `null`, the key is removed from `target`.
/// - If both `patch[k]` and `target[k]` are JSON Objects, they are recursively merged.
/// - Otherwise, `target[k]` is replaced by `patch[k]`.
pub fn deep_merge_and_delete(target: &mut serde_json::Value, patch: &serde_json::Value) {
    if !patch.is_object() {
        *target = patch.clone();
        return;
    }

    if !target.is_object() {
        *target = serde_json::Value::Object(serde_json::Map::new());
    }

    let target_obj = match target.as_object_mut() {
        Some(obj) => obj,
        None => return,
    };

    if let Some(patch_obj) = patch.as_object() {
        for (key, value) in patch_obj {
            if value.is_null() {
                target_obj.remove(key);
            } else if value.is_object() && target_obj.get(key).is_some_and(|v| v.is_object()) {
                if let Some(child_target) = target_obj.get_mut(key) {
                    deep_merge_and_delete(child_target, value);
                }
            } else {
                target_obj.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Applies `extra_body` and `reasoning_effort_mapping` overrides to an outgoing JSON request body.
///
/// 1. Merges `extra_body` fields into `body` (supporting removal via `null`).
///    Embedded `"reasoning_effort_mapping"` or `"reasoning_mapping"` in `extra_body` is extracted
///    and not emitted as a literal wire field.
/// 2. If a reasoning mapping matches the current `reasoning_effort`, default OpenAI reasoning
///    fields (`reasoning` and `reasoning_effort`) are stripped unless explicitly defined in the
///    matched mapping, and the mapped fields are merged.
pub fn apply_body_overrides(
    body: &mut serde_json::Value,
    extra_body: Option<&serde_json::Value>,
    reasoning_effort: Option<&str>,
    reasoning_effort_mapping: Option<&HashMap<String, serde_json::Value>>,
) {
    if extra_body.is_none() && reasoning_effort_mapping.is_none() {
        return;
    }

    if !body.is_object() {
        *body = serde_json::Value::Object(serde_json::Map::new());
    }

    // Check if extra_body embeds a reasoning mapping
    let mut embedded_mapping: Option<HashMap<String, serde_json::Value>> = None;
    if let Some(serde_json::Value::Object(map)) = extra_body {
        let mapping_val = map
            .get("reasoning_effort_mapping")
            .or_else(|| map.get("reasoning_mapping"));
        if let Some(serde_json::Value::Object(m)) = mapping_val {
            embedded_mapping = Some(m.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        }
    }

    // 1. Apply extra_body fields (excluding the embedded mapping key)
    if let Some(serde_json::Value::Object(extra_map)) = extra_body {
        for (key, value) in extra_map {
            if key == "reasoning_effort_mapping" || key == "reasoning_mapping" {
                continue;
            }
            if value.is_null() {
                if let Some(obj) = body.as_object_mut() {
                    obj.remove(key);
                }
            } else if value.is_object()
                && body
                    .as_object()
                    .and_then(|o| o.get(key))
                    .is_some_and(|v| v.is_object())
            {
                if let Some(child_target) = body.as_object_mut().and_then(|o| o.get_mut(key)) {
                    deep_merge_and_delete(child_target, value);
                }
            } else if let Some(obj) = body.as_object_mut() {
                obj.insert(key.clone(), value.clone());
            }
        }
    }

    // 2. Resolve effective reasoning mapping
    let effective_mapping = reasoning_effort_mapping.or(embedded_mapping.as_ref());
    if let Some(mapping) = effective_mapping {
        let effort_str = reasoning_effort.map(str::trim).filter(|s| !s.is_empty());

        let matched_override = match effort_str {
            Some(effort) => {
                // Find exact or case-insensitive match
                mapping
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(effort))
                    .map(|(_, v)| v)
                    .or_else(|| {
                        if effort.eq_ignore_ascii_case("none") || effort.eq_ignore_ascii_case("off")
                        {
                            mapping
                                .iter()
                                .find(|(k, _)| {
                                    k.eq_ignore_ascii_case("none") || k.eq_ignore_ascii_case("off")
                                })
                                .map(|(_, v)| v)
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        mapping
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("default") || k.as_str() == "*")
                            .map(|(_, v)| v)
                    })
            }
            None => mapping
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("default") || k.as_str() == "*")
                .map(|(_, v)| v),
        };

        if let Some(override_val) = matched_override {
            // If the matched override doesn't explicitly specify "reasoning" or "reasoning_effort",
            // strip the default OpenAI reasoning fields to avoid upstream gateway errors.
            if let Some(obj) = body.as_object_mut() {
                if !override_val
                    .as_object()
                    .is_some_and(|o| o.contains_key("reasoning"))
                {
                    obj.remove("reasoning");
                }
                if !override_val
                    .as_object()
                    .is_some_and(|o| o.contains_key("reasoning_effort"))
                {
                    obj.remove("reasoning_effort");
                }
            }

            // Merge the matched override
            deep_merge_and_delete(body, override_val);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn adds_extra_body_fields_and_deep_merges() {
        let mut body = json!({
            "model": "qwen3.8-flash-next",
            "messages": [{"role": "user", "content": "hi"}],
            "chat_template_kwargs": {
                "system_prefix": "abc"
            }
        });

        let extra = json!({
            "chat_template_kwargs": {
                "enable_thinking": true
            },
            "custom_param": 123
        });

        apply_body_overrides(&mut body, Some(&extra), None, None);

        assert_eq!(
            body,
            json!({
                "model": "qwen3.8-flash-next",
                "messages": [{"role": "user", "content": "hi"}],
                "chat_template_kwargs": {
                    "system_prefix": "abc",
                    "enable_thinking": true
                },
                "custom_param": 123
            })
        );
    }

    #[test]
    fn removes_fields_set_to_null() {
        let mut body = json!({
            "model": "qwen3.8-flash-next",
            "reasoning": {"effort": "medium"},
            "reasoning_effort": "medium",
            "temperature": 0.7
        });

        let extra = json!({
            "reasoning": null,
            "reasoning_effort": null,
            "temperature": 0.2
        });

        apply_body_overrides(&mut body, Some(&extra), None, None);

        assert_eq!(
            body,
            json!({
                "model": "qwen3.8-flash-next",
                "temperature": 0.2
            })
        );
    }

    #[test]
    fn applies_reasoning_effort_mapping_and_strips_defaults() {
        let mut body = json!({
            "model": "qwen3.8-flash-next",
            "messages": [],
            "reasoning": {"effort": "medium"},
            "reasoning_effort": "medium"
        });

        let mapping: HashMap<String, serde_json::Value> = [
            (
                "none".to_string(),
                json!({
                    "chat_template_kwargs": { "enable_thinking": false }
                }),
            ),
            (
                "medium".to_string(),
                json!({
                    "chat_template_kwargs": { "enable_thinking": true, "reasoning_effort": "medium" }
                }),
            ),
        ]
        .into_iter()
        .collect();

        apply_body_overrides(&mut body, None, Some("medium"), Some(&mapping));

        // Note: reasoning and reasoning_effort should be automatically removed when mapping matches
        assert_eq!(
            body,
            json!({
                "model": "qwen3.8-flash-next",
                "messages": [],
                "chat_template_kwargs": {
                    "enable_thinking": true,
                    "reasoning_effort": "medium"
                }
            })
        );
    }

    #[test]
    fn applies_embedded_reasoning_effort_mapping_inside_extra_body() {
        let mut body = json!({
            "model": "qwen3.8-flash-next",
            "messages": [],
            "reasoning_effort": "high"
        });

        let extra = json!({
            "chat_template_kwargs": { "extra_flag": true },
            "reasoning_effort_mapping": {
                "none": {
                    "chat_template_kwargs": { "enable_thinking": false }
                },
                "high": {
                    "chat_template_kwargs": { "enable_thinking": true, "reasoning_effort": "high" }
                }
            }
        });

        apply_body_overrides(&mut body, Some(&extra), Some("high"), None);

        // embedded reasoning_effort_mapping is NOT emitted as a wire field
        assert_eq!(
            body,
            json!({
                "model": "qwen3.8-flash-next",
                "messages": [],
                "chat_template_kwargs": {
                    "extra_flag": true,
                    "enable_thinking": true,
                    "reasoning_effort": "high"
                }
            })
        );
    }

    #[test]
    fn reasoning_mapping_off_none_lookup() {
        let mut body = json!({
            "model": "qwen",
            "reasoning_effort": "none"
        });

        let mapping: HashMap<String, serde_json::Value> = [(
            "off".to_string(),
            json!({
                "chat_template_kwargs": { "enable_thinking": false }
            }),
        )]
        .into_iter()
        .collect();

        apply_body_overrides(&mut body, None, Some("none"), Some(&mapping));

        assert_eq!(
            body,
            json!({
                "model": "qwen",
                "chat_template_kwargs": {
                    "enable_thinking": false
                }
            })
        );
    }

    #[test]
    fn no_op_when_no_overrides() {
        let mut body = json!({
            "model": "gpt-4o",
            "messages": []
        });

        let orig = body.clone();
        apply_body_overrides(&mut body, None, Some("medium"), None);
        assert_eq!(body, orig);
    }
}
