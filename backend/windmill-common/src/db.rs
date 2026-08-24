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
/// # How a connection gets into that state
///
/// sqlx queues its rollback-on-drop only once `begin()` has returned: the guard keys on a
/// transaction depth raised *after* the `BEGIN` round trip. A future cancelled in between —
/// a disconnecting API client, a `tokio::time::timeout`, an aborted task — therefore leaves
/// the server in a transaction with nothing queued to end it. sqlx's on-release `ping` is a
/// bare `wait_until_ready`: it drains the `ReadyForQuery` but never inspects its
/// transaction-status byte, so the connection is judged healthy and reused. The next
/// borrower's statements then run inside that transaction and hold its locks for as long as
/// it lives, and the first error turns the session into `idle in transaction (aborted)`,
/// after which *every* unrelated query on that connection fails with `25P02`. Nothing in
/// sqlx recovers it — the depth is still zero, so no rollback is ever queued — and
/// `idle_in_transaction_session_timeout` never fires on a connection that is in constant
/// use, leaving `max_lifetime` half an hour later as the only cure.
///
/// # Why the check is armed rather than always on
///
/// Postgres reports transaction status on every response, but sqlx keeps it private
/// (`PgConnection::in_transaction` is `pub(crate)`, and the public `is_in_transaction`
/// returns the client-side depth, which is precisely the value that is wrong here). Without
/// it, the only way to know is to issue a `ROLLBACK`, and doing that on every checkout costs
/// a round trip per query — about a third of the throughput on small ones. So it is armed
/// only once Postgres has reported a state that proves a connection is carrying leftover
/// transaction state, and disarms itself after [`RESET_WINDOW`].
///
/// # Why on acquire rather than release
///
/// The connection that reports the first `25P02` is released *before* the caller has
/// converted that error, so a release-side hook would let exactly that connection back into
/// the idle queue uncleaned. Cleaning on checkout has the same cost and no such gap.
///
/// # Why best-effort detection is enough
///
/// Arming is process-wide, not per-query: the flag covers a pool shared by everything in the
/// process, so it does not matter *which* caller notices a poisoned connection, only that
/// one does. [`note_sqlx_error`] is reached from the two conversions a `sqlx::Error` usually
/// passes through, which is most queries; a caller that instead formats the error into a
/// message never converts it and reports nothing. That only delays arming until the next
/// converting query touches the same pool — on a worker the job poller alone does so every
/// few tens of milliseconds. Adding calls at individual query sites is therefore not the
/// pattern; it would suggest a per-site contract that does not exist.
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
    /// Called from `Error`'s `From<sqlx::Error>` and from `to_anyhow`; see the module docs
    /// for why those two are enough and why individual query sites should not call it.
    pub(crate) fn note_sqlx_error(err: &sqlx::Error) {
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
