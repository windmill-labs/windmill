/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::sync::Arc;

use serde_json::{from_value, Value};
use tokio::sync::RwLock;

use crate::{error, scripts::ScriptLang};

pub use windmill_types::apps::*;

lazy_static::lazy_static! {
    pub static ref APP_WORKSPACED_ROUTE: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
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
