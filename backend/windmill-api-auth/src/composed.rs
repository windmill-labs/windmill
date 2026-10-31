//! Authorizing what a restricted role composes: the references a value-only walk
//! (`check_flow_is_composition_only`) collects, checked against the caller's own permissions.

use sqlx::{Postgres, Transaction};
use windmill_common::{
    error::{self, Error},
    flows::FlowValue,
    roles::AuthorLevel,
    workspaces::{check_operator_can_build, BuilderKind},
    DB,
};

use crate::{get_scope_tags, is_super_admin_authed, ApiAuthed};

/// `validate_operator_composed_flow` on a transaction the caller already holds, so a write that
/// has one open does not wait on a second connection. It must have been begun as the caller
/// (`UserDB::begin`): its row-level security is the check.
pub async fn authorize_composed_flow(
    value: &FlowValue,
    flow_tag: &Option<String>,
    authed: &ApiAuthed,
    db: &DB,
    tx: &mut Transaction<'_, Postgres>,
    w_id: &str,
) -> error::Result<()> {
    let mut refs = windmill_common::flows::check_flow_is_composition_only(value)?;

    // A builder links only an agent it can read, and binds only the tool inputs it leaves open.
    for agent in &refs.agents {
        let path = &agent.path;
        let value = sqlx::query_scalar!(
            "SELECT value AS \"value: sqlx::types::Json<serde_json::Value>\"
                 FROM resource WHERE workspace_id = $1 AND path = $2 AND resource_type = 'ai_agent'",
            w_id,
            path,
        )
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| {
            Error::PermissionDenied(format!(
                "Agent {path} does not exist or is not readable by you"
            ))
        })?;
        if agent.bound_inputs.is_empty() {
            continue;
        }
        let tools: Vec<windmill_common::flows::AgentTool> = value
            .and_then(|v| v.0.get("tools").cloned())
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| Error::BadRequest(format!("Invalid tools in agent {path}: {e}")))?
            .unwrap_or_default();
        windmill_common::flows::check_bound_agent_inputs_are_open(
            path,
            &tools,
            &agent.bound_inputs,
        )?;
    }

    // A tag is how a step picks the worker group it runs on: unauthorized, a builder could route
    // a job onto a privileged one.
    refs.tags.extend(flow_tag.clone().filter(|t| !t.is_empty()));
    if !refs.tags.is_empty() {
        // Job-aware: a WM_TOKEN running as a superadmin must not unlock restricted tags.
        let is_super_admin = is_super_admin_authed(db, authed).await?;
        for tag in &refs.tags {
            windmill_common::jobs::check_tag_available_for_workspace_internal(
                db,
                w_id,
                tag,
                None,
                std::future::ready(w_id.to_string()),
                is_super_admin,
                get_scope_tags(authed),
            )
            .await?;
        }
    }

    if refs.runnables.is_empty() && refs.pinned_scripts.is_empty() {
        return Ok(());
    }
    // A flow can step through the same script thirty times; this runs on every write, preview and
    // dependency job.
    refs.runnables.sort();
    refs.runnables.dedup();
    refs.pinned_scripts
        .sort_by_key(|(path, hash)| (path.clone(), hash.0));
    refs.pinned_scripts
        .dedup_by_key(|(path, hash)| (path.clone(), hash.0));
    // The worker resolves a step's path with the root DB handle and runs it as that runnable's
    // `on_behalf_of`, so composing an unreadable path would run code the builder cannot see. RLS
    // on this transaction is the check.
    for (is_flow, path) in &refs.runnables {
        let readable = if *is_flow {
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM flow WHERE workspace_id = $1 AND path = $2)",
                w_id,
                path,
            )
        } else {
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM script WHERE workspace_id = $1 AND path = $2)",
                w_id,
                path,
            )
        }
        .fetch_one(&mut **tx)
        .await?
        .unwrap_or(false);
        if !readable {
            return Err(Error::PermissionDenied(format!(
                "{} {path} does not exist or is not readable by you",
                if *is_flow { "Flow" } else { "Script" }
            )));
        }
    }
    // A pinned step is dispatched by its hash alone, ignoring the path beside it, so a readable
    // path paired with another script's hash would still run that other script.
    for (path, hash) in &refs.pinned_scripts {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM script WHERE workspace_id = $1 AND path = $2 AND hash = $3)",
            w_id,
            path,
            hash.0,
        )
        .fetch_one(&mut **tx)
        .await?
        .unwrap_or(false);
        if !exists {
            return Err(Error::PermissionDenied(format!(
                "Version {hash} is not a readable version of {path}"
            )));
        }
    }
    Ok(())
}

/// What a restricted role must pass to store an `ai_agent` resource, deployed or as a draft. Its
/// tools run in the agent's own runs and in every flow step linking it, so it is held to the rules
/// of a composed flow, under the same capability. `tx` is the write's own, begun as the caller.
pub async fn validate_operator_agent(
    value: &serde_json::Value,
    authed: &ApiAuthed,
    db: &DB,
    tx: &mut Transaction<'_, Postgres>,
    w_id: &str,
) -> error::Result<()> {
    if check_operator_can_build(db, w_id, authed, BuilderKind::Flows, "write AI agents").await?
        == AuthorLevel::Full
    {
        return Ok(());
    }
    let tools = value
        .get("tools")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let flow: FlowValue = serde_json::from_value(serde_json::json!({"modules": [{
        "id": "agent",
        "value": {"type": "aiagent", "input_transforms": {}, "tools": tools}
    }]}))
    .map_err(|e| Error::BadRequest(format!("Invalid agent tools: {e}")))?;
    authorize_composed_flow(&flow, &None, authed, db, tx, w_id)
        .await
        .map_err(|e| match e {
            Error::PermissionDenied(msg) => {
                Error::PermissionDenied(msg.replace("edit this flow", "edit this agent"))
            }
            e => e,
        })
}
