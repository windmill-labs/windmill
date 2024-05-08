use std::sync::Arc;

use axum::Router;
use object_store::ObjectStore;
use windmill_common::error;

pub fn workspaced_service() -> Router {
    Router::new()
}

pub async fn upload_file_internal(
    s3_client: Arc<dyn ObjectStore>,
    file_key: &str,
    request: axum::extract::Request,
) -> error::Result<()> {
    return Err(error::Error::BadRequest(
        "upload file internal impl is not open source".to_string(),
    ));
}
