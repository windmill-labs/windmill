use std::collections::HashMap;

use futures::TryStreamExt;
use sqlx::{Postgres, Transaction};
use windmill_common::error::Result;
use windmill_common::scripts::hash_script;

/// Records what the lock now at each path hashes to, which is one half of the comparison a relock
/// skip makes against what each importer resolved against.
///
/// Writes any path in `w_id` and checks nothing: callers are responsible for having established
/// the caller's access to that workspace. A path repeated in `entries` keeps its last hash.
///
/// Callers that write the lock itself in the same statement fold the upsert into that statement
/// instead; this is for the ones with nothing to fold it into.
pub async fn record_lock_hashes(
    tx: &mut Transaction<'_, Postgres>,
    w_id: &str,
    entries: &[(String, i64)],
) -> Result<()> {
    // Postgres rejects a whole statement that resolves a conflict on one key twice, so a path
    // given more than once keeps its last hash, as it would if the two were written in order.
    let mut deduped: HashMap<&str, i64> = HashMap::with_capacity(entries.len());
    for (path, hash) in entries {
        deduped.insert(path.as_str(), *hash);
    }
    if deduped.is_empty() {
        return Ok(());
    }
    let (paths, hashes): (Vec<String>, Vec<i64>) = deduped
        .into_iter()
        .map(|(path, hash)| (path.to_string(), hash))
        .unzip();
    // Recording a hash a path already has would still cut a row version, and the no-op push this
    // is reached from is the mode a git-sync of an unchanged workspace runs in.
    sqlx::query!(
        "INSERT INTO lock_hash (workspace_id, path, lockfile_hash)
         SELECT $1, * FROM UNNEST($2::text[], $3::bigint[])
         ON CONFLICT (workspace_id, path) DO UPDATE SET lockfile_hash = EXCLUDED.lockfile_hash
         WHERE lock_hash.lockfile_hash IS DISTINCT FROM EXCLUDED.lockfile_hash",
        w_id,
        &paths[..],
        &hashes[..]
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Records the hash of every live lock in `w_id`, for a workspace whose scripts arrived without
/// going through a deploy — a clone, which copies their locks verbatim and so would otherwise hold
/// none of the hashes describing them.
///
/// Carries the same caller obligation as [`record_lock_hashes`].
///
/// `script.lock` is unbounded and a workspace holds one per script, so the rows are streamed and
/// each lock is hashed and dropped before the next arrives; only the hashes accumulate.
pub async fn record_lock_hashes_for_workspace(
    tx: &mut Transaction<'_, Postgres>,
    w_id: &str,
) -> Result<()> {
    let mut entries: Vec<(String, i64)> = Vec::new();
    {
        let mut rows = sqlx::query!(
            "SELECT DISTINCT ON (path) path, lock FROM script
             WHERE workspace_id = $1 AND NOT archived AND NOT deleted AND lock IS NOT NULL
             ORDER BY path, created_at DESC",
            w_id
        )
        .fetch(&mut **tx);

        while let Some(row) = rows.try_next().await? {
            if let Some(lock) = row.lock {
                entries.push((row.path, hash_script(&lock)));
            }
        }
    }
    record_lock_hashes(tx, w_id, &entries).await
}
