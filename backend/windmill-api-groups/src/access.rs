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
    routing::{get, post},
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
        .route("/deployers", get(get_deployers))
        .route("/set_deployers", post(set_deployers))
        .route("/deploy_as", get(deploy_as))
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
}

#[derive(Serialize)]
struct ItemAccess {
    kind: String,
    path: String,
    write: bool,
    reasons: Vec<Reason>,
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

fn items_query() -> String {
    let union = ITEM_TABLES
        .iter()
        .map(|(kind, table, filter)| {
            format!(
                "SELECT '{kind}'::text AS kind, path::text AS path, extra_perms FROM {table} \
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
        ItemAccess { kind: r.kind, path: r.path, write, reasons }
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
            "only workspace admins can view another member's access".into(),
        ));
    }
    Ok((prefix, name))
}

async fn get_access(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(AccessQuery { principal }): Query<AccessQuery>,
) -> JsonResult<AccessReport> {
    let (prefix, name) = check_principal(&authed, &principal)?;

    let email = match prefix {
        "u" => {
            let usr = sqlx::query!(
                "SELECT email, disabled FROM usr WHERE workspace_id = $1 AND username = $2",
                &w_id,
                name
            )
            .fetch_optional(&db)
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
            let exists = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM group_ WHERE workspace_id = $1 AND name = $2)",
                &w_id,
                name
            )
            .fetch_one(&db)
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

    let target = fetch_authed_from_permissioned_as(&principal, &email, &w_id, &db).await?;

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
        "SELECT name, ARRAY(SELECT k FROM jsonb_object_keys(extra_perms) k WHERE k = ANY($3) ORDER BY k) AS \"via!\"
         FROM folder WHERE workspace_id = $1 AND name = ANY($2)",
        &w_id,
        &folder_names,
        &perms
    )
    .fetch_all(&db)
    .await?
    .into_iter()
    .map(|r| (r.name, r.via))
    .collect::<std::collections::HashMap<_, _>>();
    let mut folders = target
        .folders
        .iter()
        .map(|(name, write, owner)| FolderAccess {
            name: name.clone(),
            write: *write,
            owner: *owner,
            via: via.remove(name).unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    folders.sort_by(|a, b| a.name.cmp(&b.name));

    let mut tx = user_db.begin(&target).await?;
    let items = sqlx::query_as::<_, ItemRow>(&items_query())
        .bind(&w_id)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(ItemAccess::from)
        .collect();
    tx.commit().await?;

    let mut groups = target.groups;
    groups.sort();
    Ok(Json(AccessReport {
        principal,
        is_admin: false,
        groups,
        folders,
        items,
    }))
}

#[derive(Serialize, sqlx::FromRow)]
struct BrowseFolder {
    name: String,
    /// The principal's own entry in the folder's `extra_perms`: absent, read (false) or write.
    grant: Option<bool>,
    /// Owners always keep write; `acls/add` refuses to lower it.
    target_owner: bool,
    #[sqlx(skip)]
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
    format!("Only workspace admins and owners of f/{folder} can share it")
}

async fn browse_folders(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(AccessQuery { principal }): Query<AccessQuery>,
) -> JsonResult<Vec<BrowseFolder>> {
    check_principal(&authed, &principal)?;
    let mut tx = user_db.begin(&authed).await?;
    let mut folders = sqlx::query_as::<_, BrowseFolder>(
        "SELECT name, (extra_perms ->> $2)::boolean AS \"grant\", $2 = ANY(owners) AS target_owner
         FROM folder WHERE workspace_id = $1 ORDER BY name",
    )
    .bind(&w_id)
    .bind(&principal)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    for folder in &mut folders {
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
        .filter(|(kind, ..)| q.kind.as_deref().map_or(true, |k| k == *kind))
        .collect::<Vec<_>>();
    if tables.is_empty() {
        return Err(Error::BadRequest("unknown kind".into()));
    }
    // SAFETY: table names and filters come from the `ITEM_TABLES` constant, never from the request.
    let union = tables
        .iter()
        .map(|(kind, table, filter)| {
            format!(
                "SELECT '{kind}'::text AS kind, path::text AS path, (extra_perms ->> $2)::boolean AS \"grant\"
                 FROM {table} WHERE workspace_id = $1 {filter}"
            )
        })
        .collect::<Vec<_>>()
        .join(" UNION ALL ");
    let sql = format!(
        "SELECT kind, path, \"grant\" FROM ({union}) i
         WHERE ($3::text IS NULL OR starts_with(path, $3))
            AND ($4::text IS NULL OR strpos(lower(path), lower($4)) > 0)
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
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    let has_more = items.len() as i64 > BROWSE_PAGE;
    items.truncate(BROWSE_PAGE as usize);

    // Operators may be barred from managing schedules and triggers, `/acls` included.
    let mut manage_denied = vec![];
    if !authed.is_admin {
        for kind in [ManageKind::Schedules, ManageKind::Triggers] {
            if let Err(e) = check_operator_can_manage(&db, &w_id, &authed, kind.clone()).await {
                manage_denied.push((kind, e.to_string()));
            }
        }
    }
    for item in &mut items {
        let manage_kind = match item.kind.as_str() {
            "schedule" => Some(ManageKind::Schedules),
            k if k.ends_with("_trigger") => Some(ManageKind::Triggers),
            _ => None,
        };
        item.blocked = manage_kind
            .and_then(|mk| {
                manage_denied
                    .iter()
                    .find(|(k, _)| *k == mk)
                    .map(|(_, e)| e.clone())
            })
            .or_else(|| grant_blocked(&authed, &item.path));
    }
    Ok(Json(BrowseItems { items, has_more }))
}

#[derive(Serialize)]
struct Deployers {
    deployers: Vec<String>,
}

#[derive(Deserialize)]
struct SetDeployers {
    principal: String,
    deployers: Vec<String>,
}

#[derive(Serialize)]
struct DeployAsAccount {
    username: String,
    email: String,
}

#[derive(Serialize)]
struct DeployAs {
    /// Admins and `wm_deployers` may deploy as anyone, not only the accounts listed.
    anyone: bool,
    service_accounts: Vec<DeployAsAccount>,
}

fn service_account_name(principal: &str) -> Result<&str> {
    principal
        .strip_prefix("u/")
        .filter(|name| !name.is_empty())
        .ok_or_else(|| Error::BadRequest("principal must be u/<service account>".into()))
}

async fn get_deployers(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(AccessQuery { principal }): Query<AccessQuery>,
) -> JsonResult<Deployers> {
    check_principal(&authed, &principal)?;
    let username = service_account_name(&principal)?;
    let deployers = sqlx::query_scalar!(
        "SELECT deployers FROM usr WHERE workspace_id = $1 AND username = $2 AND is_service_account",
        &w_id,
        username
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("no service account {principal}")))?;
    Ok(Json(Deployers { deployers }))
}

/// Admins only: an entry hands its holders everything the account can reach.
async fn set_deployers(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(body): Json<SetDeployers>,
) -> Result<String> {
    if !authed.is_admin {
        return Err(Error::PermissionDenied(
            "only workspace admins can choose who deploys as a service account".into(),
        ));
    }
    let username = service_account_name(&body.principal)?;
    let mut deployers = body.deployers;
    deployers.sort();
    deployers.dedup();
    for entry in &deployers {
        let exists = match entry.split_once('/') {
            Some(("u", name)) => {
                sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND username = $2)",
                    &w_id,
                    name
                )
                .fetch_one(&db)
                .await?
            }
            Some(("g", name)) => {
                sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT 1 FROM group_ WHERE workspace_id = $1 AND name = $2)",
                    &w_id,
                    name
                )
                .fetch_one(&db)
                .await?
            }
            Some(("f", name)) => {
                sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT 1 FROM folder WHERE workspace_id = $1 AND name = $2)",
                    &w_id,
                    name
                )
                .fetch_one(&db)
                .await?
            }
            _ => {
                return Err(Error::BadRequest(format!(
                    "{entry}: entries must be u/<user>, g/<group> or f/<folder>"
                )))
            }
        };
        if exists != Some(true) {
            return Err(Error::BadRequest(format!(
                "{entry} does not exist in {w_id}"
            )));
        }
    }
    let mut tx = db.begin().await?;
    let updated = sqlx::query_scalar!(
        "UPDATE usr SET deployers = $3 WHERE workspace_id = $1 AND username = $2
         AND is_service_account RETURNING username",
        &w_id,
        username,
        &deployers
    )
    .fetch_optional(&mut *tx)
    .await?;
    if updated.is_none() {
        return Err(Error::NotFound(format!(
            "no service account {}",
            body.principal
        )));
    }
    windmill_audit::audit_oss::audit_log(
        &mut *tx,
        &authed,
        "users.set_deployers",
        windmill_audit::ActionKind::Update,
        &w_id,
        Some(&body.principal),
        Some([("deployers", deployers.join(",").as_str())].into()),
    )
    .await?;
    tx.commit().await?;
    Ok(format!("deployers of {} updated", body.principal))
}

/// The service accounts the caller may deploy as, for the run-as pickers.
async fn deploy_as(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<DeployAs> {
    let anyone = windmill_common::can_preserve_on_behalf_of(&authed);
    let rows = sqlx::query!(
        "SELECT username, email, deployers FROM usr
         WHERE workspace_id = $1 AND is_service_account AND NOT disabled ORDER BY username",
        &w_id
    )
    .fetch_all(&db)
    .await?;
    let service_accounts = rows
        .into_iter()
        .filter(|r| anyone || windmill_common::deployers_admit(&r.deployers, &authed))
        .map(|r| DeployAsAccount { username: r.username, email: r.email })
        .collect();
    Ok(Json(DeployAs { anyone, service_accounts }))
}
