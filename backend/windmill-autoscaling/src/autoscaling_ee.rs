#[cfg(feature = "private")]
use crate::autoscaling_ee;

use windmill_common::DB;

pub async fn apply_all_autoscaling(db: &DB) -> anyhow::Result<()> {
    #[cfg(feature = "private")]
    {
        return autoscaling_ee::apply_all_autoscaling(db).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = db;
        // Autoscaling is an ee feature
        Ok(())
    }
}
