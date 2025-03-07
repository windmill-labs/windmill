// #[cfg(feature = "enterprise")]
// use rand::Rng;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use tokio::time::Instant;

#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
use object_store::ObjectStore;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::error;

#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
use std::sync::Arc;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub const TARGET: &str = const_format::concatcp!(std::env::consts::OS, "_", std::env::consts::ARCH);

#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
pub async fn build_tar_and_push(
    s3_client: Arc<dyn ObjectStore>,
    folder: String,
    lang: String,
    custom_folder_name: Option<String>,
) -> error::Result<()> {
    use object_store::path::Path;

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
    let tar_path = format!("{prefix}/{folder_name}_tar.tar",);

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
            &Path::from(format!("/tar/{TARGET}/{lang}/{folder_name}.tar")),
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

#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
pub async fn pull_from_tar(
    client: Arc<dyn ObjectStore>,
    folder: String,
    lang: String,
    custom_folder_name: Option<String>,
) -> error::Result<()> {
    use windmill_common::s3_helpers::attempt_fetch_bytes;

    let folder_name = if let Some(name) = custom_folder_name {
        name
    } else {
        folder.split("/").last().unwrap().to_owned()
    };

    tracing::info!("Attempting to pull tar {folder_name} from bucket");

    let start = Instant::now();

    let tar_path = format!("tar/{TARGET}/{lang}/{folder_name}.tar");
    let bytes = attempt_fetch_bytes(client, &tar_path).await?;

    windmill_common::worker::extract_tar(bytes, &folder)
        .await
        .map_err(|e| {
            tracing::error!("Failed to extract tar {folder_name}. Error: {:?}", e);
            e
        })?;

    tracing::info!(
        "Finished pulling and extracting {folder_name}. Took {:?}ms",
        start.elapsed().as_millis()
    );

    Ok(())
}
