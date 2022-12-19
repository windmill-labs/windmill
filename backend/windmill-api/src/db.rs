/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sqlx::{Pool, Postgres, Transaction};
use windmill_common::error::Error;

use crate::users::Authed;

pub type DB = Pool<Postgres>;

pub async fn migrate(db: &DB) -> Result<(), Error> {
    match sqlx::migrate!("../migrations").run(db).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }?;

    Ok(())
}

#[derive(Clone)]
pub struct UserDB {
    db: DB,
}

impl UserDB {
    pub fn new(db: DB) -> Self {
        Self { db }
    }

    pub async fn begin(
        self,
        authed: &Authed,
    ) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
        let mut tx = self.db.begin().await?;
        let user = if authed.is_admin {
            "windmill_admin"
        } else {
            "windmill_user"
        };

        sqlx::query(&format!("SET LOCAL ROLE {}", user))
            .execute(&mut tx)
            .await?;

        sqlx::query!(
            "SELECT set_config('session.user', $1, true)",
            authed.username
        )
        .fetch_optional(&mut tx)
        .await?;

        sqlx::query!(
            "SELECT set_config('session.groups', $1, true)",
            &authed.groups.join(",")
        )
        .fetch_optional(&mut tx)
        .await?;

        sqlx::query!(
            "SELECT set_config('session.pgroups', $1, true)",
            &authed
                .groups
                .iter()
                .map(|x| format!("g/{}", x))
                .collect::<Vec<_>>()
                .join(",")
        )
        .fetch_optional(&mut tx)
        .await?;

        let (folders_write, folders_read): &(Vec<_>, Vec<_>) =
            &authed.folders.clone().into_iter().partition(|x| x.1);

        let mut folders_read = folders_read.clone();
        folders_read.extend(folders_write.clone());
        sqlx::query!(
            "SELECT set_config('session.folders_read', $1, true)",
            folders_read
                .iter()
                .map(|x| x.0.clone())
                .collect::<Vec<_>>()
                .join(",")
        )
        .fetch_optional(&mut tx)
        .await?;

        sqlx::query!(
            "SELECT set_config('session.folders_write', $1, true)",
            folders_write
                .iter()
                .map(|x| x.0.clone())
                .collect::<Vec<_>>()
                .join(",")
        )
        .fetch_optional(&mut tx)
        .await?;

        Ok(tx)
    }
}
