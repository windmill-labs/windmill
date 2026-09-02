use sqlx::{Postgres, Transaction};
use windmill_common::error::Result;

/// Records what the lock now at each path hashes to, which is one half of the comparison a relock
/// skip makes against what each importer resolved against.
///
/// Callers that write the lock itself in the same statement fold the upsert into that statement
/// instead; this is for the ones with nothing to fold it into.
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
