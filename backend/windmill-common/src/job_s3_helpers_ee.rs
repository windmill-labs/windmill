use std::future::Future;

use crate::{
    error::Error,
    s3_helpers::{ObjectStoreResource, StorageResourceType},
};

pub async fn get_s3_resource_internal<'c, F, Fut>(
    _resource_type: StorageResourceType,
    _s3_resource_value_raw: serde_json::Value,
    _gen_token: F,
) -> crate::error::Result<ObjectStoreResource>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<String, Error>> + Send + 'static,
{
    todo!()
}
