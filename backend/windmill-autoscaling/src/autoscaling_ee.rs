use windmill_common::DB;

pub async fn apply_all_autoscaling(_db: &DB) -> anyhow::Result<()> {
    // Autoscaling is an ee feature
    Ok(())
}
