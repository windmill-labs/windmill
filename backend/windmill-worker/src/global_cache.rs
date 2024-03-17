#[cfg(feature = "enterprise")]
use crate::{ROOT_TMP_CACHE_DIR, TAR_PIP_TMP_CACHE_DIR};
#[cfg(feature = "enterprise")]
use itertools::Itertools;
// #[cfg(feature = "enterprise")]
// use rand::Rng;
#[cfg(feature = "enterprise")]
use std::process::Stdio;

#[cfg(feature = "enterprise")]
use tokio::{process::Command, time::Instant};

#[cfg(feature = "enterprise")]
use windmill_common::error;

#[cfg(feature = "enterprise")]
pub async fn build_tar_and_push(bucket: String, folder: String) -> error::Result<()> {
    tracing::info!("Started building and pushing piptar {folder}");
    let start = Instant::now();
    let folder_name = folder.split("/").last().unwrap();
    let tar_path = format!("{TAR_PIP_TMP_CACHE_DIR}/{folder_name}.tar",);

    if let Err(e) = execute_command(
        ROOT_TMP_CACHE_DIR,
        "tar",
        vec!["-c", "-f", &tar_path, "-C", &folder, "."],
    )
    .await
    {
        tracing::info!("Failed to tar cache. Error: {:?}", e);
        return Err(e);
    }

    let tar_metadata = tokio::fs::metadata(&tar_path).await;
    if tar_metadata.is_err() || tar_metadata.as_ref().unwrap().len() == 0 {
        tracing::info!("Failed to tar cache: {folder}");
        return Err(error::Error::ExecutionErr(format!(
            "Failed to tar cache: {folder}"
        )));
    }

    let bucket = bucket.trim_start_matches("s3://");

    if let Err(e) = execute_command(
        ROOT_TMP_CACHE_DIR,
        "rclone",
        vec![
            "copyto",
            &tar_path,
            &format!(":s3,env_auth=true:{bucket}/tar/pip/{folder_name}.tar"),
            "-v",
            "--size-only",
            "--fast-list",
            "--s3-no-check-bucket",
        ],
    )
    .await
    {
        tracing::info!("Failed to copy piptar {folder} to bucket. Error: {:?}", e);
        return Err(e);
    }

    tracing::info!(
        "Finished copying piptar {folder} to bucket {bucket} as tar, took: {:?}s. Size of tar: {}",
        start.elapsed().as_secs(),
        tar_metadata.unwrap().len()
    );
    Ok(())
}

#[cfg(feature = "enterprise")]
pub async fn pull_from_tar(bucket: &str, folder: String) -> error::Result<()> {
    use tokio::fs::metadata;
    let folder_name = folder.split("/").last().unwrap();

    tracing::info!("Attempting to pull piptar {folder_name} from bucket");

    let start = Instant::now();
    let tar_path = format!("tar/pip/{folder_name}.tar");
    let target = format!("{ROOT_TMP_CACHE_DIR}/{tar_path}.single");
    let bucket = bucket.trim_start_matches("s3://");

    if let Err(e) = execute_command(
        ROOT_TMP_CACHE_DIR,
        "rclone",
        vec![
            "copyto",
            &format!(":s3,env_auth=true:{bucket}/{tar_path}"),
            &target,
            "-v",
            "--size-only",
            "--fast-list",
        ],
    )
    .await
    {
        tracing::info!(
            "Failed to copy tar {folder_name} from bucket. Error: {:?}",
            e
        );
        return Err(e);
    }

    if metadata(&target).await.is_err() {
        tracing::info!(
            "piptar {folder_name} not found in bucket. Took {:?}ms",
            start.elapsed().as_millis()
        );
        return Err(error::Error::ExecutionErr(format!(
            "tar {folder_name} does not exist in bucket"
        )));
    }
    // tracing::info!("B: {target} {folder}");

    extract_pip_tar(&target, &folder).await?;
    tokio::fs::remove_file(&target).await?;
    tracing::info!(
        "Finished pulling and extracting {folder_name}. Took {:?}ms",
        start.elapsed().as_millis()
    );

    Ok(())
}

#[cfg(feature = "enterprise")]
pub async fn extract_pip_tar(tar: &str, folder: &str) -> error::Result<()> {
    use tokio::fs;

    let start: Instant = Instant::now();
    fs::create_dir_all(&folder).await?;
    if let Err(e) = execute_command(&folder, "tar", vec!["-xpvf", tar]).await {
        tracing::info!("Failed to untar piptar. Error: {:?}", e);
        fs::remove_dir_all(&folder).await?;
        return Err(e);
    }
    tracing::info!(
        "Finished extracting pip tar {folder}. Took {}ms",
        start.elapsed().as_millis(),
    );
    Ok(())
}

#[cfg(feature = "enterprise")]
pub async fn execute_command(dir: &str, command: &str, args: Vec<&str>) -> error::Result<()> {
    tracing::info!("Executing command: {command} {}", args.iter().join(" "));
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
