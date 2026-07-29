// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use crate::aws::STORE;
use crate::{ClientOptions, Result};

/// A specialized `Error` for object store-related errors
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Bucket '{}' not found", bucket)]
    BucketNotFound { bucket: String },

    #[error("Failed to resolve region for bucket '{}'", bucket)]
    ResolveRegion {
        bucket: String,
        source: reqwest::Error,
    },

    #[error("Failed to parse the region for bucket '{}'", bucket)]
    RegionParse { bucket: String },
}

impl From<Error> for crate::Error {
    fn from(source: Error) -> Self {
        Self::Generic {
            store: STORE,
            source: Box::new(source),
        }
    }
}

/// Get the bucket region using the [HeadBucket API]. This will fail if the bucket does not exist.
///
/// [HeadBucket API]: https://docs.aws.amazon.com/AmazonS3/latest/API/API_HeadBucket.html
pub async fn resolve_bucket_region(bucket: &str, client_options: &ClientOptions) -> Result<String> {
    use reqwest::StatusCode;

    let endpoint = format!("https://{}.s3.amazonaws.com", bucket);

    let client = client_options.client()?;

    let response = client.head(&endpoint).send().await.map_err(|source| {
        let bucket = bucket.into();
        Error::ResolveRegion { bucket, source }
    })?;

    if response.status() == StatusCode::NOT_FOUND {
        let bucket = bucket.into();
        return Err(Error::BucketNotFound { bucket }.into());
    }

    let region = response
        .headers()
        .get("x-amz-bucket-region")
        .and_then(|x| x.to_str().ok())
        .ok_or_else(|| Error::RegionParse {
            bucket: bucket.into(),
        })?;

    Ok(region.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bucket_does_not_exist() {
        let bucket = "please-dont-exist";

        let result = resolve_bucket_region(bucket, &ClientOptions::new()).await;

        assert!(result.is_err());
    }
}
