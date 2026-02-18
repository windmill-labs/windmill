use tokio::time::Instant;
use windmill_common::error;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_object_store::object_store_reexports::ObjectStore;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use std::sync::Arc;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub const TARGET: &str = const_format::concatcp!(std::env::consts::OS, "_", std::env::consts::ARCH);

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub async fn build_tar_and_push(
    s3_client: Arc<dyn ObjectStore>,
    folder: String,
    lang: String,
    custom_folder_name: Option<String>,
    platform_agnostic: bool,
) -> error::Result<()> {
    use windmill_object_store::object_store_reexports::Path;
    use tokio::fs::create_dir_all;

    use crate::TAR_PYBASE_CACHE_DIR;

    tracing::info!("Started building and pushing piptar {folder}");
    let start = Instant::now();

    // e.g. tiny==1.0.0
    let folder_name = if let Some(name) = custom_folder_name {
        name
    } else {
        folder.split("/").last().unwrap().to_owned()
    };

    let prefix = &format!("{TAR_PYBASE_CACHE_DIR}/{}", lang);
    let tar_path = format!("{prefix}/{folder_name}_tar.tar");

    create_dir_all(prefix).await?;

    let tar_file = std::fs::File::create(&tar_path)?;
    let mut tar = tar::Builder::new(tar_file);
    tar.append_dir_all(".", &folder)?;

    let tar_metadata = tokio::fs::metadata(&tar_path).await;
    if tar_metadata.is_err() || tar_metadata.as_ref().unwrap().len() == 0 {
        tracing::info!("Failed to tar cache: {folder}");
        return Err(error::Error::ExecutionErr(format!(
            "Failed to tar cache: {folder}"
        )));
    }

    // let s3_settings = S3_CACHE_SETTINGS.read().await;
    // let s3_client = s3_settings.as_ref().ok_or_else(|| {
    //     error::Error::ExecutionErr("Failed to read s3 cache settings".to_string())
    // })?;
    if let Err(e) = s3_client
        .put(
            &Path::from(format!(
                "/tar/{}/{lang}/{folder_name}.tar",
                if platform_agnostic { "" } else { TARGET }
            )),
            std::fs::read(&tar_path)?.into(),
        )
        .await
    {
        tracing::info!("Failed to put tar to s3: {tar_path}. Error: {:?}", e);
        return Err(error::Error::ExecutionErr(format!(
            "Failed to put tar to s3: {tar_path}"
        )));
    }

    tokio::fs::remove_file(&tar_path).await.map_err(|e| {
        tracing::error!("Failed to remove piptar {folder_name}. Error: {:?}", e);
        e
    })?;

    tracing::info!(
        "Finished copying piptar {folder} to bucket as tar, took: {:?}s. Size of tar: {}",
        start.elapsed().as_secs(),
        tar_metadata.unwrap().len(),
    );
    Ok(())
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub async fn pull_from_tar(
    client: Arc<dyn ObjectStore>,
    folder: String,
    lang: String,
    custom_folder_name: Option<String>,
    platform_agnostic: bool,
) -> error::Result<()> {
    use windmill_object_store::attempt_fetch_bytes;

    let folder_name = if let Some(name) = custom_folder_name {
        name
    } else {
        folder.split("/").last().unwrap().to_owned()
    };

    tracing::info!("Attempting to pull tar {folder_name} from bucket");

    let start = Instant::now();

    let tar_path = format!(
        "tar/{}/{lang}/{folder_name}.tar",
        if platform_agnostic { "" } else { TARGET }
    );
    let bytes = attempt_fetch_bytes(client, &tar_path).await?;

    extract_tar(bytes, &folder).map_err(|e| {
        tracing::error!("Failed to extract piptar {folder_name}. Error: {:?}", e);
        e
    })?;

    tracing::info!(
        "Finished pulling and extracting {folder_name}. Took {:?}ms",
        start.elapsed().as_millis()
    );

    Ok(())
}

pub fn extract_tar(tar: bytes::Bytes, folder: &str) -> error::Result<()> {
    use bytes::Buf;

    let start: Instant = Instant::now();
    std::fs::create_dir_all(&folder)?;

    let mut ar = tar::Archive::new(tar.reader());

    if let Err(e) = ar.unpack(folder) {
        tracing::info!("Failed to untar to {folder}. Error: {:?}", e);
        std::fs::remove_dir_all(&folder)?;
        return Err(error::Error::ExecutionErr(format!(
            "Failed to untar tar {folder}. Error: {:?}",
            e
        )));
    }
    tracing::info!(
        "Finished extracting tar to {folder}. Took {}ms",
        start.elapsed().as_millis(),
    );
    Ok(())
}

/// Two-tier cache load: check local disk first, then fall back to instance object store.
/// Returns `(hit, log_message)`.
pub async fn load_cache(bin_path: &str, _remote_path: &str, is_dir: bool) -> (bool, String) {
    if tokio::fs::metadata(&bin_path).await.is_ok() {
        (true, format!("loaded from local cache: {}\n", bin_path))
    } else {
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(os) = windmill_object_store::get_object_store().await {
            let started = std::time::Instant::now();

            if let Ok(mut x) = windmill_object_store::attempt_fetch_bytes(os, _remote_path).await {
                if is_dir {
                    if let Err(e) = windmill_common::worker::extract_tar(x, bin_path).await {
                        tracing::error!("could not write tar archive locally: {e:?}");
                        return (
                            false,
                            "error writing tar archive from object store".to_string(),
                        );
                    }
                } else {
                    if let Err(e) = windmill_common::worker::write_binary_file(bin_path, &mut x) {
                        tracing::error!("could not write bundle/bin file locally: {e:?}");
                        return (
                            false,
                            "error writing bundle/bin file from object store".to_string(),
                        );
                    }
                }
                tracing::info!("loaded from object store {}", bin_path);
                return (
                    true,
                    format!(
                        "loaded bin/bundle from object store {} in {}ms",
                        bin_path,
                        started.elapsed().as_millis()
                    ),
                );
            }
        }
        let _ = is_dir;
        (false, "".to_string())
    }
}

/// Check whether a binary/bundle exists in local cache or instance object store.
pub async fn exists_in_cache(bin_path: &str, _remote_path: &str) -> bool {
    if tokio::fs::metadata(&bin_path).await.is_ok() {
        return true;
    } else {
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(os) = windmill_object_store::get_object_store().await {
            return os
                .get(&windmill_object_store::object_store_reexports::Path::from(_remote_path))
                .await
                .is_ok();
        }
        return false;
    }
}

/// Two-tier cache write: upload to instance object store, then copy to local disk.
pub async fn save_cache(
    local_cache_path: &str,
    _remote_cache_path: &str,
    origin: &str,
    is_dir: bool,
) -> windmill_common::error::Result<String> {
    use std::path::PathBuf;

    let mut _cached_to_s3 = false;
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if let Some(os) = windmill_object_store::get_object_store().await {
        use windmill_object_store::object_store_reexports::Path;
        let file_to_cache = if is_dir {
            let tar_path = format!(
                "{}/tar/{}_tar.tar",
                windmill_common::worker::ROOT_CACHE_DIR,
                local_cache_path
                    .split("/")
                    .last()
                    .unwrap_or(&uuid::Uuid::new_v4().to_string())
            );
            let tar_file = std::fs::File::create(&tar_path)?;
            let mut tar = tar::Builder::new(tar_file);
            tar.append_dir_all(".", &origin)?;
            let tar_metadata = tokio::fs::metadata(&tar_path).await;
            if tar_metadata.is_err() || tar_metadata.as_ref().unwrap().len() == 0 {
                tracing::info!("Failed to tar cache: {origin}");
                return Err(error::Error::ExecutionErr(format!(
                    "Failed to tar cache: {origin}"
                )));
            }
            tar_path
        } else {
            origin.to_owned()
        };

        if let Err(e) = os
            .put(
                &Path::from(_remote_cache_path),
                std::fs::read(&file_to_cache)?.into(),
            )
            .await
        {
            tracing::error!(
                "Failed to put bin to object store: {_remote_cache_path}. Error: {:?}",
                e
            );
        } else {
            _cached_to_s3 = true;
            if is_dir {
                tokio::fs::remove_dir_all(&file_to_cache).await?;
            }
        }
    }

    if true {
        if is_dir {
            windmill_common::worker::copy_dir_recursively(
                &PathBuf::from(origin),
                &PathBuf::from(local_cache_path),
            )?;
        } else {
            std::fs::copy(origin, local_cache_path)?;
        }
        Ok(format!(
            "\nwrote cached binary: {} (backed by EE distributed object store: {_cached_to_s3})\n",
            local_cache_path
        ))
    } else if _cached_to_s3 {
        Ok(format!(
            "wrote cached binary to object store {}\n",
            local_cache_path
        ))
    } else {
        Ok("".to_string())
    }
}
