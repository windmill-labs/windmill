#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::s3_ee::*;

#[cfg(all(not(feature = "private"), feature = "parquet"))]
use std::path::{Path, PathBuf};

#[cfg(all(not(feature = "private"), feature = "parquet"))]
const CE_SNAPSHOT_SIZE_LIMIT: usize = 100 * 1024 * 1024;
#[cfg(all(not(feature = "private"), feature = "parquet"))]
const CE_VOLUME_SIZE_LIMIT: usize = 50 * 1024 * 1024;
#[cfg(all(not(feature = "private"), feature = "parquet"))]
const SNAPSHOT_CACHE_DIR: &str = "/tmp/windmill/snapshots";

#[cfg(all(not(feature = "private"), feature = "parquet"))]
pub async fn ensure_snapshot_cached(
    w_id: &str,
    name: &str,
    tag: &str,
    db: &windmill_common::DB,
) -> windmill_common::error::Result<PathBuf> {
    use windmill_common::error::Error;

    let row = sqlx::query!(
        "SELECT s3_key, content_hash, status FROM sandbox_snapshot \
         WHERE workspace_id = $1 AND name = $2 AND tag = $3",
        w_id,
        name,
        tag,
    )
    .fetch_optional(db)
    .await?;

    let row = row.ok_or_else(|| {
        Error::NotFound(format!(
            "sandbox snapshot {name}:{tag} not found in workspace {w_id}"
        ))
    })?;

    if row.status != "ready" {
        return Err(Error::ExecutionErr(format!(
            "sandbox snapshot {name}:{tag} is not ready (status: {})",
            row.status
        )));
    }

    let cache_key = if row.content_hash.is_empty() {
        format!("{w_id}_{name}_{tag}")
    } else {
        row.content_hash.clone()
    };
    let cache_path = PathBuf::from(SNAPSHOT_CACHE_DIR).join(&cache_key);

    if cache_path.exists() {
        tracing::info!("Snapshot {name}:{tag} found in cache at {}", cache_path.display());
        return Ok(cache_path);
    }

    let os = windmill_object_store::get_workspace_object_store(db, w_id).await?;

    tracing::info!("Downloading snapshot {name}:{tag} from S3 key: {}", row.s3_key);

    let bytes = windmill_object_store::fetch_bytes_from_store(os, &row.s3_key)
        .await?
        .ok_or_else(|| {
            Error::ExecutionErr(format!("Snapshot S3 object not found: {}", row.s3_key))
        })?;

    tokio::fs::create_dir_all(&cache_path)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to create snapshot cache dir: {e}")))?;

    let cache_path_clone = cache_path.clone();
    let bytes_vec = bytes.to_vec();
    tokio::task::spawn_blocking(move || crate::untar_gz(&bytes_vec, &cache_path_clone))
        .await
        .map_err(|e| Error::ExecutionErr(format!("Spawn blocking failed: {e}")))??;

    tracing::info!("Snapshot {name}:{tag} unpacked to {}", cache_path.display());
    Ok(cache_path)
}

#[cfg(all(not(feature = "private"), not(feature = "parquet")))]
pub async fn ensure_snapshot_cached(
    _w_id: &str,
    _name: &str,
    _tag: &str,
    _db: &windmill_common::DB,
) -> windmill_common::error::Result<std::path::PathBuf> {
    Err(windmill_common::error::Error::ExecutionErr(
        "Sandbox snapshots require the parquet feature (S3 object store)".to_string(),
    ))
}

#[cfg(all(not(feature = "private"), feature = "parquet"))]
pub async fn download_volume(
    w_id: &str,
    name: &str,
    local_path: &Path,
    db: &windmill_common::DB,
) -> windmill_common::error::Result<()> {
    use windmill_common::error::Error;

    let row = sqlx::query!(
        "SELECT s3_key FROM sandbox_volume WHERE workspace_id = $1 AND name = $2",
        w_id,
        name,
    )
    .fetch_optional(db)
    .await?;

    let s3_key = match row {
        Some(r) => r.s3_key,
        None => {
            let s3_key = format!("sandbox/volumes/{w_id}/{name}.tar.gz");
            sqlx::query!(
                "INSERT INTO sandbox_volume (workspace_id, name, s3_key, created_by) \
                 VALUES ($1, $2, $3, 'system') ON CONFLICT DO NOTHING",
                w_id,
                name,
                &s3_key,
            )
            .execute(db)
            .await?;
            tracing::info!("Auto-created volume {name} in workspace {w_id}");
            return Ok(());
        }
    };

    let os = windmill_object_store::get_workspace_object_store(db, w_id).await?;

    let bytes = match windmill_object_store::fetch_bytes_from_store(os, &s3_key).await? {
        Some(b) => b,
        None => {
            tracing::info!("Volume has no S3 data yet, using empty dir");
            return Ok(());
        }
    };

    let local_path = local_path.to_path_buf();
    let bytes_vec = bytes.to_vec();
    tokio::task::spawn_blocking(move || crate::untar_gz(&bytes_vec, &local_path))
        .await
        .map_err(|e| Error::ExecutionErr(format!("Spawn blocking failed: {e}")))??;

    Ok(())
}

#[cfg(all(not(feature = "private"), not(feature = "parquet")))]
pub async fn download_volume(
    _w_id: &str,
    _name: &str,
    _local_path: &std::path::Path,
    _db: &windmill_common::DB,
) -> windmill_common::error::Result<()> {
    Err(windmill_common::error::Error::ExecutionErr(
        "Sandbox volumes require the parquet feature (S3 object store)".to_string(),
    ))
}

#[cfg(all(not(feature = "private"), feature = "parquet"))]
pub async fn upload_volume(
    w_id: &str,
    name: &str,
    local_path: &Path,
    db: &windmill_common::DB,
) -> windmill_common::error::Result<()> {
    use windmill_common::error::Error;

    let os = windmill_object_store::get_workspace_object_store(db, w_id).await?;

    let s3_key = sqlx::query_scalar!(
        "SELECT s3_key FROM sandbox_volume WHERE workspace_id = $1 AND name = $2",
        w_id,
        name,
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("sandbox volume {name} not found")))?;

    let local_path = local_path.to_path_buf();
    let bytes =
        tokio::task::spawn_blocking(move || crate::tar_gz(&local_path))
            .await
            .map_err(|e| Error::ExecutionErr(format!("Spawn blocking failed: {e}")))??;

    let size = bytes.len();
    if size > CE_VOLUME_SIZE_LIMIT {
        return Err(Error::ExecutionErr(format!(
            "Volume size ({:.1} MB) exceeds the {} MB limit. \
             Upgrade to Windmill EE for unlimited volume sizes.",
            size as f64 / 1_048_576.0,
            CE_VOLUME_SIZE_LIMIT / 1_048_576
        )));
    }

    windmill_object_store::put_bytes_to_store(os, &s3_key, bytes.into()).await?;

    let size_i64 = size as i64;
    sqlx::query!(
        "UPDATE sandbox_volume SET size_bytes = $3, updated_at = now() \
         WHERE workspace_id = $1 AND name = $2",
        w_id,
        name,
        size_i64,
    )
    .execute(db)
    .await?;

    tracing::info!("Volume {name} uploaded to S3 ({size} bytes)");
    Ok(())
}

#[cfg(all(not(feature = "private"), not(feature = "parquet")))]
pub async fn upload_volume(
    _w_id: &str,
    _name: &str,
    _local_path: &std::path::Path,
    _db: &windmill_common::DB,
) -> windmill_common::error::Result<()> {
    Err(windmill_common::error::Error::ExecutionErr(
        "Sandbox volumes require the parquet feature (S3 object store)".to_string(),
    ))
}

#[cfg(all(not(feature = "private"), feature = "parquet"))]
pub async fn upload_snapshot_bytes(
    db: &windmill_common::DB,
    w_id: &str,
    name: &str,
    tag: &str,
    body: bytes::Bytes,
    username: &str,
) -> windmill_common::error::Result<String> {
    use sha2::Digest;
    use windmill_common::error::Error;

    let size = body.len();
    if size > CE_SNAPSHOT_SIZE_LIMIT {
        return Err(Error::ExecutionErr(format!(
            "Snapshot size ({:.1} MB) exceeds the {} MB limit. \
             Upgrade to Windmill EE for unlimited snapshot sizes.",
            size as f64 / 1_048_576.0,
            CE_SNAPSHOT_SIZE_LIMIT / 1_048_576
        )));
    }

    let os = windmill_object_store::get_workspace_object_store(db, w_id).await?;

    let s3_key = format!("sandbox/snapshots/{}/{}/{}.tar.gz", w_id, name, tag);
    let content_hash = format!("{:x}", sha2::Sha256::digest(&body));
    let size_bytes = body.len() as i64;

    windmill_object_store::put_bytes_to_store(os, &s3_key, body).await?;

    sqlx::query!(
        "INSERT INTO sandbox_snapshot \
         (workspace_id, name, tag, s3_key, content_hash, docker_image, size_bytes, status, created_by) \
         VALUES ($1, $2, $3, $4, $5, 'uploaded', $6, 'ready', $7) \
         ON CONFLICT (workspace_id, name, tag) DO UPDATE SET \
         s3_key = $4, content_hash = $5, size_bytes = $6, status = 'ready', \
         build_error = NULL, updated_at = now()",
        w_id,
        name,
        tag,
        &s3_key,
        &content_hash,
        size_bytes,
        username,
    )
    .execute(db)
    .await?;

    Ok(format!(
        "Snapshot {}:{} uploaded ({} bytes, hash={})",
        name, tag, size_bytes, &content_hash[..12]
    ))
}

#[cfg(all(not(feature = "private"), not(feature = "parquet")))]
pub async fn upload_snapshot_bytes(
    _db: &windmill_common::DB,
    _w_id: &str,
    _name: &str,
    _tag: &str,
    _body: bytes::Bytes,
    _username: &str,
) -> windmill_common::error::Result<String> {
    Err(windmill_common::error::Error::ExecutionErr(
        "Sandbox snapshots require the parquet feature (S3 object store)".to_string(),
    ))
}
