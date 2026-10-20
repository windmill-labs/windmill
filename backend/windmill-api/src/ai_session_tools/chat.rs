//! What the in-app chat sends beside a tool call, and what it gets back to apply in the
//! browser. MCP has neither: its tools read and write the server alone.

use axum::{extract::Path, routing::post, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use windmill_api_auth::is_effectively_unscoped;
use windmill_common::error::{Error, JsonResult};

use super::{call_session_tool, CallRefused, Ctx, SessionAccess, Surface, ToolOutcome};
use crate::db::ApiAuthed;

#[derive(Clone)]
struct BaseInternalUrl(String);

pub(crate) fn workspaced_service(base_internal_url: String) -> Router {
    Router::new()
        .route("/{name}", post(call_tool))
        .layer(Extension(BaseInternalUrl(base_internal_url)))
}

#[derive(Deserialize)]
struct ToolCall {
    #[serde(default)]
    args: Value,
    #[serde(default)]
    context: ChatContext,
}

#[derive(Serialize)]
struct ToolResponse {
    #[serde(flatten)]
    outcome: ToolOutcome,
    effects: Vec<Effect>,
}

async fn call_tool(
    authed: ApiAuthed,
    Extension(BaseInternalUrl(base_internal_url)): Extension<BaseInternalUrl>,
    Path((w_id, name)): Path<(String, String)>,
    Json(call): Json<ToolCall>,
) -> JsonResult<ToolResponse> {
    // A tool calls the API with a JWT carrying its caller's full rights, so a token
    // minted for something narrower (a job, a scope) must not reach it.
    if authed.job_id.is_some() || !is_effectively_unscoped(authed.scopes.as_deref()) {
        return Err(Error::PermissionDenied(
            "The AI session tools are only reachable with an unscoped user token".to_string(),
        ));
    }
    let mut ctx = Ctx::new(&authed, &w_id, &base_internal_url, SessionAccess::Full);
    ctx.surface = Surface::Chat;
    ctx.chat = Some(&call.context);
    let outcome =
        call_session_tool(&ctx, &name, call.args)
            .await
            .map_err(|refused| match refused {
                CallRefused::UnknownTool(message)
                | CallRefused::NotAllowed(message)
                | CallRefused::NotAvailable(message) => Error::BadRequest(message),
            })?;
    let effects = ctx.effects.into_inner().unwrap();
    Ok(Json(ToolResponse { outcome, effects }))
}

/// Browser-held state a tool needs and the server cannot see.
#[derive(Deserialize, Default)]
pub(crate) struct ChatContext {
    /// The editor open on each draft kind, at most one per kind.
    #[serde(default)]
    pub live_editors: Vec<LiveEditor>,
    /// The draft cells the browser holds: an open editor's, and any whose latest edit is
    /// not on the server yet. They are newer than the server's row.
    #[serde(default)]
    pub cells: Vec<Cell>,
    /// How many characters of an npm package's readme and of its types fit the chat's
    /// model: 1% of its context window.
    #[serde(default)]
    pub npm_text_limit: Option<usize>,
    /// Run only the check that comes before the chat's confirmation card: what a
    /// confirmed call would refuse, and what it would change.
    #[serde(default)]
    pub check_only: bool,
    /// Whether the chat's user left the skill a call reads switched on.
    #[serde(default)]
    pub skill_enabled: Option<bool>,
    /// The call comes from an AI session, which has a preview panel and builds pipeline
    /// nodes in the folder's pipeline draft.
    #[serde(default)]
    pub session: bool,
    /// The workspace's AI provider resources and the models each serves, sent once a call
    /// asked for it with `needs_ai_provider_catalog`.
    #[serde(default)]
    pub ai_provider_catalog: Option<super::ai_providers::Catalog>,
}

#[derive(Deserialize)]
pub(crate) struct LiveEditor {
    /// The draft kind (`script`, `flow`, `raw_app`, `trigger_schedule`, …).
    pub kind: String,
    /// The key the draft is stored under: `u/{user}/draft_{uuid}` for a new item.
    pub storage_path: String,
    /// The path the editor shows, which a rename leaves the storage key behind.
    pub effective_path: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct Cell {
    pub kind: String,
    pub storage_path: String,
    pub value: Value,
}

/// Something a call changed that the chat applies in the browser.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Effect {
    /// Save `value` as the draft at `storage_path`, through the editor's own draft store.
    /// `last_sync` is the server row the value was built on, when it was read from the
    /// server rather than from a cell: the save detects a conflict against it.
    DraftWrite {
        kind: String,
        storage_path: String,
        value: Value,
        last_sync: Option<String>,
        force: bool,
    },
    /// Delete the draft at `storage_path`. `keep_live_editor` leaves an open editor's
    /// copy in place, detached from the server row.
    DraftDelete {
        kind: String,
        storage_path: String,
        last_sync: Option<String>,
        keep_live_editor: bool,
    },
    /// `get_run` read one job, which the card renders the run from.
    InspectedRun { job_id: String },
    /// What the call found that only its card shows, not the model.
    Facts(serde_json::Map<String, Value>),
    /// A patch to the tool's card, in the order the call reached it. One carrying
    /// `after_save` applies only once the chat saved the call's draft writes.
    Status(serde_json::Map<String, Value>),
    /// The call wrote nothing: it needs the AI provider catalog, which the chat builds, and
    /// is to be made again with it. `resource_refs` are the ones the flow references.
    NeedsAiProviderCatalog { resource_refs: Vec<String> },
    /// The code a write changed, for the card's diff: of a script in `language`, or of an
    /// app file at `file_path` (`before_file_path` when its language changed).
    CodeDiff {
        before: String,
        after: String,
        language: Option<String>,
        file_path: Option<String>,
        before_file_path: Option<String>,
    },
}

impl ChatContext {
    fn live_editor(&self, kind: &str) -> Option<&LiveEditor> {
        self.live_editors.iter().find(|e| e.kind == kind)
    }

    /// The key a draft named `path` is stored under: an open editor's storage key when
    /// `path` is either of its names, else `path` itself.
    pub fn storage_path<'a>(&'a self, kind: &str, path: &'a str) -> &'a str {
        match self.live_editor(kind) {
            Some(editor)
                if path == editor.storage_path
                    || editor.effective_path.as_deref() == Some(path) =>
            {
                &editor.storage_path
            }
            _ => path,
        }
    }

    /// The path a draft stored at `storage_path` is shown under, and whether an editor is
    /// open on it.
    pub fn display_path<'a>(&'a self, kind: &str, storage_path: &'a str) -> (&'a str, bool) {
        match self.live_editor(kind) {
            Some(editor) if editor.storage_path == storage_path => (
                editor
                    .effective_path
                    .as_deref()
                    .filter(|p| !p.is_empty())
                    .unwrap_or(storage_path),
                true,
            ),
            _ => (storage_path, false),
        }
    }

    pub fn cell(&self, kind: &str, storage_path: &str) -> Option<&Value> {
        self.cells
            .iter()
            .find(|c| c.kind == kind && c.storage_path == storage_path)
            .map(|c| &c.value)
    }
}

impl super::Ctx<'_> {
    /// Patch the chat's card for this call.
    pub(super) fn status(&self, content: impl Into<String>) {
        let mut patch = serde_json::Map::new();
        patch.insert("content".into(), Value::String(content.into()));
        self.effect(Effect::Status(patch));
    }

    /// `status`, applied only once the chat saved the call's draft writes and deletes.
    pub(super) fn status_after_save(&self, content: impl Into<String>, result: Option<&str>) {
        let mut patch = serde_json::Map::new();
        patch.insert("content".into(), Value::String(content.into()));
        if let Some(result) = result {
            patch.insert("result".into(), Value::String(result.to_string()));
        }
        patch.insert("after_save".into(), Value::Bool(true));
        self.effect(Effect::Status(patch));
    }

    pub(super) fn code_diff(&self, before: &str, after: &str, language: Option<&str>) {
        self.effect(Effect::CodeDiff {
            before: before.to_string(),
            after: after.to_string(),
            language: language.map(String::from),
            file_path: None,
            before_file_path: None,
        });
    }

    /// `code_diff` of an app file, whose language its path gives.
    pub(super) fn file_diff(&self, before: &str, after: &str, file_path: &str, before_file_path: &str) {
        self.effect(Effect::CodeDiff {
            before: before.to_string(),
            after: after.to_string(),
            language: None,
            file_path: Some(file_path.to_string()),
            before_file_path: (before_file_path != file_path).then(|| before_file_path.to_string()),
        });
    }

    /// Whether the call comes from an AI session rather than the side-panel chat.
    pub(super) fn in_session(&self) -> bool {
        self.chat.is_some_and(|chat| chat.session)
    }

    /// Hand the card facts the model does not read.
    pub(super) fn facts(&self, facts: Value) {
        if let Value::Object(facts) = facts {
            self.effect(Effect::Facts(facts));
        }
    }

    /// Whether the call is the chat's check before its confirmation card.
    pub(super) fn check_only(&self) -> bool {
        self.chat.is_some_and(|chat| chat.check_only)
    }

    /// Mark the card failed with `message`, and answer it as the tool's result.
    pub(super) fn failed(&self, message: String) -> String {
        let mut patch = serde_json::Map::new();
        patch.insert("content".into(), Value::String(message.clone()));
        patch.insert("error".into(), Value::String(message.clone()));
        self.effect(Effect::Status(patch));
        message
    }

    /// The key a draft named `path` is stored under; `path` itself outside the chat.
    pub(super) fn storage_path<'s>(&'s self, kind: &str, path: &'s str) -> &'s str {
        self.chat.map_or(path, |chat| chat.storage_path(kind, path))
    }

    /// The path a draft stored at `storage_path` is shown under, and whether an editor is
    /// open on it.
    pub(super) fn display_path<'s>(&'s self, kind: &str, storage_path: &'s str) -> (&'s str, bool) {
        self.chat.map_or((storage_path, false), |chat| {
            chat.display_path(kind, storage_path)
        })
    }
}
