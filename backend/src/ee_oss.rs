#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::ee::*;

#[cfg(not(feature = "private"))]
pub async fn set_license_key(_license_key: String, _db: Option<&windmill_common::db::DB>) -> () {
    // Implementation is not open source
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub async fn verify_license_key(_db: Option<&windmill_common::db::DB>) -> () {
    // Implementation is not open source
}

#[cfg(all(feature = "parquet", not(feature = "private")))]
pub async fn export_audit_logs_to_object_store(_db: &windmill_common::db::DB) {
    // Implementation is not open source (Windmill Enterprise Edition feature)
}

#[cfg(all(feature = "parquet", not(feature = "private")))]
pub async fn anchor_audit_logs_s3_checkpoint_env_var(_db: &windmill_common::db::DB) {
    // Implementation is not open source (Windmill Enterprise Edition feature)
}
