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
use sqlx::types::Json;
use sqlx::types::JsonRawValue;

use crate::{
    cache::{self, FlowExtras},
    db::{Authable, UserDB, DB},
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
    let hub_base_url = (**HUB_BASE_URL.load()).clone();
    let hub_url = format!("{hub_base_url}/flows/{flow_id}/json");

    let response = match http_get_from_hub(http_client, &hub_url, false, None, db)
        .await?
        .error_for_status()
        .map_err(to_anyhow)
    {
        Ok(response) => response,
        Err(_) if hub_base_url != DEFAULT_HUB_BASE_URL && flow_id < PRIVATE_HUB_MIN_VERSION => {
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
async fn resolve_value_for_api(
    e: &sqlx::PgPool,
    workspace_id: &str,
    value: &mut Box<JsonRawValue>,
    with_code: bool,
) -> Result<(), Error> {
    let extras = FlowExtras::capture(value);

    let mut val = serde_json::from_str::<FlowValue>(value.get()).map_err(|err| {
        Error::internal_err(format!("resolve: Failed to parse flow value: {}", err))
    })?;
    for module in &mut val.modules {
        resolve_module(e, workspace_id, &mut module.value, with_code).await?;
    }

    *value = extras.reattach(&val)?;
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

/// A deployed workspace runnable that a flow step points at.
///
/// Inline steps (`rawscript`, `flowscript`) and `hub/` references carry their own code or come
/// from the public hub, so neither is subject to a workspace ACL.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FlowStepRef {
    /// `script` step resolved by path at run time (latest deployed version).
    ScriptPath(String),
    /// `script` step pinned to a version.
    ScriptHash { path: String, hash: i64 },
    /// `flow` step (sub-flow).
    FlowPath(String),
}

impl std::fmt::Display for FlowStepRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowStepRef::ScriptPath(path) => write!(f, "script {path}"),
            FlowStepRef::ScriptHash { path, hash } => {
                write!(
                    f,
                    "script {path} (hash {})",
                    crate::scripts::ScriptHash(*hash)
                )
            }
            FlowStepRef::FlowPath(path) => write!(f, "flow {path}"),
        }
    }
}

fn is_workspace_owned_path(path: &str) -> bool {
    path.starts_with("u/") || path.starts_with("f/") || path.starts_with("g/")
}

/// Every deployed runnable the steps of `value` point at, deduplicated.
///
/// Descends into loops, branches and agent tools, and covers the failure and preprocessor
/// modules — the API is reachable directly, so anything that resolves a path at run time must be
/// collected here.
pub fn collect_flow_step_refs(value: &FlowValue) -> anyhow::Result<Vec<FlowStepRef>> {
    let mut refs: Vec<FlowStepRef> = Vec::new();
    let mut seen: std::collections::HashSet<FlowStepRef> = std::collections::HashSet::new();
    let mut collect = |module: &FlowModule| -> anyhow::Result<()> {
        let step_ref = match module.get_value() {
            Ok(FlowModuleValue::Script { path, hash, .. }) if is_workspace_owned_path(&path) => {
                match hash {
                    Some(hash) => FlowStepRef::ScriptHash { path, hash: hash.0 },
                    None => FlowStepRef::ScriptPath(path),
                }
            }
            Ok(FlowModuleValue::Flow { path, .. }) if is_workspace_owned_path(&path) => {
                FlowStepRef::FlowPath(path)
            }
            _ => return Ok(()),
        };
        if seen.insert(step_ref.clone()) {
            refs.push(step_ref);
        }
        Ok(())
    };

    FlowModule::traverse_modules(&value.modules, &mut collect)?;
    let extra_modules: Vec<FlowModule> = value
        .failure_module
        .iter()
        .chain(value.preprocessor_module.iter())
        .map(|m| (**m).clone())
        .collect();
    FlowModule::traverse_modules(&extra_modules, &mut collect)?;
    Ok(refs)
}

/// Reject a flow whose steps point at a runnable that exists in the workspace but is not
/// readable by `authed`.
///
/// Without this, any workspace member could save a step referencing a folder-protected script
/// and read its output — or inherit its `on_behalf_of` identity — through the flow, since step
/// references are resolved with a privileged connection at run time.
///
/// A reference to a runnable that does not exist is deliberately tolerated: flows are routinely
/// deployed before the scripts they call (CLI/git-sync push order), and run-time resolution
/// enforces the ACL again on whatever the reference ends up pointing at.
pub async fn require_readable_flow_step_refs<T: Authable + Sync>(
    authed: &T,
    user_db: &UserDB,
    db: &DB,
    w_id: &str,
    value: &FlowValue,
) -> Result<(), Error> {
    let refs = collect_flow_step_refs(value)
        .map_err(|e| Error::BadRequest(format!("Invalid flow value: {e:#}")))?;
    if refs.is_empty() {
        return Ok(());
    }

    let script_paths = refs
        .iter()
        .filter_map(|r| match r {
            FlowStepRef::ScriptPath(path) => Some(path.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let script_hashes = refs
        .iter()
        .filter_map(|r| match r {
            FlowStepRef::ScriptHash { hash, .. } => Some(*hash),
            _ => None,
        })
        .collect::<Vec<_>>();
    let flow_paths = refs
        .iter()
        .filter_map(|r| match r {
            FlowStepRef::FlowPath(path) => Some(path.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    let mut tx = user_db.clone().begin(authed).await?;
    let visible_script_paths = sqlx::query_scalar!(
        "SELECT DISTINCT path FROM script WHERE workspace_id = $1 AND path = ANY($2::text[]) AND deleted = false",
        w_id,
        &script_paths[..]
    )
    .fetch_all(&mut *tx)
    .await?;
    let visible_script_hashes = sqlx::query_scalar!(
        "SELECT hash FROM script WHERE workspace_id = $1 AND hash = ANY($2::bigint[])",
        w_id,
        &script_hashes[..]
    )
    .fetch_all(&mut *tx)
    .await?;
    let visible_flow_paths = sqlx::query_scalar!(
        "SELECT path FROM flow WHERE workspace_id = $1 AND path = ANY($2::text[])",
        w_id,
        &flow_paths[..]
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    let invisible = refs
        .into_iter()
        .filter(|r| match r {
            FlowStepRef::ScriptPath(path) => !visible_script_paths.contains(path),
            FlowStepRef::ScriptHash { hash, .. } => !visible_script_hashes.contains(hash),
            FlowStepRef::FlowPath(path) => !visible_flow_paths.contains(path),
        })
        .collect::<Vec<_>>();
    if invisible.is_empty() {
        return Ok(());
    }

    // Re-probe the invisible ones without the RLS filter to tell "does not exist" (tolerated)
    // from "exists but hidden" (the access the reference would otherwise borrow).
    let mut denied: Vec<String> = Vec::new();
    for step_ref in invisible {
        let exists = match &step_ref {
            FlowStepRef::ScriptPath(path) => sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM script WHERE workspace_id = $1 AND path = $2 AND deleted = false)",
                w_id,
                path
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false),
            FlowStepRef::ScriptHash { hash, .. } => sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM script WHERE workspace_id = $1 AND hash = $2)",
                w_id,
                hash
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false),
            FlowStepRef::FlowPath(path) => sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM flow WHERE workspace_id = $1 AND path = $2)",
                w_id,
                path
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false),
        };
        if exists {
            denied.push(step_ref.to_string());
        }
    }

    if denied.is_empty() {
        return Ok(());
    }

    Err(Error::NotAuthorized(format!(
        "You are not authorized to access the following runnable(s) referenced by this flow: {}",
        denied.join(", ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn collect_flow_step_refs_reaches_every_nested_step() {
        let value: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [
                { "id": "a", "value": { "type": "script", "path": "f/protected/direct" } },
                { "id": "b", "value": { "type": "script", "path": "f/protected/pinned", "hash": "000000000000002a" } },
                { "id": "c", "value": { "type": "script", "path": "hub/1234/some_hub_script" } },
                { "id": "d", "value": { "type": "rawscript", "content": "", "language": "bun" } },
                { "id": "e", "value": { "type": "forloopflow", "iterator": { "type": "static", "value": [] }, "parallel": false, "modules": [
                    { "id": "e1", "value": { "type": "flow", "path": "f/protected/subflow" } }
                ] } },
                { "id": "f", "value": { "type": "branchone", "default": [
                    { "id": "f1", "value": { "type": "script", "path": "u/someone/in_default" } }
                ], "branches": [ { "expr": "true", "modules": [
                    { "id": "f2", "value": { "type": "script", "path": "g/all/in_branch" } }
                ] } ] } }
            ],
            "failure_module": { "id": "failure", "value": { "type": "script", "path": "f/protected/on_failure" } },
            "preprocessor_module": { "id": "preprocessor", "value": { "type": "script", "path": "f/protected/preprocess" } }
        }))
        .unwrap();

        let mut refs = collect_flow_step_refs(&value)
            .unwrap()
            .into_iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>();
        refs.sort();
        assert_eq!(
            refs,
            vec![
                "flow f/protected/subflow",
                "script f/protected/direct",
                "script f/protected/on_failure",
                "script f/protected/pinned (hash 000000000000002a)",
                "script f/protected/preprocess",
                "script g/all/in_branch",
                "script u/someone/in_default",
            ]
        );
    }
}
