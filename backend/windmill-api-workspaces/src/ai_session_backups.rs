//! What the workspace key rotation and the AI session backup routes
//! (`windmill-api/src/ai_sessions.rs`) share about the backups in the workspace storage.
//!
//! The backups are ciphertext under the workspace key and live under a prefix named by a
//! fingerprint of that key. A rotation does not re-key them: once the new key is committed,
//! the routes read and write under the new key's prefix, the storage identity they answer
//! with changes with it, so every browser marks its sync state stale and pushes its sessions
//! whole again there, and the old key's prefix, which nothing writes to any more, is deleted
//! off the request at leisure. Sessions no browser holds any more are lost, which a rotation
//! (a rare operation) accepts in exchange for having no key but the current one to read with
//! and nothing to rewrite in place. A rotation that fails before its commit changes no prefix
//! and deletes nothing; two rotations racing serialize on the key row, and each deletes only
//! the prefix of the key it replaced.

use std::sync::Arc;

use futures::TryStreamExt;
use windmill_common::error::{Error, Result};
use windmill_common::utils::calculate_hash;
use windmill_common::DB;
use windmill_object_store::object_store_reexports::{
    ObjectStore, ObjectStoreError, Path as ObjectPath,
};
use windmill_object_store::{object_store_error_to_error, ObjectStoreResource};
use windmill_types::s3::LargeFileStorage;

/// The root of every AI session backup key in a workspace's storage.
pub const ROOT: &str = "windmill_ai_sessions";
/// The push body cap: no object written through the routes is larger. One that is was
/// planted by whoever holds the bucket's credentials, and is left unread.
pub const MAX_OBJECT_BYTES: usize = 32 * 1024 * 1024;

const IO_CONCURRENCY: usize = 8;

/// The segment of the object keys that names the workspace key they are under: a truncated
/// hash of the key (64 random characters), which reveals nothing of it.
pub fn key_fingerprint(key: &str) -> String {
    calculate_hash(&format!("{ROOT}:{key}"))[..16].to_string()
}

/// Names the storage the backups are in, by what locates its objects (endpoint, region,
/// bucket; never the credentials, which rotate) and by the workspace key, so a browser
/// tells that its sync state was recorded against another storage or another key.
pub fn storage_id(resource: &ObjectStoreResource, key: &str) -> String {
    let location = match resource {
        ObjectStoreResource::S3(s) => format!(
            "s3:{}:{}:{}:{}",
            s.endpoint,
            s.port.unwrap_or_default(),
            s.region,
            s.bucket
        ),
        ObjectStoreResource::Azure(a) => format!(
            "azure:{}:{}:{}",
            a.endpoint.as_deref().unwrap_or_default(),
            a.account_name,
            a.container_name
        ),
        ObjectStoreResource::Gcs(g) => format!("gcs:{}", g.bucket),
        ObjectStoreResource::Filesystem(f) => format!("fs:{}", f.root_path),
    };
    calculate_hash(&format!("{location}:{}", key_fingerprint(key)))[..16].to_string()
}

/// The workspace's primary storage, resolved without a caller: a rotation runs the
/// deletion off its own request.
async fn primary_store(db: &DB, w_id: &str) -> Result<Option<Arc<dyn ObjectStore>>> {
    let Some(lfs_json) = sqlx::query_scalar!(
        "SELECT large_file_storage FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(db)
    .await?
    .flatten() else {
        return Ok(None);
    };
    let lfs: LargeFileStorage = serde_json::from_value(lfs_json)
        .map_err(|e| Error::internal_err(format!("parsing large_file_storage: {e}")))?;
    let resource_value = if matches!(lfs, LargeFileStorage::FilesystemStorage(_)) {
        serde_json::Value::Null
    } else {
        let path = lfs.get_s3_resource_path();
        let path = path.strip_prefix("$res:").unwrap_or(path);
        windmill_common::workspaces::transform_json_value_unchecked(
            &serde_json::Value::String(format!("$res:{path}")),
            w_id,
            db,
        )
        .await?
    };
    let resource = windmill_object_store::lfs_to_object_store_resource(&lfs, resource_value)?;
    Ok(Some(
        windmill_object_store::build_object_store_client(&resource).await?,
    ))
}

/// Deletes, off the request and as the listing streams, the backups under the prefix of
/// `previous_key`, once the rotation that replaced it has committed: nothing writes there
/// any more but a push that resolved its prefix before the commit, junk the browser's next
/// push of that session rewrites under the new prefix, as is anything a deletion cut short
/// left behind. For the rotation route, which authorized its caller as a superadmin.
pub(crate) fn spawn_delete_previous(db: DB, w_id: String, previous_key: String) {
    tokio::spawn(async move {
        let store = match primary_store(&db, &w_id).await {
            Ok(Some(store)) => store,
            Ok(None) => return,
            Err(e) => {
                tracing::warn!(
                    "AI session backups of {w_id} under the previous key left in place: {e:#}"
                );
                return;
            }
        };
        let prefix = ObjectPath::from(format!("{ROOT}/{w_id}/{}", key_fingerprint(&previous_key)));
        let deleted = store
            .list(Some(&prefix))
            .map_err(object_store_error_to_error)
            .try_for_each_concurrent(IO_CONCURRENCY, |meta| {
                let store = &store;
                async move {
                    match store.delete(&meta.location).await {
                        Ok(()) | Err(ObjectStoreError::NotFound { .. }) => Ok(()),
                        Err(e) => Err(object_store_error_to_error(e)),
                    }
                }
            })
            .await;
        match deleted {
            Ok(()) => {
                tracing::info!("deleted the AI session backups of {w_id} under the previous key")
            }
            Err(e) => tracing::warn!(
                "deleting the AI session backups of {w_id} under the previous key: {e:#}"
            ),
        }
    });
}
