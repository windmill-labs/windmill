use base64::Engine;
use futures;
use ulid;
use windmill_common::{client::AuthedClient, error::Error};
use windmill_types::s3::S3Object;
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
) -> Result<(String, String), Error> {
    // Download the image from S3
    let image_bytes = client
        .download_s3_file(workspace_id, &image.s3, image.storage.clone())
        .await
        .map_err(|e| Error::internal_err(format!("Failed to download S3 image: {}", e)))?;

    // Encode as base64 data URL
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&image_bytes);

    // Determine MIME type using mime_guess from file extension, with PNG as fallback
    let mime_type = mime_guess::from_path(&image.s3).first();
    let mime_type = mime_type
        .as_ref()
        .map(|mime| mime.essence_str())
        .unwrap_or("image/png");

    Ok((mime_type.to_string(), base64_data))
}

/// Prepare messages for API by converting S3Objects to base64 ImageUrls
pub async fn prepare_messages_for_api(
    messages: &[OpenAIMessage],
    client: &AuthedClient,
    workspace_id: &str,
) -> Result<Vec<OpenAIMessage>, Error> {
    let mut prepared_messages = Vec::new();

    for message in messages {
        let mut prepared_message = message.clone();

        if let Some(content) = &message.content {
            match content {
                OpenAIContent::Text(text) => {
                    prepared_message.content = Some(OpenAIContent::Text(text.clone()));
                }
                OpenAIContent::Parts(parts) => {
                    let mut prepared_content = Vec::new();

                    for part in parts {
                        match part {
                            ContentPart::S3Object { s3_object } => {
                                // Convert S3Object to base64 image URL
                                let (mime_type, image_bytes) =
                                    download_and_encode_s3_image(s3_object, client, workspace_id)
                                        .await?;
                                prepared_content.push(ContentPart::ImageUrl {
                                    image_url: ImageUrlData {
                                        url: format!("data:{};base64,{}", mime_type, image_bytes),
                                    },
                                });
                            }
                            other => {
                                // Keep Text and ImageUrl as-is
                                prepared_content.push(other.clone());
                            }
                        }
                    }

                    prepared_message.content = Some(OpenAIContent::Parts(prepared_content));
                }
            }
        }

        prepared_messages.push(prepared_message);
    }

    Ok(prepared_messages)
}
