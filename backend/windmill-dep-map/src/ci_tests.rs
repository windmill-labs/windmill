#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::ci_tests_ee::*;

#[cfg(not(feature = "private"))]
use uuid::Uuid;

#[cfg(not(feature = "private"))]
use windmill_common::error;

#[cfg(not(feature = "private"))]
pub async fn trigger_ci_tests_for_item(
    _db: &sqlx::Pool<sqlx::Postgres>,
    _w_id: &str,
    _item_path: &str,
    _item_kind: &str,
    _email: &str,
    _username: &str,
) -> error::Result<Vec<Uuid>> {
    Ok(vec![])
}
