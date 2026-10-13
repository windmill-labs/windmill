use sha2::{Digest, Sha256};
use sqlx::{Acquire, PgConnection, PgExecutor, Pool, Postgres, Transaction};

use crate::audit::AuditAuthor;

pub type DB = Pool<Postgres>;

/// Workspace ID resolved by gateway middleware (stored in request extensions).
/// Used by auth to resolve workspace when the URL path doesn't contain one.
#[derive(Clone, Debug)]
pub struct GatewayWorkspaceId(pub String);

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

impl Authed {
    pub fn to_authed_ref(&self) -> AuthedRef<'_> {
        AuthedRef {
            email: &self.email,
            username: &self.username,
            is_admin: &self.is_admin,
            is_operator: &self.is_operator,
            groups: &self.groups,
            folders: &self.folders,
            scopes: &self.scopes,
            token_prefix: &self.token_prefix,
        }
    }
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

/// Hash the caller's full authorization context into a stable identity string.
///
/// Caches that are consulted *before* the RLS query they short-circuit must key on this,
/// so a hit can only ever be returned to a caller whose own authorized read populated it.
///
/// Email alone is **not** a sufficient scope: the same email can resolve to different
/// effective permissions (`username`, groups, folders, scopes, admin/operator) through
/// job- or owner-scoped tokens that share an email but carry a narrower `permissioned_as`.
/// Every input that determines what the caller may read is folded in, mirroring
/// `job_read_access_cache_key` in windmill-api, so a lower-privilege context can never
/// reuse a higher-privilege context's cache entry. Variable-length fields are
/// length-prefixed to keep the encoding injective.
pub fn auth_identity<A: Authable + ?Sized>(authed: &A) -> String {
    let mut hasher = Sha256::new();
    let field = |hasher: &mut Sha256, bytes: &[u8]| {
        hasher.update((bytes.len() as u32).to_be_bytes());
        hasher.update(bytes);
    };
    hasher.update([authed.is_admin() as u8, authed.is_operator() as u8]);
    field(&mut hasher, authed.email().as_bytes());
    field(&mut hasher, authed.username().as_bytes());
    let mut groups: Vec<&str> = authed.groups().iter().map(String::as_str).collect();
    groups.sort_unstable();
    hasher.update((groups.len() as u32).to_be_bytes());
    for g in groups {
        field(&mut hasher, g.as_bytes());
    }
    let mut folders: Vec<&str> = authed.folders().iter().map(|f| f.0.as_str()).collect();
    folders.sort_unstable();
    hasher.update((folders.len() as u32).to_be_bytes());
    for f in folders {
        field(&mut hasher, f.as_bytes());
    }
    match authed.scopes() {
        // u32::MAX length-prefix marks "no scopes" so it can't collide with an empty list.
        None => hasher.update(u32::MAX.to_be_bytes()),
        Some(scopes) => {
            let mut scopes: Vec<&str> = scopes.iter().map(String::as_str).collect();
            scopes.sort_unstable();
            hasher.update((scopes.len() as u32).to_be_bytes());
            for s in scopes {
                field(&mut hasher, s.as_bytes());
            }
        }
    }
    hex::encode(hasher.finalize())
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

pub enum DbWithOptAuthed<'a, T: Authable + Sync> {
    UserDB { authed: &'a T, user_db: UserDB, db: DB },
    DB { db: DB, audit_author: AuditAuthor },
}
impl<'a, T: Authable + Sync> DbWithOptAuthed<'a, T> {
    pub fn from_authed(authed: &'a T, db: DB, user_db: Option<UserDB>) -> Self {
        if let Some(user_db) = user_db {
            Self::UserDB { authed, user_db, db }
        } else {
            Self::DB { db, audit_author: AuditAuthor::from(authed) }
        }
    }
    pub fn db(&self) -> &DB {
        match self {
            DbWithOptAuthed::UserDB { db, .. } => db,
            DbWithOptAuthed::DB { db, .. } => db,
        }
    }

    pub fn authed(&self) -> Option<&T> {
        match self {
            DbWithOptAuthed::UserDB { authed, .. } => Some(authed),
            DbWithOptAuthed::DB { .. } => None,
        }
    }

    pub fn audit_author(&self) -> Option<&AuditAuthor> {
        match self {
            DbWithOptAuthed::UserDB { .. } => None,
            DbWithOptAuthed::DB { audit_author, .. } => Some(audit_author),
        }
    }
}

impl<'c, 'd, T: Authable + Sync> Acquire<'c> for &'c DbWithOptAuthed<'d, T> {
    type Database = Postgres;
    type Connection = Transaction<'c, Postgres>;

    fn acquire(self) -> futures_core::future::BoxFuture<'c, Result<Self::Connection, sqlx::Error>> {
        Box::pin(async move {
            match self {
                DbWithOptAuthed::UserDB { authed, user_db, .. } => {
                    user_db.clone().begin(&**authed).await
                }
                DbWithOptAuthed::DB { db, .. } => db.clone().begin().await,
            }
        })
    }

    fn begin(
        self,
    ) -> futures_core::future::BoxFuture<'c, Result<Transaction<'c, Postgres>, sqlx::Error>> {
        Box::pin(async move {
            match self {
                DbWithOptAuthed::UserDB { authed, user_db, .. } => {
                    user_db.clone().begin(&**authed).await
                }
                DbWithOptAuthed::DB { db, .. } => db.clone().begin().await,
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
            // SAFETY: `schema` is an operator-controlled environment variable (PG_SCHEMA), set at deploy time and never user-supplied.
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

pub trait DbExecutor<'b>: Send + Sized + PgExecutor<'b> {
    fn executor<'a>(&'a mut self) -> impl PgExecutor<'a>;
    fn populate<'a>(&'a mut self) -> impl DbExecutor<'a>;
}

impl<'b> DbExecutor<'b> for &DB {
    fn executor<'a>(&'a mut self) -> impl PgExecutor<'a> {
        &**self
    }
    fn populate<'a>(&'a mut self) -> impl DbExecutor<'a> {
        &**self
    }
}

impl<'b> DbExecutor<'b> for &'b mut PgConnection {
    fn executor<'a>(&'a mut self) -> impl PgExecutor<'a> {
        &mut **self
    }
    fn populate<'a>(&'a mut self) -> impl DbExecutor<'a> {
        &mut **self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_authed() -> Authed {
        Authed {
            email: "alice@x.dev".to_string(),
            username: "alice".to_string(),
            is_admin: false,
            is_operator: false,
            groups: vec!["all".to_string()],
            folders: vec![("shared".to_string(), false, false)],
            scopes: None,
            token_prefix: None,
        }
    }

    // Email alone must NOT determine the cache identity: two contexts that share an email
    // but resolve to different effective permissions must get distinct identities, so a
    // lower-privilege context can never reuse a higher-privilege one's cached read.
    #[test]
    fn auth_identity_is_not_just_email() {
        let base = auth_identity(&base_authed());

        let mut more_folders = base_authed();
        more_folders
            .folders
            .push(("secret".to_string(), false, false));
        assert_ne!(base, auth_identity(&more_folders), "folders must matter");

        let mut more_groups = base_authed();
        more_groups.groups.push("devs".to_string());
        assert_ne!(base, auth_identity(&more_groups), "groups must matter");

        let mut other_user = base_authed();
        other_user.username = "bob".to_string();
        assert_ne!(base, auth_identity(&other_user), "username must matter");

        let mut admin = base_authed();
        admin.is_admin = true;
        assert_ne!(base, auth_identity(&admin), "is_admin must matter");

        let mut operator = base_authed();
        operator.is_operator = true;
        assert_ne!(base, auth_identity(&operator), "is_operator must matter");

        let mut scoped = base_authed();
        scoped.scopes = Some(vec!["resources:read:f/secret/x".to_string()]);
        assert_ne!(base, auth_identity(&scoped), "scopes must matter");
    }

    // Identical authorization contexts must produce the same identity (so the same caller
    // gets a cache hit), and ordering of groups/folders must not change the identity.
    #[test]
    fn auth_identity_is_stable_and_order_independent() {
        assert_eq!(auth_identity(&base_authed()), auth_identity(&base_authed()));

        let mut reordered = base_authed();
        reordered.groups = vec!["all".to_string(), "devs".to_string()];
        let mut other_order = base_authed();
        other_order.groups = vec!["devs".to_string(), "all".to_string()];
        assert_eq!(auth_identity(&reordered), auth_identity(&other_order));
    }
}
