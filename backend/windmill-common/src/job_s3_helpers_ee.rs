#[cfg(feature = "private")]
use crate::job_s3_helpers_ee;

use std::future::Future;

use crate::{
    error::{Error, Result as WindmillResult}, // Added Result for clarity with std::result::Result
    s3_helpers::{ObjectStoreResource, StorageResourceType},
};

pub async fn get_s3_resource_internal<'c, F, Fut>(
    resource_type: StorageResourceType,
    s3_resource_value_raw: serde_json::Value,
    gen_token: F,
) -> WindmillResult<ObjectStoreResource>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = WindmillResult<String>> + Send + 'static,
{
    #[cfg(feature = "private")]
    {
        return job_s3_helpers_ee::get_s3_resource_internal(resource_type, s3_resource_value_raw, gen_token).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (resource_type, s3_resource_value_raw, gen_token);
        todo!()
    }
}
