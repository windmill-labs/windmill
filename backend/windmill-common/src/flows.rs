/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

pub use windmill_types::flows::*;

use anyhow::Context;
use serde::Deserialize;
use serde::Serialize;
use sqlx::types::Json;
use sqlx::types::JsonRawValue;

use crate::{
    cache::{self, FlowExtras},
    db::DB,
    error::{to_anyhow, Error},
    utils::{http_get_from_hub, StripPath},
    worker::{to_raw_value, Connection},
    DEFAULT_HUB_BASE_URL, HUB_BASE_URL, PRIVATE_HUB_MIN_VERSION,
};

#[derive(Deserialize)]
pub struct HubFlow {
    pub value: FlowValue,
}

#[derive(Deserialize)]
struct HubFlowResponse {
    flow: HubFlow,
}

fn extract_hub_flow_id_from_path(path: &str) -> Result<i32, Error> {
    let hub_flow_path = path.strip_prefix("hub/flows/").ok_or_else(|| {
        Error::BadRequest(format!(
            "expected hub flow path to start with hub/flows/ (got {path})"
        ))
    })?;

    let flow_id = hub_flow_path
        .split('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| {
            Error::BadRequest(format!(
                "expected hub flow path to include a numeric id after hub/flows/ (got {path})"
            ))
        })?;

    let flow_id = flow_id.parse::<i32>().map_err(|_| {
        Error::BadRequest(format!(
            "expected hub flow path to include a numeric id after hub/flows/ (got {path})"
        ))
    })?;

    if flow_id <= 0 {
        return Err(Error::BadRequest(format!(
            "expected hub flow path to include a positive numeric id after hub/flows/ (got {path})"
        )));
    }

    Ok(flow_id)
}

pub async fn get_full_hub_flow_by_path(
    path: StripPath,
    http_client: &reqwest::Client,
    db: Option<&DB>,
) -> crate::error::Result<HubFlow> {
    let path = path.to_path();
    let flow_id = extract_hub_flow_id_from_path(&path)?;
    let hub_base_url = HUB_BASE_URL.read().await.clone();
    let hub_url = format!("{hub_base_url}/flows/{flow_id}/json");

    let response = match http_get_from_hub(http_client, &hub_url, false, None, db)
        .await?
        .error_for_status()
        .map_err(to_anyhow)
    {
        Ok(response) => response,
        Err(_) if hub_base_url != DEFAULT_HUB_BASE_URL && flow_id < PRIVATE_HUB_MIN_VERSION =>
        {
            tracing::info!("Not found on private hub, fallback to default hub for hub flow {path}");
            let fallback_url = format!("{DEFAULT_HUB_BASE_URL}/flows/{flow_id}/json");
            http_get_from_hub(http_client, &fallback_url, false, None, db)
                .await?
                .error_for_status()
                .map_err(to_anyhow)?
        }
        Err(err) => return Err(err.into()),
    };

    Ok(response
        .json::<HubFlowResponse>()
        .await
        .context(format!("Decoding hub response for flow at path {path}"))?
        .flow)
}

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

    #[test]
    fn extract_hub_flow_id_accepts_id_only_paths() {
        assert_eq!(extract_hub_flow_id_from_path("hub/flows/76").unwrap(), 76);
    }

    #[test]
    fn extract_hub_flow_id_accepts_id_and_slug_paths() {
        assert_eq!(
            extract_hub_flow_id_from_path("hub/flows/76/send-message-to-company-ai-assistant")
                .unwrap(),
            76
        );
    }

    #[test]
    fn extract_hub_flow_id_rejects_non_numeric_ids() {
        let err = extract_hub_flow_id_from_path("hub/flows/send_message").unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
    }

    #[test]
    fn extract_hub_flow_id_rejects_missing_ids() {
        let err = extract_hub_flow_id_from_path("hub/flows/").unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
    }

    #[test]
    fn extract_hub_flow_id_rejects_zero_ids() {
        let err = extract_hub_flow_id_from_path("hub/flows/0").unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
    }
}
