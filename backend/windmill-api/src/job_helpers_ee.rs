#[cfg(feature = "private")]
use crate::job_helpers_ee;

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

#[cfg(feature = "parquet")]
use bytes::Bytes;
#[cfg(feature = "parquet")]
use futures::Stream;

#[cfg(feature = "parquet")]
use axum::response::Response;
#[cfg(feature = "parquet")]
use serde::Deserialize;

#[derive(Serialize)]
pub struct UploadFileResponse {
    pub file_key: String,
}

#[derive(Deserialize)]
pub struct LoadImagePreviewQuery {
    #[allow(dead_code)]
    pub file_key: String,
    #[allow(dead_code)]
    pub storage: Option<String>,
}

#[derive(Deserialize)]
pub struct DownloadFileQuery {
    #[allow(dead_code)]
    pub file_key: String,
    #[allow(dead_code)]
    pub storage: Option<String>,
    #[allow(dead_code)]
    pub s3_resource_path: Option<String>,
}

pub fn workspaced_service() -> Router {
    #[cfg(feature = "private")]
    {
        return job_helpers_ee::workspaced_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}

#[cfg(feature = "parquet")]
pub async fn get_workspace_s3_resource<'c>(
    authed: &ApiAuthed,
    db: &DB,
    user_db: Option<UserDB>,
    token: &str,
    w_id: &str,
    storage: Option<String>,
) -> windmill_common::error::Result<(Option<bool>, Option<ObjectStoreResource>)> {
    #[cfg(feature = "private")]
    {
        return job_helpers_ee::get_workspace_s3_resource(authed, db, user_db, token, w_id, storage).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (authed, db, user_db, token, w_id, storage);
        // implementation is not open source
        Ok((None, None))
    }
}

pub fn get_random_file_name(file_extension: Option<String>) -> String {
    #[cfg(feature = "private")]
    {
        return job_helpers_ee::get_random_file_name(file_extension);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = file_extension;
        unimplemented!("Not implemented in Windmill's Open Source repository")
    }
}

pub async fn get_s3_resource<'c>(
    authed: &ApiAuthed,
    db: &DB,
    user_db: Option<UserDB>,
    token: &str,
    w_id: &str,
    resource_path: &str,
    resource_type: Option<StorageResourceType>,
    job_id: Option<Uuid>,
) -> error::Result<ObjectStoreResource> {
    #[cfg(feature = "private")]
    {
        return job_helpers_ee::get_s3_resource(authed, db, user_db, token, w_id, resource_path, resource_type, job_id).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (authed, db, user_db, token, w_id, resource_path, resource_type, job_id);
        Err(error::Error::internal_err(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }
}

#[cfg(feature = "parquet")]
pub async fn upload_file_from_req(
    s3_client: Arc<dyn ObjectStore>,
    file_key: &str,
    req: axum::extract::Request,
    options: PutMultipartOpts,
) -> error::Result<()> {
    #[cfg(feature = "private")]
    {
        return job_helpers_ee::upload_file_from_req(s3_client, file_key, req, options).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (s3_client, file_key, req, options);
        Err(error::Error::internal_err(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }
}

#[cfg(feature = "parquet")]
pub async fn upload_file_internal(
    s3_client: Arc<dyn ObjectStore>,
    file_key: &str,
    stream: impl Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
    options: PutMultipartOpts,
) -> error::Result<()> {
    #[cfg(feature = "private")]
    {
        return job_helpers_ee::upload_file_internal(s3_client, file_key, stream, options).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (s3_client, file_key, stream, options);
        Err(error::Error::internal_err(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }
}

#[cfg(feature = "parquet")]
pub async fn download_s3_file_internal(
    authed: ApiAuthed,
    db: &DB,
    user_db: Option<UserDB>,
    token: &str,
    w_id: &str,
    query: DownloadFileQuery,
) -> error::Result<Response> {
    #[cfg(feature = "private")]
    {
        return job_helpers_ee::download_s3_file_internal(authed, db, user_db, token, w_id, query).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (authed, db, user_db, token, w_id, query);
        Err(error::Error::internal_err(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }
}
