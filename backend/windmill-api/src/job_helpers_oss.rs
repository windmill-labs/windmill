#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::job_helpers_ee::*;

#[cfg(not(feature = "private"))]
use axum::Router;
#[cfg(not(feature = "private"))]
use uuid::Uuid;
#[cfg(not(feature = "private"))]
use windmill_types::s3::StorageResourceType;

#[cfg(all(feature = "parquet", not(feature = "private")))]
use crate::db::{ApiAuthed, OptJobAuthed, DB};
#[cfg(all(feature = "parquet", not(feature = "private")))]
use std::sync::Arc;
#[cfg(all(feature = "parquet", not(feature = "private")))]
use windmill_common::db::UserDB;
#[cfg(not(feature = "private"))]
use windmill_common::error;
#[cfg(all(feature = "parquet", not(feature = "private")))]
use windmill_object_store::object_store_reexports::{ObjectStore, PutMultipartOpts, PutResult};
#[cfg(not(feature = "private"))]
use windmill_object_store::ObjectStoreResource;

#[cfg(all(feature = "parquet", not(feature = "private")))]
use bytes::Bytes;
#[cfg(all(feature = "parquet", not(feature = "private")))]
use futures::Stream;

#[cfg(all(feature = "parquet", not(feature = "private")))]
use axum::response::Response;
#[cfg(all(feature = "parquet", not(feature = "private")))]
use serde::Deserialize;

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
use windmill_common::db::DbWithOptAuthed;

#[cfg(not(feature = "private"))]
pub async fn get_s3_resource<'c>(
    _db_with_opt_authed: &DbWithOptAuthed<'c, ApiAuthed>,
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
    _max_size: Option<usize>,
) -> error::Result<(PutResult, usize)> {
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
    _max_size: Option<usize>,
) -> error::Result<(PutResult, usize)> {
    Err(error::Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

// These stubs stand in for the CE quota helpers in a pure-OSS build; their only
// callers (apps.rs / args.rs uploads) are `not(enterprise)`, so gate them the
// same way — an enterprise-without-private build compiles neither.
#[cfg(all(
    feature = "parquet",
    not(feature = "private"),
    not(feature = "enterprise")
))]
pub async fn ce_storage_quota_remaining(
    _db: &DB,
    _w_id: &str,
    _exclude_upload_id: Option<&str>,
) -> error::Result<i64> {
    Ok(i64::MAX)
}

#[cfg(all(
    feature = "parquet",
    not(feature = "private"),
    not(feature = "enterprise")
))]
pub fn reject_reserved_volume_key(_file_key: &str) -> error::Result<()> {
    Ok(())
}

#[cfg(all(
    feature = "parquet",
    not(feature = "private"),
    not(feature = "enterprise")
))]
pub struct CeUploadBudget {
    pub max_size: usize,
    pub existing_size: i64,
}

#[cfg(all(
    feature = "parquet",
    not(feature = "private"),
    not(feature = "enterprise")
))]
pub async fn ce_upload_budget(
    _db: &DB,
    _w_id: &str,
    _s3_client: &Arc<dyn ObjectStore>,
    _file_key: &str,
    _content_length: Option<i64>,
) -> error::Result<CeUploadBudget> {
    Ok(CeUploadBudget { max_size: usize::MAX, existing_size: 0 })
}

#[cfg(all(
    feature = "parquet",
    not(feature = "private"),
    not(feature = "enterprise")
))]
pub async fn bump_storage_usage(_db: &DB, _w_id: &str, _storage: &str, _delta: i64) {}

#[cfg(all(
    feature = "parquet",
    not(feature = "private"),
    not(feature = "enterprise")
))]
pub fn spawn_storage_usage_recount_floored(_db: &DB, _w_id: &str) {}

#[cfg(all(feature = "parquet", not(feature = "private")))]
pub async fn download_s3_file_internal(
    _authed: OptJobAuthed,
    _db: &DB,
    _user_db: Option<UserDB>,
    _w_id: &str,
    _query: DownloadFileQuery,
) -> error::Result<Response> {
    Err(error::Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[allow(dead_code)]
#[cfg(not(feature = "private"))]
pub async fn read_object_streamable(
    _s3_client: Arc<dyn ObjectStore>,
    _file_key: &str,
) -> error::Result<Response> {
    Err(error::Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[allow(dead_code)]
#[cfg(not(feature = "private"))]
pub async fn delete_s3_file_internal(
    _authed: OptJobAuthed,
    _db: &DB,
    _user_db: Option<windmill_common::db::UserDB>,
    _w_id: &str,
    _query: DeleteS3FileQuery,
) -> error::Result<()> {
    Err(error::Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct DeleteS3FileQuery {
    pub file_key: String,
    pub storage: Option<String>,
    pub s3_resource_path: Option<String>,
}

// Stubs for the app-scoped S3 display ops (mirrors the EE `*_internal` helpers +
// their query/response structs). Only compiled for a CE build with `parquet` but
// without `private`; the real implementations live in `job_helpers_ee.rs`.
#[cfg(all(feature = "parquet", not(feature = "private")))]
mod app_s3_display_stubs {
    use super::*;
    use serde::Serialize;
    use serde_json::value::RawValue;

    #[derive(Deserialize)]
    #[allow(dead_code)]
    pub struct LoadFileMetadataQuery {
        pub file_key: String,
        pub storage: Option<String>,
        pub s3_resource_path: Option<String>,
    }

    #[derive(Serialize)]
    pub struct LoadFileMetadataResponse {}

    // Mirror the EE query's required/optional fields so the CE build enforces the
    // same query contract (e.g. the mandatory byte range) at the extraction layer.
    #[derive(Deserialize)]
    #[allow(dead_code)]
    pub struct LoadFilePreviewQuery {
        pub storage: Option<String>,
        pub s3_resource_path: Option<String>,
        pub file_key: String,
        pub file_size_in_bytes: Option<u64>,
        pub file_mime_type: Option<String>,
        pub csv_separator: Option<String>,
        pub csv_has_header: Option<bool>,
        pub read_bytes_from: u64,
        pub read_bytes_length: u64,
    }

    #[derive(Serialize)]
    pub struct LoadFilePreviewResponse {}

    #[derive(Deserialize)]
    #[allow(dead_code)]
    pub struct LoadCountQuery {
        pub search_col: Option<String>,
        pub search_term: Option<String>,
        pub storage: Option<String>,
    }

    #[derive(Serialize)]
    pub struct TableCount {}

    #[derive(Deserialize)]
    #[allow(dead_code)]
    pub struct LoadPreviewQuery {
        pub limit: Option<u32>,
        pub offset: Option<i64>,
        pub sort_col: Option<String>,
        pub sort_desc: Option<bool>,
        pub search_col: Option<String>,
        pub search_term: Option<String>,
        pub storage: Option<String>,
        pub csv_separator: Option<String>,
    }

    pub async fn load_file_metadata_internal(
        _authed: OptJobAuthed,
        _db: &DB,
        _user_db: Option<UserDB>,
        _w_id: &str,
        _query: LoadFileMetadataQuery,
    ) -> error::Result<LoadFileMetadataResponse> {
        Err(error::Error::internal_err(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }

    pub async fn load_file_preview_internal(
        _authed: OptJobAuthed,
        _db: &DB,
        _user_db: Option<UserDB>,
        _w_id: &str,
        _query: LoadFilePreviewQuery,
    ) -> error::Result<LoadFilePreviewResponse> {
        Err(error::Error::internal_err(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }

    pub async fn load_table_count_internal(
        _authed: OptJobAuthed,
        _db: &DB,
        _w_id: &str,
        _file_key: String,
        _query: LoadCountQuery,
    ) -> error::Result<TableCount> {
        Err(error::Error::internal_err(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }

    pub async fn load_preview_internal(
        _authed: OptJobAuthed,
        _db: &DB,
        _w_id: &str,
        _file_key: String,
        _query: LoadPreviewQuery,
        _is_parquet: bool,
    ) -> error::Result<Box<RawValue>> {
        Err(error::Error::internal_err(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }

    pub async fn download_s3_parquet_file_as_csv_internal(
        _authed: OptJobAuthed,
        _db: &DB,
        _user_db: Option<UserDB>,
        _w_id: &str,
        _query: DownloadFileQuery,
    ) -> error::Result<Response> {
        Err(error::Error::internal_err(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }
}

#[cfg(all(feature = "parquet", not(feature = "private")))]
pub use app_s3_display_stubs::*;

#[cfg(not(feature = "private"))]
pub async fn get_workspace_s3_resource_and_check_paths<'c>(
    _db_with_opt_authed: &DbWithOptAuthed<'c, ApiAuthed>,
    _authed_api: Option<&ApiAuthed>,
    _w_id: &str,
    _storage: Option<String>,
    _paths: &[(&str, windmill_types::s3::S3Permission)],
    _job_id: Option<uuid::Uuid>,
) -> windmill_common::error::Result<(
    Option<bool>,
    Option<windmill_types::s3::ObjectStoreResource>,
)> {
    Err(windmill_common::error::Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}
