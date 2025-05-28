use crate::db::DB;
use axum::{Router, response::Response};
use serde::{Deserialize, Serialize};
use windmill_common::{error, object_store::ObjectStoreResource};

#[derive(Serialize, Deserialize)]
pub struct UploadFileResponse {
    pub file_key: String,
}

#[derive(Deserialize)]
pub struct LoadImagePreviewQuery;

#[derive(Deserialize)]
pub struct DownloadFileQuery;

pub fn workspaced_service() -> Router {
    crate::job_helpers_ee::workspaced_service()
}

pub async fn get_workspace_s3_resource(
    _authed: &windmill_common::users::Authed,
    _w_id: &str,
    _db: &DB,
) -> windmill_common::error::Result<(Option<bool>, Option<ObjectStoreResource>)> {
    crate::job_helpers_ee::get_workspace_s3_resource(_authed, _w_id, _db).await
}

pub fn get_random_file_name(_file_extension: Option<String>) -> String {
    crate::job_helpers_ee::get_random_file_name(_file_extension)
}

pub async fn get_s3_resource(
    _authed: &windmill_common::users::Authed,
    _w_id: &str,
    _db: &DB,
    _disable_s3_internal: bool,
) -> error::Result<ObjectStoreResource> {
    crate::job_helpers_ee::get_s3_resource(_authed, _w_id, _db, _disable_s3_internal).await
}

pub async fn upload_file_from_req(
    _authed: windmill_common::users::Authed,
    _w_id: axum::extract::Path<String>,
    _db: DB,
    _req: axum::extract::Request,
) -> error::Result<()> {
    crate::job_helpers_ee::upload_file_from_req(_authed, _w_id, _db, _req).await
}

pub async fn upload_file_internal(
    _authed: &windmill_common::users::Authed,
    _w_id: &str,
    _db: &DB,
    _file_content: bytes::Bytes,
    _file_extension: Option<String>,
    _file_name: Option<String>,
    _s3_resource_opt: Option<ObjectStoreResource>,
    _disable_s3_internal: bool,
) -> error::Result<()> {
    crate::job_helpers_ee::upload_file_internal(
        _authed,
        _w_id,
        _db,
        _file_content,
        _file_extension,
        _file_name,
        _s3_resource_opt,
        _disable_s3_internal,
    ).await
}

pub async fn download_s3_file_internal(
    _authed: &windmill_common::users::Authed,
    _w_id: &str,
    _db: &DB,
    _file_key: &str,
    _s3_resource_opt: Option<ObjectStoreResource>,
    _disable_s3_internal: bool,
) -> error::Result<Response> {
    crate::job_helpers_ee::download_s3_file_internal(
        _authed,
        _w_id,
        _db,
        _file_key,
        _s3_resource_opt,
        _disable_s3_internal,
    ).await
}