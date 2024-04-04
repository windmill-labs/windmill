#[cfg(all(feature = "enterprise", feature = "parquet"))]
use crate::{PIP_CACHE_DIR, ROOT_CACHE_DIR};

// #[cfg(feature = "enterprise")]
// use rand::Rng;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use tokio::time::Instant;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use object_store::ObjectStore;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::error;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use std::sync::Arc;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub async fn build_tar_and_push(
    s3_client: Arc<dyn ObjectStore>,
    folder: String,
) -> error::Result<()> {
    use bytes::Bytes;
    use object_store::path::Path;

    tracing::info!("Started building and pushing piptar {folder}");
    let start = Instant::now();
    let folder_name = folder.split("/").last().unwrap();
    let tar_path = format!("{PIP_CACHE_DIR}/{folder_name}_tar.tar",);

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
            &Path::from(format!("/tar/pip/{folder_name}.tar")),
            Bytes::from(std::fs::read(&tar_path)?),
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
pub const X25: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub async fn pull_from_tar(client: Arc<dyn ObjectStore>, folder: String) -> error::Result<()> {
    use object_store::path::Path;
    use tokio::fs::metadata;
    let folder_name = folder.split("/").last().unwrap();

    tracing::info!("Attempting to pull piptar {folder_name} from bucket");

    let start = Instant::now();
    let tar_path = format!("tar/pip/{folder_name}.tar");

    let object = client
        .get(&Path::from(format!("tar/pip/{folder_name}.tar")))
        .await;
    if let Err(e) = object {
        tracing::info!("Failed to pull tar from s3: {tar_path}. Error: {:?}", e);
        return Err(error::Error::ExecutionErr(format!(
            "Failed to pull tar from s3: {tar_path}"
        )));
    }

    let bytes = object.unwrap().bytes().await.unwrap();
    tracing::info!(
        "{tar_path} checksum: {}, len: {}",
        X25.checksum(&bytes),
        bytes.len()
    );

    if bytes.len() == 0 {
        tracing::info!(
            "piptar {folder_name} not found in bucket. Took {:?}ms",
            start.elapsed().as_millis()
        );
        return Err(error::Error::ExecutionErr(format!(
            "tar {folder_name} does not exist in bucket"
        )));
    }
    // tracing::info!("B: {target} {folder}");

    extract_pip_tar(bytes, &folder).await.map_err(|e| {
        tracing::error!("Failed to extract piptar {folder_name}. Error: {:?}", e);
        e
    })?;

    tracing::info!(
        "Finished pulling and extracting {folder_name}. Took {:?}ms",
        start.elapsed().as_millis()
    );

    Ok(())
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub async fn extract_pip_tar(tar: bytes::Bytes, folder: &str) -> error::Result<()> {
    use bytes::Buf;
    use tokio::fs::{self};

    let start: Instant = Instant::now();
    fs::create_dir_all(&folder).await?;

    let mut ar = tar::Archive::new(tar.reader());

    if let Err(e) = ar.unpack(folder) {
        tracing::info!("Failed to untar piptar. Error: {:?}", e);
        fs::remove_dir_all(&folder).await?;
        return Err(error::Error::ExecutionErr(format!(
            "Failed to untar piptar {folder}"
        )));
    }
    tracing::info!(
        "Finished extracting pip tar {folder}. Took {}ms",
        start.elapsed().as_millis(),
    );
    Ok(())
}
