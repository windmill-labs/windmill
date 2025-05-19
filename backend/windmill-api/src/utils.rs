/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::fmt::Display;

use axum::{body::Body, response::Response};
use regex::Regex;
use serde::{Deserialize, Deserializer};
use sqlx::{Postgres, Transaction};
#[cfg(feature = "enterprise")]
use windmill_common::worker::CLOUD_HOSTED;
use windmill_common::{
    auth::{is_devops_email, is_super_admin_email},
    error::{self, Error},
    DB,
};

#[cfg(feature = "enterprise")]
use windmill_common::error::JsonResult;

#[cfg(feature = "enterprise")]
use axum::Json;

#[derive(Deserialize)]
pub struct WithStarredInfoQuery {
    pub with_starred_info: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunnableKind {
    Script,
    Flow,
}

impl Display for RunnableKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let runnable_kind = match self {
            RunnableKind::Script => "script",
            RunnableKind::Flow => "flow"
        };
        write!(f, "{}", runnable_kind)
    }
}

pub async fn require_super_admin(db: &DB, email: &str) -> error::Result<()> {
    let is_admin = is_super_admin_email(db, email).await?;

    if !is_admin {
        Err(Error::NotAuthorized(
            "This endpoint requires the caller to be a super admin".to_owned(),
        ))
    } else {
        Ok(())
    }
}

pub async fn require_devops_role(db: &DB, email: &str) -> error::Result<()> {
    let is_devops = is_devops_email(db, email).await?;

    if is_devops {
        Ok(())
    } else {
        Err(Error::NotAuthorized(
            "This endpoint requires the caller to have the `devops` role".to_string(),
        ))
    }
}

lazy_static::lazy_static! {
    pub static ref INVALID_USERNAME_CHARS: Regex = Regex::new(r"[^A-Za-z0-9_]").unwrap();
}

pub async fn generate_instance_wide_unique_username<'c>(
    tx: &mut Transaction<'c, Postgres>,
    email: &str,
) -> error::Result<String> {
    let mut username = email.split('@').next().unwrap().to_string();

    username = INVALID_USERNAME_CHARS
        .replace_all(&mut username, "")
        .to_string();

    if username.is_empty() {
        username = "user".to_string()
    }

    let base_username = username.clone();
    let mut username_conflict = true;
    let mut i = 1;
    while username_conflict {
        if i > 1000 {
            return Err(Error::internal_err(format!(
                "too many username conflicts for {}",
                email
            )));
        }
        if i > 1 {
            username = format!("{}{}", base_username, i)
        }
        username_conflict = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 and email != $2 UNION SELECT 1 FROM password WHERE username = $1 UNION SELECT 1 FROM pending_user WHERE username = $1)",
                &username,
                &email
            )
            .fetch_one(&mut **tx)
            .await?
            .unwrap_or(false);
        i += 1;
    }

    Ok(username)
}

pub async fn generate_instance_username_for_all_users(db: &DB) -> error::Result<()> {
    let mut tx = db.begin().await?;
    // get users that have a no instance username and either 1 or 0 workspace usernames
    let users = sqlx::query!(r#"SELECT p.email as "email!", u.username as "username?" FROM password p LEFT JOIN usr u ON p.email = u.email WHERE p.username IS NULL AND (SELECT COUNT(DISTINCT username) FROM usr WHERE email = p.email) <= 1"#)
        .fetch_all(&mut *tx)
        .await?;

    for user in users {
        let username = if let Some(username) = user.username {
            // if has workspace username, check that username is unique
            let username_conflict = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 and email != $2 UNION SELECT 1 FROM password WHERE username = $1 UNION SELECT 1 FROM pending_user WHERE username = $1)",
                &username,
                &user.email
            ).fetch_one(&mut *tx).await?.unwrap_or(false);

            if !username_conflict {
                username
            } else {
                generate_instance_wide_unique_username(&mut tx, &user.email).await?
            }
        } else {
            generate_instance_wide_unique_username(&mut tx, &user.email).await?
        };

        sqlx::query!(
            "UPDATE password SET username = $1 WHERE email = $2",
            &username,
            &user.email
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn get_instance_username_or_create_pending<'c>(
    tx: &mut Transaction<'c, Postgres>,
    email: &str,
) -> error::Result<String> {
    let user = sqlx::query_scalar!("SELECT username FROM password WHERE email = $1", email)
        .fetch_optional(&mut **tx)
        .await?;

    if let Some(opt_username) = user {
        if let Some(username) = opt_username {
            Ok(username)
        } else {
            Err(Error::BadRequest(format!("No instance-wide username found for {email}. The user has different usernames for different workspaces. Ask the instance administrator to solve the conflict in the instance settings.")))
        }
    } else {
        let pending_username =
            sqlx::query_scalar!("SELECT username FROM pending_user WHERE email = $1", email)
                .fetch_optional(&mut **tx)
                .await?;

        if let Some(username) = pending_username {
            Ok(username)
        } else {
            let username = generate_instance_wide_unique_username(&mut *tx, email).await?;

            sqlx::query!(
                "INSERT INTO pending_user (email, username) VALUES ($1, $2)",
                email,
                username
            )
            .execute(&mut **tx)
            .await
            .map_err(|e| Error::internal_err(format!("creating pending user: {e:#}")))?;

            Ok(username)
        }
    }
}

pub fn content_plain(body: Body) -> Response {
    use axum::http::header;
    Response::builder()
        .header(header::CONTENT_TYPE, "text/plain")
        .body(body)
        .unwrap()
}

#[allow(unused)]
pub fn non_empty_str<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let o: Option<String> = Option::deserialize(deserializer)?;
    Ok(o.filter(|s| !s.trim().is_empty()))
}

use serde::Serialize;

#[derive(Serialize)]
pub struct CriticalAlert {
    id: i32,
    alert_type: String,
    message: String,
    created_at: chrono::DateTime<chrono::Utc>,
    acknowledged: Option<bool>,
    workspace_id: Option<String>,
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize, Debug)]
pub struct AlertQueryParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub acknowledged: Option<bool>,
}

#[cfg(feature = "enterprise")]
pub async fn get_critical_alerts(
    db: DB,
    params: AlertQueryParams,
    workspace_id: Option<String>,
) -> JsonResult<serde_json::Value> {
    // Returning total rows and total pages
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(10).min(100) as i64;
    let offset = ((page - 1) * page_size as i32) as i64;

    // Count total rows
    let total_rows = if let Some(workspace_id) = &workspace_id {
        if params.acknowledged.is_none() {
            sqlx::query_scalar!(
                "SELECT COUNT(*)
                 FROM alerts
                 WHERE workspace_id = $1",
                workspace_id
            )
            .fetch_one(&db)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*)
                 FROM alerts
                 WHERE workspace_id = $1 AND COALESCE(acknowledged_workspace, false) = $2",
                workspace_id,
                params.acknowledged
            )
            .fetch_one(&db)
            .await?
        }
    } else {
        if params.acknowledged.is_none() {
            sqlx::query_scalar!(
                "SELECT COUNT(*)
                 FROM alerts"
            )
            .fetch_one(&db)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*)
                 FROM alerts
                 WHERE COALESCE(acknowledged, false) = $1",
                params.acknowledged
            )
            .fetch_one(&db)
            .await?
        }
    };

    // Fetch paginated rows
    let alerts = if let Some(workspace_id) = workspace_id {
        // `workspace_id` is provided => workspace admin
        if params.acknowledged.is_none() {
            // Case: return all rows where `workspace_id` matches
            sqlx::query_as!(
                CriticalAlert,
                "SELECT id, alert_type, message, created_at, COALESCE(acknowledged_workspace, false) AS acknowledged, workspace_id
                 FROM alerts
                 WHERE workspace_id = $1
                 ORDER BY created_at DESC
                 LIMIT $2 OFFSET $3",
                workspace_id,
                page_size,
                offset
            )
            .fetch_all(&db)
            .await?
        } else {
            // Case: return rows where `acknowledged_workspace` matches `params.acknowledged`
            sqlx::query_as!(
                CriticalAlert,
                "SELECT id, alert_type, message, created_at, COALESCE(acknowledged_workspace, false) AS acknowledged, workspace_id
                 FROM alerts
                 WHERE workspace_id = $1 AND COALESCE(acknowledged_workspace, false) = $2
                 ORDER BY created_at DESC
                 LIMIT $3 OFFSET $4",
                workspace_id,
                params.acknowledged,
                page_size,
                offset
            )
            .fetch_all(&db)
            .await?
        }
    } else {
        // `workspace_id` is not provided => superadmin
        if params.acknowledged.is_none() {
            // Case: Return all rows unfiltered with global acknowledged as acknowledged
            sqlx::query_as!(
                CriticalAlert,
                "SELECT id, alert_type, message, created_at, COALESCE(acknowledged, false) AS acknowledged, workspace_id
                 FROM alerts
                 ORDER BY created_at DESC
                 LIMIT $1 OFFSET $2",
                page_size,
                offset
            )
            .fetch_all(&db)
            .await?
        } else {
            // Case: Return rows where global acknowledged matches params.acknowledged
            sqlx::query_as!(
                CriticalAlert,
                "SELECT id, alert_type, message, created_at, COALESCE(acknowledged, false) AS acknowledged, workspace_id
                 FROM alerts
                 WHERE COALESCE(acknowledged, false) = $1
                 ORDER BY created_at DESC
                 LIMIT $2 OFFSET $3",
                params.acknowledged,
                page_size,
                offset
            )
            .fetch_all(&db)
            .await?
        }
    };

    let total_rows = total_rows.unwrap_or(0);
    let total_pages = ((total_rows as f64) / (page_size as f64)).ceil() as i64;

    Ok(Json(serde_json::json!({
        "alerts": alerts,
        "total_rows": total_rows,
        "total_pages": total_pages
    })))
}

#[cfg(feature = "enterprise")]
pub async fn acknowledge_critical_alert(
    db: DB,
    workspace_id: Option<String>,
    id: i32,
) -> error::Result<String> {
    sqlx::query!(
        "UPDATE alerts
         SET
           acknowledged = true,
           acknowledged_workspace = CASE
             WHEN $3 THEN
               CASE
                 WHEN $2::text IS NOT NULL AND workspace_id = $2 THEN true
                 ELSE acknowledged_workspace
               END
             ELSE true
           END
         WHERE id = $1",
        id,
        workspace_id,
        *CLOUD_HOSTED
    )
    .execute(&db)
    .await?;

    tracing::info!(
        "Acknowledged critical alert with id: {}{}",
        id,
        workspace_id.map_or_else(|| "".to_string(), |w| format!(" for workspace_id: {}", w))
    );
    Ok("Critical alert acknowledged".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn acknowledge_all_critical_alerts(
    db: DB,
    workspace_id: Option<String>,
) -> error::Result<String> {
    sqlx::query!(
        "UPDATE alerts 
         SET
           acknowledged = true,
           acknowledged_workspace = CASE
             WHEN $2 THEN
               CASE
                 WHEN $1::text IS NOT NULL THEN true
                 ELSE acknowledged_workspace
               END
             ELSE true
           END
         WHERE ($1::text IS NOT NULL AND workspace_id = $1)
            OR ($1::text IS NULL)",
        workspace_id,
        *CLOUD_HOSTED
    )
    .execute(&db)
    .await?;

    tracing::info!(
        "Acknowledged all unacknowledged critical alerts{}",
        workspace_id.map_or_else(|| "".to_string(), |w| format!(" for workspace_id: {}", w))
    );
    Ok("All unacknowledged critical alerts acknowledged".to_string())
}

#[cfg(feature = "http_trigger")]
#[derive(Clone)]
pub struct ExpiringCacheEntry<T> {
    pub value: T,
    pub expiry: std::time::Instant,
}
