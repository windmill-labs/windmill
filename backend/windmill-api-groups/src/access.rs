/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! What a workspace member or group can reach, and why.

use axum::{
    extract::{Extension, Path, Query},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use windmill_api_auth::ApiAuthed;
use windmill_common::{
    auth::fetch_authed_from_permissioned_as,
    db::UserDB,
    error::{Error, JsonResult, Result},
    workspaces::{check_operator_can_manage, ManageKind},
    DB,
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/get", get(get_access))
        .route("/browse_folders", get(browse_folders))
        .route("/browse_items", get(browse_items))
}

const BROWSE_PAGE: i64 = 100;

/// `(kind, table, extra filter)`. Captures, capture configs and eval datasets are left out on
/// purpose: they are test data attached to an item, not something access is granted to.
const ITEM_TABLES: [(&str, &str, &str); 17] = [
    (
        "script",
        "script",
        "AND archived = false AND deleted = false",
    ),
    ("flow", "flow", "AND archived = false"),
    ("app", "app", ""),
    ("resource", "resource", ""),
    ("variable", "variable", ""),
    ("schedule", "schedule", ""),
    ("http_trigger", "http_trigger", ""),
    ("websocket_trigger", "websocket_trigger", ""),
    ("kafka_trigger", "kafka_trigger", ""),
    ("nats_trigger", "nats_trigger", ""),
    ("postgres_trigger", "postgres_trigger", ""),
    ("mqtt_trigger", "mqtt_trigger", ""),
    ("amqp_trigger", "amqp_trigger", ""),
    ("gcp_trigger", "gcp_trigger", ""),
    ("azure_trigger", "azure_trigger", ""),
    ("sqs_trigger", "sqs_trigger", ""),
    ("email_trigger", "email_trigger", ""),
];

#[derive(Deserialize)]
struct AccessQuery {
    /// `u/<username>` or `g/<group>`.
    principal: String,
}

#[derive(Serialize)]
struct AccessReport {
    principal: String,
    is_admin: bool,
    groups: Vec<String>,
    folders: Vec<FolderAccess>,
    items: Vec<ItemAccess>,
}

#[derive(Serialize)]
struct FolderAccess {
    name: String,
    write: bool,
    owner: bool,
    /// The `extra_perms` keys (`u/<user>` or `g/<group>`) that grant the folder.
    via: Vec<String>,
    /// The principal's own entry in `extra_perms`: absent when none, false for read, true for write.
    #[serde(skip_serializing_if = "Option::is_none")]
    grant: Option<bool>,
    /// Why the caller cannot change the principal's own entry. Set only where there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    blocked: Option<String>,
}

#[derive(Serialize)]
struct ItemAccess {
    kind: String,
    path: String,
    write: bool,
    reasons: Vec<Reason>,
    /// Why the caller cannot change the principal's own grant. Set only where there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    blocked: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Reason {
    /// Under the principal's own `u/<user>/` path.
    Own,
    /// Under `g/<group>/` for a group the principal is in.
    GroupPath { group: String },
    /// Shared with the principal directly.
    User { write: bool },
    /// Shared with one of the principal's groups.
    Group { group: String, write: bool },
    /// In a folder the principal can read.
    Folder { folder: String, write: bool },
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    kind: String,
    path: String,
    own: bool,
    path_group: Option<String>,
    user_grant: Option<bool>,
    group_read: Vec<String>,
    group_write: Vec<String>,
    folder: Option<String>,
    folder_write: bool,
}

/// The `kind` column for a row of `table`. Full-code apps live in `app` too, but their grants must
/// be written with kind `raw_app`, which git sync dispatches to `<path>.raw_app` rather than `.app`.
fn kind_column(kind: &str, table: &str) -> String {
    if kind == "app" {
        format!(
            "CASE WHEN (SELECT raw_app FROM app_version \
                 WHERE id = {table}.versions[array_upper({table}.versions, 1)]) \
             THEN 'raw_app' ELSE 'app' END"
        )
    } else {
        format!("'{kind}'::text")
    }
}

fn items_query() -> String {
    let union = ITEM_TABLES
        .iter()
        .map(|(kind, table, filter)| {
            let kind = kind_column(kind, table);
            format!(
                "SELECT {kind} AS kind, path::text AS path, extra_perms FROM {table} \
                 WHERE workspace_id = $1 {filter}"
            )
        })
        .collect::<Vec<_>>()
        .join(" UNION ALL ");
    // Rows come back through the principal's RLS session, so the set is what their jobs can read;
    // the reasons below restate the same policies (see `pg_policies`) from the same session values.
    format!(
        "WITH s AS (
            SELECT current_setting('session.user') AS usr,
                regexp_split_to_array(current_setting('session.groups'), ',') AS groups,
                regexp_split_to_array(current_setting('session.pgroups'), ',') AS pgroups,
                regexp_split_to_array(current_setting('session.folders_read'), ',') AS fr,
                regexp_split_to_array(current_setting('session.folders_write'), ',') AS fw
        ), items AS ({union})
        SELECT i.kind, i.path,
            (split_part(i.path, '/', 1) = 'u' AND split_part(i.path, '/', 2) = s.usr) AS own,
            CASE WHEN split_part(i.path, '/', 1) = 'g' AND split_part(i.path, '/', 2) = ANY(s.groups)
                THEN split_part(i.path, '/', 2) END AS path_group,
            (i.extra_perms ->> ('u/' || s.usr))::boolean AS user_grant,
            ARRAY(SELECT substr(e.key, 3) FROM jsonb_each_text(i.extra_perms) e
                WHERE e.key = ANY(s.pgroups) ORDER BY e.key) AS group_read,
            ARRAY(SELECT substr(e.key, 3) FROM jsonb_each_text(i.extra_perms) e
                WHERE e.key = ANY(s.pgroups) AND e.value::boolean ORDER BY e.key) AS group_write,
            CASE WHEN split_part(i.path, '/', 1) = 'f' AND split_part(i.path, '/', 2) = ANY(s.fr)
                THEN split_part(i.path, '/', 2) END AS folder,
            (split_part(i.path, '/', 1) = 'f' AND split_part(i.path, '/', 2) = ANY(s.fw)) AS folder_write
        FROM items i CROSS JOIN s
        ORDER BY i.path, i.kind"
    )
}

impl From<ItemRow> for ItemAccess {
    fn from(r: ItemRow) -> Self {
        let mut reasons = vec![];
        if r.own {
            reasons.push(Reason::Own);
        }
        if let Some(group) = r.path_group {
            reasons.push(Reason::GroupPath { group });
        }
        if let Some(write) = r.user_grant {
            reasons.push(Reason::User { write });
        }
        for group in r.group_read {
            let write = r.group_write.contains(&group);
            reasons.push(Reason::Group { group, write });
        }
        if let Some(folder) = r.folder {
            reasons.push(Reason::Folder { folder, write: r.folder_write });
        }
        let write = reasons.iter().any(|r| match r {
            Reason::Own | Reason::GroupPath { .. } => true,
            Reason::User { write } | Reason::Group { write, .. } | Reason::Folder { write, .. } => {
                *write
            }
        });
        ItemAccess { kind: r.kind, path: r.path, write, reasons, blocked: None }
    }
}

/// Splits `u/<user>` or `g/<group>` and refuses callers who may not look at that principal.
fn check_principal<'a>(authed: &ApiAuthed, principal: &'a str) -> Result<(&'a str, &'a str)> {
    let (prefix, name) = principal
        .split_once('/')
        .filter(|(prefix, name)| matches!(*prefix, "u" | "g") && !name.is_empty())
        .ok_or_else(|| Error::BadRequest("principal must be u/<user> or g/<group>".into()))?;

    // Another principal's list names paths the caller may not be able to read. A group's reach is
    // contained in each member's, so its members may see it.
    let allowed = authed.is_admin
        || match prefix {
            "u" => name == authed.username,
            _ => authed.groups.iter().any(|g| g == name),
        };
    if !allowed {
        return Err(Error::PermissionDenied(
            "only workspace admins can view the access of another member or of a group you are not in"
                .into(),
        ));
    }
    Ok((prefix, name))
}

/// The principal as it would be authenticated, with its groups and the folders it reaches.
async fn fetch_target(
    db: &DB,
    w_id: &str,
    prefix: &str,
    name: &str,
    principal: &str,
) -> Result<windmill_common::db::Authed> {
    let email = match prefix {
        "u" => {
            let usr = sqlx::query!(
                "SELECT email, disabled FROM usr WHERE workspace_id = $1 AND username = $2",
                w_id,
                name
            )
            .fetch_optional(db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("user {name} not found")))?;
            if usr.disabled {
                return Err(Error::BadRequest(format!(
                    "{name} is disabled, so nothing runs as them"
                )));
            }
            usr.email
        }
        "g" => {
            // Instance groups are granted through the same `g/<name>` keys, and the callers' check
            // lets their members ask, so a name from either table is a group here.
            let exists = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM group_ WHERE workspace_id = $1 AND name = $2)
                    OR EXISTS(SELECT 1 FROM instance_group WHERE name = $2)",
                w_id,
                name
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false);
            if !exists {
                return Err(Error::NotFound(format!("group {name} not found")));
            }
            String::new()
        }
        _ => {
            return Err(Error::BadRequest(
                "principal must be u/<user> or g/<group>".into(),
            ))
        }
    };

    let mut target = fetch_authed_from_permissioned_as(principal, &email, w_id, db).await?;
    // A workspace group and an instance group can share a name, and both come back.
    target.groups.sort();
    target.groups.dedup();
    Ok(target)
}

async fn get_access(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(AccessQuery { principal }): Query<AccessQuery>,
) -> JsonResult<AccessReport> {
    let (prefix, name) = check_principal(&authed, &principal)?;

    let target = fetch_target(&db, &w_id, prefix, name, &principal).await?;

    if target.is_admin {
        return Ok(Json(AccessReport {
            principal,
            is_admin: true,
            groups: target.groups,
            folders: vec![],
            items: vec![],
        }));
    }

    let mut perms = target
        .groups
        .iter()
        .map(|g| format!("g/{g}"))
        .collect::<Vec<_>>();
    if prefix == "u" {
        perms.insert(0, format!("u/{name}"));
    }
    let folder_names = target
        .folders
        .iter()
        .map(|f| f.0.clone())
        .collect::<Vec<_>>();
    let mut via = sqlx::query!(
        "SELECT name, ARRAY(SELECT k FROM jsonb_object_keys(extra_perms) k WHERE k = ANY($3) ORDER BY k) AS \"via!\",
            (extra_perms ->> $4)::boolean AS \"grant\"
         FROM folder WHERE workspace_id = $1 AND name = ANY($2)",
        &w_id,
        &folder_names,
        &perms,
        &principal
    )
    .fetch_all(&db)
    .await?
    .into_iter()
    .map(|r| (r.name, (r.via, r.grant)))
    .collect::<std::collections::HashMap<_, _>>();
    let mut folders = target
        .folders
        .iter()
        .map(|(name, write, owner)| {
            let (via, grant) = via.remove(name).unwrap_or_default();
            // An owner keeps write whatever its entry says, so there is nothing to change.
            let blocked =
                (grant.is_some() && !*owner && !windmill_api_auth::is_owner(&authed, name))
                    .then(|| folder_blocked(name));
            FolderAccess { name: name.clone(), write: *write, owner: *owner, via, grant, blocked }
        })
        .collect::<Vec<_>>();
    folders.sort_by(|a, b| a.name.cmp(&b.name));

    let mut tx = user_db.begin(&target).await?;
    let rows = sqlx::query_as::<_, ItemRow>(&items_query())
        .bind(&w_id)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    let denied = manage_denied(&db, &w_id, &authed).await;
    let items = rows
        .into_iter()
        .map(|r| {
            let direct = match prefix {
                "u" => r.user_grant.is_some(),
                _ => r.group_read.iter().any(|g| g == name),
            };
            let mut item = ItemAccess::from(r);
            if direct {
                item.blocked = item_blocked(&authed, &denied, &item.kind, &item.path);
            }
            item
        })
        .collect();

    Ok(Json(AccessReport {
        principal,
        is_admin: false,
        groups: target.groups,
        folders,
        items,
    }))
}

#[derive(Serialize)]
struct BrowseFolder {
    name: String,
    /// The principal's own entry in the folder's `extra_perms`: absent, read (false) or write.
    grant: Option<bool>,
    /// Owners always keep write; `acls/add` refuses to lower it.
    target_owner: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    blocked: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
struct BrowseItem {
    kind: String,
    path: String,
    grant: Option<bool>,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    blocked: Option<String>,
}

#[derive(Serialize)]
struct BrowseItems {
    items: Vec<BrowseItem>,
    has_more: bool,
}

#[derive(Deserialize)]
struct BrowseItemsQuery {
    principal: String,
    /// Only paths starting with this, e.g. `f/billing/` or `u/alice/`.
    prefix: Option<String>,
    kind: Option<String>,
    search: Option<String>,
    offset: Option<i64>,
}

/// Why the caller cannot change grants on `path`, mirroring the checks `acls/add` makes.
fn grant_blocked(authed: &ApiAuthed, path: &str) -> Option<String> {
    if authed.is_admin {
        return None;
    }
    let mut parts = path.splitn(3, '/');
    match (parts.next(), parts.next()) {
        (Some("u"), Some(owner)) if owner == authed.username => None,
        (Some("u"), Some(owner)) => {
            Some(format!("Only u/{owner} and workspace admins can share it"))
        }
        (Some("f"), Some(folder)) if windmill_api_auth::is_owner(authed, folder) => None,
        (Some("f"), Some(folder)) => Some(folder_blocked(folder)),
        _ => Some("Only workspace admins can share items under g/".to_string()),
    }
}

fn folder_blocked(folder: &str) -> String {
    format!("Only admins of the workspace or of f/{folder} can share it")
}

async fn browse_folders(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(AccessQuery { principal }): Query<AccessQuery>,
) -> JsonResult<Vec<BrowseFolder>> {
    let (prefix, name) = check_principal(&authed, &principal)?;
    // Ownership can come through one of the principal's groups, as in `/access/get`.
    let target = fetch_target(&db, &w_id, prefix, name, &principal).await?;
    let owned = target
        .folders
        .iter()
        .filter(|(_, _, owner)| *owner)
        .map(|(name, ..)| name.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut tx = user_db.begin(&authed).await?;
    let mut folders = sqlx::query_as!(
        BrowseFolder,
        "SELECT name, (extra_perms ->> $2)::boolean AS \"grant\",
            $2 = ANY(owners) AS \"target_owner!\", NULL::text AS blocked
         FROM folder WHERE workspace_id = $1 ORDER BY name",
        &w_id,
        &principal
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    for folder in &mut folders {
        folder.target_owner |= owned.contains(folder.name.as_str());
        if !windmill_api_auth::is_owner(&authed, &folder.name) {
            folder.blocked = Some(folder_blocked(&folder.name));
        }
    }
    Ok(Json(folders))
}

async fn browse_items(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(q): Query<BrowseItemsQuery>,
) -> JsonResult<BrowseItems> {
    check_principal(&authed, &q.principal)?;
    let tables = ITEM_TABLES
        .iter()
        .filter(|(kind, ..)| {
            q.kind
                .as_deref()
                .map_or(true, |k| k == *kind || (k == "raw_app" && *kind == "app"))
        })
        .collect::<Vec<_>>();
    if tables.is_empty() {
        return Err(Error::BadRequest("unknown kind".into()));
    }
    // SAFETY: table names and filters come from the `ITEM_TABLES` constant, never from the request.
    let union = tables
        .iter()
        .map(|(kind, table, filter)| {
            let kind = kind_column(kind, table);
            format!(
                "SELECT {kind} AS kind, path::text AS path, (extra_perms ->> $2)::boolean AS \"grant\"
                 FROM {table} WHERE workspace_id = $1 {filter}"
            )
        })
        .collect::<Vec<_>>()
        .join(" UNION ALL ");
    let sql = format!(
        "SELECT kind, path, \"grant\" FROM ({union}) i
         WHERE ($3::text IS NULL OR starts_with(path, $3))
            AND ($4::text IS NULL OR strpos(lower(path), lower($4)) > 0)
            AND ($7::text IS NULL OR kind = $7)
         ORDER BY path, kind LIMIT $5 OFFSET $6"
    );
    let mut tx = user_db.begin(&authed).await?;
    let mut items = sqlx::query_as::<_, BrowseItem>(&sql)
        .bind(&w_id)
        .bind(&q.principal)
        .bind(q.prefix.as_deref().filter(|p| !p.is_empty()))
        .bind(q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .bind(BROWSE_PAGE + 1)
        .bind(q.offset.unwrap_or(0).max(0))
        .bind(q.kind.as_deref())
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    let has_more = items.len() as i64 > BROWSE_PAGE;
    items.truncate(BROWSE_PAGE as usize);

    let denied = manage_denied(&db, &w_id, &authed).await;
    for item in &mut items {
        item.blocked = item_blocked(&authed, &denied, &item.kind, &item.path);
    }
    Ok(Json(BrowseItems { items, has_more }))
}

/// Operators may be barred from managing schedules and triggers, `/acls` included.
async fn manage_denied(db: &DB, w_id: &str, authed: &ApiAuthed) -> Vec<(ManageKind, String)> {
    let mut denied = vec![];
    if !authed.is_admin {
        for kind in [ManageKind::Schedules, ManageKind::Triggers] {
            if let Err(e) = check_operator_can_manage(db, w_id, authed, kind).await {
                // Shown as a lock text, a plain sentence like the path rules' texts, so without
                // the "Permission denied: " prefix the error's display adds.
                let reason = match e {
                    Error::PermissionDenied(reason) => reason,
                    e => e.to_string(),
                };
                denied.push((kind, reason));
            }
        }
    }
    denied
}

/// Why the caller cannot change grants on this item: the operator gate first, then the path.
fn item_blocked(
    authed: &ApiAuthed,
    denied: &[(ManageKind, String)],
    kind: &str,
    path: &str,
) -> Option<String> {
    let manage_kind = match kind {
        "schedule" => Some(ManageKind::Schedules),
        k if k.ends_with("_trigger") => Some(ManageKind::Triggers),
        _ => None,
    };
    manage_kind
        .and_then(|mk| {
            denied
                .iter()
                .find(|(k, _)| *k == mk)
                .map(|(_, e)| e.clone())
        })
        .or_else(|| grant_blocked(authed, path))
}
