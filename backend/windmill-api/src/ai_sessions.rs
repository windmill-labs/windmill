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
//! windmill_ai_sessions/{w_id}/{sha256(email)}/index/{sid}
//! ```
//!
//! The index marker is empty, written last by every push of the session, and is what a
//! listing reads: one object per session, whatever the session holds, its `last_modified`
//! the session's `updated_at`.

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
use windmill_api_workspaces::ai_session_backups::{storage_id, MAX_OBJECT_BYTES, ROOT};
use windmill_common::error::{Error, JsonResult, Result};
use windmill_common::utils::calculate_hash;
use windmill_common::variables::{crypt_from_key_with_suffix, get_workspace_key};
use windmill_object_store::object_store_reexports::{
    ObjectStore, ObjectStoreError, Path as ObjectPath, PutPayload,
};
use windmill_object_store::{build_object_store_client, object_store_error_to_error};

const PUSH_BODY_LIMIT: usize = MAX_OBJECT_BYTES;
/// A pull names at most MAX_PULL_IDS ids of 64 bytes; anything larger is not a pull.
const PULL_BODY_LIMIT: usize = 64 * 1024;
/// A pull answer larger than this hands the remaining ids back as `deferred`.
const PULL_RESPONSE_BUDGET: usize = 32 * 1024 * 1024;
const MAX_HEAD_BYTES: usize = 1024 * 1024;
/// What the cipher adds to a plaintext at most (a block of padding): an object stored at a
/// cap is that much larger than the cap when read back.
const CIPHER_PADDING: usize = 16;
/// The browser bounds an image to a 1568 px edge and re-encodes past 700 KB; this is
/// well above what that produces.
const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_PULL_IDS: usize = 20;
const MAX_PUSH_SESSIONS: usize = 100;
const MAX_REMOVED: usize = 200;
const MAX_CHATS_PER_ENTRY: usize = 100;
const MAX_IMAGES_PER_ENTRY: usize = 500;
const MAX_DELETES_PER_ENTRY: usize = 1000;
const MAX_OPERATIONS_PER_PUSH: usize = 4000;
/// Entries of listing metadata a pull holds per page of a session.
const MAX_LISTED_OBJECTS: usize = 5000;
/// Session markers a listing scans, and the newest sessions it answers with.
const MAX_LIST_SCAN: usize = 50_000;
const LIST_MAX: usize = 500;
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
    /// Names the storage and key the objects are under, for the browser's sync state.
    storage_id: String,
}

impl Backend {
    fn index_prefix(&self) -> ObjectPath {
        ObjectPath::from(format!("{}/index/", self.prefix))
    }

    fn index_key(&self, sid: &str) -> ObjectPath {
        ObjectPath::from(format!("{}/index/{sid}", self.prefix))
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
    ///
    /// `max` is what the listing said the object holds, or the cap of its kind for one read
    /// without a listing: checked before buffering, since whoever holds the bucket's
    /// credentials can put anything at a predictable key, and an object grown past it since
    /// the listing was replaced under this read's feet and is left for the next one.
    async fn get(&self, key: &ObjectPath, max: usize) -> Result<Option<String>> {
        let result = match self.store.get(key).await {
            Ok(result) => result,
            Err(ObjectStoreError::NotFound { .. }) => return Ok(None),
            Err(e) => return Err(object_store_error_to_error(e)),
        };
        if result.meta.size as usize > max {
            tracing::warn!("AI session backup object {key} is larger than expected; left unread");
            return Ok(None);
        }
        let bytes = result.bytes().await.map_err(object_store_error_to_error)?;
        // The objects are JSON and data URLs: a wrong key's output failing UTF-8 tells it
        // apart beyond the cipher's padding check, which a wrong key passes now and then.
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

    /// The entries under `prefix` past `after` in key order, as many as fit `budget` bytes;
    /// `true` when more follow. Every key past `after` is seen and the MAX_LISTED_OBJECTS
    /// smallest kept (a max-heap dropping its largest), since a page is defined by key
    /// order and the store promises none; that cap is what bounds a pull's memory, a
    /// session growing by valid pushes without limit. With `at_least_one`, the first entry
    /// is taken whatever its size, so an answer owed the session makes progress on it (no
    /// object exceeds the push body cap).
    async fn list_within(
        &self,
        prefix: &ObjectPath,
        after: Option<&ObjectPath>,
        budget: usize,
        at_least_one: bool,
    ) -> Result<(Vec<(ObjectPath, usize)>, bool)> {
        let mut kept: std::collections::BinaryHeap<(ObjectPath, usize)> = Default::default();
        let mut dropped = false;
        let mut stream = match after {
            Some(after) => self.store.list_with_offset(Some(prefix), after),
            None => self.store.list(Some(prefix)),
        };
        while let Some(meta) = stream.next().await {
            let meta = meta.map_err(object_store_error_to_error)?;
            kept.push((meta.location, meta.size as usize));
            if kept.len() > MAX_LISTED_OBJECTS {
                kept.pop();
                dropped = true;
            }
        }
        let mut entries = vec![];
        let mut total = 0;
        for (key, size) in kept.into_sorted_vec() {
            if total + size > budget && !(at_least_one && entries.is_empty()) {
                return Ok((entries, true));
            }
            total += size;
            entries.push((key, size));
        }
        Ok((entries, dropped))
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

    /// Deletes as the listing streams, so a prefix of any size costs bounded memory.
    async fn delete_prefix(&self, prefix: &ObjectPath) -> Result<()> {
        self.store
            .list(Some(prefix))
            .map_err(object_store_error_to_error)
            .try_for_each_concurrent(IO_CONCURRENCY, |meta| async move {
                self.delete(&meta.location).await
            })
            .await
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

/// Images travel as base64 data URLs and are stored verbatim, so they serialize back into
/// a pull answer at exactly their stored size; anything else (control characters,
/// quotes) could grow several times under JSON escaping and defeat the pull budget.
fn require_data_url(data_url: &str) -> Result<()> {
    let ok = data_url.len() <= MAX_IMAGE_BYTES
        && data_url
            .strip_prefix("data:")
            .and_then(|rest| rest.split_once(";base64,"))
            .is_some_and(|(mime, payload)| {
                !mime.is_empty()
                    && mime.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'+' | b'-')
                    })
                    && payload
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
            });
    if !ok {
        return Err(Error::BadRequest(
            "an image must be a base64 data URL within the size cap".to_string(),
        ));
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
    let key = get_workspace_key(w_id, db).await?;
    let mc = crypt_from_key_with_suffix(&key, &user);
    let storage_id = storage_id(&resource, &key);
    let prefix = format!("{ROOT}/{w_id}/{user}");
    Ok(Some(Backend { store, mc, prefix, storage_id }))
}

#[derive(Serialize)]
struct SessionListing {
    id: String,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct ListResponse {
    enabled: bool,
    /// The storage answered from; a browser whose sync state names another one starts over.
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_id: Option<String>,
    sessions: Vec<SessionListing>,
    /// The user has more sessions than the answer names.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    truncated: bool,
}

/// A session is listed once a push entry of it landed whole (its marker is written last);
/// a push that failed before that left objects the listing does not name.
async fn list(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<ListResponse> {
    require_plain_user_token(&authed)?;
    let Some(backend) = backend(&authed, &db, &w_id).await? else {
        return Ok(Json(ListResponse {
            enabled: false,
            storage_id: None,
            sessions: vec![],
            truncated: false,
        }));
    };
    let prefix = backend.index_prefix();
    let mut stream = backend.store.list(Some(&prefix));
    // One marker per session, whatever the session holds: the newest LIST_MAX are kept as
    // the scan goes (a min-heap drops the oldest), and the scan itself is bounded.
    let mut newest: std::collections::BinaryHeap<
        std::cmp::Reverse<(chrono::DateTime<chrono::Utc>, String)>,
    > = Default::default();
    let mut scanned = 0;
    let mut truncated = false;
    while let Some(meta) = stream.next().await {
        let meta = meta.map_err(object_store_error_to_error)?;
        scanned += 1;
        if scanned > MAX_LIST_SCAN {
            truncated = true;
            break;
        }
        // `Path` drops the trailing delimiter, so the remainder starts with one.
        let Some(rel) = meta.location.as_ref().strip_prefix(prefix.as_ref()) else {
            continue;
        };
        let sid = rel.trim_start_matches('/');
        if sid.is_empty() || sid.contains('/') {
            continue;
        }
        newest.push(std::cmp::Reverse((meta.last_modified, sid.to_string())));
        if newest.len() > LIST_MAX {
            newest.pop();
            truncated = true;
        }
    }
    let mut sessions: Vec<SessionListing> = newest
        .into_iter()
        .map(|std::cmp::Reverse((updated_at, id))| SessionListing { id, updated_at })
        .collect();
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(Json(ListResponse {
        enabled: true,
        storage_id: Some(backend.storage_id.clone()),
        sessions,
        truncated,
    }))
}

#[derive(Deserialize)]
struct PullRequest {
    ids: Vec<String>,
    /// Picks the session an earlier answer cut up from where it stopped; `ids` then names
    /// that session alone.
    #[serde(default)]
    resume: Option<PullCursor>,
}

/// Where a pull of a session that outgrew one answer picks up: the last key the earlier
/// answer carried, in the session's prefix or, once that one is done, in its images prefix.
#[derive(Serialize, Deserialize, Clone)]
struct PullCursor {
    id: String,
    images: bool,
    after: String,
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
    /// The session did not fit this answer whole: the rest follows a pull with this cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<PullCursor>,
}

#[derive(Serialize)]
struct PullResponse {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_id: Option<String>,
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
    resume: Option<&PullCursor>,
) -> Result<PullStep> {
    let Some(head) = backend
        .get(&backend.head_key(sid), MAX_HEAD_BYTES + CIPHER_PADDING)
        .await?
    else {
        return Ok(PullStep::Absent);
    };
    let session_prefix = backend.session_prefix(sid);
    let images_prefix = backend.images_prefix(sid);
    let mut size = head.len();
    let mut fetch = vec![];
    let mut artifacts_key = None;
    let mut next = None;
    // Sizes come from the listings, and the listings stop at the budget, so nothing is read
    // past it even for the first session of the answer. One that outgrew it (chats
    // accumulate over pushes) comes back in pages, in listing order, each answer naming
    // where the next picks up; the browser imports nothing before the last page.
    let in_images = resume.is_some_and(|c| c.images);
    if !in_images {
        let after = resume.map(|c| ObjectPath::from(c.after.as_str()));
        let (entries, cut) = backend
            .list_within(
                &session_prefix,
                after.as_ref(),
                budget.saturating_sub(size),
                first,
            )
            .await?;
        if cut && !first {
            return Ok(PullStep::Deferred);
        }
        if cut {
            next = entries.last().map(|(key, _)| PullCursor {
                id: sid.to_string(),
                images: false,
                after: key.to_string(),
            });
        }
        for (key, bytes) in entries {
            let rel = key
                .as_ref()
                .strip_prefix(session_prefix.as_ref())
                .unwrap_or_default()
                .trim_start_matches('/');
            if rel == "artifacts.json" {
                artifacts_key = Some((key, bytes));
                size += bytes;
            } else if let Some(cid) = rel
                .strip_prefix("chats/")
                .and_then(|f| f.strip_suffix(".json"))
            {
                fetch.push((cid.to_string(), key, bytes));
                size += bytes;
            }
        }
    }
    let chats: Vec<PulledChat> = futures::stream::iter(fetch)
        .map(|(cid, key, bytes)| async move {
            let record = backend.get(&key, bytes).await?;
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
        Some((key, bytes)) => match backend.get(&key, bytes).await? {
            Some(text) => Some(raw("artifacts", text)?),
            None => None,
        },
        None => None,
    };
    let mut image_keys: Vec<(String, String, ObjectPath, usize)> = vec![];
    if next.is_none() {
        let after = resume
            .filter(|c| c.images)
            .map(|c| ObjectPath::from(c.after.as_str()));
        let (entries, cut) = backend
            .list_within(
                &images_prefix,
                after.as_ref(),
                budget.saturating_sub(size),
                first,
            )
            .await?;
        if cut {
            // An answer with no room for a single image names where it stood, so the pull
            // owed the session alone picks it up there.
            next = Some(PullCursor {
                id: sid.to_string(),
                images: true,
                after: entries
                    .last()
                    .map(|(key, _)| key.to_string())
                    .or_else(|| after.map(|a| a.to_string()))
                    .unwrap_or_default(),
            });
        }
        for (key, bytes) in entries {
            let Some(rel) = key.as_ref().strip_prefix(images_prefix.as_ref()) else {
                continue;
            };
            let Some((cid, iid)) = rel.trim_start_matches('/').split_once('/') else {
                continue;
            };
            size += bytes;
            image_keys.push((cid.to_string(), iid.to_string(), key, bytes));
        }
    }
    let images: Vec<ImageObject> = futures::stream::iter(image_keys)
        .map(|(chat_id, id, key, bytes)| async move {
            let data_url = backend.get(&key, bytes).await?;
            Ok::<_, Error>(data_url.map(|data_url| ImageObject { chat_id, id, data_url }))
        })
        .buffer_unordered(IO_CONCURRENCY)
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .flatten()
        .collect();
    Ok(PullStep::Fetched(
        PulledSession {
            id: sid.to_string(),
            head: raw("head", head)?,
            chats,
            images,
            artifacts,
            next,
        },
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
    if let Some(cursor) = &req.resume {
        if req.ids.len() != 1 || req.ids[0] != cursor.id || cursor.after.len() > 1024 {
            return Err(Error::BadRequest(
                "a resumed pull names the resumed session alone".to_string(),
            ));
        }
    }
    let Some(backend) = backend(&authed, &db, &w_id).await? else {
        return Ok(Json(PullResponse {
            enabled: false,
            storage_id: None,
            sessions: vec![],
            deferred: vec![],
        }));
    };
    let mut sessions = vec![];
    let mut deferred = vec![];
    let mut budget = PULL_RESPONSE_BUDGET;
    for sid in req.ids {
        let resume = req.resume.as_ref().filter(|c| c.id == sid);
        match pull_session(&backend, &sid, budget, sessions.is_empty(), resume).await? {
            PullStep::Absent => {}
            PullStep::Deferred => deferred.push(sid),
            PullStep::Fetched(session, size) => {
                budget = budget.saturating_sub(size);
                sessions.push(session);
            }
        }
    }
    Ok(Json(PullResponse {
        enabled: true,
        storage_id: Some(backend.storage_id),
        sessions,
        deferred,
    }))
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
    /// More parts of the session follow, in this push or a later one: the session is not
    /// listed on this one.
    #[serde(default)]
    partial: bool,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_id: Option<String>,
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
            require_data_url(&i.data_url)?;
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
    // Last, and by the last part only, so a session is listed once its whole entry landed.
    if !s.partial {
        backend
            .store
            .put(&backend.index_key(&s.id), PutPayload::new())
            .await
            .map_err(object_store_error_to_error)?;
    }
    Ok(written)
}

/// The marker goes first so a removal cut short leaves nothing listed, then the head so
/// nothing pulls either.
async fn remove_session(backend: &Backend, sid: &str) -> Result<()> {
    backend.delete(&backend.index_key(sid)).await?;
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
        return Ok(Json(PushResponse {
            enabled: false,
            storage_id: None,
            results: vec![],
        }));
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
    // A session split into several entries has its head on the last: once one part failed,
    // the later ones are not written, or the head would list a session missing a part.
    let mut failed: std::collections::HashSet<&str> = Default::default();
    for s in &req.sessions {
        let error = if failed.contains(s.id.as_str()) {
            Some("an earlier part of this session in the push failed".to_string())
        } else {
            match push_session(&backend, s).await {
                Ok(n) => {
                    written += n;
                    None
                }
                Err(e) => {
                    tracing::warn!("AI session backup push failed for {} in {w_id}: {e}", s.id);
                    failed.insert(&s.id);
                    Some(e.to_string())
                }
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
    Ok(Json(PushResponse {
        enabled: true,
        storage_id: Some(backend.storage_id),
        results,
    }))
}
