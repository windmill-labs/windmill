#[cfg(not(feature = "private"))]
use windmill_common::error::{Error, Result};

#[cfg(not(feature = "private"))]
fn unavailable<T>() -> Result<T> {
    Err(Error::BadRequest(
        "Two-factor authentication requires a build with the private feature".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
pub fn service() -> axum::Router {
    axum::Router::new()
}

#[cfg(not(feature = "private"))]
pub async fn check_login_code(
    _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    _db: &windmill_common::DB,
    _email: &str,
    _code: &str,
) -> Result<bool> {
    unavailable()
}

#[cfg(not(feature = "private"))]
pub async fn begin_setup(
    _db: &windmill_common::DB,
    _email: &str,
) -> Result<crate::totp::TotpSetup> {
    unavailable()
}

#[cfg(not(feature = "private"))]
pub async fn finish_setup(
    _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    _db: &windmill_common::DB,
    _email: &str,
    _code: &str,
    _author: &windmill_common::audit::AuditAuthor,
) -> Result<Option<Vec<String>>> {
    unavailable()
}

#[cfg(not(feature = "private"))]
pub enum TotpChange {
    Enabled,
}

#[cfg(not(feature = "private"))]
pub fn notify(_email: &str, _change: TotpChange) {}
