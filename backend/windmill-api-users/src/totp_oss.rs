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
) -> windmill_common::error::Result<bool> {
    Err(windmill_common::error::Error::BadRequest(
        "Two-factor authentication requires a build with the private feature".to_string(),
    ))
}
