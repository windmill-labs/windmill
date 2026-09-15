//! What the workspace key rotation, the workspace storage settings and the AI session backup
//! routes (`windmill-api/src/ai_sessions.rs`) share about the backups: the store they live
//! in, and what a rotation or a storage change deletes.
//!
//! The backups are ciphertext under the workspace key and live under a prefix named by a
//! generation the rotation bumps (`workspace_settings.ai_sessions_backup_generation`) in the
//! transaction that commits the new key. A rotation does not re-key them: once committed,
//! the routes read and write under the new generation's prefix and answer with its number
//! (`backup_generation`; the storage identity, `storage_id`, names the storage and does not
//! change), so every browser marks its sync state stale and pushes its sessions whole again
//! there, and every older generation, which nothing writes to any
//! more, is deleted off the request at leisure. Sessions no browser holds any more are lost,
//! which a rotation (a rare operation) accepts in exchange for having no key but the current
//! one to read with and nothing to rewrite in place. A generation is never reused, so no
//! deletion, however late, can touch live objects; a rotation that fails before its commit
//! bumps nothing and deletes nothing; two rotations racing serialize on the key row.
//!
//! A workspace without storage of its own keeps its backups in the instance object store
//! instead, under the same layout and key, while `ai_sessions_instance_storage_fallback`
//! allows it. Configuring a storage for the workspace moves the routes there, and deletes
//! its prefix in the instance store off the request, so no copy is left behind that a later
//! return to the instance store would bring back; the browser is told which kind of store
//! answered (`fallback`), and retires a removal owed to the instance store on any answer
//! from the workspace's own storage, since nothing of the workspace is there any more.

use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
use windmill_common::error::{Error, Result};
use windmill_common::global_settings::{
    load_value_from_global_settings, AI_SESSIONS_INSTANCE_STORAGE_FALLBACK_SETTING,
};
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
/// The storage name the workspace's backups in the instance store count under in its
/// storage usage, next to `_default_` and the secondary storages.
pub const FALLBACK_STORAGE: &str = "_ai_sessions_fallback_";

const IO_CONCURRENCY: usize = 8;

/// The prefix of one generation's objects: `windmill_ai_sessions/{w_id}/g{generation}/`.
pub fn generation_prefix(w_id: &str, generation: i64) -> String {
    format!("{ROOT}/{w_id}/g{generation}")
}

/// The prefix of everything the workspace ever backed up, whatever the generation.
fn workspace_prefix(w_id: &str) -> ObjectPath {
    ObjectPath::from(format!("{ROOT}/{w_id}"))
}

/// Names the storage the backups are in, by what locates its objects (endpoint, region,
/// bucket; never the credentials, which rotate), so a browser tells that its sync state was
/// recorded against another storage; the generation, answered alongside, tells it a
/// rotation happened in this one.
pub fn storage_id(resource: &ObjectStoreResource) -> String {
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
    calculate_hash(&location)[..16].to_string()
}

/// Names the instance store the way `storage_id` names a workspace's, by the store's own
/// description (its bucket, container or root), in a namespace of its own: a browser holds
/// the two kinds apart (see `fallback`), whatever bucket each is.
fn instance_storage_id(store: &Arc<dyn ObjectStore>) -> String {
    calculate_hash(&format!("instance:{store}"))[..16].to_string()
}

/// Where a workspace's backups live: its primary storage, or, when it has none, the
/// instance object store standing in for it.
pub struct BackupStore {
    pub store: Arc<dyn ObjectStore>,
    pub storage_id: String,
    pub fallback: bool,
}

/// The instance object store, for a workspace without storage of its own: loaded (an
/// instance setting; never on the Pro plan, see `reload_object_store_setting`) and not
/// turned off by `ai_sessions_instance_storage_fallback`, which is on unless set to false.
pub async fn fallback_store(db: &DB) -> Result<Option<BackupStore>> {
    let Some(store) = windmill_object_store::get_object_store().await else {
        return Ok(None);
    };
    let off = matches!(
        load_value_from_global_settings(db, AI_SESSIONS_INSTANCE_STORAGE_FALLBACK_SETTING).await?,
        Some(serde_json::Value::Bool(false))
    );
    if off {
        return Ok(None);
    }
    Ok(Some(BackupStore {
        storage_id: instance_storage_id(&store),
        store,
        fallback: true,
    }))
}

/// The workspace's primary storage, resolved without a caller: a rotation or a storage
/// change runs its deletion off its own request.
async fn primary_store(db: &DB, w_id: &str) -> Result<Option<BackupStore>> {
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
    Ok(Some(BackupStore {
        store: windmill_object_store::build_object_store_client(&resource).await?,
        storage_id: storage_id(&resource),
        fallback: false,
    }))
}

/// The store the workspace's backups live in, resolved without a caller.
async fn workspace_store(db: &DB, w_id: &str) -> Result<Option<BackupStore>> {
    if let Some(primary) = primary_store(db, w_id).await? {
        return Ok(Some(primary));
    }
    fallback_store(db).await
}

/// Whether a workspace's own storage is the very bucket the instance store is: what the
/// workspace holds there is then its live backups, not copies to sweep or count twice. By
/// the stores' descriptions (a bucket, container or root), which tell the same bucket apart
/// from another whatever else differs, and err towards the same.
fn same_bucket(a: &Arc<dyn ObjectStore>, b: &Arc<dyn ObjectStore>) -> bool {
    a.to_string() == b.to_string()
}

/// The generation an object key sits under, `None` for a key of no generation (an older
/// layout), which counts as older than any.
fn generation_of(w_id: &str, key: &ObjectPath) -> Option<i64> {
    key.as_ref()
        .strip_prefix(&format!("{ROOT}/{w_id}/g"))?
        .split('/')
        .next()?
        .parse()
        .ok()
}

/// Deletes, as the listing streams, every object under the prefix but the ones `keep` says.
async fn delete_under(
    store: &Arc<dyn ObjectStore>,
    prefix: &ObjectPath,
    keep: impl Fn(&ObjectPath) -> bool,
) -> Result<()> {
    store
        .list(Some(prefix))
        .map_err(object_store_error_to_error)
        .try_for_each_concurrent(IO_CONCURRENCY, |meta| {
            let (store, keep) = (store, &keep);
            async move {
                if keep(&meta.location) {
                    return Ok(());
                }
                match store.delete(&meta.location).await {
                    Ok(()) | Err(ObjectStoreError::NotFound { .. }) => Ok(()),
                    Err(e) => Err(object_store_error_to_error(e)),
                }
            }
        })
        .await
}

/// Deletes, off the request and as the listing streams, every object of the workspace's
/// backups from a generation older than `current`, once the rotation that made `current`
/// the generation has committed: nothing writes there any more but a push that resolved its
/// prefix before the commit, junk the browser's next push of that session rewrites under the
/// current prefix, as is anything a deletion cut short left behind. For the rotation route,
/// which authorized its caller as a superadmin.
pub(crate) fn spawn_delete_older(db: DB, w_id: String, current: i64) {
    tokio::spawn(async move {
        let store = match workspace_store(&db, &w_id).await {
            Ok(Some(store)) => store.store,
            Ok(None) => return,
            Err(e) => {
                tracing::warn!("older AI session backups of {w_id} left in place: {e:#}");
                return;
            }
        };
        let deleted = delete_under(&store, &workspace_prefix(&w_id), |key| {
            generation_of(&w_id, key).is_some_and(|g| g >= current)
        })
        .await;
        match deleted {
            Ok(()) => {
                tracing::info!("deleted the AI session backups of {w_id} older than g{current}")
            }
            Err(e) => tracing::warn!("deleting the older AI session backups of {w_id}: {e:#}"),
        }
    });
}

/// Deletes, off the request, everything the workspace backed up in the instance store once
/// a storage of its own is configured: the routes answer from that storage from now on, so
/// nothing writes to the instance store's prefix any more, and a copy left there would come
/// back if the workspace ever dropped its storage. Whether the fallback is on or off (copies
/// from when it was on may be there), and only while the workspace has a storage of its own
/// that is not the instance store's own bucket (there, what it holds is the live backups).
/// For the storage settings route, which authorized its caller as a workspace admin.
pub(crate) fn spawn_delete_fallback(db: DB, w_id: String) {
    tokio::spawn(async move {
        let Some(instance) = windmill_object_store::get_object_store().await else {
            return;
        };
        match primary_store(&db, &w_id).await {
            Ok(Some(primary)) if !same_bucket(&primary.store, &instance) => {}
            Ok(_) => return,
            Err(e) => {
                tracing::warn!("AI session backups of {w_id} left in the instance store: {e:#}");
                return;
            }
        }
        match delete_under(&instance, &workspace_prefix(&w_id), |_| false).await {
            Ok(()) => {
                tracing::info!("deleted the AI session backups of {w_id} from the instance store")
            }
            Err(e) => tracing::warn!(
                "deleting the AI session backups of {w_id} from the instance store: {e:#}"
            ),
        }
    });
}

/// The bytes of the workspace's backups in the instance store, for its storage usage:
/// `None` when there is no instance store, or the workspace's own storage is its bucket
/// (counted with that storage). Whether the fallback is on or off, as for the deletion.
pub async fn fallback_bytes(db: &DB, w_id: &str) -> Result<Option<i64>> {
    let Some(instance) = windmill_object_store::get_object_store().await else {
        return Ok(None);
    };
    if primary_store(db, w_id)
        .await?
        .is_some_and(|primary| same_bucket(&primary.store, &instance))
    {
        return Ok(None);
    }
    let mut total: i64 = 0;
    let mut stream = instance.list(Some(&workspace_prefix(w_id)));
    while let Some(meta) = stream.next().await {
        total += meta.map_err(object_store_error_to_error)?.size as i64;
    }
    Ok(Some(total))
}
