/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sqlx::{Postgres, Transaction};
use windmill_common::{
    error::{self, Error},
    users::SUPERADMIN_SECRET_EMAIL,
};

pub async fn require_super_admin<'c>(
    db: &mut Transaction<'c, Postgres>,
    email: &str,
) -> error::Result<()> {
    if email == SUPERADMIN_SECRET_EMAIL {
        return Ok(());
    }
    let is_admin = sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
        .fetch_one(db)
        .await
        .map_err(|e| Error::InternalErr(format!("fetching super admin: {e}")))?;
    if !is_admin {
        Err(Error::NotAuthorized(
            "This endpoint require caller to be a super admin".to_owned(),
        ))
    } else {
        Ok(())
    }
}
