/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use serde_json::json;
use windmill_common::{
    error::{self, to_anyhow, Error, Result},
    users::SUPERADMIN_SECRET_EMAIL,
    DB,
};

use crate::GIT_VERSION;

pub async fn require_super_admin(db: &DB, email: &str) -> error::Result<()> {
    if email == SUPERADMIN_SECRET_EMAIL {
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

pub async fn reporting_stats(
    instance_name: &String,
    http_client: &reqwest::Client,
    db: &DB,
) -> Result<()> {
    let uid = sqlx::query_scalar!("SELECT value FROM global_settings WHERE name = 'uid'")
        .fetch_one(db)
        .await?;

    let uid = serde_json::from_value::<String>(uid).map_err(to_anyhow)?;

    let payload = json!({
        "uid": uid,
        "version": GIT_VERSION,
        "instance_name": instance_name,
    });

    let request = http_client
        .post("https://hub.windmill.dev/stats")
        .body(serde_json::to_string(&payload).map_err(to_anyhow)?)
        .header("content-type", "application/json");

    request
        .send()
        .await
        .map_err(to_anyhow)?
        .error_for_status()
        .map_err(to_anyhow)?;

    Ok(())
}
