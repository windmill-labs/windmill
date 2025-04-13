use anyhow::Context;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::Authed,
    error::{Error, Result},
    jwt,
    users::{SUPERADMIN_NOTIFICATION_EMAIL, SUPERADMIN_SECRET_EMAIL, SUPERADMIN_SYNC_EMAIL},
    DB,
};

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
    pub job_id: Option<String>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct JobPerms {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    pub folders: Vec<serde_json::Value>,
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
        .map_err(|e| Error::internal_err(format!("fetching super admin: {e:#}")))?
        .unwrap_or(false);

    Ok(is_admin)
}

pub async fn is_devops_email(db: &DB, email: &str) -> Result<bool> {
    if is_super_admin_email(db, email).await? {
        return Ok(true);
    }

    let is_devops = sqlx::query_scalar!("SELECT devops FROM password WHERE email = $1", email)
        .fetch_optional(db)
        .await
        .map_err(|e| Error::internal_err(format!("fetching super admin: {e:#}")))?
        .unwrap_or(false);

    Ok(is_devops)
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
                .fetch_optional(db)
                .await?;
                if let Some(r) = r {
                    (r.is_admin, r.operator)
                } else {
                    return Err(Error::internal_err(format!(
                        "user {name} not found in workspace {w_id}"
                    )));
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

#[tracing::instrument(level = "trace", skip_all)]
pub async fn create_token_for_owner(
    db: &DB,
    w_id: &str,
    owner: &str,
    label: &str,
    expires_in: u64,
    email: &str,
    job_id: &Uuid,
    perms: Option<JobPerms>,
) -> crate::error::Result<String> {
    let job_perms = if perms.is_some() {
        Ok(perms)
    } else {
        sqlx::query_as!(
            JobPerms,
            "SELECT email, username, is_admin, is_operator, groups, folders FROM job_perms WHERE job_id = $1 AND workspace_id = $2",
            job_id,
            w_id
        )
        .fetch_optional(db)
        .await
    };
    let job_authed = match job_perms {
        Ok(Some(jp)) => jp.into(),
        _ => {
            tracing::warn!("Could not get permissions for job {job_id} from job_perms table, getting permissions directly...");
            fetch_authed_from_permissioned_as(owner.to_string(), email.to_string(), w_id, db)
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
                        "Could not get permissions directly for job {job_id}: {e:#}"
                    ))
                })?
        }
    };

    let payload = JWTAuthClaims {
        email: job_authed.email,
        username: job_authed.username,
        is_admin: job_authed.is_admin,
        is_operator: job_authed.is_operator,
        groups: job_authed.groups,
        folders: job_authed.folders,
        label: Some(label.to_string()),
        workspace_id: w_id.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64)).timestamp()
            as usize,
        job_id: Some(job_id.to_string()),
        scopes: None,
    };

    let token = jwt::encode_with_internal_secret(&payload)
        .await
        .with_context(|| format!("Could not encode JWT token for job {job_id}"))?;

    Ok(format!("jwt_{}", token))
}
