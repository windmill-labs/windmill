#[cfg(feature = "enterprise")]
use crate::{
    DENO_CACHE_DIR, DENO_TMP_CACHE_DIR, GO_CACHE_DIR, GO_TMP_CACHE_DIR, PIP_CACHE_DIR,
    PIP_TMP_CACHE_DIR,
};

use crate::{ROOT_CACHE_DIR, ROOT_TMP_CACHE_DIR};
#[cfg(feature = "enterprise")]
use std::process::Stdio;
#[cfg(feature = "enterprise")]
use tokio::{fs::DirBuilder, process::Command, sync::mpsc::Sender, time::Instant};
use windmill_common::error;

#[cfg(feature = "enterprise")]
const TAR_CACHE_FILENAME: &str = "entirecache.tar";

#[cfg(feature = "enterprise")]
pub async fn copy_cache_from_bucket(
    bucket: &str,
    tx: Option<Sender<()>>,
) -> Option<tokio::task::JoinHandle<()>> {
    tracing::info!("Copying cache from bucket in the background {bucket}");
    let bucket = bucket.to_string();
    let tx_is_some = tx.is_some();
    let f = async move {
        let elapsed = Instant::now();

        match Command::new("rclone")
            .arg("copy")
            .arg(format!(":s3,env_auth=true:{bucket}"))
            .arg(if tx_is_some {
                ROOT_TMP_CACHE_DIR
            } else {
                ROOT_CACHE_DIR
            })
            .arg("--size-only")
            .arg("--fast-list")
            .arg("--exclude")
            .arg(format!("\"{TAR_CACHE_FILENAME},/deno/gen/file/**\""))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
        {
            Ok(mut h) => {
                h.wait().await.unwrap();
            }
            Err(e) => tracing::warn!("Failed to run periodic job pull. Error: {:?}", e),
        }
        tracing::info!(
            "Finished copying cache from bucket {bucket}, took {:?}s",
            elapsed.elapsed().as_secs()
        );

        for x in if !tx_is_some {
            [PIP_CACHE_DIR, DENO_CACHE_DIR, GO_CACHE_DIR]
        } else {
            [PIP_TMP_CACHE_DIR, DENO_TMP_CACHE_DIR, GO_TMP_CACHE_DIR]
        } {
            DirBuilder::new()
                .recursive(true)
                .create(x)
                .await
                .expect("could not create initial worker dir");
        }

        if let Some(tx) = tx {
            tx.send(()).await.expect("can send copy cache signal");
        }
    };
    if tx_is_some {
        return Some(tokio::spawn(f));
    } else {
        f.await;
        return None;
    }
}

#[cfg(feature = "enterprise")]
pub async fn copy_cache_to_bucket(bucket: &str) {
    tracing::info!("Copying cache to bucket {bucket}");
    let elapsed = Instant::now();
    match Command::new("rclone")
        .arg("copy")
        .arg(ROOT_CACHE_DIR)
        .arg(format!(":s3,env_auth=true:{bucket}"))
        .arg("--size-only")
        .arg("--fast-list")
        .arg("--exclude")
        .arg(format!("\"{TAR_CACHE_FILENAME},/deno/gen/file/**\""))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            h.wait().await.unwrap();
        }
        Err(e) => tracing::info!("Failed to run periodic job push. Error: {:?}", e),
    }
    tracing::info!(
        "Finished copying cache to bucket {bucket}, took: {:?}s",
        elapsed.elapsed().as_secs()
    );
}

#[cfg(feature = "enterprise")]
pub async fn copy_cache_to_bucket_as_tar(bucket: &str) {
    tracing::info!("Copying cache to bucket {bucket} as tar");
    let elapsed = Instant::now();

    match Command::new("tar")
        .current_dir(ROOT_CACHE_DIR)
        .arg("-c")
        .arg("-f")
        .arg(format!("{ROOT_CACHE_DIR}{TAR_CACHE_FILENAME}"))
        .args(&["pip", "go", "deno"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            if !h.wait().await.unwrap().success() {
                tracing::info!("Failed to tar cache");
                return;
            }
        }
        Err(e) => {
            tracing::info!("Failed tar cache. Error: {e:?}");
            return;
        }
    }

    let tar_metadata = tokio::fs::metadata(format!("{ROOT_CACHE_DIR}{TAR_CACHE_FILENAME}")).await;
    if tar_metadata.is_err() || tar_metadata.as_ref().unwrap().len() == 0 {
        tracing::info!("Failed to tar cache");
        return;
    }

    match Command::new("rclone")
        .current_dir(ROOT_CACHE_DIR)
        .arg("copyto")
        .arg(TAR_CACHE_FILENAME)
        .arg(format!(":s3,env_auth=true:{bucket}/{TAR_CACHE_FILENAME}"))
        .arg("-vv")
        .arg("--size-only")
        .arg("--fast-list")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            h.wait().await.unwrap();
        }
        Err(e) => tracing::info!("Failed to copy tar cache to bucket. Error: {:?}", e),
    }

    if let Err(e) = tokio::fs::remove_file(format!("{ROOT_CACHE_DIR}{TAR_CACHE_FILENAME}")).await {
        tracing::info!("Failed to remove tar cache. Error: {:?}", e);
    };

    tracing::info!(
        "Finished copying cache to bucket {bucket} as tar, took: {:?}s. Size of new tar: {}",
        elapsed.elapsed().as_secs(),
        tar_metadata.unwrap().len()
    );
}

#[cfg(feature = "enterprise")]
pub async fn copy_cache_from_bucket_as_tar(bucket: &str) -> bool {
    tracing::info!("Copying cache from bucket {bucket} as tar");
    let elapsed = Instant::now();

    match Command::new("rclone")
        .arg("copyto")
        .arg(format!(":s3,env_auth=true:{bucket}/{TAR_CACHE_FILENAME}"))
        .arg(format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}"))
        .arg("-vv")
        .arg("--size-only")
        .arg("--fast-list")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            if !h.wait().await.unwrap().success() {
                tracing::info!("Failed to download tar cache, continuing nonetheless");
                return false;
            }
        }
        Err(e) => {
            tracing::info!("Failed to download tar cache, continuing nonetheless. Error: {e:?}");
            return false;
        }
    }

    match Command::new("tar")
        .current_dir(ROOT_TMP_CACHE_DIR)
        .arg("-xpvf")
        .arg(format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            if !h.wait().await.unwrap().success() {
                tracing::info!("Failed to untar cache, continuing nonetheless");
                return false;
            }
        }
        Err(e) => {
            tracing::warn!("Failed to untar cache, continuing nonetheless. Error: {e:?}");
            return false;
        }
    }

    if let Err(e) =
        tokio::fs::remove_file(format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}")).await
    {
        tracing::info!("Failed to remove tar cache. Error: {:?}", e);
    };

    let r = move_tmp_cache_to_cache().await.is_ok();

    tracing::info!(
        "Finished copying cache from bucket {bucket} as tar, took: {:?}s. copy success: {r}",
        elapsed.elapsed().as_secs()
    );
    return r;
}

// async fn check_if_bucket_syncable(bucket: &str) -> bool {
//     match Command::new("rclone")
//         .arg("lsf")
//         .arg(format!(":s3,env_auth=true:{bucket}/NOSYNC"))

//         .arg("-vv")
//         .arg("--fast-list")
//         .stdin(Stdio::null())
//         .stdout(Stdio::null())
//         .output()
//         .await;
//     return true;
// }

pub async fn move_tmp_cache_to_cache() -> error::Result<()> {
    tokio::fs::remove_dir_all(ROOT_CACHE_DIR).await?;
    tokio::fs::rename(ROOT_TMP_CACHE_DIR, ROOT_CACHE_DIR).await?;
    tracing::info!("Finished moving tmp cache to cache");
    Ok(())
}
