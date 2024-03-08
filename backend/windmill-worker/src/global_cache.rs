#[cfg(feature = "enterprise")]
use crate::{ROOT_CACHE_DIR, ROOT_TMP_CACHE_DIR, TAR_PIP_TMP_CACHE_DIR};
#[cfg(feature = "enterprise")]
use itertools::Itertools;
// #[cfg(feature = "enterprise")]
// use rand::Rng;
#[cfg(feature = "enterprise")]
use std::process::Stdio;
#[cfg(feature = "enterprise")]
use windmill_common::DB;

#[cfg(feature = "enterprise")]
use windmill_common::global_settings::WORKER_S3_BUCKET_SYNC;

#[cfg(feature = "enterprise")]
use tokio::{process::Command, time::Instant};

#[cfg(feature = "enterprise")]
use windmill_common::error;

// #[cfg(feature = "enterprise")]
// const TAR_CACHE_FILENAME: &str = "denogocache.tar";

#[cfg(feature = "enterprise")]
pub async fn build_tar_and_push(bucket: &str, folder: String) -> error::Result<()> {
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

// #[cfg(feature = "enterprise")]
// pub async fn cache_global(bucket: &str, _tx: Sender<()>) -> error::Result<()> {
//     // copy_cache_from_bucket(bucket, tx).await?;
//     // copy_cache_to_bucket(bucket).await?;

//     // this is to prevent excessive tar upload. 1/100*15min = each worker sync its tar once per day on average
//     if rand::thread_rng().gen_range(0..*TAR_CACHE_RATE) == 0 {
//         copy_cache_to_bucket_as_tar(bucket).await;
//     }
//     Ok(())
// }

// #[cfg(feature = "enterprise")]
// pub async fn copy_cache_from_bucket(bucket: &str, tx: Sender<()>) -> error::Result<()> {
//     tracing::info!("Copying cache from bucket in the background {bucket}");
//     let bucket = bucket.to_string();

//     let start = Instant::now();

//     if let Err(e) = execute_command(
//         ROOT_TMP_CACHE_DIR,
//         "rclone",
//         vec![
//             "copy",
//             &format!(":s3,env_auth=true:{bucket}"),
//             &ROOT_TMP_CACHE_DIR,
//             // "-l",
//             "--size-only",
//             "--fast-list",
//             "--filter",
//             "+ deno/npm/**",
//             "--filter",
//             "+ deno/deps/**",
//             // "--filter",
//             // "+ bun/**",
//             "--filter",
//             "+ go/**",
//             "--filter",
//             "+ tar/**",
//             "--filter",
//             "- *",
//         ],
//     )
//     .await
//     {
//         tracing::info!("Failed to copy cache from bucket. Error: {:?}", e);
//         return Err(e);
//     }

//     tracing::info!(
//         "Finished copying cache from bucket {bucket}, took {:?}s",
//         start.elapsed().as_secs()
//     );

//     tx.send(()).await.expect("can send copy cache signal");

//     Ok(())
// }

// #[cfg(feature = "enterprise")]
// pub async fn copy_cache_to_bucket(bucket: &str) -> error::Result<()> {
//     tracing::info!("Copying cache to bucket {bucket}");
//     let start = Instant::now();

//     if let Err(e) = execute_command(
//         ROOT_TMP_CACHE_DIR,
//         "rclone",
//         vec![
//             "copy",
//             &ROOT_TMP_CACHE_DIR,
//             &format!(":s3,env_auth=true:{bucket}"),
//             // "-l",
//             "--size-only",
//             "--fast-list",
//             "--filter",
//             "+ deno/npm/**",
//             "--filter",
//             "+ deno/deps/**",
//             // "--filter",
//             // "+ bun/**",
//             "--filter",
//             "+ go/**",
//             "--filter",
//             "- *",
//         ],
//     )
//     .await
//     {
//         tracing::info!("Failed to copy cache to bucket. Error: {:?}", e);
//         return Err(e);
//     }
//     tracing::info!(
//         "Finished copying cache to bucket {bucket}, took: {:?}s",
//         start.elapsed().as_secs()
//     );
//     Ok(())
// }

#[cfg(feature = "enterprise")]
pub async fn worker_s3_bucket_sync_enabled(db: &DB) -> bool {
    let q = sqlx::query!(
        "SELECT value FROM global_settings WHERE name = $1",
        WORKER_S3_BUCKET_SYNC
    )
    .fetch_optional(db)
    .await;

    if let Ok(q) = q {
        let r = q.map(|x| x.value.as_bool().unwrap_or(true)).unwrap_or(true);
        tracing::info!("Got global setting {WORKER_S3_BUCKET_SYNC}: {}", r);
        r
    } else {
        tracing::info!("Failed to get global setting {WORKER_S3_BUCKET_SYNC}");
        false
    }
}

// #[cfg(feature = "enterprise")]
// pub async fn copy_cache_to_bucket_as_tar(bucket: &str) {
//     tracing::info!("Copying cache to bucket {bucket} as tar");
//     let start = Instant::now();

//     if let Err(e) = execute_command(
//         ROOT_TMP_CACHE_DIR,
//         "tar",
//         vec![
//             "-c",
//             "-f",
//             &format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}"),
//             "go",
//             "deno/npm",
//             "deno/deps", // "bun",
//         ],
//     )
//     .await
//     {
//         tracing::info!("Failed to tar cache. Error: {:?}", e);
//         return;
//     }

//     let tar_metadata =
//         tokio::fs::metadata(format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}")).await;
//     if tar_metadata.is_err() || tar_metadata.as_ref().unwrap().len() == 0 {
//         tracing::info!("Failed to tar cache");
//         return;
//     }

//     if let Err(e) = execute_command(
//         ROOT_TMP_CACHE_DIR,
//         "rclone",
//         vec![
//             "copyto",
//             &format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}"),
//             &format!(":s3,env_auth=true:{bucket}/{TAR_CACHE_FILENAME}"),
//             "-v",
//             "--size-only",
//             "--fast-list",
//             "--s3-no-check-bucket",
//         ],
//     )
//     .await
//     {
//         tracing::info!("Failed to copy tar to bucket. Error: {:?}", e);
//         return;
//     }

//     if let Err(e) =
//         tokio::fs::remove_file(format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}")).await
//     {
//         tracing::info!("Failed to remove tar cache. Error: {:?}", e);
//     };

//     tracing::info!(
//         "Finished copying cache to bucket {bucket} as tar, took: {:?}s. Size of new tar: {}",
//         start.elapsed().as_secs(),
//         tar_metadata.unwrap().len()
//     );
// }

// #[cfg(feature = "enterprise")]
// pub async fn copy_denogo_cache_from_bucket_as_tar(bucket: &str) {
//     tracing::info!("Copying deno,go,bun cache from bucket {bucket} as tar");

//     let mut start: Instant = Instant::now();

//     if let Err(e) = execute_command(
//         ROOT_TMP_CACHE_DIR,
//         "rclone",
//         vec![
//             "copyto",
//             &format!(":s3,env_auth=true:{bucket}/{TAR_CACHE_FILENAME}"),
//             &format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}"),
//             "-v",
//             "--size-only",
//             "--fast-list",
//         ],
//     )
//     .await
//     {
//         tracing::info!("Failed copying deno,go,bun tar from cache. Error: {:?}", e);
//         return;
//     }

//     tracing::info!(
//         "Finished copying denogobun tar for from bucket as tar. took {}s",
//         start.elapsed().as_secs()
//     );

//     start = Instant::now();

//     if let Err(e) = execute_command(
//         ROOT_CACHE_DIR,
//         "tar",
//         vec![
//             "-xpvf",
//             &format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}"),
//         ],
//     )
//     .await
//     {
//         tracing::info!("Failed to untar denogobun tar to cache. Error: {:?}", e);
//         return;
//     }

//     tracing::info!(
//         "Finished untaring denogobun tar to cache. took: {}s",
//         start.elapsed().as_secs()
//     );

//     start = Instant::now();

//     if let Err(e) = execute_command(
//         ROOT_TMP_CACHE_DIR,
//         "tar",
//         vec![
//             "-xpvf",
//             &format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}"),
//         ],
//     )
//     .await
//     {
//         tracing::info!("Failed to untar denogobun tar to tmpcache. Error: {:?}", e);
//         return;
//     }

//     tracing::info!(
//         "Finished untaring denogobun tar to /tmpcache. took: {}s",
//         start.elapsed().as_secs()
//     );

//     if let Err(e) =
//         tokio::fs::remove_file(format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}")).await
//     {
//         tracing::info!("Failed to remove denogobuntar cache. Error: {:?}", e);
//     };
// }

#[cfg(feature = "enterprise")]
pub async fn copy_all_piptars_from_bucket(bucket: &str) {
    tracing::info!("Copying all piptars cache from bucket {bucket}");

    let start = Instant::now();

    if let Err(e) = execute_command(
        ROOT_CACHE_DIR,
        "rclone",
        vec![
            "copy",
            &format!(":s3,env_auth=true:{bucket}/tar/pip/"),
            &TAR_PIP_TMP_CACHE_DIR,
            "-v",
            "--size-only",
            "--fast-list",
        ],
    )
    .await
    {
        tracing::info!("Failed transferring all piptars from cache. Error: {:?}", e);
        return;
    }

    tracing::info!(
        "Finished transferring piptars from bucket {bucket} as tar, took: {:?}s",
        start.elapsed().as_secs()
    );
}

// #[cfg(feature = "enterprise")]
// pub async fn copy_tmp_cache_to_cache() -> error::Result<()> {
//     let start: Instant = Instant::now();
//     execute_command(
//         TMP_DIR,
//         "rclone",
//         vec![
//             "sync",
//             ROOT_TMP_CACHE_DIR,
//             ROOT_CACHE_DIR,
//             // "-l",
//             "--filter",
//             "+ deno/npm/**",
//             "--filter",
//             "+ deno/deps/**",
//             // "--filter",
//             // "+ bun/**",
//             "--filter",
//             "+ go/**",
//             "--filter",
//             "- *",
//         ],
//     )
//     .await?;

//     tracing::info!(
//         "Finished copying local tmp cache to local cache. Took {}ms",
//         start.elapsed().as_millis(),
//     );

//     if let Err(e) = untar_all_piptars().await {
//         tracing::info!("Failed to untar piptars. Error: {:?}", e);
//     }

//     Ok(())
// }

#[cfg(feature = "enterprise")]
pub async fn untar_all_piptars() -> error::Result<()> {
    use tokio::fs::{self, metadata};

    use crate::PIP_CACHE_DIR;

    let start: Instant = Instant::now();

    let mut entries = fs::read_dir(TAR_PIP_TMP_CACHE_DIR).await?;
    while let Some(entry) = entries.next_entry().await? {
        if let Err(e) = {
            let entry_path = entry.path();
            let path = entry_path.to_str().expect("Could not convert path to str");
            let folder = format!(
                "{PIP_CACHE_DIR}/{}",
                path.split('/')
                    .last()
                    .unwrap()
                    .strip_suffix(".tar")
                    .ok_or_else(|| error::Error::InternalErr(format!(
                        "Unexpected path file not ending in .tar for: {}",
                        path
                    )))?
            );
            if metadata(&folder).await.is_ok() {
                continue;
            }
            // tracing::info!("A: {path} {folder}");
            extract_pip_tar(&path, &folder).await?;
            Ok(()) as error::Result<()>
        } {
            tracing::info!("Failed to extract pip tar. Error: {:?}", e);
        }
    }

    tracing::info!(
        "Finished untarring all piptars. Took {}ms",
        start.elapsed().as_millis(),
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

// #[cfg(feature = "enterprise")]
// pub async fn copy_cache_to_tmp_cache() -> error::Result<()> {
//     let start: Instant = Instant::now();
//     execute_command(
//         TMP_DIR,
//         "rclone",
//         vec![
//             "sync",
//             ROOT_CACHE_DIR,
//             ROOT_TMP_CACHE_DIR,
//             // "-l",
//             "--filter",
//             "+ deno/npm/**",
//             "--filter",
//             "+ deno/deps/**",
//             // "--filter",
//             // "+ bun/**",
//             "--filter",
//             "+ go/**",
//             "--filter",
//             "- *",
//         ],
//     )
//     .await?;
//     tracing::info!(
//         "Finished copying local cache to local tmp cache. Took {}ms",
//         start.elapsed().as_millis()
//     );
//     Ok(())
// }

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
