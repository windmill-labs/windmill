/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! AI session artifacts their owners shared: path-addressed items whose path and
//! `extra_perms` decide who reads them, as for scripts and flows (`/acls/*/ai_artifact/<path>`
//! changes who has access).
//!
//! Artifacts live in their owner's browser. A row holds one version, the one people with access
//! see. The owner's browser sends each later version, and a write that was not based on the
//! latest version the row knows is refused, so a stale copy never overwrites a newer one.

use crate::db::{ApiAuthed, DB};
use axum::{
    extract::{DefaultBodyLimit, Extension, Json, Path, Query},
    routing::{delete, get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
};

/// The frontend's `MAX_ARTIFACT_BYTES`: no artifact the browser holds is larger.
const MAX_CONTENT_BYTES: usize = 256 * 1024;
const MAX_NAME_CHARS: usize = 255;
const MAX_ID_CHARS: usize = 255;
/// JSON escapes a control character into six bytes, so a full-size artifact made entirely of
/// them still fits.
const BODY_LIMIT: usize = MAX_CONTENT_BYTES * 6 + 64 * 1024;
const MAX_PATH_NAME_CHARS: usize = 100;
const MAX_LISTED: i64 = 1000;
const MAX_MATCH_LINE_CHARS: usize = 160;
/// Non-owners get this one answer for an artifact that is not shared with them, in the trash,
/// or gone, so a link reveals nothing about an artifact its holder cannot open.
const NOT_VISIBLE: &str = "This artifact isn't shared with you, or no longer exists";

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_artifacts))
        .route("/get/{artifact_id}", get(get_artifact))
        .route("/legacy/{id}", get(resolve_legacy_link))
        .route(
            "/publish",
            post(publish_artifact).layer(DefaultBodyLimit::max(BODY_LIMIT)),
        )
        .route(
            "/write/{artifact_id}",
            post(write_artifact).layer(DefaultBodyLimit::max(BODY_LIMIT)),
        )
        .route(
            "/show/{artifact_id}",
            post(show_version).layer(DefaultBodyLimit::max(BODY_LIMIT)),
        )
        .route("/trash/{artifact_id}", post(trash_artifact))
        .route("/restore/{artifact_id}", post(restore_artifact))
        .route("/delete/{artifact_id}", delete(delete_artifact))
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum ArtifactKind {
    Md,
    Html,
}

impl ArtifactKind {
    fn as_str(self) -> &'static str {
        match self {
            ArtifactKind::Md => "md",
            ArtifactKind::Html => "html",
        }
    }
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum ArtifactRole {
    Plan,
}

impl ArtifactRole {
    fn as_str(self) -> &'static str {
        match self {
            ArtifactRole::Plan => "plan",
        }
    }
}

struct ArtifactRow {
    artifact_id: String,
    path: String,
    extra_perms: serde_json::Value,
    email: String,
    created_by: String,
    session_id: Option<String>,
    name: String,
    kind: String,
    role: Option<String>,
    shown_version: i32,
    latest_version: i32,
    pinned: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
struct ArtifactItem {
    artifact_id: String,
    path: String,
    extra_perms: serde_json::Value,
    /// The owner's username.
    owner: String,
    is_owner: bool,
    /// The session that wrote it, for its owner only.
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    name: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    shown_version: i32,
    latest_version: i32,
    pinned: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_at: Option<DateTime<Utc>>,
}

impl ArtifactItem {
    fn from_row(row: ArtifactRow, authed: &ApiAuthed) -> Self {
        let is_owner = row.email == authed.email;
        ArtifactItem {
            artifact_id: row.artifact_id,
            path: row.path,
            extra_perms: row.extra_perms,
            owner: row.created_by,
            is_owner,
            session_id: row.session_id.filter(|_| is_owner),
            name: row.name,
            kind: row.kind,
            role: row.role,
            shown_version: row.shown_version,
            latest_version: row.latest_version,
            pinned: row.pinned,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
        }
    }
}

#[derive(Serialize)]
struct ArtifactWithContent {
    #[serde(flatten)]
    item: ArtifactItem,
    /// False once the owner has left the workspace.
    owner_is_member: bool,
    content: String,
}

#[derive(Serialize)]
struct ArtifactListEntry {
    #[serde(flatten)]
    item: ArtifactItem,
    /// False once the owner has left the workspace.
    owner_is_member: bool,
    /// The first line of the content that matches the search, when the content matched.
    #[serde(skip_serializing_if = "Option::is_none")]
    match_line: Option<String>,
}

/// What a write or a change of the shown version did. A stale write is an answer rather than an
/// error: the caller reads the latest version from it and redoes its edit on top.
#[derive(Serialize)]
struct WriteResult {
    /// `saved`, `stale` (based on an older version than `latest_version`), or `not_shared`
    /// (the server holds no live copy of this artifact of the caller's).
    outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact: Option<ArtifactItem>,
}

impl WriteResult {
    fn saved(row: ArtifactRow, authed: &ApiAuthed) -> Self {
        WriteResult {
            outcome: "saved",
            latest_version: Some(row.latest_version),
            artifact: Some(ArtifactItem::from_row(row, authed)),
        }
    }

    fn stale(latest_version: i32) -> Self {
        WriteResult { outcome: "stale", latest_version: Some(latest_version), artifact: None }
    }

    fn not_shared() -> Self {
        WriteResult { outcome: "not_shared", latest_version: None, artifact: None }
    }
}

/// Checks an id compared against a `VARCHAR(255)` column: Postgres answers a NUL in a text
/// parameter with an opaque 500.
fn check_id(what: &str, id: &str) -> Result<()> {
    if id.is_empty() || id.chars().count() > MAX_ID_CHARS {
        return Err(Error::BadRequest(format!(
            "{what} must be between 1 and {MAX_ID_CHARS} characters"
        )));
    }
    if id.contains('\0') {
        return Err(Error::BadRequest(format!(
            "{what} cannot contain NUL characters"
        )));
    }
    Ok(())
}

fn check_name(name: &str) -> Result<()> {
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(Error::BadRequest(format!(
            "Artifact name must be between 1 and {MAX_NAME_CHARS} characters"
        )));
    }
    if name.contains('\0') {
        return Err(Error::BadRequest(
            "Artifact name cannot contain NUL characters".to_string(),
        ));
    }
    Ok(())
}

fn check_content(content: &str) -> Result<()> {
    if content.len() > MAX_CONTENT_BYTES {
        return Err(Error::BadRequest(format!(
            "Artifact content is {} bytes, above the {MAX_CONTENT_BYTES} byte limit",
            content.len()
        )));
    }
    if content.contains('\0') {
        return Err(Error::BadRequest(
            "Artifact content cannot contain NUL characters".to_string(),
        ));
    }
    Ok(())
}

fn check_version(version: i32) -> Result<()> {
    if version < 1 {
        return Err(Error::BadRequest(
            "Artifact version must be at least 1".to_string(),
        ));
    }
    Ok(())
}

/// The last segment of a new artifact's path: its name in lowercase, each run of other
/// characters turned into one `_`. The migration that moved the earlier shares spells the same
/// rule in SQL.
fn path_name(name: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('_');
            }
            gap = false;
            out.push(c);
            if out.len() >= MAX_PATH_NAME_CHARS {
                break;
            }
        } else {
            gap = true;
        }
    }
    if out.is_empty() {
        "artifact".to_string()
    } else {
        out
    }
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// The first line holding `query`, cut to a window around the match when it is long: a markdown
/// paragraph is one line, and its start rarely holds the hit. The page cuts its local matches
/// the same way (`matchLine` in artifactRows.ts).
fn match_line(content: &str, query: &str) -> Option<String> {
    let query = query.to_lowercase();
    let line = content
        .lines()
        .find(|line| line.to_lowercase().contains(&query))?
        .trim();
    let len = line.chars().count();
    if len <= MAX_MATCH_LINE_CHARS {
        return Some(line.to_string());
    }
    let lower = line.to_lowercase();
    let hit = lower.find(&query).map_or(0, |b| lower[..b].chars().count());
    let lead = MAX_MATCH_LINE_CHARS.saturating_sub(query.chars().count()) / 2;
    let start = hit.saturating_sub(lead).min(len - MAX_MATCH_LINE_CHARS);
    let cut: String = line
        .chars()
        .skip(start)
        .take(MAX_MATCH_LINE_CHARS)
        .collect();
    let before = if start > 0 { "…" } else { "" };
    let after = if start + MAX_MATCH_LINE_CHARS < len {
        "…"
    } else {
        ""
    };
    Some(format!("{before}{cut}{after}"))
}

#[derive(Deserialize)]
struct ListQuery {
    search: Option<String>,
    #[serde(default)]
    include_deleted: bool,
}

/// The caller's own artifacts on the server and those shared with them, newest first. Shared
/// means named in `extra_perms`, directly or through a group, or in a folder they read; an admin
/// is shared every artifact on the server, as they see every item. Owners only see what they
/// put in the trash.
async fn list_artifacts(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> JsonResult<Vec<ArtifactListEntry>> {
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    if let Some(s) = &search {
        check_id("Search", s)?;
    }
    let pattern = search.as_deref().map(|s| format!("%{}%", escape_like(s)));
    let mut principals: Vec<String> = authed.groups.iter().map(|g| format!("g/{g}")).collect();
    principals.push(format!("u/{}", authed.username));
    // Every member is in `all`, whatever their group rows say.
    principals.push("g/all".to_string());
    let folders: Vec<String> = authed.folders.iter().map(|f| f.0.clone()).collect();

    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query!(
        r#"SELECT a.artifact_id, a.path, a.extra_perms, a.email, a.created_by, a.session_id,
                  a.name, a.kind, a.role, a.shown_version, a.latest_version, a.pinned,
                  a.created_at, a.updated_at, a.deleted_at,
                  EXISTS (
                      SELECT 1 FROM usr u WHERE u.workspace_id = a.workspace_id AND u.email = a.email
                  ) AS "owner_is_member!",
                  CASE WHEN $4::text IS NULL THEN NULL ELSE a.content END AS content
           FROM ai_artifact a
           WHERE a.workspace_id = $1
             AND (a.email = $2
                  OR (a.deleted_at IS NULL
                      AND ($8::bool
                           OR a.extra_perms ?| $3::text[]
                           OR (SPLIT_PART(a.path, '/', 1) = 'f'
                               AND SPLIT_PART(a.path, '/', 2) = ANY($5::text[])))))
             AND ($4::text IS NULL
                  OR a.name ILIKE $4 ESCAPE '\'
                  OR a.content ILIKE $4 ESCAPE '\')
             AND ($6::bool OR a.deleted_at IS NULL)
           ORDER BY a.updated_at DESC
           LIMIT $7"#,
        &w_id,
        &authed.email,
        &principals,
        pattern,
        &folders,
        query.include_deleted,
        MAX_LISTED,
        authed.is_admin,
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| {
                let match_line = match (&search, &r.content) {
                    (Some(s), Some(content)) => match_line(content, s),
                    _ => None,
                };
                ArtifactListEntry {
                    owner_is_member: r.owner_is_member,
                    match_line,
                    item: ArtifactItem::from_row(
                        ArtifactRow {
                            artifact_id: r.artifact_id,
                            path: r.path,
                            extra_perms: r.extra_perms,
                            email: r.email,
                            created_by: r.created_by,
                            session_id: r.session_id,
                            name: r.name,
                            kind: r.kind,
                            role: r.role,
                            shown_version: r.shown_version,
                            latest_version: r.latest_version,
                            pinned: r.pinned,
                            created_at: r.created_at,
                            updated_at: r.updated_at,
                            deleted_at: r.deleted_at,
                        },
                        &authed,
                    ),
                }
            })
            .collect(),
    ))
}

/// One artifact with the version people with access see. Reads go through the row's policies,
/// so a member reads only what its path or access list grants them; its owner also reads it in
/// the trash.
async fn get_artifact(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, artifact_id)): Path<(String, String)>,
) -> JsonResult<ArtifactWithContent> {
    check_id("Artifact id", &artifact_id)?;
    let mut tx = user_db.begin(&authed).await?;
    let row = sqlx::query!(
        r#"SELECT a.artifact_id, a.path, a.extra_perms, a.email, a.created_by, a.session_id,
                  a.name, a.kind, a.role, a.shown_version, a.latest_version, a.pinned,
                  a.created_at, a.updated_at, a.deleted_at, a.content,
                  EXISTS (
                      SELECT 1 FROM usr u WHERE u.workspace_id = a.workspace_id AND u.email = a.email
                  ) AS "owner_is_member!"
           FROM ai_artifact a WHERE a.workspace_id = $1 AND a.artifact_id = $2"#,
        &w_id,
        &artifact_id,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let row = row
        .filter(|r| r.deleted_at.is_none() || r.email == authed.email)
        .ok_or_else(|| Error::NotFound(NOT_VISIBLE.to_string()))?;
    Ok(Json(ArtifactWithContent {
        owner_is_member: row.owner_is_member,
        content: row.content,
        item: ArtifactItem::from_row(
            ArtifactRow {
                artifact_id: row.artifact_id,
                path: row.path,
                extra_perms: row.extra_perms,
                email: row.email,
                created_by: row.created_by,
                session_id: row.session_id,
                name: row.name,
                kind: row.kind,
                role: row.role,
                shown_version: row.shown_version,
                latest_version: row.latest_version,
                pinned: row.pinned,
                created_at: row.created_at,
                updated_at: row.updated_at,
                deleted_at: row.deleted_at,
            },
            &authed,
        ),
    }))
}

#[derive(Serialize)]
struct LegacyLink {
    artifact_id: String,
}

/// The artifact a link from before artifacts were items points at: those links carry the id of
/// the share, which the migration kept on the row.
async fn resolve_legacy_link(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> JsonResult<LegacyLink> {
    let mut tx = user_db.begin(&authed).await?;
    let row = sqlx::query!(
        "SELECT artifact_id, email, deleted_at FROM ai_artifact WHERE workspace_id = $1 AND id = $2",
        &w_id,
        id,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let row = row
        .filter(|r| r.deleted_at.is_none() || r.email == authed.email)
        .ok_or_else(|| Error::NotFound(NOT_VISIBLE.to_string()))?;
    Ok(Json(LegacyLink { artifact_id: row.artifact_id }))
}

#[derive(Deserialize)]
struct PublishArtifact {
    artifact_id: String,
    name: String,
    kind: ArtifactKind,
    role: Option<ArtifactRole>,
    session_id: Option<String>,
    version: i32,
    content: String,
}

/// Make one of the caller's artifacts an item, with this version as the one people see, before
/// its access list is first changed. An artifact already on the server keeps its path and
/// access list, and takes this version when it is newer than the latest it knows. One in the
/// trash leaves it with an empty access list: its owner deleted it, so sharing it again starts
/// from nobody, where Restore brings back who it was shared with.
async fn publish_artifact(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(payload): Json<PublishArtifact>,
) -> JsonResult<ArtifactItem> {
    let name = payload.name.trim();
    check_id("Artifact id", &payload.artifact_id)?;
    check_name(name)?;
    check_content(&payload.content)?;
    check_version(payload.version)?;
    if let Some(sid) = &payload.session_id {
        check_id("Session id", sid)?;
    }

    let mut tx = db.begin().await?;
    // One user's publishes run one at a time: two at once could both find no row for the same
    // artifact, or both take the same free path.
    sqlx::query_scalar!(
        "SELECT pg_advisory_xact_lock(hashtext('ai_artifact:' || $1 || ':' || $2))",
        &w_id,
        &authed.email,
    )
    .fetch_one(&mut *tx)
    .await?;
    let owner = sqlx::query_scalar!(
        "SELECT email FROM ai_artifact WHERE workspace_id = $1 AND artifact_id = $2 FOR UPDATE",
        &w_id,
        &payload.artifact_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let row = match owner {
        Some(email) if email != authed.email => {
            return Err(Error::NotAuthorized(
                "This artifact id belongs to someone else's artifact".to_string(),
            ));
        }
        Some(_) => {
            sqlx::query_as!(
                ArtifactRow,
                "UPDATE ai_artifact SET
                     content = CASE WHEN NOT pinned AND $3 > latest_version THEN $4 ELSE content END,
                     name = CASE WHEN NOT pinned AND $3 > latest_version THEN $5 ELSE name END,
                     updated_at = CASE WHEN NOT pinned AND $3 > latest_version THEN now() ELSE updated_at END,
                     shown_version = CASE WHEN NOT pinned AND $3 > latest_version THEN $3 ELSE shown_version END,
                     latest_version = GREATEST(latest_version, $3),
                     session_id = COALESCE(session_id, $6),
                     extra_perms = CASE WHEN deleted_at IS NULL THEN extra_perms ELSE '{}'::jsonb END,
                     deleted_at = NULL
                 WHERE workspace_id = $1 AND artifact_id = $2
                 RETURNING artifact_id, path, extra_perms, email, created_by, session_id, name, kind,
                           role, shown_version, latest_version, pinned, created_at, updated_at,
                           deleted_at",
                &w_id,
                &payload.artifact_id,
                payload.version,
                &payload.content,
                name,
                payload.session_id,
            )
            .fetch_one(&mut *tx)
            .await?
        }
        None => {
            let base = format!("u/{}/{}", authed.username, path_name(name));
            let taken = sqlx::query_scalar!(
                "SELECT path FROM ai_artifact
                 WHERE workspace_id = $1 AND (path = $2 OR starts_with(path, $2 || '_'))",
                &w_id,
                &base,
            )
            .fetch_all(&mut *tx)
            .await?;
            let path = if taken.contains(&base) {
                (2..)
                    .map(|n| format!("{base}_{n}"))
                    .find(|p| !taken.contains(p))
                    .unwrap_or(base)
            } else {
                base
            };
            let row = sqlx::query_as!(
                ArtifactRow,
                "INSERT INTO ai_artifact
                     (workspace_id, artifact_id, path, email, created_by, session_id, name, kind,
                      role, shown_version, content, latest_version)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $10)
                 RETURNING artifact_id, path, extra_perms, email, created_by, session_id, name, kind,
                           role, shown_version, latest_version, pinned, created_at, updated_at,
                           deleted_at",
                &w_id,
                &payload.artifact_id,
                &path,
                &authed.email,
                &authed.username,
                payload.session_id,
                name,
                payload.kind.as_str(),
                payload.role.map(ArtifactRole::as_str),
                payload.version,
                &payload.content,
            )
            .fetch_one(&mut *tx)
            .await?;
            audit_log(
                &mut *tx,
                &authed,
                "ai.artifacts.publish",
                ActionKind::Create,
                &w_id,
                Some(&row.path),
                Some([("name", name)].into()),
            )
            .await?;
            row
        }
    };
    tx.commit().await?;
    Ok(Json(ArtifactItem::from_row(row, &authed)))
}

#[derive(Deserialize)]
struct WriteArtifact {
    /// The version the edit was made on; the write is refused unless it is the latest one.
    base_version: i32,
    version: i32,
    name: String,
    content: String,
    session_id: Option<String>,
}

/// A new version of one of the caller's shared artifacts. People with access see it at once,
/// unless the owner pinned the version they see.
async fn write_artifact(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, artifact_id)): Path<(String, String)>,
    Json(payload): Json<WriteArtifact>,
) -> JsonResult<WriteResult> {
    let name = payload.name.trim();
    check_id("Artifact id", &artifact_id)?;
    check_name(name)?;
    check_content(&payload.content)?;
    check_version(payload.base_version)?;
    if payload.version <= payload.base_version {
        return Err(Error::BadRequest(
            "A new version must be above the version it was based on".to_string(),
        ));
    }
    if let Some(sid) = &payload.session_id {
        check_id("Session id", sid)?;
    }

    let mut tx = db.begin().await?;
    let row = sqlx::query_as!(
        ArtifactRow,
        "UPDATE ai_artifact SET
             latest_version = $4,
             content = CASE WHEN pinned THEN content ELSE $5 END,
             name = CASE WHEN pinned THEN name ELSE $6 END,
             shown_version = CASE WHEN pinned THEN shown_version ELSE $4 END,
             updated_at = CASE WHEN pinned THEN updated_at ELSE now() END,
             session_id = COALESCE(session_id, $8)
         WHERE workspace_id = $1 AND artifact_id = $2 AND email = $3 AND latest_version = $7
           AND deleted_at IS NULL
         RETURNING artifact_id, path, extra_perms, email, created_by, session_id, name, kind,
                   role, shown_version, latest_version, pinned, created_at, updated_at,
                   deleted_at",
        &w_id,
        &artifact_id,
        &authed.email,
        payload.version,
        &payload.content,
        name,
        payload.base_version,
        payload.session_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let result = match row {
        Some(row) => WriteResult::saved(row, &authed),
        None => latest_live_version(&mut tx, &w_id, &artifact_id, &authed.email)
            .await?
            .map_or_else(WriteResult::not_shared, WriteResult::stale),
    };
    tx.commit().await?;
    Ok(Json(result))
}

/// The latest version of one of `email`'s artifacts that is on the server and not in the trash.
async fn latest_live_version(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    w_id: &str,
    artifact_id: &str,
    email: &str,
) -> Result<Option<i32>> {
    Ok(sqlx::query_scalar!(
        "SELECT latest_version FROM ai_artifact
         WHERE workspace_id = $1 AND artifact_id = $2 AND email = $3 AND deleted_at IS NULL",
        w_id,
        artifact_id,
        email,
    )
    .fetch_optional(&mut **tx)
    .await?)
}

#[derive(Deserialize)]
struct ShowVersion {
    /// True keeps people on `version` while newer ones are written; false shows them the latest,
    /// which `version` must then be.
    pinned: bool,
    version: i32,
    name: String,
    content: String,
}

/// Choose the version people with access see.
async fn show_version(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, artifact_id)): Path<(String, String)>,
    Json(payload): Json<ShowVersion>,
) -> JsonResult<WriteResult> {
    let name = payload.name.trim();
    check_id("Artifact id", &artifact_id)?;
    check_name(name)?;
    check_content(&payload.content)?;
    check_version(payload.version)?;

    let mut tx = db.begin().await?;
    let row = sqlx::query_as!(
        ArtifactRow,
        "UPDATE ai_artifact SET
             pinned = $4, shown_version = $5, content = $6, name = $7, updated_at = now()
         WHERE workspace_id = $1 AND artifact_id = $2 AND email = $3 AND deleted_at IS NULL
           AND CASE WHEN $4 THEN $5 <= latest_version ELSE $5 = latest_version END
         RETURNING artifact_id, path, extra_perms, email, created_by, session_id, name, kind,
                   role, shown_version, latest_version, pinned, created_at, updated_at,
                   deleted_at",
        &w_id,
        &artifact_id,
        &authed.email,
        payload.pinned,
        payload.version,
        &payload.content,
        name,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let result = match row {
        Some(row) => {
            audit_log(
                &mut *tx,
                &authed,
                if payload.pinned {
                    "ai.artifacts.pin"
                } else {
                    "ai.artifacts.unpin"
                },
                ActionKind::Update,
                &w_id,
                Some(&row.path),
                Some([("version", payload.version.to_string().as_str())].into()),
            )
            .await?;
            WriteResult::saved(row, &authed)
        }
        None => latest_live_version(&mut tx, &w_id, &artifact_id, &authed.email)
            .await?
            .map_or_else(WriteResult::not_shared, WriteResult::stale),
    };
    tx.commit().await?;
    Ok(Json(result))
}

/// Move one of the caller's artifacts to the trash, or out of it. Nobody else reads it while it
/// is there, and the monitor purges it after 30 days.
async fn set_trashed(
    authed: &ApiAuthed,
    db: &DB,
    w_id: &str,
    artifact_id: &str,
    trashed: bool,
) -> Result<String> {
    check_id("Artifact id", artifact_id)?;
    let mut tx = db.begin().await?;
    let path = sqlx::query_scalar!(
        "UPDATE ai_artifact SET deleted_at = CASE WHEN $4 THEN now() ELSE NULL END
         WHERE workspace_id = $1 AND artifact_id = $2 AND email = $3
         RETURNING path",
        w_id,
        artifact_id,
        &authed.email,
        trashed,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(NOT_VISIBLE.to_string()))?;
    audit_log(
        &mut *tx,
        authed,
        if trashed {
            "ai.artifacts.trash"
        } else {
            "ai.artifacts.restore"
        },
        ActionKind::Update,
        w_id,
        Some(&path),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(path)
}

async fn trash_artifact(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, artifact_id)): Path<(String, String)>,
) -> Result<String> {
    let path = set_trashed(&authed, &db, &w_id, &artifact_id, true).await?;
    Ok(format!("Moved {path} to the trash"))
}

async fn restore_artifact(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, artifact_id)): Path<(String, String)>,
) -> Result<String> {
    let path = set_trashed(&authed, &db, &w_id, &artifact_id, false).await?;
    Ok(format!("Restored {path}"))
}

/// Remove the server's copy for good: its owner's Delete forever, or an admin removing one whose
/// owner is gone.
async fn delete_artifact(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, artifact_id)): Path<(String, String)>,
) -> Result<String> {
    check_id("Artifact id", &artifact_id)?;
    let mut tx = db.begin().await?;
    let path = sqlx::query_scalar!(
        "DELETE FROM ai_artifact
         WHERE workspace_id = $1 AND artifact_id = $2 AND (email = $3 OR $4::bool)
         RETURNING path",
        &w_id,
        &artifact_id,
        &authed.email,
        authed.is_admin,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(NOT_VISIBLE.to_string()))?;
    audit_log(
        &mut *tx,
        &authed,
        "ai.artifacts.delete",
        ActionKind::Delete,
        &w_id,
        Some(&path),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("Deleted {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_name_keeps_letters_and_digits() {
        assert_eq!(path_name("Q3 rollout plan"), "q3_rollout_plan");
        assert_eq!(path_name("  --Revenue: v2!  "), "revenue_v2");
        assert_eq!(path_name("日本語"), "artifact");
        assert_eq!(path_name(&"a".repeat(300)).len(), MAX_PATH_NAME_CHARS);
    }

    #[test]
    fn match_line_keeps_the_match_of_a_long_line() {
        assert_eq!(
            match_line("# Plan\n  Roll out Q3  ", "q3"),
            Some("Roll out Q3".to_string())
        );
        let line = format!("{} needle {}", "x".repeat(300), "y".repeat(300));
        let cut = match_line(&line, "NEEDLE").unwrap();
        assert!(cut.contains("needle") && cut.starts_with('…') && cut.ends_with('…'));
        assert_eq!(cut.chars().count(), MAX_MATCH_LINE_CHARS + 2);
        assert_eq!(match_line("nothing here", "needle"), None);
    }
}
