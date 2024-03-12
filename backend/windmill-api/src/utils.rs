/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use regex::Regex;
use sqlx::{Postgres, Transaction};
use windmill_common::{
    error::{self, Error},
    users::{SUPERADMIN_NOTIFICATION_EMAIL, SUPERADMIN_SECRET_EMAIL},
    DB,
};

pub async fn require_super_admin(db: &DB, email: &str) -> error::Result<()> {
    if email == SUPERADMIN_SECRET_EMAIL || email == SUPERADMIN_NOTIFICATION_EMAIL {
        return Ok(());
    }
    let is_admin = sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
        .fetch_optional(db)
        .await
        .map_err(|e| Error::InternalErr(format!("fetching super admin: {e}")))?
        .unwrap_or(false);

    if !is_admin {
        Err(Error::NotAuthorized(
            "This endpoint require caller to be a super admin".to_owned(),
        ))
    } else {
        Ok(())
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
            return Err(Error::InternalErr(format!(
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
            .map_err(|e| Error::InternalErr(format!("creating pending user: {e}")))?;

            Ok(username)
        }
    }
}

pub async fn get_and_delete_pending_username_or_generate<'c>(
    tx: &mut Transaction<'c, Postgres>,
    email: &str,
) -> error::Result<String> {
    let username = sqlx::query_scalar!("SELECT username FROM pending_user WHERE email = $1", email)
        .fetch_optional(&mut **tx)
        .await?;

    if let Some(username) = username {
        sqlx::query!("DELETE FROM pending_user WHERE email = $1", email)
            .execute(&mut **tx)
            .await?;
        Ok(username)
    } else {
        let username = generate_instance_wide_unique_username(&mut *tx, email).await?;
        Ok(username)
    }
}
