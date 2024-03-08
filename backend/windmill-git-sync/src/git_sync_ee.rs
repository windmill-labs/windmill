use windmill_common::error::Result;

use crate::{DeployedObject, DB};

pub async fn handle_deployment_metadata<'c, R: rsmq_async::RsmqConnection + Send + Clone + 'c>(
    _email: &str,
    _created_by: &str,
    _db: &DB,
    _w_id: &str,
    _obj: DeployedObject,
    _deployment_message: Option<String>,
    _rsmq: Option<R>,
    _skip_db_insert: bool,
) -> Result<()> {
    // Git sync is an enterprise feature and not part of the open-source version
    return Ok(());
}
