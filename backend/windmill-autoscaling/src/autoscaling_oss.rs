#[cfg(not(feature = "private"))]
use windmill_common::DB;

#[cfg(not(feature = "private"))]
pub async fn apply_all_autoscaling(_db: &DB) -> anyhow::Result<()> {
    // Autoscaling is an ee feature
    Ok(())
}
