/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

pub use windmill_types::flows::*;

use serde::Serialize;
use sqlx::types::Json;
use sqlx::types::JsonRawValue;

use crate::{
    cache::{self, FlowExtras},
    db::DB,
    error::Error,
    worker::{to_raw_value, Connection},
};

/// Serialize-only wrapper that combines resolved FlowValue with display-only extras.
/// flatten + RawValue is fine for serialization (only deserialization breaks).
#[derive(Serialize)]
struct FlowValueWithExtras<'a> {
    #[serde(flatten)]
    flow: &'a FlowValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<&'a Box<JsonRawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    groups: Option<&'a Box<JsonRawValue>>,
}

/// Resolve the value of a flow if any.
pub async fn resolve_maybe_value<T>(
    e: &sqlx::PgPool,
    workspace_id: &str,
    with_code: bool,
    maybe: Option<T>,
    value_mut: impl FnOnce(&mut T) -> Option<&mut Json<Box<JsonRawValue>>>,
) -> Result<Option<T>, Error> {
    let Some(mut container) = maybe else {
        return Ok(None);
    };
    let Some(value) = value_mut(&mut container) else {
        return Ok(Some(container));
    };
    resolve_value_for_api(e, workspace_id, &mut value.0, with_code).await?;
    Ok(Some(container))
}

/// Resolve modules recursively.
/// Stashes display-only fields (notes, groups) before the FlowValue round-trip
/// and re-injects them after, since FlowValue doesn't carry them.
async fn resolve_value_for_api(
    e: &sqlx::PgPool,
    workspace_id: &str,
    value: &mut Box<JsonRawValue>,
    with_code: bool,
) -> Result<(), Error> {
    let extras = serde_json::from_str::<FlowExtras>(value.get())
        .map_err(|e| tracing::warn!("Failed to parse flow extras: {e}"))
        .ok();

    let mut val = serde_json::from_str::<FlowValue>(value.get()).map_err(|err| {
        Error::internal_err(format!("resolve: Failed to parse flow value: {}", err))
    })?;
    for module in &mut val.modules {
        resolve_module(e, workspace_id, &mut module.value, with_code).await?;
    }

    let extras = extras.unwrap_or(FlowExtras { notes: None, groups: None });
    *value = to_raw_value(&FlowValueWithExtras {
        flow: &val,
        notes: extras.notes.as_ref(),
        groups: extras.groups.as_ref(),
    });
    Ok(())
}

/// Resolve module value recursively.
pub async fn resolve_module(
    db: &DB,
    workspace_id: &str,
    value: &mut Box<JsonRawValue>,
    with_code: bool,
) -> Result<(), Error> {
    use FlowModuleValue::*;

    let mut val = serde_json::from_str::<FlowModuleValue>(value.get()).map_err(|err| {
        Error::internal_err(format!(
            "resolve: Failed to parse flow module value: {}",
            err
        ))
    })?;
    match &mut val {
        FlowScript { .. } => {
            // In order to avoid an unnecessary `.clone()` of `val`, take ownership of it's content
            // using `std::mem::replace`.
            let FlowScript {
                input_transforms,
                id,
                tag,
                language,
                is_trigger,
                assets,
                concurrency_settings,
            } = std::mem::replace(&mut val, Identity)
            else {
                unreachable!()
            };
            // Load script lock file and code content.
            let (lock, content) = if !with_code {
                (Some("...".to_string()), "...".to_string())
            } else {
                cache::flow::fetch_script(&Connection::Sql(db.clone()), id)
                    .await
                    .map(|data| (data.lock.clone(), data.code.clone()))?
            };
            val = RawScript {
                input_transforms,
                content,
                lock,
                path: None,
                tag,
                language,
                is_trigger,
                assets,
                concurrency_settings,
            };
        }
        ForloopFlow { modules, modules_node, .. } | WhileloopFlow { modules, modules_node, .. } => {
            resolve_modules(db, workspace_id, modules, modules_node.take(), with_code).await?;
        }
        BranchOne { branches, default, default_node } => {
            resolve_modules(db, workspace_id, default, default_node.take(), with_code).await?;
            for branch in branches {
                resolve_modules(
                    db,
                    workspace_id,
                    &mut branch.modules,
                    branch.modules_node.take(),
                    with_code,
                )
                .await?;
            }
        }
        BranchAll { branches, .. } => {
            for branch in branches {
                resolve_modules(
                    db,
                    workspace_id,
                    &mut branch.modules,
                    branch.modules_node.take(),
                    with_code,
                )
                .await?;
            }
        }
        _ => {}
    }
    *value = to_raw_value(&val);
    Ok(())
}

pub async fn resolve_modules(
    e: &sqlx::PgPool,
    workspace_id: &str,
    modules: &mut Vec<FlowModule>,
    modules_node: Option<FlowNodeId>,
    with_code: bool,
) -> Result<(), Error> {
    // Replace the `modules_node` with the actual modules.
    if let Some(id) = modules_node {
        *modules = cache::flow::fetch_flow(e, id)
            .await
            .map(|data| data.value().modules.clone())?;
    }
    for module in modules.iter_mut() {
        Box::pin(resolve_module(
            e,
            workspace_id,
            &mut module.value,
            with_code,
        ))
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flow_value_with_extras_serializes_notes_and_groups() {
        let input = json!({
            "modules": [],
            "notes": [{"id": "n1", "text": "hello", "color": "yellow", "type": "free"}],
            "groups": [{"start_id": "a", "end_id": "b", "summary": "grp"}]
        });
        let input_str = serde_json::to_string(&input).unwrap();

        // Parse FlowValue (drops notes/groups) and FlowExtras (captures them)
        let val: FlowValue = serde_json::from_str(&input_str).unwrap();
        let extras: FlowExtras = serde_json::from_str(&input_str).unwrap();

        // Serialize via FlowValueWithExtras — should include both
        let combined = FlowValueWithExtras {
            flow: &val,
            notes: extras.notes.as_ref(),
            groups: extras.groups.as_ref(),
        };
        let output: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&combined).unwrap()).unwrap();

        assert_eq!(output["notes"], input["notes"]);
        assert_eq!(output["groups"], input["groups"]);
        assert!(output["modules"].is_array());
    }

    #[test]
    fn flow_value_with_extras_omits_none_extras() {
        let val: FlowValue = serde_json::from_str(r#"{"modules":[]}"#).unwrap();
        let combined = FlowValueWithExtras { flow: &val, notes: None, groups: None };
        let output = serde_json::to_string(&combined).unwrap();
        assert!(!output.contains("notes"));
        assert!(!output.contains("groups"));
    }
}
