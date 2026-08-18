/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::sync::atomic::AtomicBool;

use serde_json::{from_value, Value};

use crate::{error, scripts::ScriptLang};

pub use windmill_types::apps::*;

lazy_static::lazy_static! {
    pub static ref APP_WORKSPACED_ROUTE: AtomicBool = AtomicBool::new(false);
}

/// Whether the app value carries an `inlineScript` anywhere.
///
/// Deliberately not built on [`traverse_app_inline_scripts`], which only reports a script whose
/// `language` parses and stops descending as soon as it sees an `inlineScript` key. That is right
/// for locking (nothing to lock without a language) and wrong for an authorization check, where
/// the author picks the fields: this refuses the key itself, whatever it contains.
pub fn app_value_has_inline_script(value: &Value) -> bool {
    match value {
        Value::Object(object) => {
            object.contains_key("inlineScript") || object.values().any(app_value_has_inline_script)
        }
        Value::Array(array) => array.iter().any(app_value_has_inline_script),
        _ => false,
    }
}

/// Every workspace runnable the app value points a component at, as `(is_flow, path)`.
///
/// This is what the deployed bundle actually asks `execute_component` to run: it resolves a
/// `runnable_id` against the stored `runnables` and sends the referenced path. The policy's
/// triggerables are a separate surface, so both have to be authorized.
pub fn app_value_runnable_paths(value: &Value) -> Vec<(bool, String)> {
    fn walk(value: &Value, out: &mut Vec<(bool, String)>) {
        match value {
            Value::Object(object) => {
                let by_path = object
                    .get("type")
                    .and_then(Value::as_str)
                    .is_some_and(|t| t == "runnableByPath" || t == "path");
                if by_path {
                    if let Some(path) = object.get("path").and_then(Value::as_str) {
                        let is_flow =
                            object.get("runType").and_then(Value::as_str) == Some("flow");
                        out.push((is_flow, path.to_string()));
                    }
                }
                for value in object.values() {
                    walk(value, out);
                }
            }
            Value::Array(array) => array.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(value, &mut out);
    out
}

/// Traverse FlowValue while invoking provided by caller callback on leafs
// #[async_recursion::async_recursion(?Send)]
pub fn traverse_app_inline_scripts<
    C: FnMut(AppInlineScript, Option<String>) -> error::Result<()>,
>(
    value: &Value,
    // Set to None.
    container_id: Option<String>,
    cb: &mut C,
) -> error::Result<()> {
    match value {
        Value::Object(object) => {
            if let Some(Value::Object(script)) = object.get("inlineScript") {
                let (language, lock, code) = (
                    script
                        .get("language")
                        .cloned()
                        .map(|v| from_value::<ScriptLang>(v).ok())
                        .flatten(),
                    script
                        .get("lock")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    script
                        .get("content")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .ok_or(error::Error::internal_err(
                            "Missing `content` in inlineScript".to_string(),
                        ))?,
                );
                if language.is_some() {
                    cb(
                        AppInlineScript { language, content: code.to_owned(), lock },
                        container_id.clone(),
                    )?;
                }
            } else {
                for (_, value) in object {
                    traverse_app_inline_scripts(
                        value,
                        object
                            .get("id")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                            .or(container_id.clone()),
                        cb,
                    )?;
                }
            }
        }
        Value::Array(array) => {
            for value in array {
                traverse_app_inline_scripts(value, container_id.clone(), cb)?;
            }
        }
        _ => {}
    }
    Ok(())
}
