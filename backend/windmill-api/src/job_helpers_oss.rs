/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::Router;
use crate::db::DB;
use serde::{Deserialize, Serialize};

pub fn workspaced_service() -> Router {
    crate::job_helpers_ee::workspaced_service()
}

pub async fn get_workspace_s3_resource(
    w_id: &str,
    path: Option<String>,
    db: &DB,
) -> windmill_common::error::Result<Option<String>> {
    crate::job_helpers_ee::get_workspace_s3_resource(w_id, path, db).await
}

pub fn get_random_file_name(file_extension: Option<String>) -> String {
    crate::job_helpers_ee::get_random_file_name(file_extension)
}

pub async fn get_s3_resource(
    s3_resource_opt: Option<String>,
    w_id: &str,
    db: &DB,
) -> windmill_common::error::Result<Option<windmill_common::s3_helpers::S3Object>> {
    crate::job_helpers_ee::get_s3_resource(s3_resource_opt, w_id, db).await
}

pub async fn upload_file_from_req(
    req: axum::extract::Request,
    storage: Option<String>,
    s3_resource_path: Option<String>,
    file_key: Option<String>,
    resource_id: String,
    db: DB,
) -> Result<axum::Json<UploadFileResponse>, windmill_common::error::Error> {
    crate::job_helpers_ee::upload_file_from_req(req, storage, s3_resource_path, file_key, resource_id, db).await
}

pub async fn upload_file_internal(
    s3_resource_opt: Option<String>,
    w_id: &str,
    content: bytes::Bytes,
    file_key: String,
    db: &DB,
) -> windmill_common::error::Result<String> {
    crate::job_helpers_ee::upload_file_internal(s3_resource_opt, w_id, content, file_key, db).await
}

pub async fn download_s3_file_internal(
    s3_resource_opt: Option<String>,
    w_id: &str,
    file_key: &str,
    db: &DB,
) -> windmill_common::error::Result<bytes::Bytes> {
    crate::job_helpers_ee::download_s3_file_internal(s3_resource_opt, w_id, file_key, db).await
}

pub use crate::job_helpers_ee::DownloadFileQuery;
pub use crate::job_helpers_ee::LoadImagePreviewQuery;
pub use crate::job_helpers_ee::UploadFileResponse;