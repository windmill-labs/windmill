//! AI agents answering in Slack.
//!
//! An agent answers only in the Slack channels it is shared in, to the workspace members whose
//! email matches the asker's Slack email, as Windmill would run it for them: through their own
//! permissions, so only if they can read it. With `slack_anyone`, set in its settings and kept
//! only while it runs on behalf of an identity, it answers anyone in those channels instead: the
//! channel is the trust boundary, as Slack decides who is in it, and everyone runs it as that
//! identity, who agreed to it by deploying the agent so. Who may share it is then only who may
//! edit it. A message reaches an agent when it names it (`+name` or `~name`), when its thread was
//! started with it, or when it is its channel's default. Anything else goes to the workspace's
//! command script as before, or, where there is none, is answered with the agents shared there.
//! One Slack thread is one conversation per person: a run's tool results are its asker's to see,
//! and a shared conversation would hand them to whoever replies next.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use uuid::Uuid;
use windmill_api_auth::{
    fetch_api_authed, fetch_api_authed_from_permissioned_as, require_is_writer,
};
use windmill_common::{
    error::{Error, JsonResult, Result},
    oauth2::WORKSPACE_SLACK_BOT_TOKEN_PATH,
    utils::{calculate_hash, StripPath},
    variables::{get_secret_value_as_admin, get_workspace_key},
    BASE_URL,
};

use crate::{
    agent_runs::run_agent_as,
    db::{ApiAuthed, DB},
    jobs::RunJobQuery,
    HTTP_CLIENT,
};
use windmill_common::db::UserDB;

/// How long the reply waits for the agent before pointing at the run instead.
const MAX_WAIT_SECS: u64 = 3600;
/// Under the 12,000 characters Slack takes in a markdown block.
const MAX_REPLY_CHARS: usize = 11_000;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/channels", get(list_workspace_channels))
        .route("/available_channels", get(list_available_channels))
        .route(
            "/agent_channels/{*path}",
            get(get_agent_slack).post(share_agent).delete(unshare_agent),
        )
        .route("/status", get(status))
}

#[derive(Serialize)]
struct SlackAgentsStatus {
    signing_secret_set: bool,
}

async fn status(_authed: ApiAuthed) -> Json<SlackAgentsStatus> {
    Json(SlackAgentsStatus { signing_secret_set: signing_secret_set() })
}

/// Agents answer only when the server verifies Slack's request signatures.
fn signing_secret_set() -> bool {
    #[cfg(feature = "oauth2")]
    {
        crate::SLACK_SIGNING_SECRET.is_some()
    }
    #[cfg(not(feature = "oauth2"))]
    {
        false
    }
}

#[derive(Serialize)]
struct SharedChannel {
    channel_id: String,
    channel_name: String,
    agent_path: String,
    is_default: bool,
}

/// Listed through the caller's permissions: an agent they cannot read stays unnamed.
async fn list_workspace_channels(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<SharedChannel>> {
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as!(
        SharedChannel,
        "SELECT c.channel_id, c.channel_name, c.agent_path, c.is_default
         FROM slack_agent_channel c
         JOIN resource r ON r.workspace_id = c.workspace_id AND r.path = c.agent_path
         WHERE c.workspace_id = $1 ORDER BY c.channel_name, c.agent_path",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

#[derive(Serialize)]
struct AgentSlack {
    /// The connected Slack workspace's name; `None` when Slack is not connected.
    slack_team_name: Option<String>,
    /// Agents answer only when the instance verifies Slack's request signatures.
    signing_secret_set: bool,
    /// The bot's Slack handle, what people mention; `None` when Slack can't be reached.
    bot_name: Option<String>,
    /// Agent scopes the workspace's connection lacks: it was made without AI agents enabled.
    missing_scopes: Vec<String>,
    /// Who the agent runs as, in Slack and elsewhere; `None` unless it runs on behalf of someone.
    runs_as: Option<String>,
    /// Whether anyone in its channels can ask it, rather than workspace members only.
    anyone: bool,
    /// Whether the caller can change its channels: anyone who can edit it.
    can_share: bool,
    channels: Vec<SharedChannel>,
}

/// The agent's identity when it runs on behalf of one, and whether anyone in its channels can
/// ask it; the agent is read with the caller's permissions, so one they cannot read is not found.
async fn agent_identity(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    path: &str,
) -> Result<(Option<String>, bool)> {
    let mut tx = user_db.clone().begin(authed).await?;
    let row = sqlx::query!(
        "SELECT value->'run_on_behalf_of' = 'true'::jsonb AS run_on_behalf_of,
                value->>'on_behalf_of' AS on_behalf_of,
                value->'slack_anyone' = 'true'::jsonb AS slack_anyone
         FROM resource WHERE workspace_id = $1 AND path = $2 AND resource_type = 'ai_agent'",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let row = row.ok_or_else(|| Error::NotFound(format!("Agent {path} not found")))?;
    let runs_as = row.on_behalf_of.filter(|_| row.run_on_behalf_of == Some(true));
    let anyone = runs_as.is_some() && row.slack_anyone == Some(true);
    Ok((runs_as, anyone))
}

async fn is_writer(authed: &ApiAuthed, db: &DB, w_id: &str, path: &str) -> Result<bool> {
    Ok(require_is_writer(
        authed,
        path,
        w_id,
        db.clone(),
        "SELECT extra_perms FROM resource WHERE path = $1 AND workspace_id = $2",
        "resource",
    )
    .await
    .is_ok())
}

async fn get_agent_slack(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<AgentSlack> {
    let path = path.to_path();
    let (runs_as, anyone) = agent_identity(&authed, &user_db, &w_id, path).await?;
    let can_share = is_writer(&authed, &db, &w_id, path).await?;
    let slack_team_name = sqlx::query_scalar!(
        "SELECT COALESCE(slack_name, slack_team_id) FROM workspace_settings
         WHERE workspace_id = $1 AND slack_team_id IS NOT NULL",
        w_id
    )
    .fetch_optional(&db)
    .await?
    .flatten();
    let channels = sqlx::query_as!(
        SharedChannel,
        "SELECT channel_id, channel_name, agent_path, is_default
         FROM slack_agent_channel WHERE workspace_id = $1 AND agent_path = $2 ORDER BY channel_name",
        w_id,
        path
    )
    .fetch_all(&db)
    .await?;
    let (bot_name, missing_scopes) = match slack_team_name {
        Some(_) => bot_identity(&db, &w_id).await.unzip(),
        None => (None, None),
    };
    Ok(Json(AgentSlack {
        slack_team_name,
        signing_secret_set: signing_secret_set(),
        bot_name,
        missing_scopes: missing_scopes.unwrap_or_default(),
        runs_as,
        anyone,
        can_share,
        channels,
    }))
}

/// The bot's handle and the agent scopes its token lacks; `None` when Slack can't be reached.
async fn bot_identity(db: &DB, w_id: &str) -> Option<(String, Vec<String>)> {
    let token = bot_token(db, w_id).await.ok()?;
    let call = async {
        let res = HTTP_CLIENT
            .post("https://slack.com/api/auth.test")
            .bearer_auth(&token)
            .send()
            .await
            .ok()?;
        // Slack lists the token's scopes in this header on every Web API response.
        let granted = res
            .headers()
            .get("x-oauth-scopes")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();
        let body: serde_json::Value = res.json().await.ok()?;
        let name = body["user"].as_str()?.to_string();
        let missing = crate::SLACK_AGENT_SCOPES
            .iter()
            .filter(|s| !granted.iter().any(|g| g == *s))
            .map(|s| s.to_string())
            .collect();
        Some((name, missing))
    };
    // On every agent page load: a slow Slack must not hold the page for the client's 20s timeout.
    tokio::time::timeout(std::time::Duration::from_secs(3), call)
        .await
        .ok()?
}

#[derive(Deserialize)]
struct ShareAgent {
    channel_id: String,
    channel_name: String,
    #[serde(default)]
    is_default: bool,
}

/// Shares the agent in a channel, or changes how it answers there.
async fn share_agent(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(body): Json<ShareAgent>,
) -> Result<String> {
    let path = path.to_path();
    agent_identity(&authed, &user_db, &w_id, path).await?;
    if !is_writer(&authed, &db, &w_id, path).await? {
        return Err(Error::PermissionDenied(format!(
            "Only those who can edit {path} can share it."
        )));
    }
    let team_id = connected_team(&db, &w_id).await?;

    if body.is_default {
        // A channel belongs to the Slack team, which several workspaces may share: one agent
        // must not take over the default another one holds.
        let current = sqlx::query!(
            "SELECT workspace_id, agent_path FROM slack_agent_channel
             WHERE slack_team_id = $1 AND channel_id = $2 AND is_default",
            team_id,
            body.channel_id
        )
        .fetch_optional(&db)
        .await?;
        match current {
            Some(c) if c.workspace_id == w_id && c.agent_path == path => {}
            Some(c) => {
                return Err(Error::BadRequest(format!(
                    "{} already answers in #{} without being named. Make it stop there first.",
                    c.agent_path, body.channel_name
                )));
            }
            // It takes the channel's unnamed messages from the command script an admin set up.
            None if !authed.is_admin => {
                return Err(Error::PermissionDenied(
                    "Only workspace admins can make an agent answer a channel without being named."
                        .to_string(),
                ));
            }
            None => {}
        }
    }
    sqlx::query!(
        "INSERT INTO slack_agent_channel
            (slack_team_id, channel_id, channel_name, workspace_id, agent_path, is_default)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (slack_team_id, channel_id, workspace_id, agent_path) DO UPDATE
            SET channel_name = EXCLUDED.channel_name, is_default = EXCLUDED.is_default",
        team_id,
        body.channel_id,
        body.channel_name,
        w_id,
        path,
        body.is_default
    )
    .execute(&db)
    .await?;
    Ok(format!("{path} is shared in #{}", body.channel_name))
}

#[derive(Deserialize)]
struct UnshareQuery {
    channel_id: String,
}

/// Taking an agent out of a channel lends nothing, so anyone who can edit it can.
async fn unshare_agent(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(q): Query<UnshareQuery>,
) -> Result<String> {
    let path = path.to_path();
    if !is_writer(&authed, &db, &w_id, path).await? {
        return Err(Error::PermissionDenied(format!(
            "Only those who can edit {path} can stop sharing it."
        )));
    }
    sqlx::query!(
        "DELETE FROM slack_agent_channel
         WHERE workspace_id = $1 AND agent_path = $2 AND channel_id = $3",
        w_id,
        path,
        q.channel_id
    )
    .execute(&db)
    .await?;
    Ok(format!("{path} is no longer shared there"))
}

#[derive(Serialize)]
struct SlackChannel {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct ChannelsQuery {
    cursor: Option<String>,
}

#[derive(Serialize)]
struct SlackChannels {
    channels: Vec<SlackChannel>,
    next_cursor: Option<String>,
}

/// The team's public channels, a page at a time, for picking one. Private channels stay out:
/// their names are for their members only.
async fn list_available_channels(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(q): Query<ChannelsQuery>,
) -> JsonResult<SlackChannels> {
    if authed.is_operator {
        return Err(Error::PermissionDenied(
            "Operators can't share agents in Slack".to_string(),
        ));
    }
    let token = bot_token(&db, &w_id).await?;
    let mut form = vec![
        ("types", "public_channel".to_string()),
        ("exclude_archived", "true".to_string()),
        ("limit", "200".to_string()),
    ];
    if let Some(cursor) = q.cursor.filter(|c| !c.is_empty()) {
        form.push(("cursor", cursor));
    }
    let res = slack_call(&token, "conversations.list", &form)
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?;
    let channels = res["channels"]
        .as_array()
        .map(|cs| {
            cs.iter()
                .filter_map(|c| {
                    Some(SlackChannel {
                        id: c["id"].as_str()?.to_string(),
                        name: c["name"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let next_cursor = res["response_metadata"]["next_cursor"]
        .as_str()
        .filter(|c| !c.is_empty())
        .map(str::to_string);
    Ok(Json(SlackChannels { channels, next_cursor }))
}

async fn connected_team(db: &DB, w_id: &str) -> Result<String> {
    sqlx::query_scalar!(
        "SELECT slack_team_id FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(db)
    .await?
    .flatten()
    .ok_or_else(|| Error::BadRequest("Slack is not connected to this workspace".to_string()))
}

async fn bot_token(db: &DB, w_id: &str) -> Result<String> {
    get_secret_value_as_admin(db, w_id, WORKSPACE_SLACK_BOT_TOKEN_PATH).await
}

/// A Slack Web API call, form-encoded since read methods such as `users.info` take no JSON.
/// Slack answers 200 with `ok: false` on failure; that is returned as the error.
async fn slack_call(
    token: &str,
    method: &str,
    form: &[(&str, String)],
) -> anyhow::Result<serde_json::Value> {
    let res: serde_json::Value = HTTP_CLIENT
        .post(format!("https://slack.com/api/{method}"))
        .bearer_auth(token)
        .form(form)
        .send()
        .await?
        .json()
        .await?;
    if res["ok"].as_bool() != Some(true) {
        let error = res["error"].as_str().unwrap_or("unknown_error");
        let needed = res["needed"].as_str().unwrap_or_default();
        anyhow::bail!("{method}: {error} {needed}");
    }
    Ok(res)
}

/// A message addressed to Windmill, from a mention, a direct message or `/windmill`.
pub struct SlackMessage {
    pub team_id: String,
    pub channel_id: String,
    pub user_id: String,
    /// With the bot mention stripped, still escaped the way Slack sends it.
    pub text: String,
    /// The message's own ts; `None` for a slash command, which has no message to reply under.
    pub ts: Option<String>,
    pub thread_ts: Option<String>,
    /// A slash command's, where it can be answered when the bot cannot post in the channel.
    pub response_url: Option<String>,
    /// Whether this request's Slack signature was verified. Carried from the check rather than
    /// re-read from the setting, which can be enabled between the two.
    pub signed: bool,
}

enum Route {
    Agent(String),
    /// No agent to run: say which ones are shared here, after `note` when there is one.
    Help {
        note: Option<String>,
    },
}

struct SharedHere {
    workspace_id: String,
    agent_path: String,
    is_default: bool,
}

/// Whether an agent answers this message; if so, the answer is under way when this returns.
/// `false` leaves the message to the workspace's command script.
pub async fn try_answer(db: &DB, msg: SlackMessage) -> Result<bool> {
    // Without it the Slack endpoints take unsigned requests, and anyone could post as a channel.
    if !msg.signed {
        return Ok(false);
    }
    let text = decode_entities(&msg.text);
    let (name, question) = split_agent_name(&text);
    let thread_root = msg.thread_ts.clone().or_else(|| msg.ts.clone());

    // Only a workspace still connected to the team answers: disconnecting keeps its bot token.
    let shared = sqlx::query_as!(
        SharedHere,
        "SELECT c.workspace_id, c.agent_path, c.is_default FROM slack_agent_channel c
         JOIN workspace_settings ws ON ws.workspace_id = c.workspace_id
            AND ws.slack_team_id = c.slack_team_id
         WHERE c.slack_team_id = $1 AND c.channel_id = $2 ORDER BY c.agent_path",
        msg.team_id,
        msg.channel_id
    )
    .fetch_all(db)
    .await?;
    let thread = match &thread_root {
        Some(root) => sqlx::query!(
            "SELECT workspace_id, agent_path FROM slack_thread_agent
             WHERE slack_team_id = $1 AND channel_id = $2 AND thread_ts = $3",
            msg.team_id,
            msg.channel_id,
            root
        )
        .fetch_optional(db)
        .await?
        // A thread's agent answers only while it is still shared here.
        .and_then(|t| {
            shared
                .iter()
                .find(|s| s.workspace_id == t.workspace_id && s.agent_path == t.agent_path)
        }),
        None => None,
    };

    let named = name.filter(|_| !shared.is_empty()).map(|name| {
        let matches = shared
            .iter()
            .filter(|s| s.agent_path == name || s.agent_path.ends_with(&format!("/{name}")))
            .collect::<Vec<_>>();
        (name, matches)
    });
    let (here, route, source) = match named {
        Some((_, matches)) if matches.len() == 1 => (
            matches[0],
            Route::Agent(matches[0].agent_path.clone()),
            "mention",
        ),
        Some((name, matches)) => {
            let note = if matches.is_empty() {
                format!("No agent named *{name}* is shared in this channel.")
            } else {
                format!("Several agents shared here are named *{name}*, use the whole path.")
            };
            (&shared[0], Route::Help { note: Some(note) }, "help")
        }
        None => match thread.or_else(|| shared.iter().find(|s| s.is_default)) {
            Some(s) => (
                s,
                Route::Agent(s.agent_path.clone()),
                if thread.is_some() {
                    "thread"
                } else {
                    "channel_default"
                },
            ),
            None => match workspace_without_command_script(db, &msg.team_id).await? {
                Some(w_id) => {
                    return start_answer(
                        db,
                        w_id,
                        Route::Help { note: None },
                        "help",
                        question,
                        msg,
                    )
                    .await;
                }
                None => return Ok(false),
            },
        },
    };
    let w_id = here.workspace_id.clone();

    // Recorded before answering, so a follow-up sent while the first answer is on its way
    // already finds its agent.
    if let (Route::Agent(path), Some(root)) = (&route, &thread_root) {
        if let Err(e) =
            record_thread_agent(db, &msg.team_id, &msg.channel_id, root, &w_id, path).await
        {
            tracing::warn!("Could not record the Slack thread's agent: {e:#}");
        }
    }
    start_answer(db, w_id, route, source, question, msg).await
}

async fn start_answer(
    db: &DB,
    w_id: String,
    route: Route,
    source: &str,
    question: &str,
    msg: SlackMessage,
) -> Result<bool> {
    if let Some(ts) = &msg.ts {
        let first = sqlx::query_scalar!(
            "INSERT INTO slack_answered_message (slack_team_id, channel_id, ts)
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING RETURNING true",
            msg.team_id,
            msg.channel_id,
            ts
        )
        .fetch_optional(db)
        .await?
        .is_some();
        if !first {
            return Ok(true);
        }
    }
    if matches!(route, Route::Agent(_)) {
        windmill_common::feature_usage::log_feature_usage("slack_agent", "invoked", source);
    }
    let db = db.clone();
    let question = question.to_string();
    // ponytail: the reply waits in this API process, so a restart mid-run loses the reply (not
    // the run, which still shows in Windmill); a job-completion hook would make it durable.
    tokio::spawn(async move {
        prune(&db).await;
        if let Err(e) = answer(&db, &w_id, route, &question, msg).await {
            tracing::error!("Slack agent answer failed: {e:#}");
        }
    });
    Ok(true)
}

async fn record_thread_agent(
    db: &DB,
    team_id: &str,
    channel_id: &str,
    thread_ts: &str,
    w_id: &str,
    agent_path: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO slack_thread_agent (slack_team_id, channel_id, thread_ts, workspace_id, agent_path)
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
        team_id,
        channel_id,
        thread_ts,
        w_id,
        agent_path
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Best effort: a message already marked answered must still be answered if this fails.
async fn prune(db: &DB) {
    let res = sqlx::query!(
        "WITH a AS (DELETE FROM slack_answered_message WHERE created_at < now() - interval '1 day')
         DELETE FROM slack_thread_agent WHERE created_at < now() - interval '90 days'"
    )
    .execute(db)
    .await;
    if let Err(e) = res {
        tracing::warn!("Could not prune Slack agent routing: {e:#}");
    }
}

/// `+name rest` or `~name rest` → (name, rest).
fn split_agent_name(text: &str) -> (Option<&str>, &str) {
    let text = text.trim();
    let Some(after) = text.strip_prefix('+').or_else(|| text.strip_prefix('~')) else {
        return (None, text);
    };
    let (name, rest) = after.split_once(char::is_whitespace).unwrap_or((after, ""));
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/' | '.'))
    {
        return (None, text);
    }
    (Some(name), rest.trim())
}

/// Slack escapes these three in message text.
fn decode_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// The team's workspace when no workspace of the team has a command script to take the message:
/// the only one connected to it.
async fn workspace_without_command_script(db: &DB, team_id: &str) -> Result<Option<String>> {
    let rows = sqlx::query!(
        "SELECT workspace_id, slack_command_script IS NOT NULL AS \"has_script!\"
         FROM workspace_settings WHERE slack_team_id = $1
         ORDER BY slack_command_script IS NOT NULL DESC LIMIT 2",
        team_id
    )
    .fetch_all(db)
    .await?;
    Ok(match rows.as_slice() {
        [only] if !only.has_script => Some(only.workspace_id.clone()),
        _ => None,
    })
}

/// The agents shared in the message's channel, and how to ask one.
async fn help_text(db: &DB, msg: &SlackMessage, note: Option<String>) -> Result<String> {
    let agents = sqlx::query_scalar!(
        "SELECT c.agent_path FROM slack_agent_channel c
         JOIN workspace_settings ws ON ws.workspace_id = c.workspace_id
            AND ws.slack_team_id = c.slack_team_id
         WHERE c.slack_team_id = $1 AND c.channel_id = $2 ORDER BY c.agent_path LIMIT 11",
        msg.team_id,
        msg.channel_id
    )
    .fetch_all(db)
    .await?;
    let note = note.map(|n| format!("{n}\n\n")).unwrap_or_default();
    let Some(first) = agents.first() else {
        let text = if msg.channel_id.starts_with('D') {
            "Agents answer in the Slack channels they're shared in, not in direct messages."
        } else {
            "No agent is shared in this channel. Anyone who can edit an agent in Windmill can share it here from its settings."
        };
        return Ok(format!("{note}{text}"));
    };
    let handle = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
    let mut list = agents
        .iter()
        .take(10)
        .map(|p| format!("• `+{}` ({p})", handle(p)))
        .collect::<Vec<_>>()
        .join("\n");
    if agents.len() > 10 {
        list.push_str("\n• …");
    }
    Ok(format!(
        "{note}Start your message with an agent's name, for example `+{} what can you do?`\n\nAgents shared in this channel:\n{list}",
        handle(first)
    ))
}

/// Who sent a message, as far as agents are concerned.
enum Asker {
    /// With their Slack email, `None` when Slack hides it or the connection lacks the scope.
    Person { email: Option<String> },
    /// A bot or a workflow: never answered, so two bots cannot keep each other talking, and an
    /// agent's identity is never lent to something no person typed.
    Bot,
    /// Someone from another Slack organization, in a channel shared with it.
    Outsider,
}

async fn asker(token: &str, msg: &SlackMessage) -> anyhow::Result<Asker> {
    if msg.user_id.is_empty() || msg.user_id == "USLACKBOT" {
        return Ok(Asker::Bot);
    }
    let info = match slack_call(token, "users.info", &[("user", msg.user_id.clone())]).await {
        Ok(info) => info,
        // Slack hides some external users from the team's token.
        Err(e) if e.to_string().contains("user_not_found") => return Ok(Asker::Outsider),
        Err(e) => return Err(e),
    };
    let user = &info["user"];
    if user["is_bot"].as_bool() == Some(true) {
        return Ok(Asker::Bot);
    }
    if user["is_stranger"].as_bool() == Some(true)
        || user["team_id"].as_str() != Some(msg.team_id.as_str())
    {
        return Ok(Asker::Outsider);
    }
    Ok(Asker::Person { email: user["profile"]["email"].as_str().map(str::to_string) })
}

async fn answer(
    db: &DB,
    w_id: &str,
    route: Route,
    question: &str,
    msg: SlackMessage,
) -> anyhow::Result<()> {
    let token = bot_token(db, w_id).await?;
    let channel = msg.channel_id.clone();

    let asker = match asker(&token, &msg).await {
        Ok(asker) => asker,
        Err(e) if e.to_string().contains("missing_scope") => {
            tracing::warn!("Slack agents in {w_id} need the connection's users:read scope: {e:#}");
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    if matches!(asker, Asker::Bot) {
        return Ok(());
    }

    // A slash command leaves no message to thread under, so the question is posted first.
    let thread_root = match msg.thread_ts.clone().or_else(|| msg.ts.clone()) {
        Some(ts) => ts,
        None => {
            // Special mentions (`<!channel>`, `<!here>`) defused: posted by the bot, they would
            // notify the whole channel on the asker's behalf.
            let echoed = msg.text.replace("<!", "&lt;!");
            let posted = slack_call(
                &token,
                "chat.postMessage",
                &[
                    ("channel", channel.clone()),
                    ("text", format!("<@{}>: {}", msg.user_id, echoed)),
                ],
            )
            .await;
            match posted {
                Ok(posted) => posted["ts"].as_str().unwrap_or_default().to_string(),
                Err(e) => {
                    let not_member = ["not_in_channel", "channel_not_found"]
                        .iter()
                        .any(|code| e.to_string().contains(code));
                    if let (true, Some(url)) = (not_member, &msg.response_url) {
                        let _ = HTTP_CLIENT
                            .post(url)
                            .json(&serde_json::json!({
                                "text": "Add the Windmill bot to this conversation to ask agents here."
                            }))
                            .send()
                            .await;
                    }
                    return Err(e);
                }
            }
        }
    };
    // A slash command's thread root exists only now, so its agent could not be recorded before.
    if let (Route::Agent(path), None) = (&route, msg.thread_ts.as_ref().or(msg.ts.as_ref())) {
        if let Err(e) =
            record_thread_agent(db, &msg.team_id, &channel, &thread_root, w_id, path).await
        {
            tracing::warn!("Could not record the Slack thread's agent: {e:#}");
        }
    }
    let reply = Reply::start(&token, &channel, &thread_root).await?;

    let email = match asker {
        Asker::Person { email } => email,
        _ => {
            return reply
                .send(
                    "Agents answer only people from this Slack workspace.".to_string(),
                    None,
                )
                .await;
        }
    };
    let agent_path = match route {
        Route::Help { note } => return reply.send(help_text(db, &msg, note).await?, None).await,
        Route::Agent(path) => path,
    };

    if question.is_empty() {
        return reply
            .send(format!("What would you like to ask `{agent_path}`?"), None)
            .await;
    }

    // Read at each message: who can ask follows the agent's last deploy.
    let share = sqlx::query!(
        "SELECT COALESCE(r.value->'run_on_behalf_of' = 'true'::jsonb
                    AND r.value->'slack_anyone' = 'true'::jsonb, false) AS \"anyone!\",
                r.value->>'on_behalf_of' AS on_behalf_of
         FROM slack_agent_channel c
         JOIN resource r ON r.workspace_id = c.workspace_id AND r.path = c.agent_path
         WHERE c.slack_team_id = $1 AND c.channel_id = $2 AND c.workspace_id = $3
           AND c.agent_path = $4 AND r.resource_type = 'ai_agent'",
        msg.team_id,
        channel,
        w_id,
        agent_path
    )
    .fetch_optional(db)
    .await?;
    let members_only = share.as_ref().is_some_and(|s| !s.anyone);
    let authed = match share {
        None => {
            return reply
                .send(
                    format!("`{agent_path}` is no longer shared in this channel."),
                    None,
                )
                .await;
        }
        Some(share) if !share.anyone => {
            let Some(email) = email else {
                return reply.send(
                    "Windmill can't read your Slack email. A workspace admin needs to reconnect Slack in the workspace settings to grant it.".to_string(),
                    None,
                )
                .await;
            };
            let member = sqlx::query!(
                "SELECT username, email FROM usr
                 WHERE workspace_id = $1 AND lower(email) = lower($2) AND NOT disabled",
                w_id,
                email
            )
            .fetch_optional(db)
            .await?;
            let Some(member) = member else {
                return reply.send(
                    format!(
                        "Your Slack email isn't a member of the Windmill workspace *{w_id}*. Ask an admin to invite you."
                    ),
                    None,
                )
                .await;
            };
            fetch_api_authed(member.username, member.email, w_id, db, None).await?
        }
        Some(share) => {
            let on_behalf_of = windmill_common::on_behalf_of_from_permissioned_as(
                share.on_behalf_of.as_deref(),
                w_id,
                db,
            )
            .await?;
            let Some(on_behalf_of) = on_behalf_of else {
                return reply
                    .send(
                        format!(
                            "`{agent_path}` doesn't run on behalf of anyone, so it can't answer in Slack."
                        ),
                        None,
                    )
                    .await;
            };
            fetch_api_authed_from_permissioned_as(
                on_behalf_of.permissioned_as,
                on_behalf_of.email,
                w_id,
                db,
                None,
            )
            .await?
        }
    };
    let user_db = UserDB::new(db.clone());

    // Keyed with the workspace's secret: anyone who can read the agent's runs could rebuild the
    // plain thread and user ids and load this person's conversation, tool results included.
    let workspace_key = get_workspace_key(w_id, db).await?;
    let run_query = RunJobQuery {
        memory_id: Some(calculate_hash(&format!(
            "{workspace_key}:slack:{}:{}:{}:{}",
            msg.team_id, channel, thread_root, msg.user_id
        ))),
        ..Default::default()
    };
    let args: HashMap<String, Box<RawValue>> = [(
        "user_message".to_string(),
        windmill_common::worker::to_raw_value(&question),
    )]
    .into();
    let job_id =
        match run_agent_as(&authed, db, &user_db, w_id, &agent_path, &run_query, &args).await {
            Ok(id) => id,
            Err(Error::NotFound(_)) if members_only => {
                return reply
                    .send(
                        format!("You don't have access to `{agent_path}` in Windmill."),
                        None,
                    )
                    .await;
            }
            Err(e) => {
                return reply
                    .send(format!("`{agent_path}` couldn't start: {e}"), None)
                    .await;
            }
        };

    let run_link = format!(
        "<{}/run/{job_id}?workspace={w_id}|View run> · `{agent_path}`",
        BASE_URL.load()
    );
    let thinking = reply.keep_thinking();
    let result = wait_for_result(db, w_id, job_id).await;
    if let Some(thinking) = thinking {
        thinking.abort();
    }
    let text = match result? {
        Some((result, true)) => answer_text(&result),
        Some((result, false)) => format!("The agent failed: {}", error_text(&result)),
        None => "Still working, follow it in the run.".to_string(),
    };
    reply.send(text, Some(run_link)).await
}

async fn wait_for_result(
    db: &DB,
    w_id: &str,
    job_id: Uuid,
) -> Result<Option<(serde_json::Value, bool)>> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(MAX_WAIT_SECS);
    while std::time::Instant::now() < deadline {
        let row = sqlx::query!(
            "SELECT result AS \"result: sqlx::types::Json<serde_json::Value>\",
                    status = 'success' AS \"success!\"
             FROM v2_job_completed WHERE id = $1 AND workspace_id = $2",
            job_id,
            w_id
        )
        .fetch_optional(db)
        .await?;
        if let Some(row) = row {
            return Ok(Some((
                row.result.map(|r| r.0).unwrap_or_default(),
                row.success,
            )));
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    Ok(None)
}

/// The agent step's `output`: its text, or a structured output as a JSON block.
fn answer_text(result: &serde_json::Value) -> String {
    match &result["output"] {
        serde_json::Value::String(s) if !s.trim().is_empty() => s.clone(),
        serde_json::Value::Null => "The agent returned no answer.".to_string(),
        other => format!(
            "```\n{}\n```",
            serde_json::to_string_pretty(other).unwrap_or_default()
        ),
    }
}

fn error_text(result: &serde_json::Value) -> String {
    result["error"]["message"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| result.to_string())
}

/// The bot's reply in a thread. While it is being worked out the thread shows the bot as thinking,
/// or, where Slack refuses that status, a placeholder message the reply then replaces.
#[derive(Clone)]
struct Reply {
    token: String,
    channel: String,
    thread_ts: String,
    placeholder: Option<String>,
}

impl Reply {
    async fn start(token: &str, channel: &str, thread_ts: &str) -> anyhow::Result<Self> {
        let mut reply = Reply {
            token: token.to_string(),
            channel: channel.to_string(),
            thread_ts: thread_ts.to_string(),
            placeholder: None,
        };
        if let Err(e) = reply.set_status("is thinking…").await {
            tracing::debug!("Slack refused the thinking status, posting a placeholder: {e:#}");
            let posted = slack_call(
                token,
                "chat.postMessage",
                &[
                    ("channel", channel.to_string()),
                    ("thread_ts", thread_ts.to_string()),
                    ("text", "Thinking…".to_string()),
                ],
            )
            .await?;
            reply.placeholder = posted["ts"].as_str().map(str::to_string);
        }
        Ok(reply)
    }

    async fn set_status(&self, status: &str) -> anyhow::Result<()> {
        slack_call(
            &self.token,
            "assistant.threads.setStatus",
            &[
                ("channel_id", self.channel.clone()),
                ("thread_ts", self.thread_ts.clone()),
                ("status", status.to_string()),
            ],
        )
        .await
        .map(|_| ())
    }

    /// Slack drops the status after two minutes without a reply, so a long run renews it.
    fn keep_thinking(&self) -> Option<tokio::task::JoinHandle<()>> {
        if self.placeholder.is_some() {
            return None;
        }
        let reply = self.clone();
        Some(tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(90)).await;
                let _ = reply.set_status("is thinking…").await;
            }
        }))
    }

    async fn send(&self, text: String, footer: Option<String>) -> anyhow::Result<()> {
        let (text, blocks) = reply_blocks(&text, footer)?;
        let mut form = vec![
            ("channel", self.channel.clone()),
            ("text", text),
            ("blocks", blocks),
        ];
        match &self.placeholder {
            Some(ts) => {
                form.push(("ts", ts.clone()));
                slack_call(&self.token, "chat.update", &form).await?;
            }
            None => {
                form.push(("thread_ts", self.thread_ts.clone()));
                slack_call(&self.token, "chat.postMessage", &form).await?;
                let _ = self.set_status("").await;
            }
        }
        Ok(())
    }
}

/// The reply's notification text and its blocks: the answer as markdown, and an optional footer.
fn reply_blocks(text: &str, footer: Option<String>) -> anyhow::Result<(String, String)> {
    let text = if text.chars().count() > MAX_REPLY_CHARS {
        let cut: String = text.chars().take(MAX_REPLY_CHARS).collect();
        format!("{cut}\n\n_The answer is cut short here; the run has all of it._")
    } else {
        text.to_string()
    };
    let mut blocks = vec![serde_json::json!({ "type": "markdown", "text": text })];
    if let Some(footer) = footer {
        blocks.push(serde_json::json!({
            "type": "context",
            "elements": [{ "type": "mrkdwn", "text": footer }]
        }));
    }
    let fallback: String = text.chars().take(300).collect();
    Ok((fallback, serde_json::to_string(&blocks)?))
}

#[cfg(test)]
mod tests {
    use super::split_agent_name;

    #[test]
    fn agent_name_prefix() {
        assert_eq!(
            split_agent_name("+sql_analyst how many?"),
            (Some("sql_analyst"), "how many?")
        );
        assert_eq!(
            split_agent_name("~f/data/sql_analyst  hi"),
            (Some("f/data/sql_analyst"), "hi")
        );
        assert_eq!(split_agent_name("+helper"), (Some("helper"), ""));
        assert_eq!(split_agent_name("what is +x"), (None, "what is +x"));
        assert_eq!(split_agent_name("+ spaced"), (None, "+ spaced"));
    }
}
