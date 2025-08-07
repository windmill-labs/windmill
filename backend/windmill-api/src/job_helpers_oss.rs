#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::job_helpers_ee::*;

#[cfg(not(feature = "private"))]
use axum::Router;
#[cfg(not(feature = "private"))]
use serde::Serialize;
#[cfg(not(feature = "private"))]
use uuid::Uuid;
#[cfg(not(feature = "private"))]
use windmill_common::s3_helpers::StorageResourceType;

#[cfg(all(feature = "parquet", not(feature = "private")))]
use crate::db::{ApiAuthed, DB};
#[cfg(all(feature = "parquet", not(feature = "private")))]
use object_store::{ObjectStore, PutMultipartOpts};
#[cfg(all(feature = "parquet", not(feature = "private")))]
use std::sync::Arc;
#[cfg(not(feature = "private"))]
use windmill_common::error;
#[cfg(all(feature = "parquet", not(feature = "private")))]
use windmill_common::{db::UserDB, s3_helpers::ObjectStoreResource};

#[cfg(all(feature = "parquet", not(feature = "private")))]
use bytes::Bytes;
#[cfg(all(feature = "parquet", not(feature = "private")))]
use futures::Stream;

#[cfg(all(feature = "parquet", not(feature = "private")))]
use axum::response::Response;
#[cfg(all(feature = "parquet", not(feature = "private")))]
use serde::Deserialize;

#[derive(Serialize)]
#[cfg(not(feature = "private"))]
pub struct UploadFileResponse {
    pub file_key: String,
}

#[derive(Deserialize)]
#[cfg(not(feature = "private"))]
pub struct LoadImagePreviewQuery {
    #[allow(dead_code)]
    pub file_key: String,
    #[allow(dead_code)]
    pub storage: Option<String>,
}

#[derive(Deserialize)]
#[cfg(not(feature = "private"))]
pub struct DownloadFileQuery {
    #[allow(dead_code)]
    pub file_key: String,
    #[allow(dead_code)]
    pub storage: Option<String>,
    #[allow(dead_code)]
    pub s3_resource_path: Option<String>,
}

#[cfg(not(feature = "private"))]
pub fn workspaced_service() -> Router {
    Router::new()
}

#[cfg(all(feature = "parquet", not(feature = "private")))]
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

#[cfg(not(feature = "private"))]
pub fn get_random_file_name(_file_extension: Option<String>) -> String {
    unimplemented!("Not implemented in Windmill's Open Source repository")
}

#[cfg(not(feature = "private"))]
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
    Err(error::Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[cfg(all(feature = "parquet", not(feature = "private")))]
pub async fn upload_file_from_req(
    _s3_client: Arc<dyn ObjectStore>,
    _file_key: &str,
    _req: axum::extract::Request,
    _options: PutMultipartOpts,
) -> error::Result<()> {
    Err(error::Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[cfg(all(feature = "parquet", not(feature = "private")))]
pub async fn upload_file_internal(
    _s3_client: Arc<dyn ObjectStore>,
    _file_key: &str,
    _stream: impl Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
    _options: PutMultipartOpts,
) -> error::Result<()> {
    Err(error::Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[cfg(all(feature = "parquet", not(feature = "private")))]
pub async fn download_s3_file_internal(
    _authed: ApiAuthed,
    _db: &DB,
    _user_db: Option<UserDB>,
    _token: &str,
    _w_id: &str,
    _query: DownloadFileQuery,
) -> error::Result<Response> {
    Err(error::Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
pub async fn read_object_streamable(
    s3_client: Arc<dyn ObjectStore>,
    file_key: &str,
) -> error::Result<Response> {
    Err(error::Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
pub async fn delete_s3_file_internal(
    authed: &ApiAuthed,
    db: &DB,
    UserDB: Option<UserDB>,
    token: &str,
    w_id: &str,
    query: DeleteS3FileQuery,
) -> error::Result<()> {
    Err(error::Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
#[derive(Deserialize)]
pub struct DeleteS3FileQuery {
    pub file_key: String,
    pub storage: Option<String>,
}
