use std::hash::{DefaultHasher, Hash, Hasher};

use sqlx::{Acquire, Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::SHARD_ID_TO_DB_INSTANCE;

pub type DB = Pool<Postgres>;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Authed {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    // (folder name, can write, is owner)
    pub folders: Vec<(String, bool, bool)>,
    pub scopes: Option<Vec<String>>,
    pub token_prefix: Option<String>,
}

#[derive(Clone, Debug, Hash)]
pub struct AuthedRef<'a> {
    pub email: &'a str,
    pub username: &'a str,
    pub is_admin: &'a bool,
    pub is_operator: &'a bool,
    pub groups: &'a Vec<String>,
    // (folder name, can write, is owner)
    pub folders: &'a Vec<(String, bool, bool)>,
    pub scopes: &'a Option<Vec<String>>,
    pub token_prefix: &'a Option<String>,
}

impl Authable for AuthedRef<'_> {
    fn email(&self) -> &str {
        self.email
    }
    fn username(&self) -> &str {
        self.username
    }
    fn is_admin(&self) -> bool {
        *self.is_admin
    }
    fn is_operator(&self) -> bool {
        *self.is_operator
    }
    fn groups(&self) -> &[String] {
        self.groups
    }
    fn folders(&self) -> &[(String, bool, bool)] {
        self.folders
    }
    fn scopes(&self) -> Option<&[String]> {
        self.scopes.as_ref().map(|x| x.as_slice())
    }
}

#[derive(Clone)]
pub struct UserDB {
    db: DB,
}

pub trait Authable {
    fn email(&self) -> &str;
    fn username(&self) -> &str;
    fn is_admin(&self) -> bool;
    fn is_operator(&self) -> bool;
    fn groups(&self) -> &[String];
    fn folders(&self) -> &[(String, bool, bool)];
    fn scopes(&self) -> Option<&[String]>;
}

impl Authable for Authed {
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

lazy_static::lazy_static! {
    pub static ref PG_SCHEMA: Option<String> = std::env::var("PG_SCHEMA").ok();
}

pub struct UserDbWithAuthed<'c, T: Authable + Sync> {
    pub authed: &'c T,
    pub db: UserDB,
}

impl<'c, 'd, T: Authable + Sync> Acquire<'c> for &'c UserDbWithAuthed<'d, T> {
    type Database = Postgres;
    type Connection = Transaction<'c, Postgres>;

    fn acquire(self) -> futures_core::future::BoxFuture<'c, Result<Self::Connection, sqlx::Error>> {
        Box::pin(async move { self.db.clone().begin(self.authed).await })
    }

    fn begin(
        self,
    ) -> futures_core::future::BoxFuture<'c, Result<Transaction<'c, Postgres>, sqlx::Error>> {
        Box::pin(async move { self.db.clone().begin(self.authed).await })
    }
}

pub struct UserDbWithOptAuthed<'c, T: Authable + Sync> {
    pub authed: &'c T,
    pub user_db: Option<UserDB>,
    pub db: DB,
}

impl<'c, 'd, T: Authable + Sync> Acquire<'c> for &'c UserDbWithOptAuthed<'d, T> {
    type Database = Postgres;
    type Connection = Transaction<'c, Postgres>;

    fn acquire(self) -> futures_core::future::BoxFuture<'c, Result<Self::Connection, sqlx::Error>> {
        Box::pin(async move {
            if let Some(db) = &self.user_db {
                db.clone().begin(self.authed).await
            } else {
                self.db.clone().begin().await
            }
        })
    }

    fn begin(
        self,
    ) -> futures_core::future::BoxFuture<'c, Result<Transaction<'c, Postgres>, sqlx::Error>> {
        Box::pin(async move {
            if let Some(db) = &self.user_db {
                db.clone().begin(self.authed).await
            } else {
                self.db.clone().begin().await
            }
        })
    }
}

impl UserDB {
    pub fn new(db: DB) -> Self {
        Self { db }
    }

    pub async fn begin<T>(self, authed: &T) -> Result<Transaction<'static, Postgres>, sqlx::Error>
    where
        T: Authable,
    {
        let (folders_write, folders_read): &(Vec<_>, Vec<_>) =
            &authed.folders().into_iter().partition(|x| x.1);

        let mut folders_read = folders_read.clone();
        folders_read.extend(folders_write.clone());

        // tracing::debug!(
        //     "Setting role to {} {:?} {:?} {:?} {:?}",
        //     user,
        //     authed.username(),
        //     authed.groups(),
        //     folders_read,
        //     folders_write
        // );

        let mut tx = self.db.begin().await?;

        if let Some(schema) = PG_SCHEMA.as_ref() {
            sqlx::query(&format!("SET LOCAL search_path TO {}", schema))
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query!(
            "SELECT set_session_context($1, $2, $3, $4, $5, $6)",
            authed.is_admin(),
            authed.username(),
            authed.groups().join(","),
            authed
                .groups()
                .iter()
                .map(|x| format!("g/{}", x))
                .collect::<Vec<_>>()
                .join(","),
            folders_read
                .iter()
                .map(|x| x.0.clone())
                .collect::<Vec<_>>()
                .join(","),
            folders_write
                .iter()
                .map(|x| x.0.clone())
                .collect::<Vec<_>>()
                .join(",")
        )
        .execute(&mut *tx)
        .await?;

        Ok(tx)
    }
}

pub fn get_shard_id_from_job_uuid(job_id: Uuid) -> usize {
    let shard_count = SHARD_ID_TO_DB_INSTANCE.get()
        .map(|shards| shards.len())
        .unwrap_or(1);

    let mut hasher = DefaultHasher::new();
    job_id.hash(&mut hasher);

    (hasher.finish() as usize) % shard_count
}

pub fn get_shard_db_from_shard_id(shard_id: usize) -> Option<&'static Pool<Postgres>> {
    SHARD_ID_TO_DB_INSTANCE.get()
        .and_then(|shards| shards.get(&shard_id))
}

pub fn get_shard_db_from_job_uuid(job_id: Uuid) -> Option<&'static Pool<Postgres>> {
    let shard_id = get_shard_id_from_job_uuid(job_id);
    get_shard_db_from_shard_id(shard_id)
}
