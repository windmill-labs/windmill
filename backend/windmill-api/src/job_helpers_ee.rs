use axum::Router;
use serde::Serialize;
use uuid::Uuid;
use windmill_common::s3_helpers::StorageResourceType;

#[cfg(feature = "parquet")]
use crate::db::{ApiAuthed, DB};
#[cfg(feature = "parquet")]
use object_store::{ObjectStore, PutMultipartOpts};
#[cfg(feature = "parquet")]
use std::sync::Arc;
use windmill_common::error;
#[cfg(feature = "parquet")]
use windmill_common::{db::UserDB, s3_helpers::ObjectStoreResource};
#[derive(Serialize)]
pub struct UploadFileResponse {
    pub file_key: String,
}

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

pub fn get_random_file_name(_file_extension: Option<String>) -> String {
    todo!()
}

pub async fn get_s3_resource<'c>(
    _authed: &ApiAuthed,
    _db: &DB,
    _user_db: Option<UserDB>,
    _token: &str,
    _w_id: &str,
    _resource_path: &str,
    _resource_type: Option<StorageResourceType>,
    _job_id: Option<Uuid>,
) -> error::Result<ObjectStoreResource> {
    todo!()
}

pub async fn upload_file_internal(
    _s3_client: Arc<dyn ObjectStore>,
    _file_key: &str,
    _request: axum::extract::Request,
    _options: PutMultipartOpts,
) -> error::Result<()> {
    todo!()
}
