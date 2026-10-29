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
    error::{Error, JsonResult},
    DB,
};

pub fn workspaced_service() -> Router {
    Router::new().route("/get", get(get_access))
}

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

async fn get_access(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(AccessQuery { principal }): Query<AccessQuery>,
) -> JsonResult<AccessReport> {
    let (prefix, name) = principal
        .split_once('/')
        .filter(|(_, name)| !name.is_empty())
        .ok_or_else(|| Error::BadRequest("principal must be u/<user> or g/<group>".into()))?;

    // Another principal's list names paths the caller may not be able to read. A group's reach is
    // contained in each member's, so its members may see it.
    let allowed = authed.is_admin
        || match prefix {
            "u" => name == authed.username,
            "g" => authed.groups.iter().any(|g| g == name),
            _ => false,
        };
    if !allowed {
        return Err(Error::PermissionDenied(
            "only workspace admins can view another member's access".into(),
        ));
    }

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
