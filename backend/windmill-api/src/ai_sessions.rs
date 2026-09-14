//! Lazily replicated backups of the browser's AI sessions in the workspace's object storage.
//!
//! The browser keeps the sessions in IndexedDB and pushes changed pieces here in batches; an
//! empty browser restores from what was pushed. The server owns the key layout, keeps the
//! caller's own prefix the only one it can reach, and encrypts every object with the
//! workspace key so bucket credentials do not read transcripts:
//!
//! ```text
//! windmill_ai_sessions/{w_id}/{sha256(email)}/sessions/{sid}/head.json
//! windmill_ai_sessions/{w_id}/{sha256(email)}/sessions/{sid}/chats/{cid}.json
//! windmill_ai_sessions/{w_id}/{sha256(email)}/sessions/{sid}/artifacts.json
//! windmill_ai_sessions/{w_id}/{sha256(email)}/images/{sid}/{cid}/{iid}
//! ```
//!
//! Images sit outside the `sessions/` prefix so that one listing of it enumerates a user's
//! sessions at a few keys per session.

use crate::db::{ApiAuthed, DB};
use axum::{
    extract::{DefaultBodyLimit, Path},
    routing::{get, post},
    Extension, Json, Router,
};
use futures::{StreamExt, TryStreamExt};
use magic_crypt::{MagicCrypt256, MagicCryptTrait};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::sync::Arc;
use windmill_api_auth::is_effectively_unscoped;
use windmill_common::error::{Error, JsonResult, Result};
use windmill_common::utils::calculate_hash;
use windmill_common::variables::build_crypt_with_key_suffix;
use windmill_object_store::object_store_reexports::{
    ObjectStore, ObjectStoreError, Path as ObjectPath, PutPayload,
};
use windmill_object_store::{build_object_store_client, object_store_error_to_error};

const ROOT: &str = "windmill_ai_sessions";
const PUSH_BODY_LIMIT: usize = 32 * 1024 * 1024;
/// A pull names at most MAX_PULL_IDS ids of 64 bytes; anything larger is not a pull.
const PULL_BODY_LIMIT: usize = 64 * 1024;
/// A pull answer larger than this hands the remaining ids back as `deferred`.
const PULL_RESPONSE_BUDGET: usize = 32 * 1024 * 1024;
const MAX_HEAD_BYTES: usize = 1024 * 1024;
const MAX_PULL_IDS: usize = 20;
const MAX_PUSH_SESSIONS: usize = 100;
const MAX_REMOVED: usize = 200;
const MAX_CHATS_PER_ENTRY: usize = 100;
const MAX_IMAGES_PER_ENTRY: usize = 500;
const MAX_DELETES_PER_ENTRY: usize = 1000;
const MAX_OPERATIONS_PER_PUSH: usize = 4000;
const IO_CONCURRENCY: usize = 8;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list))
        .route(
            "/pull",
            post(pull).layer(DefaultBodyLimit::max(PULL_BODY_LIMIT)),
        )
        .route(
            "/push",
            post(push).layer(DefaultBodyLimit::max(PUSH_BODY_LIMIT)),
        )
}

/// The user's prefix in the workspace storage, plus what reads and writes it.
struct Backend {
    store: Arc<dyn ObjectStore>,
    mc: MagicCrypt256,
    prefix: String,
}

impl Backend {
    fn sessions_prefix(&self) -> ObjectPath {
        ObjectPath::from(format!("{}/sessions/", self.prefix))
    }

    fn session_prefix(&self, sid: &str) -> ObjectPath {
        ObjectPath::from(format!("{}/sessions/{sid}/", self.prefix))
    }

    fn head_key(&self, sid: &str) -> ObjectPath {
        ObjectPath::from(format!("{}/sessions/{sid}/head.json", self.prefix))
    }

    fn chat_key(&self, sid: &str, cid: &str) -> ObjectPath {
        ObjectPath::from(format!("{}/sessions/{sid}/chats/{cid}.json", self.prefix))
    }

    fn artifacts_key(&self, sid: &str) -> ObjectPath {
        ObjectPath::from(format!("{}/sessions/{sid}/artifacts.json", self.prefix))
    }

    fn images_prefix(&self, sid: &str) -> ObjectPath {
        ObjectPath::from(format!("{}/images/{sid}/", self.prefix))
    }

    fn chat_images_prefix(&self, sid: &str, cid: &str) -> ObjectPath {
        ObjectPath::from(format!("{}/images/{sid}/{cid}/", self.prefix))
    }

    fn image_key(&self, sid: &str, cid: &str, iid: &str) -> ObjectPath {
        ObjectPath::from(format!("{}/images/{sid}/{cid}/{iid}", self.prefix))
    }

    fn seal(&self, plaintext: &[u8]) -> Vec<u8> {
        self.mc.encrypt_bytes_to_bytes(plaintext)
    }

    /// Bytes written.
    async fn put_sealed(&self, key: &ObjectPath, ciphertext: Vec<u8>) -> Result<usize> {
        let written = ciphertext.len();
        self.store
            .put(key, PutPayload::from(ciphertext))
            .await
            .map_err(object_store_error_to_error)?;
        Ok(written)
    }

    async fn put(&self, key: &ObjectPath, plaintext: &[u8]) -> Result<usize> {
        self.put_sealed(key, self.seal(plaintext)).await
    }

    /// `None` when the object does not exist, and also when it does not decrypt under this
    /// user's key: an object written under another user's or workspace's key is nobody's
    /// to read, and one such object must not take the rest of the session down with it.
    async fn get(&self, key: &ObjectPath) -> Result<Option<String>> {
        let result = match self.store.get(key).await {
            Ok(result) => result,
            Err(ObjectStoreError::NotFound { .. }) => return Ok(None),
            Err(e) => return Err(object_store_error_to_error(e)),
        };
        let bytes = result.bytes().await.map_err(object_store_error_to_error)?;
        let text = self
            .mc
            .decrypt_bytes_to_bytes(&bytes)
            .ok()
            .and_then(|plaintext| String::from_utf8(plaintext).ok());
        if text.is_none() {
            tracing::warn!("AI session backup object {key} does not decrypt for its reader");
        }
        Ok(text)
    }

    async fn delete(&self, key: &ObjectPath) -> Result<()> {
        match self.store.delete(key).await {
            Ok(()) | Err(ObjectStoreError::NotFound { .. }) => Ok(()),
            Err(e) => Err(object_store_error_to_error(e)),
        }
    }

    async fn list_keys(&self, prefix: &ObjectPath) -> Result<Vec<ObjectPath>> {
        Ok(self
            .list_entries(prefix)
            .await?
            .into_iter()
            .map(|(key, _)| key)
            .collect())
    }

    /// Keys under the prefix with their stored size.
    async fn list_entries(&self, prefix: &ObjectPath) -> Result<Vec<(ObjectPath, usize)>> {
        self.store
            .list(Some(prefix))
            .map_ok(|meta| (meta.location, meta.size as usize))
            .try_collect()
            .await
            .map_err(object_store_error_to_error)
    }

    /// Bytes written. Sealed up front so every stream item is owned: an item borrowing
    /// from the request makes the future higher-ranked over that lifetime, which the
    /// handler's `Send` bound cannot prove.
    async fn put_all(&self, puts: Vec<(ObjectPath, Vec<u8>)>) -> Result<usize> {
        futures::stream::iter(puts)
            .map(|(key, ciphertext)| async move { self.put_sealed(&key, ciphertext).await })
            .buffer_unordered(IO_CONCURRENCY)
            .try_fold(0, |acc, n| async move { Ok::<_, Error>(acc + n) })
            .await
    }

    async fn delete_all(&self, keys: Vec<ObjectPath>) -> Result<()> {
        futures::stream::iter(keys)
            .map(|key| async move { self.delete(&key).await })
            .buffer_unordered(IO_CONCURRENCY)
            .try_collect::<Vec<()>>()
            .await?;
        Ok(())
    }

    async fn delete_prefix(&self, prefix: &ObjectPath) -> Result<()> {
        let keys = self.list_keys(prefix).await?;
        self.delete_all(keys).await
    }
}

/// Backups are the user's own browser state and nothing else may reach them: a job token
/// may carry an `on_behalf_of` identity, and every scoped token (guest, embed, app policy,
/// MCP) is minted for something narrower than the user's whole assistant history.
fn require_plain_user_token(authed: &ApiAuthed) -> Result<()> {
    if authed.job_id.is_some() || !is_effectively_unscoped(authed.scopes.as_deref()) {
        return Err(Error::PermissionDenied(
            "AI session backups are only reachable with an unscoped user token".to_string(),
        ));
    }
    Ok(())
}

/// Every key is assembled server-side from ids the browser mints (`createLongHash` and
/// `randomUUID` forms), so anything outside this alphabet is a forged id, not a real one.
fn require_valid_id(kind: &str, id: &str) -> Result<()> {
    let ok = !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if !ok {
        return Err(Error::BadRequest(format!("invalid {kind} id: {id:?}")));
    }
    Ok(())
}

fn require_json_object(kind: &str, raw: &RawValue, max_bytes: usize) -> Result<()> {
    let text = raw.get();
    if !text.trim_start().starts_with('{') {
        return Err(Error::BadRequest(format!("{kind} must be a JSON object")));
    }
    if text.len() > max_bytes {
        return Err(Error::BadRequest(format!(
            "{kind} exceeds {max_bytes} bytes"
        )));
    }
    Ok(())
}

/// `None` when the workspace has nowhere to keep backups: no primary storage configured, or
/// the admin switched them off. Both read as `enabled: false` so the browser stops trying.
async fn backend(authed: &ApiAuthed, db: &DB, w_id: &str) -> Result<Option<Backend>> {
    let disabled = sqlx::query_scalar::<_, Option<bool>>(
        "SELECT (ai_config->>'sessions_storage_disabled')::bool FROM workspace_settings WHERE workspace_id = $1",
    )
    .bind(w_id)
    .fetch_optional(db)
    .await?
    .flatten()
    .unwrap_or(false);
    if disabled {
        return Ok(None);
    }
    let (_, resource) =
        crate::job_helpers_oss::get_workspace_s3_resource(authed, db, None, w_id, None).await?;
    let Some(resource) = resource else {
        return Ok(None);
    };
    let store = build_object_store_client(&resource).await?;
    let user = calculate_hash(&authed.email);
    // Keyed per user, not per workspace: anyone who can write the bucket could otherwise copy
    // another member's ciphertext under their own prefix and have `pull` decrypt it for them.
    let mc = build_crypt_with_key_suffix(db, w_id, &user).await?;
    let prefix = format!("{ROOT}/{w_id}/{user}");
    Ok(Some(Backend { store, mc, prefix }))
}

#[derive(Serialize)]
struct SessionListing {
    id: String,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct ListResponse {
    enabled: bool,
    sessions: Vec<SessionListing>,
}

/// A session is listed once its head landed; a push that failed between its chats and its
/// head left objects nothing points at, and pull skips those the same way.
async fn list(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<ListResponse> {
    require_plain_user_token(&authed)?;
    let Some(backend) = backend(&authed, &db, &w_id).await? else {
        return Ok(Json(ListResponse { enabled: false, sessions: vec![] }));
    };
    let prefix = backend.sessions_prefix();
    let mut updated: std::collections::HashMap<String, chrono::DateTime<chrono::Utc>> =
        Default::default();
    let mut with_head = std::collections::HashSet::new();
    let mut stream = backend.store.list(Some(&prefix));
    while let Some(meta) = stream.next().await {
        let meta = meta.map_err(object_store_error_to_error)?;
        // `Path` drops the trailing delimiter, so the remainder starts with one.
        let Some(rel) = meta.location.as_ref().strip_prefix(prefix.as_ref()) else {
            continue;
        };
        let Some((sid, rest)) = rel.trim_start_matches('/').split_once('/') else {
            continue;
        };
        if rest == "head.json" {
            with_head.insert(sid.to_string());
        }
        let entry = updated.entry(sid.to_string()).or_insert(meta.last_modified);
        if meta.last_modified > *entry {
            *entry = meta.last_modified;
        }
    }
    let mut sessions: Vec<SessionListing> = updated
        .into_iter()
        .filter(|(sid, _)| with_head.contains(sid))
        .map(|(id, updated_at)| SessionListing { id, updated_at })
        .collect();
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(Json(ListResponse { enabled: true, sessions }))
}

#[derive(Deserialize)]
struct PullRequest {
    ids: Vec<String>,
}

#[derive(Serialize)]
struct PulledChat {
    id: String,
    record: Box<RawValue>,
}

#[derive(Serialize, Deserialize)]
struct ImageObject {
    chat_id: String,
    id: String,
    data_url: String,
}

#[derive(Serialize)]
struct PulledSession {
    id: String,
    head: Box<RawValue>,
    chats: Vec<PulledChat>,
    images: Vec<ImageObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifacts: Option<Box<RawValue>>,
}

#[derive(Serialize)]
struct PullResponse {
    enabled: bool,
    sessions: Vec<PulledSession>,
    /// Ids that did not fit the response budget; ask for them again.
    deferred: Vec<String>,
}

fn raw(kind: &str, text: String) -> Result<Box<RawValue>> {
    RawValue::from_string(text)
        .map_err(|e| Error::internal_err(format!("stored {kind} is not JSON: {e}")))
}

enum PullStep {
    Absent,
    Deferred,
    Fetched(PulledSession, usize),
}

/// Fetch one session: its head, then every chat and artifact object under its prefix, and
/// as many of its images as the budget allows (a missing image hydrates to a placeholder
/// in the browser). Sizes come from the listings, so a session that would not fit is
/// deferred before anything of it is read, unless it is the first of the response, which
/// must carry something. `Absent` when it has no head.
async fn pull_session(
    backend: &Backend,
    sid: &str,
    budget: usize,
    first: bool,
) -> Result<PullStep> {
    let Some(head) = backend.get(&backend.head_key(sid)).await? else {
        return Ok(PullStep::Absent);
    };
    let session_prefix = backend.session_prefix(sid);
    let mut chat_keys = vec![];
    let mut artifacts_key = None;
    let mut core = head.len();
    for (key, bytes) in backend.list_entries(&session_prefix).await? {
        let rel = key
            .as_ref()
            .strip_prefix(session_prefix.as_ref())
            .unwrap_or_default()
            .trim_start_matches('/');
        if rel == "artifacts.json" {
            artifacts_key = Some((key, bytes));
            core += bytes;
        } else if let Some(cid) = rel
            .strip_prefix("chats/")
            .and_then(|f| f.strip_suffix(".json"))
        {
            chat_keys.push((cid.to_string(), key, bytes));
            core += bytes;
        }
    }
    if !first && core > budget {
        return Ok(PullStep::Deferred);
    }
    // Sizes come from the listings, so nothing is read past the budget even for the first
    // session of the answer: one that outgrew it (chats accumulate over pushes) comes back
    // with the chats that fit, in listing order, rather than being read whole.
    let mut size = head.len();
    let mut fetch = vec![];
    for (cid, key, bytes) in chat_keys {
        if size + bytes > budget {
            tracing::warn!(
                "AI session backup {sid} chat {cid} is beyond the pull budget; left out"
            );
            continue;
        }
        size += bytes;
        fetch.push((cid, key));
    }
    let chats: Vec<PulledChat> = futures::stream::iter(fetch)
        .map(|(cid, key)| async move {
            let record = backend.get(&key).await?;
            Ok::<_, Error>(record.map(|r| (cid, r)))
        })
        .buffer_unordered(IO_CONCURRENCY)
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .flatten()
        .map(|(cid, record)| Ok(PulledChat { id: cid, record: raw("chat", record)? }))
        .collect::<Result<_>>()?;
    let artifacts = match artifacts_key {
        Some((key, bytes)) if size + bytes <= budget => {
            size += bytes;
            match backend.get(&key).await? {
                Some(text) => Some(raw("artifacts", text)?),
                None => None,
            }
        }
        _ => None,
    };
    let images_prefix = backend.images_prefix(sid);
    let mut image_keys: Vec<(String, String, ObjectPath)> = vec![];
    for (key, bytes) in backend.list_entries(&images_prefix).await? {
        let Some(rel) = key.as_ref().strip_prefix(images_prefix.as_ref()) else {
            continue;
        };
        let Some((cid, iid)) = rel.trim_start_matches('/').split_once('/') else {
            continue;
        };
        if size + bytes > budget {
            continue;
        }
        size += bytes;
        image_keys.push((cid.to_string(), iid.to_string(), key));
    }
    let images: Vec<ImageObject> = futures::stream::iter(image_keys)
        .map(|(chat_id, id, key)| async move {
            let data_url = backend.get(&key).await?;
            Ok::<_, Error>(data_url.map(|data_url| ImageObject { chat_id, id, data_url }))
        })
        .buffer_unordered(IO_CONCURRENCY)
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .flatten()
        .collect();
    Ok(PullStep::Fetched(
        PulledSession { id: sid.to_string(), head: raw("head", head)?, chats, images, artifacts },
        size,
    ))
}

async fn pull(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<PullRequest>,
) -> JsonResult<PullResponse> {
    require_plain_user_token(&authed)?;
    if req.ids.len() > MAX_PULL_IDS {
        return Err(Error::BadRequest(format!(
            "at most {MAX_PULL_IDS} sessions per pull"
        )));
    }
    for id in &req.ids {
        require_valid_id("session", id)?;
    }
    let Some(backend) = backend(&authed, &db, &w_id).await? else {
        return Ok(Json(PullResponse {
            enabled: false,
            sessions: vec![],
            deferred: vec![],
        }));
    };
    let mut sessions = vec![];
    let mut deferred = vec![];
    let mut budget = PULL_RESPONSE_BUDGET;
    for sid in req.ids {
        match pull_session(&backend, &sid, budget, sessions.is_empty()).await? {
            PullStep::Absent => {}
            PullStep::Deferred => deferred.push(sid),
            PullStep::Fetched(session, size) => {
                budget = budget.saturating_sub(size);
                sessions.push(session);
            }
        }
    }
    Ok(Json(PullResponse { enabled: true, sessions, deferred }))
}

#[derive(Deserialize)]
struct PushedChat {
    id: String,
    record: Box<RawValue>,
}

#[derive(Deserialize)]
struct ImageRef {
    chat_id: String,
    id: String,
}

#[derive(Deserialize)]
struct PushedSession {
    id: String,
    #[serde(default)]
    head: Option<Box<RawValue>>,
    #[serde(default)]
    chats: Vec<PushedChat>,
    #[serde(default)]
    images: Vec<ImageObject>,
    #[serde(default)]
    artifacts: Option<Box<RawValue>>,
    #[serde(default)]
    delete_chats: Vec<String>,
    #[serde(default)]
    delete_images: Vec<ImageRef>,
}

#[derive(Deserialize)]
struct PushRequest {
    /// The email the browser believes it is acting for. An in-place account switch can
    /// leave a flush prepared for the previous user; the server refuses it rather than
    /// filing that user's sessions under the caller's prefix.
    owner: String,
    #[serde(default)]
    sessions: Vec<PushedSession>,
    #[serde(default)]
    removed: Vec<String>,
}

#[derive(Serialize)]
struct PushResult {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct PushResponse {
    enabled: bool,
    results: Vec<PushResult>,
}

fn validate_push(req: &PushRequest) -> Result<()> {
    if req.sessions.len() > MAX_PUSH_SESSIONS || req.removed.len() > MAX_REMOVED {
        return Err(Error::BadRequest(
            "too many sessions in one push".to_string(),
        ));
    }
    // Every nested entry costs an object-store call (a deleted chat two), so the lists are
    // bounded per entry and across the request; the browser sends far fewer.
    let mut operations = req.removed.len();
    for s in &req.sessions {
        if s.chats.len() > MAX_CHATS_PER_ENTRY
            || s.images.len() > MAX_IMAGES_PER_ENTRY
            || s.delete_chats.len() > MAX_DELETES_PER_ENTRY
            || s.delete_images.len() > MAX_DELETES_PER_ENTRY
        {
            return Err(Error::BadRequest(format!(
                "too many pieces for session {} in one push",
                s.id
            )));
        }
        operations += s.chats.len() + s.images.len() + s.delete_chats.len() + s.delete_images.len();
    }
    if operations > MAX_OPERATIONS_PER_PUSH {
        return Err(Error::BadRequest("too many pieces in one push".to_string()));
    }
    for sid in &req.removed {
        require_valid_id("session", sid)?;
    }
    for s in &req.sessions {
        require_valid_id("session", &s.id)?;
        if let Some(head) = &s.head {
            require_json_object("head", head, MAX_HEAD_BYTES)?;
        }
        for c in &s.chats {
            require_valid_id("chat", &c.id)?;
            require_json_object("chat record", &c.record, PUSH_BODY_LIMIT)?;
        }
        if let Some(a) = &s.artifacts {
            require_json_object("artifacts", a, PUSH_BODY_LIMIT)?;
        }
        for i in &s.images {
            require_valid_id("chat", &i.chat_id)?;
            require_valid_id("image", &i.id)?;
        }
        for c in &s.delete_chats {
            require_valid_id("chat", c)?;
        }
        for i in &s.delete_images {
            require_valid_id("chat", &i.chat_id)?;
            require_valid_id("image", &i.id)?;
        }
    }
    Ok(())
}

fn push_payload_bytes(req: &PushRequest) -> usize {
    req.sessions
        .iter()
        .map(|s| {
            s.head.as_ref().map_or(0, |h| h.get().len())
                + s.artifacts.as_ref().map_or(0, |a| a.get().len())
                + s.chats.iter().map(|c| c.record.get().len()).sum::<usize>()
                + s.images.iter().map(|i| i.data_url.len()).sum::<usize>()
        })
        .sum()
}

/// Objects land before the head that makes them visible, and deletes run last, so a push
/// cut short never leaves a listed session pointing at chats that are not there.
async fn push_session(backend: &Backend, s: &PushedSession) -> Result<usize> {
    let mut written = 0;
    written += backend
        .put_all(
            s.images
                .iter()
                .map(|img| {
                    (
                        backend.image_key(&s.id, &img.chat_id, &img.id),
                        backend.seal(img.data_url.as_bytes()),
                    )
                })
                .collect(),
        )
        .await?;
    written += backend
        .put_all(
            s.chats
                .iter()
                .map(|c| {
                    (
                        backend.chat_key(&s.id, &c.id),
                        backend.seal(c.record.get().as_bytes()),
                    )
                })
                .collect(),
        )
        .await?;
    if let Some(a) = &s.artifacts {
        written += backend
            .put(&backend.artifacts_key(&s.id), a.get().as_bytes())
            .await?;
    }
    if let Some(h) = &s.head {
        written += backend
            .put(&backend.head_key(&s.id), h.get().as_bytes())
            .await?;
    }
    for cid in &s.delete_chats {
        backend.delete(&backend.chat_key(&s.id, cid)).await?;
        backend
            .delete_prefix(&backend.chat_images_prefix(&s.id, cid))
            .await?;
    }
    backend
        .delete_all(
            s.delete_images
                .iter()
                .map(|i| backend.image_key(&s.id, &i.chat_id, &i.id))
                .collect(),
        )
        .await?;
    Ok(written)
}

/// The head goes first so a removal cut short leaves nothing listed.
async fn remove_session(backend: &Backend, sid: &str) -> Result<()> {
    backend.delete(&backend.head_key(sid)).await?;
    backend.delete_prefix(&backend.session_prefix(sid)).await?;
    backend.delete_prefix(&backend.images_prefix(sid)).await
}

async fn push(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<PushRequest>,
) -> JsonResult<PushResponse> {
    require_plain_user_token(&authed)?;
    if req.owner != authed.email {
        return Err(Error::Generic(
            http::StatusCode::CONFLICT,
            "this push was prepared for another user".to_string(),
        ));
    }
    validate_push(&req)?;
    let Some(backend) = backend(&authed, &db, &w_id).await? else {
        return Ok(Json(PushResponse { enabled: false, results: vec![] }));
    };
    #[cfg(not(feature = "enterprise"))]
    {
        let remaining =
            crate::job_helpers_oss::ce_storage_quota_remaining(&db, &w_id, None).await?;
        if push_payload_bytes(&req) as i64 > remaining {
            return Err(Error::QuotaExceeded(
                "the workspace storage quota leaves no room for this AI session backup".to_string(),
            ));
        }
    }
    #[cfg(feature = "enterprise")]
    let _ = push_payload_bytes(&req);

    let mut results = Vec::with_capacity(req.sessions.len() + req.removed.len());
    let mut written: usize = 0;
    for s in &req.sessions {
        let error = match push_session(&backend, s).await {
            Ok(n) => {
                written += n;
                None
            }
            Err(e) => {
                tracing::warn!("AI session backup push failed for {} in {w_id}: {e}", s.id);
                Some(e.to_string())
            }
        };
        results.push(PushResult { id: s.id.clone(), error });
    }
    for sid in &req.removed {
        let error = remove_session(&backend, sid).await.err().map(|e| {
            tracing::warn!("AI session backup removal failed for {sid} in {w_id}: {e}");
            e.to_string()
        });
        results.push(PushResult { id: sid.clone(), error });
    }
    // Overwrites and deletes make this an over-count; the periodic recount the quota check
    // schedules once usage is stale settles it.
    #[cfg(not(feature = "enterprise"))]
    if written > 0 {
        crate::job_helpers_oss::bump_storage_usage(
            &db,
            &w_id,
            windmill_object_store::DEFAULT_STORAGE,
            written as i64,
        )
        .await;
    }
    #[cfg(feature = "enterprise")]
    let _ = written;
    Ok(Json(PushResponse { enabled: true, results }))
}
