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

/// Guards the pool against a Postgres session left inside a transaction sqlx is not
/// tracking.
///
/// sqlx only queues its rollback-on-drop once `begin()` has returned, so a future
/// cancelled while `BEGIN` is still in flight — a disconnecting API client, a
/// `tokio::time::timeout`, an aborted task — hands the connection back with the
/// transaction still open server-side. sqlx's on-release `ping` is a bare
/// `wait_until_ready` that reports such a connection as healthy, so it is reused: the
/// next borrower's statements silently run inside that transaction and hold their locks
/// for as long as it lives, and the first error turns the session into
/// `idle in transaction (aborted)`, after which every unrelated query on it fails with
/// `25P02` until `max_lifetime` recycles it half an hour later.
///
/// Rolling back on every checkout would add a round trip to every query, which measured at
/// about a third of the throughput on small ones, so the reset is armed only once Postgres
/// has reported a state that proves a connection is carrying such leftover state.
///
/// The reset runs on **acquire** rather than release: the connection that reports the first
/// `25P02` is released before its error has been converted, so a release-side hook would let
/// exactly that connection back into the idle queue uncleaned.
pub mod connection_reset {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::LazyLock;
    use std::time::{Duration, Instant};

    /// Origin for the millisecond clock below. Initialized on first use rather than at
    /// startup, which is fine: every reader compares deadlines derived from this same base.
    static CLOCK_BASE: LazyLock<Instant> = LazyLock::new(Instant::now);
    /// Milliseconds since `CLOCK_BASE` until which checkouts roll back; 0 = never armed.
    static RESET_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

    /// Only has to outlast the connections checked out when the first poisoned one surfaced,
    /// which under load is milliseconds; the margin covers a pool that is mostly idle.
    const RESET_WINDOW: Duration = Duration::from_secs(60);

    fn now_ms() -> u64 {
        CLOCK_BASE.elapsed().as_millis() as u64
    }

    fn arm() {
        let now = now_ms();
        let previous =
            RESET_UNTIL_MS.fetch_max(now + RESET_WINDOW.as_millis() as u64, Ordering::Relaxed);
        // Every window costs throughput, so each one must be attributable in the logs —
        // hence the comparison against `now` rather than against zero, which would report
        // only the first incident a process ever sees. Re-arming inside a live window is
        // the common case and stays quiet.
        if previous <= now {
            tracing::warn!(
                "a pooled connection was found in an aborted transaction; rolling back on \
                 checkout for the next {}s",
                RESET_WINDOW.as_secs()
            );
        }
    }

    /// Reading the clock is skipped entirely while the pool has never been armed, which
    /// is the steady state.
    fn armed() -> bool {
        let until = RESET_UNTIL_MS.load(Ordering::Relaxed);
        until != 0 && until > now_ms()
    }

    /// Body of the pool's `before_acquire` hook. `ROLLBACK` ends both a leaked-open and an
    /// aborted transaction; on a session with neither it succeeds and Postgres answers with
    /// a `there is no transaction in progress` warning, which is why `sqlx::postgres::notice`
    /// is filtered in `tracing_init`. Reporting the failure makes sqlx discard the
    /// connection, which is also what a session we could not clean deserves.
    pub async fn reset_before_acquire(conn: &mut sqlx::PgConnection) -> Result<bool, sqlx::Error> {
        if armed() {
            use sqlx::Executor;
            conn.execute("ROLLBACK").await?;
        }
        Ok(true)
    }

    /// Arms the reset when an error proves a session is stuck in an aborted transaction.
    /// `Error`'s `From<sqlx::Error>` and `to_anyhow` call this, so it covers `?` into a
    /// `windmill_common::error::Result` and an explicit `map_err(to_anyhow)`. Everything
    /// else has to call it: `?` into an `anyhow::Result` goes through anyhow's own `From`,
    /// and a caller that inspects the `sqlx::Error` in place — formatting it into a
    /// message, matching on it — never converts it at all. A poisoned connection reported
    /// only down one of those paths goes unnoticed until something else reports it.
    pub fn note_sqlx_error(err: &sqlx::Error) {
        let sqlx::Error::Database(db_err) = err else {
            return;
        };
        // 25P02 `in_failed_sql_transaction`: the connection this query ran on is sitting
        // in a failed transaction block that nothing in sqlx will ever roll back.
        if db_err.code().as_deref() == Some("25P02") {
            arm();
        }
    }
}
