//! Re-keys the AI session backup objects (`windmill_ai_sessions/{w_id}/...`) after a
//! workspace key rotation. Each object is ciphertext under the workspace key with the user
//! segment of its key as the cipher suffix, so the walk needs no email.
//!
//! A rotation records the key it replaced in `ai_session_backup_rekey` inside its own
//! transaction (`set_encryption_key`, on every build). The read path decrypts with the
//! recorded keys as well, for good: a workspace may point at several storages over time,
//! and objects in one that is not primary at the moment are never rewritten. The walk notes
//! each storage it has rewritten every object of, and every use of the backups starts it
//! again for a storage not noted yet, so a server restart mid-walk leaves nothing behind.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use futures::TryStreamExt;
use magic_crypt::{MagicCrypt256, MagicCryptTrait};
use windmill_common::error::{Error, Result};
use windmill_common::utils::calculate_hash;
use windmill_common::variables::{crypt_from_key_with_suffix, get_workspace_key};
use windmill_common::DB;
use windmill_object_store::object_store_reexports::{
    ObjectMeta, ObjectStore, ObjectStoreError, Path as ObjectPath, PutMode, PutOptions, PutPayload,
    UpdateVersion,
};
use windmill_object_store::{object_store_error_to_error, ObjectStoreResource};
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

/// The keys every rotation replaced, oldest first (recorded by the rotation itself, in its
/// transaction), each with the storages the walk has completed on since.
async fn recorded_keys(db: &DB, w_id: &str) -> Result<Vec<(String, Vec<String>)>> {
    Ok(sqlx::query_as::<_, (String, Vec<String>)>(
        "SELECT previous_key, walked_storages FROM ai_session_backup_rekey \
         WHERE workspace_id = $1 ORDER BY started_at",
    )
    .bind(w_id)
    .fetch_all(db)
    .await?)
}

/// The ciphers, with `key_suffix`, of the keys rotations replaced (an object may still be
/// under one of them), and whether the walk is still owed on the storage `storage_id`.
/// Derives ciphers rather than handing out the keys, and is for a route that already
/// authorized its caller to the workspace's backups: it does no authorization of its own.
pub async fn pending_ciphers(
    db: &DB,
    w_id: &str,
    key_suffix: &str,
    storage_id: &str,
) -> Result<(Vec<MagicCrypt256>, bool)> {
    let rows = recorded_keys(db, w_id).await?;
    let needs_walk = rows
        .iter()
        .any(|(_, walked)| !walked.iter().any(|s| s == storage_id));
    let ciphers = rows
        .iter()
        .map(|(key, _)| crypt_from_key_with_suffix(key, key_suffix))
        .collect();
    Ok((ciphers, needs_walk))
}

/// Names a storage by what locates its objects (endpoint, region, bucket; never the
/// credentials, which rotate): the walk notes the storages it completed on, and a browser
/// tells its sync state was recorded against another one.
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

/// The workspace's primary storage and its id, resolved without a caller: a rotation runs
/// the walk off its own request.
pub(crate) async fn primary_store(
    db: &DB,
    w_id: &str,
) -> Result<Option<(Arc<dyn ObjectStore>, String)>> {
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
    let store = windmill_object_store::build_object_store_client(&resource).await?;
    Ok(Some((store, storage_id(&resource))))
}

/// Starts the walk of `store` (the workspace's storage as the caller resolved it under its
/// own authorization, named by `storage_id`) unless this process is already running one
/// for the workspace.
pub fn spawn_rekey(db: DB, w_id: String, store: Arc<dyn ObjectStore>, storage_id: String) {
    if !in_flight().insert(w_id.clone()) {
        return;
    }
    tokio::spawn(async move {
        match rekey(&db, &w_id, store, &storage_id).await {
            Ok(n) => tracing::info!("re-keyed {n} AI session backup objects in {w_id}"),
            Err(e) => tracing::error!("re-keying the AI session backups in {w_id}: {e:#}"),
        }
        in_flight().remove(&w_id);
    });
}

async fn rekey(
    db: &DB,
    w_id: &str,
    store: Arc<dyn ObjectStore>,
    storage_id: &str,
) -> Result<usize> {
    let rows = recorded_keys(db, w_id).await?;
    // Every recorded key can open an object; the walk is owed to the keys not yet noted
    // as walked on this storage.
    let previous: Vec<String> = rows.iter().map(|(key, _)| key.clone()).collect();
    let owed: Vec<String> = rows
        .iter()
        .filter(|(_, walked)| !walked.iter().any(|s| s == storage_id))
        .map(|(key, _)| key.clone())
        .collect();
    if owed.is_empty() {
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
    // Every object listed in this storage reads under the current key now, or was
    // rewritten by a push that used it. The note is not made on any failure above, so the
    // next use of the backups walks again; the keys themselves stay recorded, since a
    // storage that is not primary now may still hold objects under them.
    sqlx::query(
        "UPDATE ai_session_backup_rekey SET walked_storages = array_append(walked_storages, $3) \
         WHERE workspace_id = $1 AND previous_key = ANY($2) AND NOT ($3 = ANY(walked_storages))",
    )
    .bind(w_id)
    .bind(&owed)
    .bind(storage_id)
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
