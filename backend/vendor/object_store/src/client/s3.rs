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

//! The list and multipart API used by both GCS and S3

use crate::multipart::PartId;
use crate::path::Path;
use crate::{ListResult, ObjectMeta, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListResponse {
    #[serde(default)]
    pub contents: Vec<ListContents>,
    #[serde(default)]
    pub common_prefixes: Vec<ListPrefix>,
    #[serde(default)]
    pub next_continuation_token: Option<String>,
}

impl TryFrom<ListResponse> for ListResult {
    type Error = crate::Error;

    fn try_from(value: ListResponse) -> Result<Self> {
        let common_prefixes = value
            .common_prefixes
            .into_iter()
            .map(|x| Ok(Path::parse(x.prefix)?))
            .collect::<Result<_>>()?;

        let objects = value
            .contents
            .into_iter()
            .map(TryFrom::try_from)
            .collect::<Result<_>>()?;

        Ok(Self {
            common_prefixes,
            objects,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListPrefix {
    pub prefix: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListContents {
    pub key: String,
    pub size: u64,
    pub last_modified: DateTime<Utc>,
    #[serde(rename = "ETag")]
    pub e_tag: Option<String>,
}

impl TryFrom<ListContents> for ObjectMeta {
    type Error = crate::Error;

    fn try_from(value: ListContents) -> Result<Self> {
        Ok(Self {
            location: Path::parse(value.key)?,
            last_modified: value.last_modified,
            size: value.size,
            e_tag: value.e_tag,
            version: None,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct InitiateMultipartUploadResult {
    pub upload_id: String,
}

#[cfg(feature = "aws")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct CopyPartResult {
    #[serde(rename = "ETag")]
    pub e_tag: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct CompleteMultipartUpload {
    pub part: Vec<MultipartPart>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PartMetadata {
    pub e_tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_sha256: Option<String>,
}

impl From<Vec<PartId>> for CompleteMultipartUpload {
    fn from(value: Vec<PartId>) -> Self {
        let part = value
            .into_iter()
            .enumerate()
            .map(|(part_idx, part)| {
                let md = match quick_xml::de::from_str::<PartMetadata>(&part.content_id) {
                    Ok(md) => md,
                    // fallback to old way
                    Err(_) => PartMetadata {
                        e_tag: part.content_id.clone(),
                        checksum_sha256: None,
                    },
                };
                MultipartPart {
                    e_tag: md.e_tag,
                    part_number: part_idx + 1,
                    checksum_sha256: md.checksum_sha256,
                }
            })
            .collect();
        Self { part }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct MultipartPart {
    #[serde(rename = "ETag")]
    pub e_tag: String,
    #[serde(rename = "PartNumber")]
    pub part_number: usize,
    #[serde(rename = "ChecksumSHA256")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct CompleteMultipartUploadResult {
    #[serde(rename = "ETag")]
    pub e_tag: String,
}
