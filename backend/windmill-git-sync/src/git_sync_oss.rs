use sqlx::{Postgres, Transaction};

pub async fn handle_deployment_metadata(
    _tx: &mut Transaction<'_, Postgres>,
    _workspace_id: &str,
    _path: &str,
    _raw_value: &serde_json::Value,
) -> anyhow::Result<()> {
    crate::git_sync_ee::handle_deployment_metadata(_tx, _workspace_id, _path, _raw_value).await
}