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
    db::DB,
    error::{to_anyhow, Error},
    scripts::ScriptHash,
    utils::{http_get_from_hub, parse_hub_id, StripPath},
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

pub async fn get_full_hub_flow_by_path(
    path: StripPath,
    http_client: &reqwest::Client,
    db: Option<&DB>,
) -> crate::error::Result<HubFlow> {
    let path = path.to_path();
    let flow_id = parse_hub_id(&path, "hub/flows/")?;
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

/// Checks a flow value contains nothing but composition of runnables that already exist, which is
/// all an operator with builder rights may author. Walks the modules, the preprocessor and failure
/// modules, every branch, and the `tools` of an AI agent step.
///
/// Returns what the caller still has to authorize against its own permissions, which this
/// value-only walk cannot: every runnable and linked agent the steps reference, the worker tags
/// they pin, and the `(path, hash)` pairs of version-pinned steps. See [`ComposedFlowRefs`] for why each one is not
/// already settled by the walk.
pub fn check_flow_is_composition_only(value: &FlowValue) -> Result<ComposedFlowRefs, Error> {
    let mut refs = ComposedFlowRefs::default();
    for module in value
        .modules
        .iter()
        .chain(value.preprocessor_module.as_deref())
        .chain(value.failure_module.as_deref())
    {
        check_module_is_composition_only(module, &mut refs)?;
    }
    Ok(refs)
}

/// What [`check_flow_is_composition_only`] collects for the caller to authorize.
#[derive(Default)]
pub struct ComposedFlowRefs {
    pub tags: Vec<String>,
    /// Every workspace runnable a step references, as `(is_flow, path)`.
    pub runnables: Vec<(bool, String)>,
    /// Version-pinned script steps, as `(path, hash)`.
    pub pinned_scripts: Vec<(String, ScriptHash)>,
    /// The `ai_agent` resources that agent steps link.
    pub agents: Vec<LinkedAgent>,
}

/// An agent step's link, with the tool inputs the step binds, keyed by tool id.
pub struct LinkedAgent {
    pub path: String,
    pub bound_inputs: Vec<(String, Vec<String>)>,
}

/// A builder's step may bind only the tool inputs its linked agent leaves open, passing them what
/// it could pass a deployed runnable's inputs. An input the agent fills is part of what its author
/// wrote, and the step's binding would replace it at run time. Every tool sharing an id is
/// checked, as the overlay binds them all.
pub fn check_bound_agent_inputs_are_open(
    agent: &str,
    tools: &[AgentTool],
    bound_inputs: &[(String, Vec<String>)],
) -> Result<(), Error> {
    for (tool_id, keys) in bound_inputs {
        for tool in tools.iter().filter(|t| &t.id == tool_id) {
            check_tool_inputs_are_open(agent, tool_id, &tool.value, keys)?;
        }
    }
    Ok(())
}

fn check_tool_inputs_are_open(
    agent: &str,
    tool_id: &str,
    value: &ToolValue,
    keys: &[String],
) -> Result<(), Error> {
    let ToolValue::FlowModule(value) = value else {
        return Ok(());
    };
    let input_transforms = match value {
        FlowModuleValue::Script { input_transforms, .. }
        | FlowModuleValue::RawScript { input_transforms, .. }
        | FlowModuleValue::FlowScript { input_transforms, .. }
        | FlowModuleValue::AIAgent { input_transforms, .. }
        | FlowModuleValue::AIDecision { input_transforms, .. } => input_transforms,
        // Fail closed on any other kind, as the worker's `overlay_tool_inputs` skips them.
        _ => {
            return Err(Error::PermissionDenied(format!(
                "Tool {tool_id} of agent {agent} takes no inputs a builder can set"
            )))
        }
    };
    match keys
        .iter()
        .find(|k| input_transforms.get(*k).is_some_and(|t| t.is_completed()))
    {
        Some(key) => Err(Error::PermissionDenied(format!(
            "Input {key} of tool {tool_id} is set by agent {agent}, so a builder cannot \
             change it. Bind only the inputs the agent leaves to the model."
        ))),
        None => Ok(()),
    }
}

fn check_module_is_composition_only(
    module: &FlowModule,
    refs: &mut ComposedFlowRefs,
) -> Result<(), Error> {
    let value = module
        .get_value()
        .map_err(|e| Error::BadRequest(format!("Step {} could not be read: {e}", module.id)))?;
    check_module_value_is_composition_only(&value, &module.id, refs)
}

fn check_module_value_is_composition_only(
    value: &FlowModuleValue,
    id: &str,
    refs: &mut ComposedFlowRefs,
) -> Result<(), Error> {
    let refuse = |what: &str| Err(refused(id, what));
    // A node id points at code in a `flow_node` row, which only the dependency job produces by
    // hoisting a step's code, so an authored value carrying one names another flow's code and slips
    // past this walk. Editor payloads come from the un-hoisted `flow_version.value`, so refusing
    // them costs nothing legitimate.
    let refuse_node = |node: &Option<FlowNodeId>| match node {
        Some(_) => refuse("references code stored outside the flow"),
        None => Ok(()),
    };
    let mut push_tag = |tag: &Option<String>| {
        if let Some(tag) = tag.as_deref().filter(|t| !t.is_empty()) {
            refs.tags.push(tag.to_string());
        }
    };

    match value {
        FlowModuleValue::RawScript { .. } => return refuse("has inline code"),
        FlowModuleValue::FlowScript { .. } => {
            return refuse("references code stored outside the flow")
        }
        FlowModuleValue::Identity => {}
        FlowModuleValue::Script { path, hash, tag_override, .. } => {
            check_composable_path(path, id)?;
            push_tag(tag_override);
            refs.runnables.push((false, path.clone()));
            if let Some(hash) = hash {
                refs.pinned_scripts.push((path.clone(), *hash));
            }
        }
        FlowModuleValue::Flow { path, .. } => {
            check_composable_path(path, id)?;
            refs.runnables.push((true, path.clone()));
        }
        FlowModuleValue::ForloopFlow { modules, modules_node, .. }
        | FlowModuleValue::WhileloopFlow { modules, modules_node, .. } => {
            refuse_node(modules_node)?;
            for module in modules {
                check_module_is_composition_only(module, refs)?;
            }
        }
        FlowModuleValue::BranchOne { branches, default, default_node } => {
            refuse_node(default_node)?;
            for module in default {
                check_module_is_composition_only(module, refs)?;
            }
            check_branches_are_composition_only(branches, id, refs)?;
        }
        FlowModuleValue::BranchAll { branches, .. } => {
            check_branches_are_composition_only(branches, id, refs)?
        }
        FlowModuleValue::AIAgent { tools, tag, agent, tool_inputs, .. } => {
            // A linked agent's tools are resolved from its resource at run time, outside this
            // walk. That is safe only because an operator's every write of an agent passes this
            // same check (`validate_operator_agent`), as a developer's agent is code they wrote.
            if let Some(agent) = agent {
                let path = agent
                    .trim_start_matches("$res:")
                    .trim_start_matches("res://");
                let bound_inputs = tool_inputs
                    .iter()
                    .map(|(tool, inputs)| (tool.clone(), inputs.keys().cloned().collect()))
                    .collect();
                refs.agents
                    .push(LinkedAgent { path: path.to_string(), bound_inputs });
            }
            push_tag(tag);
            for tool in tools {
                if let ToolValue::FlowModule(value) = &tool.value {
                    check_module_value_is_composition_only(value, &tool.id, refs)?;
                }
            }
        }
        // Runs no code: it sends its inputs to the decision model.
        FlowModuleValue::AIDecision { tag, .. } => push_tag(tag),
    }
    Ok(())
}

fn check_branches_are_composition_only(
    branches: &[Branch],
    id: &str,
    refs: &mut ComposedFlowRefs,
) -> Result<(), Error> {
    for branch in branches {
        if branch.modules_node.is_some() {
            return Err(refused(
                id,
                "has a branch that references code stored outside the flow",
            ));
        }
        for module in &branch.modules {
            check_module_is_composition_only(module, refs)?;
        }
    }
    Ok(())
}

fn refused(id: &str, what: &str) -> Error {
    Error::PermissionDenied(format!(
        "Step {id} {what}, so only a developer can edit this flow. Builders can compose scripts \
         and flows that are already deployed."
    ))
}

fn check_composable_path(path: &str, id: &str) -> Result<(), Error> {
    if path.starts_with("hub/") {
        return Err(Error::PermissionDenied(format!(
            "Step {id}: hub runnables are not available to operators with builder rights. Deploy \
             it to the workspace first."
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_hub_flow_id_accepts_id_only_paths() {
        assert_eq!(parse_hub_id("hub/flows/76", "hub/flows/").unwrap(), 76);
    }

    #[test]
    fn extract_hub_flow_id_accepts_id_and_slug_paths() {
        assert_eq!(
            parse_hub_id(
                "hub/flows/76/send-message-to-company-ai-assistant",
                "hub/flows/"
            )
            .unwrap(),
            76
        );
    }

    #[test]
    fn extract_hub_flow_id_rejects_non_numeric_ids() {
        let err = parse_hub_id("hub/flows/send_message", "hub/flows/").unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
    }

    #[test]
    fn extract_hub_flow_id_rejects_missing_ids() {
        let err = parse_hub_id("hub/flows/", "hub/flows/").unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
    }

    #[test]
    fn extract_hub_flow_id_rejects_zero_ids() {
        let err = parse_hub_id("hub/flows/0", "hub/flows/").unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
    }

    fn flow(value: serde_json::Value) -> FlowValue {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn composition_check_accepts_a_composed_flow_and_collects_its_tags() {
        let refs = check_flow_is_composition_only(&flow(serde_json::json!({"modules": [{
            "id": "a",
            "value": {"type": "forloopflow", "iterator": {"type": "static", "value": []},
                "parallel": false, "modules": [
                    {"id": "b", "value": {"type": "script", "path": "f/x/s", "tag_override": "gpu"}},
                    {"id": "c", "value": {"type": "flow", "path": "f/x/f"}},
                    {"id": "d", "value": {"type": "aiagent", "input_transforms": {}, "tag": "ai",
                        "tools": [{"id": "t", "value": {"tool_type": "flowmodule",
                            "type": "script", "path": "f/x/tool"}}]}}
                ]}
        }]})))
        .unwrap();
        assert_eq!(refs.tags, vec!["gpu".to_string(), "ai".to_string()]);
    }

    /// The walk covers `modules`, so a node reference is a way past it: it names code hoisted
    /// into a `flow_node` row, possibly another flow's.
    #[test]
    fn composition_check_rejects_node_references() {
        for value in [
            serde_json::json!({"modules": [{"id": "a", "value": {"type": "forloopflow",
                "iterator": {"type": "static", "value": []}, "parallel": false,
                "modules": [], "modules_node": 7}}]}),
            serde_json::json!({"modules": [{"id": "a", "value": {"type": "branchone",
                "branches": [], "default": [], "default_node": 7}}]}),
            serde_json::json!({"modules": [{"id": "a", "value": {"type": "branchall",
                "branches": [{"expr": "true", "modules": [], "modules_node": 7}]}}]}),
            serde_json::json!({"modules": [{"id": "a", "value": {"type": "flowscript",
                "id": 7, "language": "bun"}}]}),
        ] {
            assert!(check_flow_is_composition_only(&flow(value)).is_err());
        }
    }

    #[test]
    fn composition_check_rejects_code_reachable_through_an_ai_agent() {
        let value = serde_json::json!({"modules": [{"id": "a", "value": {"type": "aiagent",
            "input_transforms": {}, "tools": [{"id": "t", "value": {"tool_type": "flowmodule",
                "type": "rawscript", "content": "x", "language": "bun"}}]}}]});
        assert!(check_flow_is_composition_only(&flow(value)).is_err());
    }

    /// A builder may link only an agent they can read, which the walk cannot know: it has to
    /// surface the path for the caller to check.
    #[test]
    fn composition_check_reports_linked_agents() {
        let refs = check_flow_is_composition_only(&flow(serde_json::json!({"modules": [{
            "id": "a", "value": {"type": "aiagent", "input_transforms": {}, "tools": [],
                "agent": "$res:f/x/agent", "tool_inputs": {"t": {}}}}]})))
        .unwrap();
        assert_eq!(refs.agents[0].path, "f/x/agent");
        assert_eq!(refs.agents[0].bound_inputs, vec![("t".to_string(), vec![])]);
    }

    #[test]
    fn bound_agent_inputs_must_be_left_to_the_model() {
        // Two tools share the id: the overlay binds both, so the second's `query` counts.
        let tools: Vec<AgentTool> = serde_json::from_value(serde_json::json!([
            {"id": "t", "value": {"tool_type": "flowmodule", "type": "rawscript", "content": "x",
                "language": "bun", "input_transforms": {}}},
            {"id": "t", "value": {"tool_type": "flowmodule", "type": "rawscript", "content": "x",
                "language": "bun", "input_transforms": {
                    "query": {"type": "static", "value": "SELECT 1"},
                    "limit": {"type": "ai"},
                    "note": {"type": "static", "value": null}}}}]))
        .unwrap();
        let bind = |keys: &[&str]| {
            check_bound_agent_inputs_are_open(
                "f/x/agent",
                &tools,
                &[(
                    "t".to_string(),
                    keys.iter().map(|k| k.to_string()).collect(),
                )],
            )
        };
        assert!(bind(&["limit", "note", "absent"]).is_ok());
        assert!(bind(&["limit", "query"]).is_err());
    }

    #[test]
    fn composition_check_rejects_code_in_every_module_slot() {
        let inline = serde_json::json!({"type": "rawscript", "content": "x", "language": "bun"});
        for value in [
            serde_json::json!({"modules": [{"id": "a", "value": inline}]}),
            serde_json::json!({"modules": [], "failure_module": {"id": "f", "value": inline}}),
            serde_json::json!({"modules": [], "preprocessor_module": {"id": "p", "value": inline}}),
        ] {
            assert!(check_flow_is_composition_only(&flow(value)).is_err());
        }
    }

    /// A step carrying a `hash` dispatches on that hash alone: the caller must verify the pair
    /// exists and is readable, so the walk has to surface it rather than pass it through.
    #[test]
    fn composition_check_reports_version_pinned_steps() {
        let refs = check_flow_is_composition_only(&flow(serde_json::json!({"modules": [
            {"id": "a", "value": {"type": "script", "path": "f/x/s", "hash": "000000000000007b"}},
            {"id": "b", "value": {"type": "script", "path": "f/x/t"}}
        ]})))
        .unwrap();
        assert_eq!(
            refs.pinned_scripts,
            vec![("f/x/s".to_string(), ScriptHash(123))]
        );
    }

    #[test]
    fn composition_check_rejects_hub_runnables() {
        for kind in ["script", "flow"] {
            let value = serde_json::json!({"modules": [{"id": "a",
                "value": {"type": kind, "path": "hub/1234/thing"}}]});
            assert!(check_flow_is_composition_only(&flow(value)).is_err());
        }
    }
}
