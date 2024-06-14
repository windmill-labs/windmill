use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    db::Authed,
    error::{Error, Result},
    users::{SUPERADMIN_NOTIFICATION_EMAIL, SUPERADMIN_SECRET_EMAIL, SUPERADMIN_SYNC_EMAIL},
    DB,
};

lazy_static::lazy_static! {
  pub static ref JWT_SECRET : Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));
}

#[derive(Deserialize, Serialize)]
pub struct JWTAuthClaims {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    pub folders: Vec<(String, bool, bool)>,
    pub label: Option<String>,
    pub workspace_id: String,
    pub exp: usize,
}

#[derive(Deserialize)]
pub struct JobPerms {
    pub workspace_id: String,
    pub job_id: String,
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    pub folders: Vec<serde_json::Value>,
    pub created_at: chrono::NaiveDateTime,
}

impl From<JobPerms> for Authed {
    fn from(value: JobPerms) -> Self {
        Self {
            email: value.email,
            username: value.username,
            is_admin: value.is_admin,
            is_operator: value.is_operator,
            groups: value.groups,
            folders: value
                .folders
                .into_iter()
                .filter_map(|x| serde_json::from_value::<(String, bool, bool)>(x).ok())
                .collect(),
            scopes: None,
        }
    }
}

pub async fn is_super_admin_email(db: &DB, email: &str) -> Result<bool> {
    if email == SUPERADMIN_SECRET_EMAIL || email == SUPERADMIN_NOTIFICATION_EMAIL {
        return Ok(true);
    }

    let is_admin = sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
        .fetch_optional(db)
        .await
        .map_err(|e| Error::InternalErr(format!("fetching super admin: {e:#}")))?
        .unwrap_or(false);

    Ok(is_admin)
}

pub fn permissioned_as_to_username(permissioned_as: &str) -> String {
    if let Some((prefix, name)) = permissioned_as.split_once('/') {
        if prefix == "u" {
            name.to_string()
        } else {
            format!("group-{}", name)
        }
    } else {
        permissioned_as.to_string()
    }
}

pub async fn fetch_authed_from_permissioned_as(
    permissioned_as: String,
    email: String,
    w_id: &str,
    db: &DB,
) -> Result<Authed> {
    let super_admin =
        permissioned_as == SUPERADMIN_SYNC_EMAIL || is_super_admin_email(db, &email).await?;
    if let Some((prefix, name)) = permissioned_as.split_once('/') {
        if prefix == "u" {
            let (is_admin, is_operator) = if super_admin {
                (true, false)
            } else {
                let r = sqlx::query!(
                    "SELECT is_admin, operator FROM usr where username = $1 AND \
                                                 workspace_id = $2 AND disabled = false",
                    name,
                    &w_id
                )
                .fetch_one(db)
                .await
                .ok();
                if let Some(r) = r {
                    (r.is_admin, r.operator)
                } else {
                    (false, true)
                }
            };

            let groups = get_groups_for_user(w_id, &name, &email, db).await?;

            let folders = get_folders_for_user(w_id, &name, &groups, db).await?;

            Ok(Authed {
                email: email,
                username: name.to_string(),
                is_admin,
                is_operator,
                groups,
                folders,
                scopes: None,
            })
        } else {
            let groups = vec![name.to_string()];
            let folders = get_folders_for_user(&w_id, "", &groups, db).await?;
            Ok(Authed {
                email: email,
                username: format!("group-{name}"),
                is_admin: false,
                groups,
                is_operator: false,
                folders,
                scopes: None,
            })
        }
    } else {
        let groups = vec![];
        let folders = vec![];
        Ok(Authed {
            email: email,
            username: permissioned_as,
            is_admin: super_admin,
            is_operator: true,
            groups,
            folders,
            scopes: None,
        })
    }
}

pub async fn get_folders_for_user(
    w_id: &str,
    username: &str,
    groups: &[String],
    db: &DB,
) -> Result<Vec<(String, bool, bool)>> {
    let mut perms = groups
        .into_iter()
        .map(|x| format!("g/{}", x))
        .collect::<Vec<_>>();
    perms.insert(0, format!("u/{}", username));
    let folders = sqlx::query!(
        "SELECT name, (EXISTS (SELECT 1 FROM (SELECT key, value FROM jsonb_each_text(extra_perms) WHERE key = ANY($1)) t  WHERE value::boolean IS true)) as write, $1 && owners::text[] as owner  FROM folder
        WHERE extra_perms ?| $1  AND workspace_id = $2",
        &perms[..],
        w_id,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|x| (x.name, x.write.unwrap_or(false), x.owner.unwrap_or(false)))
    .collect();

    Ok(folders)
}

pub async fn get_groups_for_user(
    w_id: &str,
    username: &str,
    email: &str,
    db: &DB,
) -> Result<Vec<String>> {
    let groups = sqlx::query_scalar!(
        "SELECT group_ FROM usr_to_group where usr = $1 AND workspace_id = $2 UNION ALL SELECT igroup FROM email_to_igroup WHERE email = $3",
        username,
        w_id,
        email
    )
    .fetch_all(db)
    .await?
    .into_iter().filter_map(|x| x)
    .collect();
    Ok(groups)
}
