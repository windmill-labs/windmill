//! Lazily replicated backups of the browser's AI sessions in the workspace's object storage.
//!
//! The browser keeps the sessions in IndexedDB and pushes changed pieces here in batches; an
//! empty browser restores from what was pushed. The server owns the key layout, keeps the
//! caller's own prefix the only one it can reach, and encrypts every object with the
//! workspace key so bucket credentials do not read transcripts:
//!
//! ```text
//! windmill_ai_sessions/{w_id}/g{generation}/{sha256(email)}/sessions/{sid}/head.json
//! windmill_ai_sessions/{w_id}/g{generation}/{sha256(email)}/sessions/{sid}/chats/{cid}.json
//! windmill_ai_sessions/{w_id}/g{generation}/{sha256(email)}/sessions/{sid}/artifacts.json
//! windmill_ai_sessions/{w_id}/g{generation}/{sha256(email)}/images/{sid}/{cid}/{iid}
//! windmill_ai_sessions/{w_id}/g{generation}/{sha256(email)}/index/{sid}/{epoch}
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
use windmill_api_workspaces::ai_session_backups::{
    generation_prefix, primary_store, storage_id, MAX_OBJECT_BYTES,
};
use windmill_api_workspaces::workspaces::sessions_retention_days;
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
/// Sessions the retention sweep deletes per workspace and pass at most; the rest wait for
/// the next pass.
const SWEEP_MAX_PER_WORKSPACE: usize = 1000;
/// Session-level advisory lock of the retention sweep: one server at a time runs it.
const SWEEP_LOCK_ID: i64 = 0x5745_4550_4149;
/// The name of the sweep's record next to a session's markers (see `Backend::sweep_key`).
const SWEEP_RECORD: &str = "sweep";
/// The name of a split push's token next to a session's markers (see `Backend::push_key`).
const PUSH_TOKEN: &str = "push";

/// A marker modified before this is past a retention of `days`.
fn retention_cutoff(days: u32) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now() - chrono::Duration::days(i64::from(days))
}

/// What a key under the `index/` prefix is.
enum IndexEntry {
    /// The marker that lists the session, named by its epoch.
    Marker(u32),
    /// The retention sweep's record (see `Backend::sweep_key`).
    Sweep,
    /// The token of a push split over parts (see `Backend::push_key`).
    Push,
}

/// The session a key under the `index/` prefix belongs to, and what the key is.
fn index_entry<'a>(index: &ObjectPath, key: &'a ObjectPath) -> Option<(&'a str, IndexEntry)> {
    // `Path` drops the trailing delimiter, so the remainder starts with one.
    let rel = key.as_ref().strip_prefix(index.as_ref())?;
    let (sid, name) = rel.trim_start_matches('/').split_once('/')?;
    if sid.is_empty() {
        return None;
    }
    let entry = match name {
        SWEEP_RECORD => IndexEntry::Sweep,
        PUSH_TOKEN => IndexEntry::Push,
        epoch => IndexEntry::Marker(epoch.parse().ok()?),
    };
    Some((sid, entry))
}

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

/// What reading an object yields. `Gone`: not there (deleted since the listing, or never
/// pushed). `Grown`: larger than expected, so replaced since the listing (or planted), and
/// left unread; a pull answers with a page ending before it rather than without it.
/// `Foreign`: it does not decrypt for this user (written under another user's or
/// workspace's key), and must not take the rest of the session down with it.
enum Read {
    Text(String),
    Gone,
    Grown,
    Foreign,
}

/// The user's prefix in the workspace storage, plus what reads and writes it.
struct Backend {
    store: Arc<dyn ObjectStore>,
    mc: MagicCrypt256,
    prefix: String,
    /// Name the storage and the generation the objects are under, for the browser's sync
    /// state: a row recorded against another storage or generation is stale, a removal is
    /// owed to the storage alone (a rotation deleted the older generation's copy anyway).
    storage_id: String,
    generation: i64,
    /// `ai_config.sessions_retention_days`: a session whose marker is older is not listed,
    /// whether or not the sweep has deleted it yet.
    retention_days: Option<u32>,
}

impl Backend {
    fn index_prefix(&self) -> ObjectPath {
        ObjectPath::from(format!("{}/index/", self.prefix))
    }

    /// The moment a marker's modification time must reach to count as live, under the
    /// workspace's retention; `None` without one.
    fn retention_cutoff(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.retention_days.map(retention_cutoff)
    }

    /// The marker that lists the session, named by the session's move count so that of a
    /// session two workspaces list, the copy moved last is told from the listing alone.
    fn index_key(&self, sid: &str, epoch: u32) -> ObjectPath {
        ObjectPath::from(format!("{}/index/{sid}/{epoch}", self.prefix))
    }

    fn index_session_prefix(&self, sid: &str) -> ObjectPath {
        ObjectPath::from(format!("{}/index/{sid}/", self.prefix))
    }

    /// Written by the retention sweep before it deletes anything of a session, and deleted
    /// last (`remove_session`): what finds a removal the sweep started and could not finish,
    /// the markers being gone by then. Not an epoch, so nothing lists or pulls a session by it.
    fn sweep_key(&self, sid: &str) -> ObjectPath {
        ObjectPath::from(format!("{}/index/{sid}/{SWEEP_RECORD}", self.prefix))
    }

    /// The token of the push split over parts in progress, next to the markers so a removal
    /// or the next whole push clears it with them, and the retention sweep, which walks the
    /// markers, finds one a browser abandoned.
    fn push_key(&self, sid: &str) -> ObjectPath {
        ObjectPath::from(format!("{}/index/{sid}/{PUSH_TOKEN}", self.prefix))
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

    /// `max` is what the listing said the object holds, or the cap of its kind for one read
    /// without a listing: checked before buffering, since whoever holds the bucket's
    /// credentials can put anything at a predictable key.
    async fn get(&self, key: &ObjectPath, max: usize) -> Result<Read> {
        let result = match self.store.get(key).await {
            Ok(result) => result,
            Err(ObjectStoreError::NotFound { .. }) => return Ok(Read::Gone),
            Err(e) => return Err(object_store_error_to_error(e)),
        };
        let size = result.meta.size as usize;
        // Larger than any push writes: planted, whatever the listing said, and skipped like
        // an object of another key rather than retried like one that grew.
        if size > MAX_OBJECT_BYTES {
            tracing::warn!("AI session backup object {key} is larger than any push writes");
            return Ok(Read::Foreign);
        }
        if size > max {
            return Ok(Read::Grown);
        }
        let bytes = result.bytes().await.map_err(object_store_error_to_error)?;
        // The objects are JSON and data URLs: a wrong key's output failing UTF-8 tells it
        // apart beyond the cipher's padding check, which a wrong key passes now and then.
        match self
            .mc
            .decrypt_bytes_to_bytes(&bytes)
            .ok()
            .and_then(|plaintext| String::from_utf8(plaintext).ok())
        {
            Some(text) => Ok(Read::Text(text)),
            None => {
                tracing::warn!("AI session backup object {key} does not decrypt for its reader");
                Ok(Read::Foreign)
            }
        }
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

    /// A fingerprint of the session's marker and of everything listed under its two
    /// prefixes (key, size, modification time, entity tag and version), combined as the
    /// listing streams and in no particular order, so a session of any size costs bounded
    /// memory. `None` for a session
    /// the storage does not list. Taken before and after a page is read, so a page a push
    /// changed under is read again; pages of one pull carry it, and the browser starts the
    /// session over when it moved between two of them.
    async fn listing_fingerprint(&self, sid: &str) -> Result<Option<String>> {
        use std::hash::{DefaultHasher, Hash, Hasher};
        // The entity tag and version go in with the key, size and time: a store reports
        // modification times coarsely, and an object rewritten at the same size within that
        // grain would otherwise fingerprint the same.
        fn fold<S: Hash>(
            acc: u64,
            location: &str,
            size: S,
            modified: i64,
            e_tag: Option<&str>,
            version: Option<&str>,
        ) -> u64 {
            let mut hasher = DefaultHasher::new();
            (location, size, modified, e_tag, version).hash(&mut hasher);
            acc.wrapping_add(hasher.finish())
        }
        let mut acc = 0u64;
        let mut listed = false;
        for (marker, prefix) in [
            (true, self.index_session_prefix(sid)),
            (false, self.session_prefix(sid)),
            (false, self.images_prefix(sid)),
        ] {
            let mut stream = self.store.list(Some(&prefix));
            while let Some(meta) = stream.next().await {
                let meta = meta.map_err(object_store_error_to_error)?;
                // The sweep's record is not a marker: a session it started removing is absent.
                listed |= marker
                    && meta
                        .location
                        .filename()
                        .is_some_and(|name| name.parse::<u32>().is_ok());
                acc = fold(
                    acc,
                    meta.location.as_ref(),
                    meta.size,
                    meta.last_modified.timestamp_millis(),
                    meta.e_tag.as_deref(),
                    meta.version.as_deref(),
                );
            }
        }
        if !listed {
            return Ok(None);
        }
        Ok(Some(format!("{acc:016x}")))
    }

    async fn exists(&self, key: &ObjectPath) -> Result<bool> {
        match self.store.head(key).await {
            Ok(_) => Ok(true),
            Err(ObjectStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(object_store_error_to_error(e)),
        }
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
    let (disabled, retention, generation) =
        sqlx::query_as::<_, (Option<bool>, Option<serde_json::Value>, i64)>(
            "SELECT (ai_config->>'sessions_storage_disabled')::bool, \
                    ai_config->'sessions_retention_days', ai_sessions_backup_generation \
             FROM workspace_settings WHERE workspace_id = $1",
        )
        .bind(w_id)
        .fetch_optional(db)
        .await?
        .unwrap_or((None, None, 0));
    if disabled.unwrap_or(false) {
        return Ok(None);
    }
    let retention_days = sessions_retention_days(retention.as_ref());
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
    let storage_id = storage_id(&resource);
    let prefix = format!("{}/{user}", generation_prefix(w_id, generation));
    Ok(Some(Backend {
        store,
        mc,
        prefix,
        storage_id,
        generation,
        retention_days,
    }))
}

#[derive(Serialize)]
struct SessionListing {
    id: String,
    updated_at: chrono::DateTime<chrono::Utc>,
    /// The session's move count when this copy was pushed (see `PushedSession::epoch`).
    epoch: u32,
}

#[derive(Serialize)]
struct ListResponse {
    enabled: bool,
    /// The storage answered from, and the generation a key rotation bumps; a browser whose
    /// sync state names another storage or generation starts over.
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_generation: Option<i64>,
    sessions: Vec<SessionListing>,
    /// The user has more sessions than the answer names.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    truncated: bool,
}

/// A session is listed once a push entry of it landed whole (its marker is written last);
/// a push that failed before that left objects the listing does not name. One past the
/// workspace's retention is not listed either, whether or not the sweep has reached it, so a
/// browser never restores it.
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
            backup_generation: None,
            sessions: vec![],
            truncated: false,
        }));
    };
    let prefix = backend.index_prefix();
    let cutoff = backend.retention_cutoff();
    let mut stream = backend.store.list(Some(&prefix));
    // One marker per session, whatever the session holds: the newest LIST_MAX are kept as
    // the scan goes (a min-heap drops the oldest), and the scan itself is bounded.
    let mut newest: std::collections::BinaryHeap<
        std::cmp::Reverse<(chrono::DateTime<chrono::Utc>, u32, String)>,
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
        if cutoff.is_some_and(|cutoff| meta.last_modified < cutoff) {
            continue;
        }
        let Some((sid, IndexEntry::Marker(epoch))) = index_entry(&prefix, &meta.location) else {
            continue;
        };
        newest.push(std::cmp::Reverse((
            meta.last_modified,
            epoch,
            sid.to_string(),
        )));
        if newest.len() > LIST_MAX {
            newest.pop();
            truncated = true;
        }
    }
    let mut sessions: Vec<SessionListing> = newest
        .into_iter()
        .map(|std::cmp::Reverse((updated_at, epoch, id))| SessionListing { id, updated_at, epoch })
        .collect();
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(Json(ListResponse {
        enabled: true,
        storage_id: Some(backend.storage_id.clone()),
        backup_generation: Some(backend.generation),
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
    /// A fingerprint of the session's listing (marker, and every key, size, modification
    /// time, entity tag and version), so the browser tells that the backup changed between
    /// the pages it assembled.
    listing: String,
    /// The backup kept changing while this page was read (a push landing object by object),
    /// so the page may mix two versions: the browser starts the session over.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    moved: bool,
}

/// How many times a page whose listing moved while it was read is read again before it is
/// handed over as `moved`.
const PULL_REREADS: usize = 3;

#[derive(Serialize)]
struct PullResponse {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_generation: Option<i64>,
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
    // A page read while a push lands object by object may mix two versions of the session:
    // the listing is taken again once the page is read, and a page it moved under is read
    // again, a few times, then handed over as such for the browser to start over.
    for reread in 0..PULL_REREADS {
        let step = pull_page(backend, sid, budget, first, resume).await?;
        let PullStep::Fetched(mut page, size) = step else {
            return Ok(step);
        };
        if backend.listing_fingerprint(sid).await?.as_deref() == Some(page.listing.as_str()) {
            return Ok(PullStep::Fetched(page, size));
        }
        if reread + 1 == PULL_REREADS {
            page.moved = true;
            return Ok(PullStep::Fetched(page, size));
        }
    }
    unreachable!("a page is answered on the last reread")
}

async fn pull_page(
    backend: &Backend,
    sid: &str,
    budget: usize,
    first: bool,
    resume: Option<&PullCursor>,
) -> Result<PullStep> {
    // Taken before anything of the page is listed or read: an object landing after it is
    // in the next page's fingerprint, whereas one landing after the reads but before a
    // fingerprint taken then would have certified a page without it. A session the storage
    // does not list (removed, or a whole push in progress) is absent.
    let Some(listing) = backend.listing_fingerprint(sid).await? else {
        return Ok(PullStep::Absent);
    };
    let Read::Text(head) = backend
        .get(&backend.head_key(sid), MAX_HEAD_BYTES + CIPHER_PADDING)
        .await?
    else {
        return Ok(PullStep::Absent);
    };
    let session_prefix = backend.session_prefix(sid);
    let images_prefix = backend.images_prefix(sid);
    let mut size = head.len();
    let mut chats = vec![];
    let mut artifacts = None;
    let mut images = vec![];
    let mut next = None;
    let cursor = |images: bool, after: String| PullCursor { id: sid.to_string(), images, after };
    // Sizes come from the listings, and the listings stop at the budget, so nothing is read
    // past it even for the first session of the answer. One that outgrew it (chats
    // accumulate over pushes) comes back in pages, in key order, each answer naming where
    // the next picks up; the browser imports nothing before the last page. An object that
    // grew since the listing (a push replaced it) ends the page just before it, and the
    // answer names that spot: a new listing sizes it, whereas dropping it would import the
    // session without it for good.
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
            next = entries
                .last()
                .map(|(key, _)| cursor(false, key.to_string()));
        }
        let to_read: Vec<(ObjectPath, usize, Option<String>)> = entries
            .into_iter()
            .filter_map(|(key, bytes)| {
                let rel = key
                    .as_ref()
                    .strip_prefix(session_prefix.as_ref())
                    .unwrap_or_default()
                    .trim_start_matches('/');
                if rel == "artifacts.json" {
                    Some((key, bytes, None))
                } else {
                    let cid = rel
                        .strip_prefix("chats/")?
                        .strip_suffix(".json")?
                        .to_string();
                    Some((key, bytes, Some(cid)))
                }
            })
            .collect();
        let reads: Vec<(ObjectPath, usize, Option<String>, Read)> = futures::stream::iter(to_read)
            .map(|(key, bytes, cid)| async move {
                let read = backend.get(&key, bytes).await?;
                Ok::<_, Error>((key, bytes, cid, read))
            })
            .buffered(IO_CONCURRENCY)
            .try_collect()
            .await?;
        let mut before = resume.map(|c| c.after.clone()).unwrap_or_default();
        for (key, bytes, cid, read) in reads {
            match (cid, read) {
                (_, Read::Grown) => {
                    next = Some(cursor(false, before));
                    break;
                }
                (None, Read::Text(text)) => {
                    artifacts = Some(raw("artifacts", text)?);
                    size += bytes;
                }
                (Some(cid), Read::Text(text)) => {
                    chats.push(PulledChat { id: cid, record: raw("chat", text)? });
                    size += bytes;
                }
                _ => {}
            }
            before = key.to_string();
        }
    }
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
        let mut before = after.map(|a| a.to_string()).unwrap_or_default();
        if cut {
            // An answer with no room for a single image names where it stood, so the pull
            // owed the session alone picks it up there.
            next = Some(cursor(
                true,
                entries
                    .last()
                    .map(|(key, _)| key.to_string())
                    .unwrap_or_else(|| before.clone()),
            ));
        }
        let to_read: Vec<(ObjectPath, usize, String, String)> = entries
            .into_iter()
            .filter_map(|(key, bytes)| {
                let rel = key.as_ref().strip_prefix(images_prefix.as_ref())?;
                let (cid, iid) = rel.trim_start_matches('/').split_once('/')?;
                Some((key.clone(), bytes, cid.to_string(), iid.to_string()))
            })
            .collect();
        let reads: Vec<(ObjectPath, usize, String, String, Read)> = futures::stream::iter(to_read)
            .map(|(key, bytes, cid, iid)| async move {
                let read = backend.get(&key, bytes).await?;
                Ok::<_, Error>((key, bytes, cid, iid, read))
            })
            .buffered(IO_CONCURRENCY)
            .try_collect()
            .await?;
        for (key, bytes, chat_id, id, read) in reads {
            match read {
                Read::Grown => {
                    next = Some(cursor(true, before));
                    break;
                }
                Read::Text(data_url) => {
                    images.push(ImageObject { chat_id, id, data_url });
                    size += bytes;
                }
                _ => {}
            }
            before = key.to_string();
        }
    }
    Ok(PullStep::Fetched(
        PulledSession {
            id: sid.to_string(),
            head: raw("head", head)?,
            chats,
            images,
            artifacts,
            next,
            listing,
            moved: false,
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
            backup_generation: None,
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
        backup_generation: Some(backend.generation),
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
    /// A part of a push of the session whole: the head is on the part that opens it, which
    /// replaces whatever the storage holds of the session, and every piece the browser has
    /// is on one of them. An incremental part instead rides on a session the storage lists,
    /// and is refused with `needs_whole` when it lists none.
    #[serde(default)]
    whole: bool,
    /// A push split over several parts names itself on each of them with a token the
    /// browser draws; the part that `opens` it unlists the session (a pull between two parts
    /// would otherwise take a mix of old and new pieces for the backup) and the last part
    /// lists it again. A later part is written only while that token is the one there, so a
    /// part of a push another one superseded is refused with `needs_whole`.
    #[serde(default)]
    push: Option<String>,
    #[serde(default)]
    opens: bool,
    /// The session's move count (its record's `moves`), the marker that lists the session
    /// is named by: a session moved to another workspace is listed by both until the old
    /// copy's removal lands, and the copy with the higher count is the later one. An
    /// incremental part rides on the marker of the same count.
    #[serde(default)]
    epoch: u32,
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
    /// Nothing was written; the session must be pushed whole again: an incremental part
    /// found no listed session to ride on (another device removed the backup, or a push
    /// split over parts is in progress or was abandoned), or a later part of a push split
    /// over parts found another push had superseded it.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    needs_whole: bool,
}

#[derive(Serialize)]
struct PushResponse {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_generation: Option<i64>,
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
        if let Some(token) = &s.push {
            require_valid_id("push", token)?;
        } else if s.opens || s.partial {
            // A part more parts follow belongs to a push split over parts, which names
            // itself: without the token the session would stay listed between the parts.
            return Err(Error::BadRequest(format!(
                "session {} is pushed in parts with no push token",
                s.id
            )));
        }
        if s.whole && (s.push.is_none() || s.opens) && s.head.is_none() {
            return Err(Error::BadRequest(format!(
                "session {} is pushed whole without its head",
                s.id
            )));
        }
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

/// Runs under the session's lock (see `lock_session`). The part that opens a whole push
/// (the one with the head) replaces the backup: the marker goes first, so nothing lists the
/// session until the last part, then everything else. An incremental part assumes the rest
/// of the session is in the storage, which a removal since would have taken, or a push
/// split over parts may still be bringing: it is refused unless the session is listed, and
/// unlists the session itself while it changes more than one object (a pull between two
/// writes would otherwise take a mix of old and new pieces for the backup). A push split
/// over parts names itself with a token: the part that opens it unlists the session and
/// writes the token, and a later part is written only while that token is
/// the one there, so two devices pushing the session at once cannot list a mix of their
/// pieces: the push that opened later wins, the other is refused and goes again. Every
/// refusal comes before anything of the part lands. Deletes run last, and the marker only
/// by the last part, so a push cut short never leaves a listed session pointing at chats
/// that are not there.
/// Bytes written, and whether the part was refused for the session to go whole.
async fn push_session(backend: &Backend, s: &PushedSession) -> Result<(usize, bool)> {
    match &s.push {
        Some(token) if !s.opens => {
            match backend
                .get(&backend.push_key(&s.id), token.len() + CIPHER_PADDING)
                .await?
            {
                Read::Text(current) if current == *token => {}
                _ => return Ok((0, true)),
            }
        }
        _ => {
            if s.whole {
                backend
                    .delete_prefix(&backend.index_session_prefix(&s.id))
                    .await?;
                backend
                    .delete_prefix(&backend.session_prefix(&s.id))
                    .await?;
                backend.delete_prefix(&backend.images_prefix(&s.id)).await?;
            } else {
                if !backend.exists(&backend.index_key(&s.id, s.epoch)).await? {
                    return Ok((0, true));
                }
                // Unlisted while more than one object changes (a push split over parts, or
                // one part touching several pieces): a pull between two of the writes, or
                // after one of them failed, would otherwise take a mix of old and new
                // pieces for the backup. One object changing is one write.
                let pieces = s.chats.len()
                    + s.images.len()
                    + usize::from(s.artifacts.is_some())
                    + usize::from(s.head.is_some())
                    + s.delete_chats.len()
                    + s.delete_images.len();
                if s.push.is_some() || pieces > 1 {
                    backend
                        .delete_prefix(&backend.index_session_prefix(&s.id))
                        .await?;
                }
            }
            if let Some(token) = &s.push {
                backend
                    .put(&backend.push_key(&s.id), token.as_bytes())
                    .await?;
            }
        }
    }
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
    if s.partial {
        return Ok((written, false));
    }
    // Last, and by the last part only, so a session is listed once its whole entry landed.
    backend
        .store
        .put(&backend.index_key(&s.id, s.epoch), PutPayload::new())
        .await
        .map_err(object_store_error_to_error)?;
    if s.push.is_some() {
        backend.delete(&backend.push_key(&s.id)).await?;
    }
    Ok((written, false))
}

/// The markers go first so a removal cut short leaves nothing listed, then the head so
/// nothing pulls either, and no push takes it for a session still there (see `push_session`).
/// The retention sweep's record goes last (see `Backend::sweep_key`).
async fn remove_session(backend: &Backend, sid: &str) -> Result<()> {
    let sweep = backend.sweep_key(sid);
    backend
        .store
        .list(Some(&backend.index_session_prefix(sid)))
        .map_err(object_store_error_to_error)
        .try_for_each_concurrent(IO_CONCURRENCY, |meta| {
            let sweep = &sweep;
            async move {
                if meta.location == *sweep {
                    return Ok(());
                }
                backend.delete(&meta.location).await
            }
        })
        .await?;
    backend.delete(&backend.head_key(sid)).await?;
    backend.delete_prefix(&backend.session_prefix(sid)).await?;
    backend.delete_prefix(&backend.images_prefix(sid)).await?;
    backend.delete(&sweep).await
}

/// One writer per session at a time, across servers: a push and a removal of the same
/// session interleaving object by object could leave a listed session missing pieces, or a
/// marker over nothing. The lock lives in a transaction that writes no rows; it is released
/// when the transaction ends. A wait past the timeout fails that entry only, and the
/// browser retries it with backoff.
async fn lock_session(
    db: &DB,
    backend: &Backend,
    sid: &str,
) -> Result<sqlx::Transaction<'static, sqlx::Postgres>> {
    let mut tx = db.begin().await?;
    sqlx::query("SET LOCAL lock_timeout = '30s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0::int8))")
        .bind(format!("ai_session_backup:{}/{sid}", backend.prefix))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "another device is writing the backup of session {sid}; retried later ({e})"
            ))
        })?;
    Ok(tx)
}

async fn push_session_locked(
    db: &DB,
    backend: &Backend,
    s: &PushedSession,
) -> Result<(usize, bool)> {
    let tx = lock_session(db, backend, &s.id).await?;
    let result = push_session(backend, s).await;
    tx.commit().await?;
    result
}

async fn remove_session_locked(db: &DB, backend: &Backend, sid: &str) -> Result<()> {
    let tx = lock_session(db, backend, sid).await?;
    let result = remove_session(backend, sid).await;
    tx.commit().await?;
    result
}

/// Deletes, in every workspace with `ai_config.sessions_retention_days`, the backups of the
/// sessions whose marker is older than that: the marker is rewritten by every push that
/// completes, so its modification time is the session's last activity as the storage clocks
/// it. For the monitor, on every server: a session-level advisory lock keeps one pass at a
/// time across them. The walk reads markers only, one object per session and nothing of what
/// the sessions hold, under each user's prefix in turn (`list_with_delimiter` names the
/// users), and deletes at most `SWEEP_MAX_PER_WORKSPACE` sessions per workspace and pass. A
/// session goes under its lock (`lock_session`), once its markers are listed again there and
/// still all older (see `sweep_session`). A removal cut short leaves the sweep's record next
/// to the markers, which the walk also collects, so the next pass finishes it.
pub async fn sweep_expired_ai_session_backups(db: &DB) {
    let mut lock_conn = match db.acquire().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("AI session retention: could not acquire a connection: {e:#}");
            return;
        }
    };
    let locked: bool = match sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(SWEEP_LOCK_ID)
        .fetch_one(&mut *lock_conn)
        .await
    {
        Ok(locked) => locked,
        Err(e) => {
            tracing::error!("AI session retention: advisory lock failed: {e:#}");
            return;
        }
    };
    if !locked {
        return;
    }
    if let Err(e) = sweep_workspaces(db).await {
        tracing::error!("AI session retention sweep failed: {e:#}");
    }
    if let Err(e) = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(SWEEP_LOCK_ID)
        .execute(&mut *lock_conn)
        .await
    {
        tracing::error!("AI session retention: advisory unlock failed: {e:#}");
    }
}

async fn sweep_workspaces(db: &DB) -> Result<()> {
    let workspaces = sqlx::query_as::<_, (String, Option<serde_json::Value>, i64)>(
        "SELECT workspace_id, ai_config->'sessions_retention_days', ai_sessions_backup_generation \
         FROM workspace_settings \
         WHERE ai_config->'sessions_retention_days' IS NOT NULL \
           AND large_file_storage IS NOT NULL",
    )
    .fetch_all(db)
    .await?;
    for (w_id, retention, generation) in workspaces {
        let Some(days) = sessions_retention_days(retention.as_ref()) else {
            continue;
        };
        match sweep_workspace(db, &w_id, days, generation).await {
            Ok(0) => {}
            Ok(deleted) => tracing::info!(
                "AI session retention deleted {deleted} session backups of {w_id} older than {days} days"
            ),
            Err(e) => tracing::warn!("AI session retention sweep of {w_id}: {e:#}"),
        }
    }
    Ok(())
}

async fn sweep_workspace(db: &DB, w_id: &str, days: u32, generation: i64) -> Result<usize> {
    let Some((store, resource)) = primary_store(db, w_id).await? else {
        return Ok(0);
    };
    let key = get_workspace_key(w_id, db).await?;
    let storage_id = storage_id(&resource);
    let cutoff = retention_cutoff(days);
    let root = ObjectPath::from(generation_prefix(w_id, generation));
    let users = store
        .list_with_delimiter(Some(&root))
        .await
        .map_err(object_store_error_to_error)?
        .common_prefixes;
    let mut deleted = 0;
    for user_prefix in users {
        let Some(user) = user_prefix.filename() else {
            continue;
        };
        // The sweep decrypts nothing; the cipher is only what a `Backend` is made of.
        let backend = Backend {
            store: store.clone(),
            mc: crypt_from_key_with_suffix(&key, user),
            prefix: user_prefix.to_string(),
            storage_id: storage_id.clone(),
            generation,
            retention_days: Some(days),
        };
        let index = backend.index_prefix();
        let mut markers = backend.store.list(Some(&index));
        let mut expired = std::collections::BTreeSet::new();
        while let Some(meta) = markers.next().await {
            let meta = meta.map_err(object_store_error_to_error)?;
            let sid = match index_entry(&index, &meta.location) {
                Some((sid, IndexEntry::Sweep)) => sid,
                Some((sid, IndexEntry::Marker(_) | IndexEntry::Push))
                    if meta.last_modified < cutoff =>
                {
                    sid
                }
                _ => continue,
            };
            expired.insert(sid.to_string());
            if deleted + expired.len() >= SWEEP_MAX_PER_WORKSPACE {
                break;
            }
        }
        for sid in expired {
            match sweep_session(db, &backend, &sid, cutoff).await {
                Ok(true) => deleted += 1,
                Ok(false) => {}
                Err(e) => tracing::warn!(
                    "AI session retention left the backup of {sid} in {w_id} for the next pass: {e:#}"
                ),
            }
        }
        if deleted >= SWEEP_MAX_PER_WORKSPACE {
            break;
        }
    }
    Ok(deleted)
}

/// True when the session was deleted. Under the session's lock its markers are listed again:
/// one a push renewed since the walk keeps the session. A session with none is left alone
/// while a push split over parts is between two of them (its token younger than the
/// retention) or it is gone, unless the sweep's record says a removal was started; an older
/// token is a split push a browser abandoned, whose landed parts nothing lists. The record is
/// written before anything is deleted and removed last, so a removal cut short is found again
/// by the next pass.
async fn sweep_session(
    db: &DB,
    backend: &Backend,
    sid: &str,
    cutoff: chrono::DateTime<chrono::Utc>,
) -> Result<bool> {
    let tx = lock_session(db, backend, sid).await?;
    let result = async {
        let (sweep, push) = (backend.sweep_key(sid), backend.push_key(sid));
        let mut entries = backend.store.list(Some(&backend.index_session_prefix(sid)));
        let (mut listed, mut renewed, mut started, mut abandoned) = (false, false, false, false);
        while let Some(meta) = entries.next().await {
            let meta = meta.map_err(object_store_error_to_error)?;
            if meta.location == sweep {
                started = true;
            } else if meta.location == push {
                abandoned = meta.last_modified < cutoff;
            } else {
                listed = true;
                renewed |= meta.last_modified >= cutoff;
            }
        }
        if renewed {
            // A push listed the session again over a removal cut short before its markers
            // went, which had deleted nothing else.
            if started {
                backend.delete(&sweep).await?;
            }
            return Ok(false);
        }
        if !listed && !started && !abandoned {
            return Ok(false);
        }
        if !started {
            backend
                .store
                .put(&sweep, PutPayload::new())
                .await
                .map_err(object_store_error_to_error)?;
        }
        remove_session(backend, sid).await?;
        Ok(true)
    }
    .await;
    tx.commit().await?;
    result
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
            backup_generation: None,
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
    // A session split into several entries is listed by the last: once one part failed, the
    // later ones are not written, or the marker would list a session missing a part.
    let mut failed: std::collections::HashSet<&str> = Default::default();
    for s in &req.sessions {
        let (error, needs_whole) = if failed.contains(s.id.as_str()) {
            (
                Some("an earlier part of this session in the push failed".to_string()),
                false,
            )
        } else {
            match push_session_locked(&db, &backend, s).await {
                Ok((n, needs_whole)) => {
                    written += n;
                    (None, needs_whole)
                }
                Err(e) => {
                    tracing::warn!("AI session backup push failed for {} in {w_id}: {e}", s.id);
                    failed.insert(&s.id);
                    (Some(e.to_string()), false)
                }
            }
        };
        results.push(PushResult { id: s.id.clone(), error, needs_whole });
    }
    for sid in &req.removed {
        let error = remove_session_locked(&db, &backend, sid)
            .await
            .err()
            .map(|e| {
                tracing::warn!("AI session backup removal failed for {sid} in {w_id}: {e}");
                e.to_string()
            });
        results.push(PushResult { id: sid.clone(), error, needs_whole: false });
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
        backup_generation: Some(backend.generation),
        results,
    }))
}
