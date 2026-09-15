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
//! allows it. Configuring a storage for such a workspace bumps the generation in the
//! transaction that sets it, so everything the workspace left in any instance store sits
//! under a generation the routes never read again: a later return to the instance store,
//! whichever it is by then, starts from a newer one. That is what lets a storage change
//! delete the older generations from the instance store without fencing against what
//! happens next, and a browser retire a removal owed to an instance store once the
//! workspace's own storage answered.

use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
use windmill_common::error::{Error, Result};
use windmill_common::utils::calculate_hash;
use windmill_common::DB;
use windmill_object_store::object_store_reexports::{
    ObjectStore, ObjectStoreError, Path as ObjectPath,
};
use windmill_object_store::{
    object_store_error_to_error, object_store_location, ObjectStoreResource,
};
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
    calculate_hash(&object_store_location(resource))[..16].to_string()
}

/// Where a workspace's backups live: its primary storage, or the instance object store
/// standing in for it.
pub struct BackupStore {
    pub store: Arc<dyn ObjectStore>,
    pub storage_id: String,
    pub fallback: bool,
}

/// The instance object store, for a workspace without storage of its own: loaded from
/// settings that say where its objects are, and not turned off by
/// `ai_sessions_instance_storage_fallback`, which is on unless set to false. Named like a
/// workspace storage, by that location, in a namespace of its own. Never on the Pro plan,
/// checked on every call: a store loaded before a switch to Pro stays loaded. Never in a
/// build without `private`, which has neither workspace storage nor the quota the fallback
/// counts toward.
///
/// Authorizes nothing, and the store reaches every workspace's objects: the caller must have
/// authorized the user for the workspace and keep what it reads and writes under that
/// user's prefix in it, as the backup routes do.
pub async fn fallback_store(db: &DB) -> Result<Option<BackupStore>> {
    #[cfg(not(feature = "private"))]
    {
        let _ = db;
        Ok(None)
    }
    #[cfg(feature = "private")]
    {
        if matches!(
            windmill_common::ee_oss::get_license_plan().await,
            windmill_common::ee_oss::LicensePlan::Pro
        ) {
            return Ok(None);
        }
        let Some((store, Some(location))) =
            windmill_object_store::get_object_store_with_location().await
        else {
            return Ok(None);
        };
        let setting = windmill_common::global_settings::load_value_from_global_settings(
            db,
            windmill_common::global_settings::AI_SESSIONS_INSTANCE_STORAGE_FALLBACK_SETTING,
        )
        .await?;
        if matches!(setting, Some(serde_json::Value::Bool(false))) {
            return Ok(None);
        }
        Ok(Some(BackupStore {
            storage_id: calculate_hash(&format!("instance:{location}"))[..16].to_string(),
            store,
            fallback: true,
        }))
    }
}

/// The workspace's primary storage, resolved without a caller: a rotation runs its deletion
/// off its own request.
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

/// Deletes, as the listing streams, every object of the workspace's backups in the store
/// from a generation older than `current`.
async fn delete_older(store: &Arc<dyn ObjectStore>, w_id: &str, current: i64) -> Result<()> {
    store
        .list(Some(&workspace_prefix(w_id)))
        .map_err(object_store_error_to_error)
        .try_for_each_concurrent(IO_CONCURRENCY, |meta| async move {
            if generation_of(w_id, &meta.location).is_some_and(|g| g >= current) {
                return Ok(());
            }
            match store.delete(&meta.location).await {
                Ok(()) | Err(ObjectStoreError::NotFound { .. }) => Ok(()),
                Err(e) => Err(object_store_error_to_error(e)),
            }
        })
        .await
}

/// Deletes, off the request, every object of the workspace's backups from a generation
/// older than `current`, once the rotation that made `current` the generation has
/// committed: nothing writes there any more but a push that resolved its prefix before the
/// commit, junk the browser's next push of that session rewrites under the current prefix,
/// as is anything a deletion cut short left behind. For the rotation route, which
/// authorized its caller as a superadmin.
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
        match delete_older(&store, &w_id, current).await {
            Ok(()) => {
                tracing::info!("deleted the AI session backups of {w_id} older than g{current}")
            }
            Err(e) => tracing::warn!("deleting the older AI session backups of {w_id}: {e:#}"),
        }
    });
}

/// Deletes, off the request, what the workspace's backups left in the instance store under
/// a generation older than `current`, the one a storage settings change committed. Nothing
/// reads there: the routes use the workspace's own storage, or, back in the instance store,
/// `current` or a newer generation, since configuring a storage over the fallback bumped
/// it. So it runs whatever the storage is now and whatever the setting says (copies from
/// when it was on may be there), and a deletion that is slow, cut short or overtaken by a
/// later change deletes nothing live. For the storage settings route, which authorized its
/// caller as a workspace admin.
pub(crate) fn spawn_delete_fallback(w_id: String, current: i64) {
    tokio::spawn(async move {
        let Some(instance) = windmill_object_store::get_object_store().await else {
            return;
        };
        match delete_older(&instance, &w_id, current).await {
            Ok(()) => tracing::info!(
                "deleted the AI session backups of {w_id} older than g{current} from the instance store"
            ),
            Err(e) => tracing::warn!(
                "deleting the AI session backups of {w_id} from the instance store: {e:#}"
            ),
        }
    });
}

/// The bytes of the workspace's backups in the instance store, for its storage usage while
/// it has no storage of its own (once it has one nothing writes there, and the change
/// deleted what was): `None` when it has one, when there is no instance store, or when
/// there is nothing, so no empty usage entry shows up. Whether the setting is on or off,
/// since copies from when it was on may be there.
///
/// Authorizes nothing: for the storage usage recount, which reports a total for the
/// workspace it was run for and hands out nothing it read.
pub async fn fallback_bytes(db: &DB, w_id: &str) -> Result<Option<i64>> {
    let has_storage = sqlx::query_scalar!(
        r#"SELECT large_file_storage IS NOT NULL AS "has_storage!" FROM workspace_settings WHERE workspace_id = $1"#,
        w_id
    )
    .fetch_optional(db)
    .await?
    .unwrap_or(false);
    if has_storage {
        return Ok(None);
    }
    let Some(instance) = windmill_object_store::get_object_store().await else {
        return Ok(None);
    };
    let mut total: i64 = 0;
    let mut stream = instance.list(Some(&workspace_prefix(w_id)));
    while let Some(meta) = stream.next().await {
        total += meta.map_err(object_store_error_to_error)?.size as i64;
    }
    Ok((total > 0).then_some(total))
}
