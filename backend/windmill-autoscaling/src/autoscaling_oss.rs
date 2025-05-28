use windmill_common::DB;

pub async fn apply_all_autoscaling(_db: &DB) -> anyhow::Result<()> {
    crate::autoscaling_ee::apply_all_autoscaling(_db).await
}