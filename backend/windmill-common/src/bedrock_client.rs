// Copyright Windmill Labs. All rights reserved.

//! Bedrock SDK client wrapper with bearer token authentication
//!
//! This module provides a wrapper around the AWS Bedrock Runtime client
//! that uses bearer token authentication and extracts the region from
//! the base URL.

use crate::bedrock_auth::BearerTokenProvider;
use crate::error::{Error, Result};
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::Client as BedrockRuntimeClient;

/// Wrapper around AWS Bedrock Runtime client
///
/// Provides a simplified interface for creating Bedrock clients
/// with bearer token authentication.
pub struct BedrockClient {
    client: BedrockRuntimeClient,
}

impl BedrockClient {
    /// Create client from bearer token (API key) and region
    ///
    /// # Arguments
    /// * `bearer_token` - API key from Windmill resource
    /// * `region` - AWS region (e.g., "us-east-1")
    ///
    /// # Example
    /// ```no_run
    /// # use windmill_common::bedrock_client::BedrockClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = BedrockClient::from_bearer_token(
    ///     "my-api-key".to_string(),
    ///     "us-east-1".to_string()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_bearer_token(bearer_token: String, region: String) -> Result<Self> {
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region))
            .token_provider(BearerTokenProvider::new(bearer_token))
            .load()
            .await;

        Ok(Self { client: BedrockRuntimeClient::new(&sdk_config) })
    }

    /// Get reference to underlying AWS SDK client
    ///
    /// This allows access to the full AWS SDK API while still
    /// benefiting from the simplified authentication setup.
    pub fn client(&self) -> &BedrockRuntimeClient {
        &self.client
    }
}

/// Extract AWS region from Bedrock base URL
///
/// # Arguments
/// * `base_url` - Bedrock endpoint URL
///
/// # Returns
/// The extracted AWS region name
///
/// # Examples
/// ```
/// # use windmill_common::bedrock_client::extract_region_from_url;
/// assert_eq!(
///     extract_region_from_url("https://bedrock-runtime.us-east-1.amazonaws.com").unwrap(),
///     "us-east-1"
/// );
/// assert_eq!(
///     extract_region_from_url("https://bedrock-runtime.eu-west-1.amazonaws.com").unwrap(),
///     "eu-west-1"
/// );
/// ```
pub fn extract_region_from_url(base_url: &str) -> Result<String> {
    let url_lower = base_url.to_lowercase();

    // Pattern: bedrock-runtime.{region}.amazonaws.com
    if let Some(start) = url_lower.find("bedrock-runtime.") {
        let after_prefix = &url_lower[start + "bedrock-runtime.".len()..];
        if let Some(end) = after_prefix.find(".amazonaws.com") {
            let region = &after_prefix[..end];
            if !region.is_empty() {
                return Ok(region.to_string());
            }
        }
    }

    Err(Error::BadRequest(format!(
        "Could not extract region from Bedrock URL: {}",
        base_url
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_region_us_east_1() {
        assert_eq!(
            extract_region_from_url("https://bedrock-runtime.us-east-1.amazonaws.com").unwrap(),
            "us-east-1"
        );
    }

    #[test]
    fn test_extract_region_us_west_2() {
        assert_eq!(
            extract_region_from_url("https://bedrock-runtime.us-west-2.amazonaws.com").unwrap(),
            "us-west-2"
        );
    }

    #[test]
    fn test_extract_region_eu_west_1() {
        assert_eq!(
            extract_region_from_url("https://bedrock-runtime.eu-west-1.amazonaws.com").unwrap(),
            "eu-west-1"
        );
    }

    #[test]
    fn test_extract_region_ap_southeast_1() {
        assert_eq!(
            extract_region_from_url("https://bedrock-runtime.ap-southeast-1.amazonaws.com")
                .unwrap(),
            "ap-southeast-1"
        );
    }

    #[test]
    fn test_extract_region_with_https() {
        assert_eq!(
            extract_region_from_url("https://bedrock-runtime.us-east-1.amazonaws.com/").unwrap(),
            "us-east-1"
        );
    }

    #[test]
    fn test_extract_region_invalid_url() {
        assert!(extract_region_from_url("https://invalid-url.com").is_err());
    }

    #[test]
    fn test_extract_region_no_region() {
        assert!(extract_region_from_url("https://bedrock-runtime.amazonaws.com").is_err());
    }

    #[test]
    fn test_extract_region_case_insensitive() {
        assert_eq!(
            extract_region_from_url("HTTPS://BEDROCK-RUNTIME.US-EAST-1.AMAZONAWS.COM").unwrap(),
            "us-east-1"
        );
    }
}
