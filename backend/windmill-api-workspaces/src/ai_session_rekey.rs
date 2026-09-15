//! Re-keys the AI session backup objects (`windmill_ai_sessions/{w_id}/...`) after a
//! workspace key rotation. Each object is ciphertext under the workspace key with the user
//! segment of its key as the cipher suffix, so the walk needs no email.
//!
//! A rotation records the key it replaced in `ai_session_backup_rekey` inside its own
//! transaction (`set_encryption_key`, on every build), and the walk deletes that row only
//! once every object it found reads under the current key. Until then the read path decrypts with the recorded keys as well, and
//! every use of the backups starts the walk again, so a server restart mid-walk leaves
//! nothing unreadable and nothing under the old key for good.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use futures::TryStreamExt;
use magic_crypt::{MagicCrypt256, MagicCryptTrait};
use windmill_common::error::{Error, Result};
use windmill_common::variables::{crypt_from_key_with_suffix, get_workspace_key};
use windmill_common::DB;
use windmill_object_store::object_store_error_to_error;
use windmill_object_store::object_store_reexports::{
    ObjectMeta, ObjectStore, ObjectStoreError, Path as ObjectPath, PutMode, PutOptions, PutPayload,
    UpdateVersion,
};
use windmill_types::s3::LargeFileStorage;

/// The root of every AI session backup key in a workspace's storage.
pub const ROOT: &str = "windmill_ai_sessions";
/// The push body cap: no object written through the routes is larger. One that is was
/// planted by whoever holds the bucket's credentials, and is left unread.
pub const MAX_OBJECT_BYTES: usize = 32 * 1024 * 1024;

const IO_CONCURRENCY: usize = 8;

lazy_static::lazy_static! {
    /// Workspaces whose walk this process is running: one at a time per workspace, since
    /// every use of the backups asks for one while a rotation is pending.
    static ref IN_FLIGHT: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

fn in_flight() -> MutexGuard<'static, HashSet<String>> {
    IN_FLIGHT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The keys of rotations whose walk has not completed, oldest first (recorded by the
/// rotation itself, in its transaction).

async fn previous_keys(db: &DB, w_id: &str) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT previous_key FROM ai_session_backup_rekey WHERE workspace_id = $1 \
         ORDER BY started_at",
    )
    .bind(w_id)
    .fetch_all(db)
    .await?)
}

/// The ciphers, with `key_suffix`, of rotations whose walk has not completed: an object may
/// still be under one of them. Empty once nothing is pending. Derives ciphers rather than
/// handing out the keys, and is for a route that already authorized its caller to the
/// workspace's backups: it does no authorization of its own.
pub async fn pending_ciphers(db: &DB, w_id: &str, key_suffix: &str) -> Result<Vec<MagicCrypt256>> {
    Ok(previous_keys(db, w_id)
        .await?
        .iter()
        .map(|key| crypt_from_key_with_suffix(key, key_suffix))
        .collect())
}

/// The workspace's primary storage, resolved without a caller: a rotation runs the walk
/// off its own request.
pub(crate) async fn primary_store(db: &DB, w_id: &str) -> Result<Option<Arc<dyn ObjectStore>>> {
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
    let store = windmill_object_store::build_object_store_client(
        &windmill_object_store::lfs_to_object_store_resource(&lfs, resource_value)?,
    )
    .await?;
    Ok(Some(store))
}

/// Starts the walk for the workspace unless this process is already running one. `store`
/// is the workspace's storage as the caller resolved it under its own authorization.
pub fn spawn_rekey(db: DB, w_id: String, store: Arc<dyn ObjectStore>) {
    if !in_flight().insert(w_id.clone()) {
        return;
    }
    tokio::spawn(async move {
        match rekey(&db, &w_id, store).await {
            Ok(n) => tracing::info!("re-keyed {n} AI session backup objects in {w_id}"),
            Err(e) => tracing::error!("re-keying the AI session backups in {w_id}: {e:#}"),
        }
        in_flight().remove(&w_id);
    });
}

async fn rekey(db: &DB, w_id: &str, store: Arc<dyn ObjectStore>) -> Result<usize> {
    let previous = previous_keys(db, w_id).await?;
    if previous.is_empty() {
        return Ok(0);
    }
    let current = get_workspace_key(w_id, db).await?;
    let prefix = ObjectPath::from(format!("{ROOT}/{w_id}"));
    // Objects are re-keyed as the listing streams: a workspace's backups are not bounded.
    let rekeyed = AtomicUsize::new(0);
    store
        .list(Some(&prefix))
        .map_err(object_store_error_to_error)
        .try_for_each_concurrent(IO_CONCURRENCY, |meta| {
            let (rekeyed, store, current, previous) = (&rekeyed, &store, &current, &previous);
            async move {
                if rekey_object(store, current, previous, meta).await? {
                    rekeyed.fetch_add(1, Ordering::Relaxed);
                }
                Ok(())
            }
        })
        .await?;
    let rekeyed = rekeyed.into_inner();
    // Every object listed reads under the current key now, or was rewritten by a push that
    // used it: the recorded keys have nothing left to open. The rows stay on any failure
    // above, for the next walk.
    sqlx::query(
        "DELETE FROM ai_session_backup_rekey WHERE workspace_id = $1 AND previous_key = ANY($2)",
    )
    .bind(w_id)
    .bind(&previous)
    .execute(db)
    .await?;
    Ok(rekeyed)
}

/// `true` when the object was rewritten under the current key.
async fn rekey_object(
    store: &Arc<dyn ObjectStore>,
    current: &str,
    previous: &[String],
    meta: ObjectMeta,
) -> Result<bool> {
    // The session index markers are empty, and under no key.
    if meta.size == 0 {
        return Ok(false);
    }
    if meta.size as usize > MAX_OBJECT_BYTES {
        tracing::warn!(
            "AI session backup object {} is larger than any push writes; left unread",
            meta.location
        );
        return Ok(false);
    }
    let key = meta.location;
    // `windmill_ai_sessions/{w_id}/{user}/...`: the user segment is the cipher suffix.
    let Some(user) = key.parts().nth(2).map(|p| p.as_ref().to_string()) else {
        return Ok(false);
    };
    let result = match store.get(&key).await {
        Ok(result) => result,
        Err(ObjectStoreError::NotFound { .. }) => return Ok(false),
        Err(e) => return Err(object_store_error_to_error(e)),
    };
    let version =
        UpdateVersion { e_tag: result.meta.e_tag.clone(), version: result.meta.version.clone() };
    // The listing's size again, from the read itself: the object may have been replaced.
    if result.meta.size as usize > MAX_OBJECT_BYTES {
        tracing::warn!(
            "AI session backup object {key} is larger than any push writes; left unread"
        );
        return Ok(false);
    }
    let bytes = result.bytes().await.map_err(object_store_error_to_error)?;
    let current = crypt_from_key_with_suffix(current, &user);
    if open(&current, &bytes).is_some() {
        return Ok(false);
    }
    let Some(plaintext) = previous
        .iter()
        .find_map(|key| open(&crypt_from_key_with_suffix(key, &user), &bytes))
    else {
        tracing::warn!("AI session backup object {key} is under none of the workspace's keys");
        return Ok(false);
    };
    let payload = PutPayload::from(current.encrypt_bytes_to_bytes(&plaintext));
    // Conditional on the version read: a push or a delete landing in between used the
    // current key already, and writing over it would bring back what it replaced.
    let opts = PutOptions { mode: PutMode::Update(version), ..Default::default() };
    match store.put_opts(&key, payload.clone(), opts).await {
        Ok(_) => Ok(true),
        Err(ObjectStoreError::Precondition { .. }) | Err(ObjectStoreError::NotFound { .. }) => {
            Ok(false)
        }
        // A store without conditional writes (the filesystem one) keeps that window.
        Err(ObjectStoreError::NotImplemented) => {
            store
                .put(&key, payload)
                .await
                .map_err(object_store_error_to_error)?;
            Ok(true)
        }
        Err(e) => Err(object_store_error_to_error(e)),
    }
}

/// The plaintext under `mc` if the object is one of its. The objects are JSON and data
/// URLs, so a wrong key's output failing UTF-8 tells them apart beyond the cipher's own
/// padding check, which a wrong key passes about once in 256 tries.
pub fn open(mc: &MagicCrypt256, bytes: &[u8]) -> Option<Vec<u8>> {
    let plaintext = mc.decrypt_bytes_to_bytes(bytes).ok()?;
    std::str::from_utf8(&plaintext).ok()?;
    Some(plaintext)
}
