//! Running a saved AI agent from its own page, and the one-step flow its runs and evals share.
//!
//! The run is built here from the stored agent rather than sent by the client, which is what lets
//! anyone who can read the agent run it, operators included, the way a deployed flow runs by path:
//! a preview takes arbitrary code, this takes only the run's inputs. It runs as the caller, or as
//! the agent's `on_behalf_of` when its `run_on_behalf_of` is set; the server resolves
//! `on_behalf_of` on every write of the agent.

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
    variables::untrusted_arg_interpolates,
};
use windmill_queue::{push, PushArgs, PushIsolationLevel};

use crate::ai_evals::run::config_to_draft;
use crate::ai_evals::subject::AgentDraft;
use crate::db::{ApiAuthed, DB};
use crate::jobs::{
    handle_chat_conversation_messages, run_end_user_email, set_flow_memory_id, RunJobQuery,
};

/// The id the agent's step carries in the run, as the editor's chat names it. A token scoped to
/// `jobs:run:agents:<path>` reads its runs back by it (`require_job_within_read_scope`).
pub(crate) const AGENT_NODE_ID: &str = "__wm_agent_root";

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
    let path = path.to_path();
    check_scopes(&authed, || format!("jobs:run:agents:{path}"))?;

    let uuid = run_agent_as(&authed, &db, &user_db, &w_id, path, &run_query, &args).await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

/// [`run_agent`] for a caller already authenticated, such as the identity of an agent shared in a
/// Slack channel.
pub(crate) async fn run_agent_as(
    authed: &ApiAuthed,
    db: &DB,
    user_db: &UserDB,
    w_id: &str,
    path: &str,
    run_query: &RunJobQuery,
    args: &HashMap<String, Box<RawValue>>,
) -> Result<uuid::Uuid> {
    #[cfg(feature = "enterprise")]
    crate::jobs::check_license_key_valid().await?;

    // Read through the caller's own permissions: reading the agent is what allows running it.
    let mut tx = user_db.clone().begin(authed).await?;
    let value = sqlx::query_scalar!(
        "SELECT value AS \"value: sqlx::types::Json<serde_json::Value>\"
         FROM resource WHERE workspace_id = $1 AND path = $2 AND resource_type = 'ai_agent'",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let Some(sqlx::types::Json(mut value)) = not_found_if_none(value, "Agent", path)? else {
        return Err(Error::BadRequest(format!(
            "Agent {path} has no configuration"
        )));
    };
    // Who the agent runs as and who can ask it in Slack, not inputs: its identity when it opts
    // in, as a deployed flow does.
    let obj = value.as_object_mut();
    let (agent_obo, run_obo) = obj
        .map(|o| {
            o.remove("slack_anyone");
            (o.remove("on_behalf_of"), o.remove("run_on_behalf_of"))
        })
        .unwrap_or_default();
    let agent_obo = agent_obo
        .filter(|_| run_obo.and_then(|v| v.as_bool()).unwrap_or(false))
        .and_then(|v| v.as_str().map(str::to_string));
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

    let on_behalf_of =
        windmill_common::on_behalf_of_from_permissioned_as(agent_obo.as_deref(), w_id, db).await?;
    // As `run_flow` picks the identity of a deployed flow: the agent's own when it has one, the
    // caller's otherwise, which lends nobody's permissions.
    let (email, permissioned_as, push_authed, isolation) = if let Some(obo) = on_behalf_of.as_ref()
    {
        (
            &obo.email,
            obo.permissioned_as.clone(),
            None,
            PushIsolationLevel::IsolatedRoot(db.clone()),
        )
    } else {
        (
            &authed.email,
            username_to_permissioned_as(&authed.username),
            Some(authed.clone().into()),
            PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into()),
        )
    };
    if let Some(obo) = on_behalf_of.as_ref() {
        // The worker would interpolate such an input with the job's token, which is the
        // identity's.
        if args.values().any(|v| untrusted_arg_interpolates(v.get())) {
            return Err(Error::BadRequest(format!(
                "Agent {path} runs on behalf of {}, so its inputs cannot reference variables or resources",
                obo.permissioned_as
            )));
        }
        #[cfg(all(feature = "parquet", feature = "private"))]
        check_attachments_readable(db, authed, w_id, args).await?;
    }
    let scope_ceiling = windmill_api_auth::caller_scope_ceiling(db, authed).await?;
    let end_user_email = run_end_user_email(db, w_id, authed, None).await?;
    let (uuid, mut tx) = push(
        db,
        isolation,
        w_id,
        JobPayload::RawFlow {
            value: flow_value,
            path: Some(run_path.clone()),
            restarted_from: None,
        },
        PushArgs::from(args),
        authed.display_username(),
        email,
        permissioned_as,
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
        push_authed.as_ref(),
        false,
        end_user_email,
        authed.trigger_or_fallback(None),
        None,
        scope_ceiling.as_deref(),
    )
    .await?;

    if let Some(memory_id) = run_query.memory_key(w_id, &run_path) {
        set_flow_memory_id(&mut tx, uuid, memory_id).await?;
    }
    if chat {
        // The caller's conversation, whoever the agent runs as, and a real one: this is the
        // agent as deployed, not a draft under test.
        handle_chat_conversation_messages(
            &mut tx,
            authed,
            w_id,
            &run_path,
            run_query,
            args.get("user_message"),
            uuid,
            false,
            args,
        )
        .await?;
    }
    tx.commit().await?;

    Ok(uuid)
}

/// The step fetches the message's files with its job's token, so a run as the agent's identity
/// would read any file that identity can: a caller attaches only what they could download
/// themselves, checked as `sign_s3_objects` checks a key before signing it.
#[cfg(all(feature = "parquet", feature = "private"))]
async fn check_attachments_readable(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    args: &HashMap<String, Box<RawValue>>,
) -> Result<()> {
    use crate::job_helpers_oss::{
        get_large_file_storage, get_workspace_s3_resource_and_check_paths,
    };
    use windmill_common::db::DbWithOptAuthed;
    use windmill_types::s3::{S3Object, S3Permission};

    let Some(raw) = args.get("user_attachments") else {
        return Ok(());
    };
    let attachments: Option<Vec<S3Object>> = serde_json::from_str(raw.get())
        .map_err(|e| Error::BadRequest(format!("invalid user_attachments: {e}")))?;
    for attachment in attachments.iter().flatten().filter(|a| !a.s3.is_empty()) {
        // The path check is skipped, not failed, when the storage does not resolve.
        if get_large_file_storage(db, w_id, attachment.storage.clone())
            .await?
            .is_none()
        {
            return Err(windmill_object_store::workspace_storage_not_found(
                attachment.storage.as_deref(),
            ));
        }
        get_workspace_s3_resource_and_check_paths(
            &DbWithOptAuthed::from_authed(authed, db.clone(), None),
            Some(authed),
            w_id,
            attachment.storage.clone(),
            &[(&attachment.s3, S3Permission::READ)],
            None,
        )
        .await?;
    }
    Ok(())
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
