#[cfg(feature = "enterprise")]
use crate::{ROOT_CACHE_DIR, ROOT_TMP_CACHE_DIR, TAR_CACHE_RATE, TMP_DIR};
#[cfg(feature = "enterprise")]
use itertools::Itertools;
#[cfg(feature = "enterprise")]
use rand::Rng;
#[cfg(feature = "enterprise")]
use std::process::Stdio;

#[cfg(feature = "enterprise")]
use tokio::{process::Command, sync::mpsc::Sender, time::Instant};

#[cfg(feature = "enterprise")]
use windmill_common::error;

#[cfg(feature = "enterprise")]
const TAR_CACHE_FILENAME: &str = "entirecache.tar";

#[cfg(feature = "enterprise")]
pub async fn cache_global(bucket: &str, tx: Sender<()>) -> error::Result<()> {
    copy_cache_from_bucket(bucket, tx).await?;
    copy_cache_to_bucket(bucket).await?;

    // this is to prevent excessive tar upload. 1/100*15min = each worker sync its tar once per day on average
    if rand::thread_rng().gen_range(0..*TAR_CACHE_RATE) == 0 {
        copy_cache_to_bucket_as_tar(bucket).await;
    }
    Ok(())
}

#[cfg(feature = "enterprise")]
pub async fn copy_cache_from_bucket(bucket: &str, tx: Sender<()>) -> error::Result<()> {
    tracing::info!("Copying cache from bucket in the background {bucket}");
    let bucket = bucket.to_string();

    let start = Instant::now();

    if let Err(e) = execute_command(
        ROOT_TMP_CACHE_DIR,
        "rclone",
        vec![
            "copy",
            &format!(":s3,env_auth=true:{bucket}"),
            &ROOT_TMP_CACHE_DIR,
            "--size-only",
            "--fast-list",
            "--exclude",
            &format!("deno/gen/file/tmp/windmill/**"),
            "--exclude",
            &format!("{TAR_CACHE_FILENAME}"),
        ],
    )
    .await
    {
        tracing::info!("Failed to to copy cache from bucket. Error: {:?}", e);
        return Err(e);
    }

    tracing::info!(
        "Finished copying cache from bucket {bucket}, took {:?}s",
        start.elapsed().as_secs()
    );

    tx.send(()).await.expect("can send copy cache signal");

    Ok(())
}

#[cfg(feature = "enterprise")]
pub async fn copy_cache_to_bucket(bucket: &str) -> error::Result<()> {
    tracing::info!("Copying cache to bucket {bucket}");
    let start = Instant::now();

    if let Err(e) = execute_command(
        ROOT_TMP_CACHE_DIR,
        "rclone",
        vec![
            "copy",
            &ROOT_TMP_CACHE_DIR,
            &format!(":s3,env_auth=true:{bucket}"),
            "--size-only",
            "--fast-list",
            "--exclude",
            &format!("deno/gen/file/tmp/windmill/**"),
            "--exclude",
            &format!("{TAR_CACHE_FILENAME}"),
        ],
    )
    .await
    {
        tracing::info!("Failed to to copy cache to bucket. Error: {:?}", e);
        return Err(e);
    }
    tracing::info!(
        "Finished copying cache to bucket {bucket}, took: {:?}s",
        start.elapsed().as_secs()
    );
    Ok(())
}

#[cfg(feature = "enterprise")]
pub async fn copy_cache_to_bucket_as_tar(bucket: &str) {
    tracing::info!("Copying cache to bucket {bucket} as tar");
    let start = Instant::now();

    if let Err(e) = execute_command(
        ROOT_TMP_CACHE_DIR,
        "tar",
        vec![
            "-c",
            "-f",
            &format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}"),
            "pip",
            "go",
            "deno",
        ],
    )
    .await
    {
        tracing::info!("Failed to tar cache. Error: {:?}", e);
        return;
    }

    let tar_metadata =
        tokio::fs::metadata(format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}")).await;
    if tar_metadata.is_err() || tar_metadata.as_ref().unwrap().len() == 0 {
        tracing::info!("Failed to tar cache");
        return;
    }

    if let Err(e) = execute_command(
        ROOT_TMP_CACHE_DIR,
        "rclone",
        vec![
            "copyto",
            &format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}"),
            &format!(":s3,env_auth=true:{bucket}/{TAR_CACHE_FILENAME}"),
            "-v",
            "--size-only",
            "--fast-list",
        ],
    )
    .await
    {
        tracing::info!("Failed to copy tar to bucket. Error: {:?}", e);
        return;
    }

    if let Err(e) =
        tokio::fs::remove_file(format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}")).await
    {
        tracing::info!("Failed to remove tar cache. Error: {:?}", e);
    };

    tracing::info!(
        "Finished copying cache to bucket {bucket} as tar, took: {:?}s. Size of new tar: {}",
        start.elapsed().as_secs(),
        tar_metadata.unwrap().len()
    );
}

#[cfg(feature = "enterprise")]
pub async fn copy_cache_from_bucket_as_tar(bucket: &str) {
    use tokio::fs::metadata;

    tracing::info!("Copying cache from bucket {bucket} as tar");

    if metadata(&ROOT_TMP_CACHE_DIR).await.is_ok() {
        if let Err(e) = tokio::fs::remove_dir_all(&ROOT_TMP_CACHE_DIR).await {
            tracing::info!(error = %e, "Could not remove root tmp cache dir");
        }
    }

    tokio::fs::create_dir_all(&ROOT_TMP_CACHE_DIR)
        .await
        .expect("Could not create root tmp cache dir");

    let elapsed = Instant::now();

    if let Err(e) = execute_command(
        ROOT_CACHE_DIR,
        "rclone",
        vec![
            "copyto",
            &format!(":s3,env_auth=true:{bucket}/{TAR_CACHE_FILENAME}"),
            &format!("{ROOT_CACHE_DIR}{TAR_CACHE_FILENAME}"),
            "-v",
            "--size-only",
            "--fast-list",
        ],
    )
    .await
    {
        tracing::info!("Failed copy tar from cache. Error: {:?}", e);
        return;
    }

    if let Err(e) = execute_command(
        ROOT_CACHE_DIR,
        "tar",
        vec!["-xpvf", &format!("{ROOT_CACHE_DIR}{TAR_CACHE_FILENAME}")],
    )
    .await
    {
        tracing::info!("Failed to untar cache. Error: {:?}", e);
        return;
    }

    if let Err(e) =
        tokio::fs::remove_dir_all(format!("{ROOT_CACHE_DIR}deno/gen/file/tmp/windmill")).await
    {
        tracing::info!("Failed to remove tmp gen windmill. Error: {:?}", e);
    };

    if let Err(e) = tokio::fs::remove_file(format!("{ROOT_CACHE_DIR}{TAR_CACHE_FILENAME}")).await {
        tracing::info!("Failed to remove tar cache. Error: {:?}", e);
        return;
    };

    tracing::info!(
        "Finished copying cache from bucket {bucket} as tar, took: {:?}s",
        elapsed.elapsed().as_secs()
    );

    for x in ["deno", "go", "pip"] {
        if let Err(e) = execute_command(
            TMP_DIR,
            "cp",
            vec!["-r", &format!("{ROOT_CACHE_DIR}{x}"), &ROOT_TMP_CACHE_DIR],
        )
        .await
        {
            tracing::info!(error = %e, "Could not copy root dir to tmp root dir");
        }
    }
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

#[cfg(feature = "enterprise")]
pub async fn copy_tmp_cache_to_cache() -> error::Result<()> {
    let start: Instant = Instant::now();
    execute_command(
        TMP_DIR,
        "rclone",
        vec![
            "sync",
            ROOT_TMP_CACHE_DIR,
            ROOT_CACHE_DIR,
            "--exclude",
            TAR_CACHE_FILENAME,
        ],
    )
    .await?;
    tracing::info!(
        "Finished copying local tmp cache to local cache. Took {}ms",
        start.elapsed().as_millis(),
    );
    Ok(())
}

#[cfg(feature = "enterprise")]
pub async fn copy_cache_to_tmp_cache() -> error::Result<()> {
    let start: Instant = Instant::now();
    execute_command(
        TMP_DIR,
        "rclone",
        vec![
            "sync",
            ROOT_CACHE_DIR,
            ROOT_TMP_CACHE_DIR,
            "--exclude",
            TAR_CACHE_FILENAME,
        ],
    )
    .await?;
    tracing::info!(
        "Finished copying local cache to local tmp cache. Took {}ms",
        start.elapsed().as_millis()
    );
    Ok(())
}

#[cfg(feature = "enterprise")]
pub async fn execute_command(dir: &str, command: &str, args: Vec<&str>) -> error::Result<()> {
    match Command::new(command)
        .current_dir(dir)
        .args(args.clone())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            if !h.wait().await.unwrap().success() {
                return Err(error::Error::ExecutionErr(format!(
                    "Failed to apply {command} with args: {}",
                    args.iter().join(" ")
                )));
            }
        }
        Err(e) => {
            return Err(error::Error::ExecutionErr(format!(
                "Failed to apply {command} with args: {}. Error: {e:?}",
                args.iter().join(" ")
            )));
        }
    }
    Ok(())
}
