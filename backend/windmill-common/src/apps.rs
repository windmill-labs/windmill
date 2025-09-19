/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::{from_value, Value};
use tokio::sync::RwLock;

use crate::{error, scripts::ScriptLang};

lazy_static::lazy_static! {
    pub static ref APP_WORKSPACED_ROUTE: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
}

/// Id in the `app_script` table.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[serde(transparent)]
pub struct AppScriptId(pub i64);

#[derive(Deserialize)]
pub struct ListAppQuery {
    pub starred_only: Option<bool>,
    pub path_exact: Option<String>,
    pub path_start: Option<String>,
    pub include_draft_only: Option<bool>,
    pub with_deployment_msg: Option<bool>,
}

#[derive(Deserialize)]
pub struct RawAppValue {
    pub files: HashMap<String, String>,
}

pub struct AppInlineScript {
    pub language: Option<ScriptLang>,
    pub content: String,
    pub lock: Option<String>,
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
// async fn reduce_app(
//     db: &sqlx::Pool<sqlx::Postgres>,
//     path: &str,
//     value: &mut Value,
//     app: i64,
// ) -> Result<()> {
//     match value {
//         Value::Object(object) => {
//             if let Some(Value::Object(script)) = object.get_mut("inlineScript") {
//                 let language = script.get("language").cloned();
//                 if language == Some(Value::String("frontend".to_owned())) {
//                     return Ok(());
//                 }
//                 // replace `content` with an empty string:
//                 let Some(Value::String(code)) = script.get_mut("content").map(std::mem::take)
//                 else {
//                     return Err(error::Error::internal_err(
//                         "Missing `content` in inlineScript".to_string(),
//                     ));
//                 };
//                 // remove `lock`:
//                 let lock = script.remove("lock").and_then(|x| match x {
//                     Value::String(s) => Some(s),
//                     _ => None,
//                 });
//                 let id = insert_app_script(
//                     db,
//                     path,
//                     app,
//                     code,
//                     lock,
//                     language.map(|v| from_value(v).ok()).flatten(),
//                 )
//                 .await?;
//                 // insert the `id` into the `script` object:
//                 script.insert("id".to_string(), json!(id.0));
//             } else {
//                 for (_, value) in object {
//                     Box::pin(reduce_app(db, path, value, app)).await?;
//                 }
//             }
//         }
//         Value::Array(array) => {
//             for value in array {
//                 Box::pin(reduce_app(db, path, value, app)).await?;
//             }
//         }
//         _ => {}
//     }
//     Ok(())
// }
