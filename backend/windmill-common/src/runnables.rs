//! Queries spanning both scripts and flows.

use crate::{error::Error, DB};

/// An opaque value that changes whenever a workspace's live scripts or flows are added,
/// removed, moved or redeployed: a redeploy is a new script hash or a new last flow
/// version, so schema and description changes register too. Not filtered by caller,
/// so it reveals nothing beyond whether it changed.
pub async fn runnable_list_fingerprint(db: &DB, workspace_id: &str) -> Result<String, Error> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT COALESCE(md5(string_agg(k, ',' ORDER BY k)), '') FROM (
            SELECT 's' || path || ':' || hash FROM script
            WHERE workspace_id = $1 AND archived = false
            UNION ALL
            SELECT 'f' || path || ':' || COALESCE(versions[array_upper(versions, 1)], 0) FROM flow
            WHERE workspace_id = $1 AND archived = false
        ) t(k)",
    )
    .bind(workspace_id)
    .fetch_one(db)
    .await?)
}
