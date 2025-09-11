use base64::Engine;
use futures;
use serde_json;
use ulid;
use windmill_common::{
    client::AuthedClient,
    error::Error,
    s3_helpers::S3Object,
};
use windmill_queue::MiniPulledJob;

use crate::ai::types::*;

/// Upload image to S3 and return S3Object
pub async fn upload_image_to_s3(
    base64_image: &str,
    job: &MiniPulledJob,
    client: &AuthedClient,
) -> Result<S3Object, Error> {
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_image)
        .map_err(|e| Error::internal_err(format!("Failed to decode base64 image: {}", e)))?;

    // Generate unique S3 key
    let unique_id = ulid::Ulid::new().to_string();
    let s3_key = format!("ai_images/{}/{}.png", job.id, unique_id);

    // Create byte stream
    let byte_stream = futures::stream::once(async move {
        Ok::<_, std::convert::Infallible>(bytes::Bytes::from(image_bytes))
    });

    // Upload to S3
    client
        .upload_s3_file(
            &job.workspace_id,
            s3_key.clone(),
            None, // storage - use default
            byte_stream,
        )
        .await
        .map_err(|e| Error::internal_err(format!("Failed to upload image to S3: {}", e)))?;

    Ok(S3Object {
        s3: s3_key,
        storage: None,
        filename: Some("generated_image.png".to_string()),
        presigned: None,
    })
}

/// Download an S3 image and convert it to a base64 data URL
pub async fn download_and_encode_s3_image(
    image: &S3Object,
    client: &AuthedClient,
    workspace_id: &str,
) -> Result<String, Error> {
    // Download the image from S3
    let image_bytes = client
        .download_s3_file(workspace_id, &image.s3, image.storage.clone())
        .await
        .map_err(|e| Error::internal_err(format!("Failed to download S3 image: {}", e)))?;

    // Encode as base64 data URL
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&image_bytes);

    // Determine MIME type based on file extension or default to PNG
    let mime_type = if image.s3.ends_with(".jpg") || image.s3.ends_with(".jpeg") {
        "image/jpeg"
    } else if image.s3.ends_with(".gif") {
        "image/gif"
    } else if image.s3.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png" // default
    };

    Ok(format!("data:{};base64,{}", mime_type, base64_data))
}