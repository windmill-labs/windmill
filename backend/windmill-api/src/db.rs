/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Transaction};
use std::time::Duration;
use windmill_common::{error::Error, users::Authed};

pub type DB = Pool<Postgres>;

pub async fn connect(database_url: &str, max_connections: u32) -> Result<DB, Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .max_lifetime(Duration::from_secs(30 * 60)) // 30 mins
        .connect(database_url)
        .await
        .map_err(|err| Error::ConnectingToDatabase(err.to_string()))
}

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
        Ok(tx)
    }
}
