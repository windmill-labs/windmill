use axum::Router;

#[cfg(feature = "parquet")]
use crate::db::{ApiAuthed, DB};
#[cfg(feature = "parquet")]
use windmill_common::{db::UserDB, s3_helpers::ObjectStoreResource};

pub fn workspaced_service() -> Router {
    Router::new()
}

#[cfg(feature = "parquet")]
pub async fn get_workspace_s3_resource<'c>(
    _authed: &ApiAuthed,
    _db: &DB,
    _user_db: Option<UserDB>,
    _token: &str,
    _w_id: &str,
    _storage: Option<String>,
) -> windmill_common::error::Result<(Option<bool>, Option<ObjectStoreResource>)> {
    // implementation is not open source
    Ok((None, None))
}
