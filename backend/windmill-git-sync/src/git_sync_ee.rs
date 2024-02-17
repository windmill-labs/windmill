use windmill_common::error::Result;

use crate::{DeployedObject, DB};

pub async fn handle_deployment_metadata<'c, R: rsmq_async::RsmqConnection + Send + Clone + 'c>(
    email: &str,
    created_by: &str,
    db: &DB,
    w_id: &str,
    obj: DeployedObject,
    deployment_message: Option<String>,
    rsmq: Option<R>,
    skip_db_insert: bool,
) -> Result<()> {
    // Git sync is an enterprise feature and not part of the open-source version
    return Ok(());
}
