/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sqlx::{Pool, Postgres};
use windmill_common::{
    db::{Authable, Authed},
    error::Error,
};

pub type DB = Pool<Postgres>;

pub async fn migrate(db: &DB) -> Result<(), Error> {
    match sqlx::migrate!("../migrations").run(db).await {
        Ok(_) => Ok(()),
        Err(sqlx::migrate::MigrateError::VersionMissing(e)) => {
            tracing::error!("Database had been applied more migrations than this container. 
            This usually mean than another container on a more recent version migrated the database and this one is on an earlier version.
            Please update the container to latest. Not critical, but may cause issues if migration introduced a breaking change. Version missing: {e}");
            Ok(())
        }
        Err(err) => Err(err),
    }?;

    Ok(())
}

#[derive(Clone, Debug)]
pub struct ApiAuthed {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    // (folder name, can write, is owner)
    pub folders: Vec<(String, bool, bool)>,
    pub scopes: Option<Vec<String>>,
}

impl From<ApiAuthed> for Authed {
    fn from(value: ApiAuthed) -> Self {
        Self {
            email: value.email,
            username: value.username,
            is_admin: value.is_admin,
            is_operator: value.is_operator,
            groups: value.groups,
            folders: value.folders,
            scopes: value.scopes,
        }
    }
}

impl Authable for ApiAuthed {
    fn is_admin(&self) -> bool {
        self.is_admin
    }

    fn is_operator(&self) -> bool {
        self.is_operator
    }

    fn groups(&self) -> &[String] {
        &self.groups
    }

    fn folders(&self) -> &[(String, bool, bool)] {
        &self.folders
    }

    fn scopes(&self) -> Option<&[std::string::String]> {
        self.scopes.as_ref().map(|x| x.as_slice())
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn username(&self) -> &str {
        &self.username
    }
}
