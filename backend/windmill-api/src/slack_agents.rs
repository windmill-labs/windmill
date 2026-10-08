//! AI agents answering in Slack.
//!
//! A message reaches an agent when it names one (`+name` or `~name`), when its thread was started
//! with one, or when its channel has a default agent. Anything else goes to the workspace's
//! command script as before, or, where there is none, is answered with how to reach an agent.
//! The agent runs as the workspace member whose email the Slack user
//! has, so a Slack user who is not a member gets nothing, and what the agent may read or call is
//! that member's. One Slack thread is one conversation per person: a run's tool results are its
//! runner's to see, and a shared conversation would hand them to whoever replies next.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    routing::{delete, get},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use uuid::Uuid;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    oauth2::WORKSPACE_SLACK_BOT_TOKEN_PATH,
    utils::{require_admin, StripPath},
    variables::get_secret_value_as_admin,
    BASE_URL,
};

use crate::{
    agent_runs::run_agent_as,
    db::{ApiAuthed, DB},
    jobs::RunJobQuery,
    users::fetch_api_authed,
    HTTP_CLIENT,
};

/// How long the reply waits for the agent before pointing at the run instead.
const MAX_WAIT_SECS: u64 = 3600;
/// Under the 12,000 characters Slack takes in a markdown block.
const MAX_REPLY_CHARS: usize = 11_000;

pub fn workspaced_service() -> Router {
    Router::new()
        .route(
            "/channels",
            get(list_channel_agents).post(set_channel_agent),
        )
        .route("/channels/{channel_id}", delete(remove_channel_agent))
        .route("/available_channels", get(list_available_channels))
        .route("/agent_channels/{*path}", get(list_agent_channels))
}

#[derive(Serialize)]
struct ChannelAgent {
    channel_id: String,
    channel_name: String,
    agent_path: String,
}

async fn list_channel_agents(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<ChannelAgent>> {
    let rows = sqlx::query_as!(
        ChannelAgent,
        "SELECT channel_id, channel_name, agent_path FROM slack_channel_agent
         WHERE workspace_id = $1 ORDER BY channel_name",
        w_id
    )
    .fetch_all(&db)
    .await?;
    Ok(Json(rows))
}

#[derive(Serialize)]
struct AgentSlack {
    /// The connected Slack workspace's name; `None` when Slack is not connected.
    slack_team_name: Option<String>,
    channels: Vec<ChannelAgent>,
}

async fn list_agent_channels(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<AgentSlack> {
    let slack_team_name = sqlx::query_scalar!(
        "SELECT COALESCE(slack_name, slack_team_id) FROM workspace_settings
         WHERE workspace_id = $1 AND slack_team_id IS NOT NULL",
        w_id
    )
    .fetch_optional(&db)
    .await?
    .flatten();
    let channels = sqlx::query_as!(
        ChannelAgent,
        "SELECT channel_id, channel_name, agent_path FROM slack_channel_agent
         WHERE workspace_id = $1 AND agent_path = $2 ORDER BY channel_name",
        w_id,
        path.to_path()
    )
    .fetch_all(&db)
    .await?;
    Ok(Json(AgentSlack { slack_team_name, channels }))
}

#[derive(Deserialize)]
struct SetChannelAgent {
    channel_id: String,
    channel_name: String,
    agent_path: String,
}

async fn set_channel_agent(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(body): Json<SetChannelAgent>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let team_id = connected_team(&db, &w_id).await?;

    let is_agent = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM resource
         WHERE workspace_id = $1 AND path = $2 AND resource_type = 'ai_agent')",
        w_id,
        body.agent_path
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    if !is_agent {
        return Err(Error::BadRequest(format!(
            "{} is not an agent",
            body.agent_path
        )));
    }

    // A channel belongs to the Slack team, which several workspaces may share: one workspace
    // must not take over a channel another one set.
    let taken = sqlx::query!(
        "INSERT INTO slack_channel_agent
            (slack_team_id, channel_id, channel_name, workspace_id, agent_path)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (slack_team_id, channel_id) DO UPDATE
            SET channel_name = EXCLUDED.channel_name, agent_path = EXCLUDED.agent_path
            WHERE slack_channel_agent.workspace_id = EXCLUDED.workspace_id
         RETURNING channel_id",
        team_id,
        body.channel_id,
        body.channel_name,
        w_id,
        body.agent_path
    )
    .fetch_optional(&db)
    .await?
    .is_none();
    if taken {
        return Err(Error::BadRequest(format!(
            "#{} already has a default agent set from another workspace",
            body.channel_name
        )));
    }
    Ok(format!(
        "{} answers in #{}",
        body.agent_path, body.channel_name
    ))
}

async fn remove_channel_agent(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, channel_id)): Path<(String, String)>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    sqlx::query!(
        "DELETE FROM slack_channel_agent WHERE workspace_id = $1 AND channel_id = $2",
        w_id,
        channel_id
    )
    .execute(&db)
    .await?;
    Ok("Removed".to_string())
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

/// The team's public channels, a page at a time, for picking one.
async fn list_available_channels(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(q): Query<ChannelsQuery>,
) -> JsonResult<SlackChannels> {
    require_admin(authed.is_admin, &authed.username)?;
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
}

enum Route {
    Named(String),
    Agent(String),
    /// Nothing answers here: say how to reach an agent.
    Help,
}

/// Whether an agent answers this message; if so, the answer is under way when this returns.
/// `false` leaves the message to the workspace's command script.
pub async fn try_answer(db: &DB, msg: SlackMessage) -> Result<bool> {
    let text = decode_entities(&msg.text);
    let (name, question) = split_agent_name(&text);
    let thread_root = msg.thread_ts.clone().or_else(|| msg.ts.clone());

    // Only a workspace still connected to the team answers: disconnecting keeps its bot token.
    let thread = match &thread_root {
        Some(root) => sqlx::query!(
            "SELECT t.workspace_id, t.agent_path FROM slack_thread_agent t
             JOIN workspace_settings ws ON ws.workspace_id = t.workspace_id
                AND ws.slack_team_id = t.slack_team_id
             WHERE t.slack_team_id = $1 AND t.channel_id = $2 AND t.thread_ts = $3",
            msg.team_id,
            msg.channel_id,
            root
        )
        .fetch_optional(db)
        .await?
        .map(|r| (r.workspace_id, r.agent_path)),
        None => None,
    };
    let channel = sqlx::query!(
        "SELECT c.workspace_id, c.agent_path FROM slack_channel_agent c
         JOIN workspace_settings ws ON ws.workspace_id = c.workspace_id
            AND ws.slack_team_id = c.slack_team_id
         WHERE c.slack_team_id = $1 AND c.channel_id = $2",
        msg.team_id,
        msg.channel_id
    )
    .fetch_optional(db)
    .await?
    .map(|r| (r.workspace_id, r.agent_path));

    let (w_id, route, source) = if let Some(name) = name {
        // The thread's or the channel's workspace, where several share the team.
        let w_id = match thread.as_ref().or(channel.as_ref()) {
            Some((w_id, _)) => Some(w_id.clone()),
            None => workspace_for_team(db, &msg.team_id).await?,
        };
        let Some(w_id) = w_id else {
            return Ok(false);
        };
        (w_id, Route::Named(name.to_string()), "mention")
    } else if let Some((w_id, path)) = thread {
        (w_id, Route::Agent(path), "thread")
    } else if let Some((w_id, path)) = channel {
        (w_id, Route::Agent(path), "channel_default")
    } else if let Some(w_id) = workspace_without_command_script(db, &msg.team_id).await? {
        (w_id, Route::Help, "help")
    } else {
        return Ok(false);
    };

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
        sqlx::query!(
            "DELETE FROM slack_answered_message WHERE created_at < now() - interval '1 day'"
        )
        .execute(db)
        .await?;
    }

    if !matches!(route, Route::Help) {
        windmill_common::feature_usage::log_feature_usage("slack_agent", "invoked", source);
    }
    let db = db.clone();
    let question = question.to_string();
    // ponytail: the reply waits in this API process, so a restart mid-run loses the reply (not
    // the run, which still shows in Windmill); a job-completion hook would make it durable.
    tokio::spawn(async move {
        if let Err(e) = answer(&db, &w_id, route, &question, msg).await {
            tracing::error!("Slack agent answer failed: {e:#}");
        }
    });
    Ok(true)
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

/// The workspace a message naming an agent goes to when neither its thread nor its channel says:
/// the one whose command script the team's messages already reach, else the team's only one.
async fn workspace_for_team(db: &DB, team_id: &str) -> Result<Option<String>> {
    let rows = sqlx::query!(
        "SELECT workspace_id, slack_command_script IS NOT NULL AS \"has_script!\"
         FROM workspace_settings WHERE slack_team_id = $1
         ORDER BY slack_command_script IS NOT NULL DESC LIMIT 2",
        team_id
    )
    .fetch_all(db)
    .await?;
    Ok(match rows.as_slice() {
        [only] => Some(only.workspace_id.clone()),
        [first, ..] if first.has_script => Some(first.workspace_id.clone()),
        _ => None,
    })
}

/// The team's workspace when no workspace of the team has a command script to take the message.
async fn workspace_without_command_script(db: &DB, team_id: &str) -> Result<Option<String>> {
    let has_script = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM workspace_settings
         WHERE slack_team_id = $1 AND slack_command_script IS NOT NULL)",
        team_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);
    if has_script {
        return Ok(None);
    }
    workspace_for_team(db, team_id).await
}

/// How to reach an agent, with the ones this member can use.
async fn help_text(user_db: &UserDB, authed: &ApiAuthed, w_id: &str) -> Result<String> {
    let mut tx = user_db.clone().begin(authed).await?;
    let agents = sqlx::query_scalar!(
        "SELECT path FROM resource WHERE workspace_id = $1 AND resource_type = 'ai_agent'
         ORDER BY path LIMIT 11",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let Some(first) = agents.first() else {
        return Ok(format!(
            "No agent answers here by default, and there is no agent in the Windmill workspace *{w_id}* you can use yet."
        ));
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
        "No agent answers here by default. Start your message with an agent's name, for example `+{} what can you do?`\n\nAgents you can use:\n{list}\n\nA workspace admin can make an agent the default for a channel from the agent's page in Windmill.",
        handle(first)
    ))
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
    let reply = Reply::start(&token, &channel, &thread_root).await?;

    let authed = match member_for_slack_user(db, &token, w_id, &msg.user_id).await {
        Ok(Some(authed)) => authed,
        Ok(None) => {
            return reply.send(
                format!(
                    "Your Slack email isn't a member of the Windmill workspace *{w_id}*. Ask an admin to invite you."
                ),
                None,
            )
            .await;
        }
        Err(e) if e.to_string().contains("missing_scope") => {
            return reply.send(
                "Windmill can't read your Slack email. A workspace admin needs to reconnect Slack in the workspace settings to grant it.".to_string(),
                None,
            )
            .await;
        }
        Err(e) => return Err(e),
    };
    let user_db = UserDB::new(db.clone());

    let agent_path = match route {
        Route::Help => {
            return reply
                .send(help_text(&user_db, &authed, w_id).await?, None)
                .await;
        }
        Route::Agent(path) => path,
        Route::Named(name) => {
            let mut tx = user_db.clone().begin(&authed).await?;
            let matches = sqlx::query_scalar!(
                "SELECT path FROM resource
                 WHERE workspace_id = $1 AND resource_type = 'ai_agent'
                   AND (path = $2 OR right(path, length($2) + 1) = '/' || $2)
                 ORDER BY path LIMIT 6",
                w_id,
                name
            )
            .fetch_all(&mut *tx)
            .await?;
            tx.commit().await?;
            match matches.as_slice() {
                [only] => only.clone(),
                [] => {
                    return reply
                        .send(
                            format!("I can't find an agent named *{name}* that you can use."),
                            None,
                        )
                        .await;
                }
                several => {
                    let list = several
                        .iter()
                        .map(|p| format!("`+{p}`"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    return reply
                        .send(
                            format!("Several agents are named *{name}*, pick one: {list}"),
                            None,
                        )
                        .await;
                }
            }
        }
    };

    if question.is_empty() {
        return reply
            .send(format!("What would you like to ask `{agent_path}`?"), None)
            .await;
    }

    sqlx::query!(
        "INSERT INTO slack_thread_agent (slack_team_id, channel_id, thread_ts, workspace_id, agent_path)
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
        msg.team_id,
        channel,
        thread_root,
        w_id,
        agent_path
    )
    .execute(db)
    .await?;

    let run_query = RunJobQuery {
        memory_id: Some(format!(
            "slack:{}:{}:{}:{}",
            msg.team_id, channel, thread_root, msg.user_id
        )),
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

async fn member_for_slack_user(
    db: &DB,
    token: &str,
    w_id: &str,
    slack_user: &str,
) -> anyhow::Result<Option<ApiAuthed>> {
    let info = slack_call(token, "users.info", &[("user", slack_user.to_string())]).await?;
    let Some(email) = info["user"]["profile"]["email"].as_str() else {
        return Ok(None);
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
        return Ok(None);
    };
    Ok(Some(
        fetch_api_authed(member.username, member.email, w_id, db, None).await?,
    ))
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
