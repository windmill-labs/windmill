use futures::TryStreamExt;
use sqlx::{Postgres, Transaction};
use windmill_common::error::Result;
use windmill_common::scripts::hash_script;

/// Records what the lock now at each path hashes to, which is one half of the comparison a relock
/// skip makes against what each importer resolved against.
///
/// Writes any path in `w_id` and checks nothing: callers are responsible for having established
/// the caller's access to that workspace. Callers that write the lock itself in the same statement
/// fold the upsert into that statement instead; this is for the ones with nothing to fold it into.
pub async fn record_lock_hashes(
    tx: &mut Transaction<'_, Postgres>,
    w_id: &str,
    entries: &[(String, i64)],
) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    let (paths, hashes): (Vec<String>, Vec<i64>) = entries.iter().cloned().unzip();
    sqlx::query!(
        "INSERT INTO lock_hash (workspace_id, path, lockfile_hash)
         SELECT $1, * FROM UNNEST($2::text[], $3::bigint[])
         ON CONFLICT (workspace_id, path) DO UPDATE SET lockfile_hash = EXCLUDED.lockfile_hash",
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
