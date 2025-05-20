#[cfg(feature = "private")]
use crate::git_sync_ee;

use windmill_common::error::Result;

use crate::{DeployedObject, DB};

pub async fn handle_deployment_metadata<'c>(
    email: &str,
    created_by: &str,
    db: &DB,
    w_id: &str,
    obj: DeployedObject,
    deployment_message: Option<String>,
    skip_db_insert: bool,
) -> Result<()> {
    #[cfg(feature = "private")]
    {
        return git_sync_ee::handle_deployment_metadata(email, created_by, db, w_id, obj, deployment_message, skip_db_insert).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (email, created_by, db, w_id, obj, deployment_message, skip_db_insert);
        // Git sync is an enterprise feature and not part of the open-source version
        return Ok(());
    }
}
