//! Running a saved AI agent from its own page, and the one-step flow its runs and evals share.
//!
//! The run is built here from the stored agent rather than sent by the client, which is what lets
//! anyone who can read the agent run it, operators included, the way a deployed flow runs by path:
//! a preview takes arbitrary code, this takes only the run's inputs. It runs as the caller, who
//! needs access to the agent's AI resource and to whatever its tools use.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use hyper::StatusCode;
use serde_json::value::RawValue;
use windmill_api_auth::check_scopes;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    jobs::JobPayload,
    users::username_to_permissioned_as,
    utils::{not_found_if_none, StripPath},
};
use windmill_queue::{push, PushArgs, PushIsolationLevel};

use crate::ai_evals::run::config_to_draft;
use crate::ai_evals::subject::AgentDraft;
use crate::db::{ApiAuthed, DB};
use crate::jobs::{handle_chat_conversation_messages, set_flow_memory_id, RunJobQuery};

/// The id the agent's step carries in the run, as the editor's chat names it. A token scoped to
/// `jobs:run:agents:<path>` reads its runs back by it (`require_job_within_run_scope`).
const AGENT_NODE_ID: &str = "__wm_agent_root";

/// A turn of the agent's chat when `memory_id` names a conversation, a single run otherwise.
/// Returns the job id.
pub(crate) async fn run_agent(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    Json(args): Json<HashMap<String, Box<RawValue>>>,
) -> Result<(StatusCode, String)> {
    #[cfg(feature = "enterprise")]
    crate::jobs::check_license_key_valid().await?;

    let path = path.to_path();
    check_scopes(&authed, || format!("jobs:run:agents:{path}"))?;

    // Read through the caller's own permissions: reading the agent is what allows running it.
    let mut tx = user_db.clone().begin(&authed).await?;
    let value = sqlx::query_scalar!(
        "SELECT value AS \"value: sqlx::types::Json<serde_json::Value>\"
         FROM resource WHERE workspace_id = $1 AND path = $2 AND resource_type = 'ai_agent'",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let Some(sqlx::types::Json(value)) = not_found_if_none(value, "Agent", path)? else {
        return Err(Error::BadRequest(format!(
            "Agent {path} has no configuration"
        )));
    };
    let config = config_to_draft(value)?;

    // A chat turn is filed where the agent's chat lists its conversations, which a flow cannot
    // name (flow paths carry no `.`), so the two never share a conversation list.
    let chat = run_query.memory_id.is_some();
    let run_path = if chat {
        format!("{path}.chat")
    } else {
        path.to_string()
    };
    // A step memory id would replace the conversation's, and the agent would forget the turns
    // before; history comes from the conversation alone.
    let flow_value = agent_step_flow(
        &config,
        AGENT_NODE_ID,
        chat,
        &["memory_id", "previous_messages"],
    )?;

    let push_authed = authed.clone().into();
    let (uuid, mut tx) = push(
        &db,
        PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into()),
        &w_id,
        JobPayload::RawFlow {
            value: flow_value,
            path: Some(run_path.clone()),
            restarted_from: None,
        },
        PushArgs::from(&args),
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        authed.username_override.as_deref(),
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        None,
        None,
        None,
        None,
        Some(&push_authed),
        false,
        None,
        authed.trigger_or_fallback(None),
        None,
    )
    .await?;

    if let Some(memory_id) = run_query.memory_key(&w_id, &run_path) {
        set_flow_memory_id(&mut tx, uuid, memory_id).await?;
    }
    if chat {
        // A real conversation, not a test one: this is the agent as deployed, not a draft.
        handle_chat_conversation_messages(
            &mut tx,
            &authed,
            &w_id,
            &run_path,
            &run_query,
            args.get("user_message"),
            uuid,
            false,
            &args,
        )
        .await?;
    }
    tx.commit().await?;

    Ok((StatusCode::CREATED, uuid.to_string()))
}

/// The agent as a one-step flow, validated by deserializing through `FlowValue` rather than
/// trusted as raw JSON. Its own transforms are the step's, minus `without`, and the run's inputs
/// supply the message and the attachments over the top. Always inlined, never a link to the
/// resource: a linked step would resolve the agent when it runs, not as it was read here.
pub(crate) fn agent_step_flow(
    config: &AgentDraft,
    id: &str,
    chat: bool,
    without: &[&str],
) -> Result<windmill_common::flows::FlowValue> {
    let mut input_transforms = match &config.input_transforms {
        serde_json::Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    for key in without {
        input_transforms.remove(*key);
    }
    for key in ["user_message", "user_attachments"] {
        input_transforms.insert(
            key.to_string(),
            serde_json::json!({ "type": "javascript", "expr": format!("flow_input.{}", key) }),
        );
    }
    let mut flow = serde_json::json!({
        "modules": [{
            "id": id,
            "value": {
                "type": "aiagent",
                "tools": config.tools,
                "input_transforms": serde_json::Value::Object(input_transforms),
            }
        }]
    });
    if chat {
        flow["chat_input_enabled"] = serde_json::json!(true);
    }
    Ok(serde_json::from_value(flow)?)
}
