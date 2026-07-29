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

use super::credential::AzureCredential;
use crate::azure::credential::*;
use crate::azure::{AzureCredentialProvider, STORE};
use crate::client::builder::HttpRequestBuilder;
use crate::client::get::GetClient;
use crate::client::header::{get_put_result, HeaderConfig};
use crate::client::list::ListClient;
use crate::client::retry::RetryExt;
use crate::client::{GetOptionsExt, HttpClient, HttpError, HttpRequest, HttpResponse};
use crate::multipart::PartId;
use crate::path::DELIMITER;
use crate::util::{deserialize_rfc1123, GetRange};
use crate::{
    Attribute, Attributes, ClientOptions, GetOptions, ListResult, ObjectMeta, Path, PutMode,
    PutMultipartOpts, PutOptions, PutPayload, PutResult, Result, RetryConfig, TagSet,
};
use async_trait::async_trait;
use base64::prelude::{BASE64_STANDARD, BASE64_STANDARD_NO_PAD};
use base64::Engine;
use bytes::{Buf, Bytes};
use chrono::{DateTime, Utc};
use http::{
    header::{HeaderMap, HeaderValue, CONTENT_LENGTH, CONTENT_TYPE, IF_MATCH, IF_NONE_MATCH},
    HeaderName, Method,
};
use rand::Rng as _;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

const VERSION_HEADER: &str = "x-ms-version-id";
const USER_DEFINED_METADATA_HEADER_PREFIX: &str = "x-ms-meta-";
static MS_CACHE_CONTROL: HeaderName = HeaderName::from_static("x-ms-blob-cache-control");
static MS_CONTENT_TYPE: HeaderName = HeaderName::from_static("x-ms-blob-content-type");
static MS_CONTENT_DISPOSITION: HeaderName =
    HeaderName::from_static("x-ms-blob-content-disposition");
static MS_CONTENT_ENCODING: HeaderName = HeaderName::from_static("x-ms-blob-content-encoding");
static MS_CONTENT_LANGUAGE: HeaderName = HeaderName::from_static("x-ms-blob-content-language");

static TAGS_HEADER: HeaderName = HeaderName::from_static("x-ms-tags");

/// A specialized `Error` for object store-related errors
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Error performing get request {}: {}", path, source)]
    GetRequest {
        source: crate::client::retry::RetryError,
        path: String,
    },

    #[error("Error performing put request {}: {}", path, source)]
    PutRequest {
        source: crate::client::retry::RetryError,
        path: String,
    },

    #[error("Error performing delete request {}: {}", path, source)]
    DeleteRequest {
        source: crate::client::retry::RetryError,
        path: String,
    },

    #[error("Error performing bulk delete request: {}", source)]
    BulkDeleteRequest {
        source: crate::client::retry::RetryError,
    },

    #[error("Error receiving bulk delete request body: {}", source)]
    BulkDeleteRequestBody { source: HttpError },

    #[error(
        "Bulk delete request failed due to invalid input: {} (code: {})",
        reason,
        code
    )]
    BulkDeleteRequestInvalidInput { code: String, reason: String },

    #[error("Got invalid bulk delete response: {}", reason)]
    InvalidBulkDeleteResponse { reason: String },

    #[error(
        "Bulk delete request failed for key {}: {} (code: {})",
        path,
        reason,
        code
    )]
    DeleteFailed {
        path: String,
        code: String,
        reason: String,
    },

    #[error("Error performing list request: {}", source)]
    ListRequest {
        source: crate::client::retry::RetryError,
    },

    #[error("Error getting list response body: {}", source)]
    ListResponseBody { source: HttpError },

    #[error("Got invalid list response: {}", source)]
    InvalidListResponse { source: quick_xml::de::DeError },

    #[error("Unable to extract metadata from headers: {}", source)]
    Metadata {
        source: crate::client::header::Error,
    },

    #[error("ETag required for conditional update")]
    MissingETag,

    #[error("Error requesting user delegation key: {}", source)]
    DelegationKeyRequest {
        source: crate::client::retry::RetryError,
    },

    #[error("Error getting user delegation key response body: {}", source)]
    DelegationKeyResponseBody { source: HttpError },

    #[error("Got invalid user delegation key response: {}", source)]
    DelegationKeyResponse { source: quick_xml::de::DeError },

    #[error("Generating SAS keys with SAS tokens auth is not supported")]
    SASforSASNotSupported,

    #[error("Generating SAS keys while skipping signatures is not supported")]
    SASwithSkipSignature,
}

impl From<Error> for crate::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::GetRequest { source, path }
            | Error::DeleteRequest { source, path }
            | Error::PutRequest { source, path } => source.error(STORE, path),
            _ => Self::Generic {
                store: STORE,
                source: Box::new(err),
            },
        }
    }
}

/// Configuration for [AzureClient]
#[derive(Debug)]
pub(crate) struct AzureConfig {
    pub account: String,
    pub container: String,
    pub credentials: AzureCredentialProvider,
    pub retry_config: RetryConfig,
    pub service: Url,
    pub is_emulator: bool,
    pub skip_signature: bool,
    pub disable_tagging: bool,
    pub client_options: ClientOptions,
}

impl AzureConfig {
    pub(crate) fn path_url(&self, path: &Path) -> Url {
        let mut url = self.service.clone();
        {
            let mut path_mut = url.path_segments_mut().unwrap();
            if self.is_emulator {
                path_mut.push(&self.account);
            }
            path_mut.push(&self.container).extend(path.parts());
        }
        url
    }
    async fn get_credential(&self) -> Result<Option<Arc<AzureCredential>>> {
        if self.skip_signature {
            Ok(None)
        } else {
            Some(self.credentials.get_credential().await).transpose()
        }
    }
}

/// A builder for a put request allowing customisation of the headers and query string
struct PutRequest<'a> {
    path: &'a Path,
    config: &'a AzureConfig,
    payload: PutPayload,
    builder: HttpRequestBuilder,
    idempotent: bool,
}

impl PutRequest<'_> {
    fn header(self, k: &HeaderName, v: &str) -> Self {
        let builder = self.builder.header(k, v);
        Self { builder, ..self }
    }

    fn query<T: Serialize + ?Sized + Sync>(self, query: &T) -> Self {
        let builder = self.builder.query(query);
        Self { builder, ..self }
    }

    fn idempotent(self, idempotent: bool) -> Self {
        Self { idempotent, ..self }
    }

    fn with_tags(mut self, tags: TagSet) -> Self {
        let tags = tags.encoded();
        if !tags.is_empty() && !self.config.disable_tagging {
            self.builder = self.builder.header(&TAGS_HEADER, tags);
        }
        self
    }

    fn with_attributes(self, attributes: Attributes) -> Self {
        let mut builder = self.builder;
        let mut has_content_type = false;
        for (k, v) in &attributes {
            builder = match k {
                Attribute::CacheControl => builder.header(&MS_CACHE_CONTROL, v.as_ref()),
                Attribute::ContentDisposition => {
                    builder.header(&MS_CONTENT_DISPOSITION, v.as_ref())
                }
                Attribute::ContentEncoding => builder.header(&MS_CONTENT_ENCODING, v.as_ref()),
                Attribute::ContentLanguage => builder.header(&MS_CONTENT_LANGUAGE, v.as_ref()),
                Attribute::ContentType => {
                    has_content_type = true;
                    builder.header(&MS_CONTENT_TYPE, v.as_ref())
                }
                Attribute::Metadata(k_suffix) => builder.header(
                    &format!("{}{}", USER_DEFINED_METADATA_HEADER_PREFIX, k_suffix),
                    v.as_ref(),
                ),
            };
        }

        if !has_content_type {
            if let Some(value) = self.config.client_options.get_content_type(self.path) {
                builder = builder.header(&MS_CONTENT_TYPE, value);
            }
        }
        Self { builder, ..self }
    }

    fn with_extensions(self, extensions: ::http::Extensions) -> Self {
        let builder = self.builder.extensions(extensions);
        Self { builder, ..self }
    }

    async fn send(self) -> Result<HttpResponse> {
        let credential = self.config.get_credential().await?;
        let sensitive = credential
            .as_deref()
            .map(|c| c.sensitive_request())
            .unwrap_or_default();
        let response = self
            .builder
            .header(CONTENT_LENGTH, self.payload.content_length())
            .with_azure_authorization(&credential, &self.config.account)
            .retryable(&self.config.retry_config)
            .sensitive(sensitive)
            .idempotent(self.idempotent)
            .payload(Some(self.payload))
            .send()
            .await
            .map_err(|source| {
                let path = self.path.as_ref().into();
                Error::PutRequest { path, source }
            })?;

        Ok(response)
    }
}

#[inline]
fn extend(dst: &mut Vec<u8>, data: &[u8]) {
    dst.extend_from_slice(data);
}

// Write header names as title case. The header name is assumed to be ASCII.
// We need it because Azure is not always treating headers as case insensitive.
fn title_case(dst: &mut Vec<u8>, name: &[u8]) {
    dst.reserve(name.len());

    // Ensure first character is uppercased
    let mut prev = b'-';
    for &(mut c) in name {
        if prev == b'-' {
            c.make_ascii_uppercase();
        }
        dst.push(c);
        prev = c;
    }
}

fn write_headers(headers: &HeaderMap, dst: &mut Vec<u8>) {
    for (name, value) in headers {
        // We need special case handling here otherwise Azure returns 400
        // due to `Content-Id` instead of `Content-ID`
        if name == "content-id" {
            extend(dst, b"Content-ID");
        } else {
            title_case(dst, name.as_str().as_bytes());
        }
        extend(dst, b": ");
        extend(dst, value.as_bytes());
        extend(dst, b"\r\n");
    }
}

// https://docs.oasis-open.org/odata/odata/v4.0/errata02/os/complete/part1-protocol/odata-v4.0-errata02-os-part1-protocol-complete.html#_Toc406398359
fn serialize_part_delete_request(
    dst: &mut Vec<u8>,
    boundary: &str,
    idx: usize,
    request: HttpRequest,
    relative_url: String,
) {
    // Encode start marker for part
    extend(dst, b"--");
    extend(dst, boundary.as_bytes());
    extend(dst, b"\r\n");

    // Encode part headers
    let mut part_headers = HeaderMap::new();
    part_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/http"));
    part_headers.insert(
        "Content-Transfer-Encoding",
        HeaderValue::from_static("binary"),
    );
    // Azure returns 400 if we send `Content-Id` instead of `Content-ID`
    part_headers.insert("Content-ID", HeaderValue::from(idx));
    write_headers(&part_headers, dst);
    extend(dst, b"\r\n");

    // Encode the subrequest request-line
    extend(dst, b"DELETE ");
    extend(dst, format!("/{} ", relative_url).as_bytes());
    extend(dst, b"HTTP/1.1");
    extend(dst, b"\r\n");

    // Encode subrequest headers
    write_headers(request.headers(), dst);
    extend(dst, b"\r\n");
    extend(dst, b"\r\n");
}

fn parse_multipart_response_boundary(response: &HttpResponse) -> Result<String> {
    let invalid_response = |msg: &str| Error::InvalidBulkDeleteResponse {
        reason: msg.to_string(),
    };

    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .ok_or_else(|| invalid_response("missing Content-Type"))?;

    let boundary = content_type
        .as_ref()
        .strip_prefix(b"multipart/mixed; boundary=")
        .ok_or_else(|| invalid_response("invalid Content-Type value"))?
        .to_vec();

    let boundary =
        String::from_utf8(boundary).map_err(|_| invalid_response("invalid multipart boundary"))?;

    Ok(boundary)
}

fn invalid_response(msg: &str) -> Error {
    Error::InvalidBulkDeleteResponse {
        reason: msg.to_string(),
    }
}

#[derive(Debug)]
struct MultipartField {
    headers: HeaderMap,
    content: Bytes,
}

fn parse_multipart_body_fields(body: Bytes, boundary: &[u8]) -> Result<Vec<MultipartField>> {
    let start_marker = [b"--", boundary, b"\r\n"].concat();
    let next_marker = &start_marker[..start_marker.len() - 2];
    let end_marker = [b"--", boundary, b"--\r\n"].concat();

    // There should be at most 256 responses per batch
    let mut fields = Vec::with_capacity(256);
    let mut remaining: &[u8] = body.as_ref();
    loop {
        remaining = remaining
            .strip_prefix(start_marker.as_slice())
            .ok_or_else(|| invalid_response("missing start marker for field"))?;

        // The documentation only mentions two headers for fields, we leave some extra margin
        let mut scratch = [httparse::EMPTY_HEADER; 10];
        let mut headers = HeaderMap::new();
        match httparse::parse_headers(remaining, &mut scratch) {
            Ok(httparse::Status::Complete((pos, headers_slice))) => {
                remaining = &remaining[pos..];
                for header in headers_slice {
                    headers.insert(
                        HeaderName::from_bytes(header.name.as_bytes()).expect("valid"),
                        HeaderValue::from_bytes(header.value).expect("valid"),
                    );
                }
            }
            _ => return Err(invalid_response("unable to parse field headers").into()),
        };

        let next_pos = remaining
            .windows(next_marker.len())
            .position(|window| window == next_marker)
            .ok_or_else(|| invalid_response("early EOF while seeking to next boundary"))?;

        fields.push(MultipartField {
            headers,
            content: body.slice_ref(&remaining[..next_pos]),
        });

        remaining = &remaining[next_pos..];

        // Support missing final CRLF
        if remaining == end_marker || remaining == &end_marker[..end_marker.len() - 2] {
            break;
        }
    }
    Ok(fields)
}

async fn parse_blob_batch_delete_body(
    batch_body: Bytes,
    boundary: String,
    paths: &[Path],
) -> Result<Vec<Result<Path>>> {
    let mut results: Vec<Result<Path>> = paths.iter().cloned().map(Ok).collect();

    for field in parse_multipart_body_fields(batch_body, boundary.as_bytes())? {
        let id = field
            .headers
            .get("content-id")
            .and_then(|v| std::str::from_utf8(v.as_bytes()).ok())
            .and_then(|v| v.parse::<usize>().ok());

        // Parse part response headers
        // Documentation mentions 5 headers and states that other standard HTTP headers
        // may be provided, in order to not incurr in more complexity to support an arbitrary
        // amount of headers we chose a conservative amount and error otherwise
        // https://learn.microsoft.com/en-us/rest/api/storageservices/delete-blob?tabs=microsoft-entra-id#response-headers
        let mut headers = [httparse::EMPTY_HEADER; 48];
        let mut part_response = httparse::Response::new(&mut headers);
        match part_response.parse(&field.content) {
            Ok(httparse::Status::Complete(_)) => {}
            _ => return Err(invalid_response("unable to parse response").into()),
        };

        match (id, part_response.code) {
            (Some(_id), Some(code)) if (200..300).contains(&code) => {}
            (Some(id), Some(404)) => {
                results[id] = Err(crate::Error::NotFound {
                    path: paths[id].as_ref().to_string(),
                    source: Error::DeleteFailed {
                        path: paths[id].as_ref().to_string(),
                        code: 404.to_string(),
                        reason: part_response.reason.unwrap_or_default().to_string(),
                    }
                    .into(),
                });
            }
            (Some(id), Some(code)) => {
                results[id] = Err(Error::DeleteFailed {
                    path: paths[id].as_ref().to_string(),
                    code: code.to_string(),
                    reason: part_response.reason.unwrap_or_default().to_string(),
                }
                .into());
            }
            (None, Some(code)) => {
                return Err(Error::BulkDeleteRequestInvalidInput {
                    code: code.to_string(),
                    reason: part_response.reason.unwrap_or_default().to_string(),
                }
                .into())
            }
            _ => return Err(invalid_response("missing part response status code").into()),
        }
    }

    Ok(results)
}

#[derive(Debug)]
pub(crate) struct AzureClient {
    config: AzureConfig,
    client: HttpClient,
}

impl AzureClient {
    /// create a new instance of [AzureClient]
    pub(crate) fn new(config: AzureConfig, client: HttpClient) -> Self {
        Self { config, client }
    }

    /// Returns the config
    pub(crate) fn config(&self) -> &AzureConfig {
        &self.config
    }

    async fn get_credential(&self) -> Result<Option<Arc<AzureCredential>>> {
        self.config.get_credential().await
    }

    fn put_request<'a>(&'a self, path: &'a Path, payload: PutPayload) -> PutRequest<'a> {
        let url = self.config.path_url(path);
        let builder = self.client.request(Method::PUT, url.as_str());

        PutRequest {
            path,
            builder,
            payload,
            config: &self.config,
            idempotent: false,
        }
    }

    /// Make an Azure PUT request <https://docs.microsoft.com/en-us/rest/api/storageservices/put-blob>
    pub(crate) async fn put_blob(
        &self,
        path: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        let PutOptions {
            mode,
            tags,
            attributes,
            extensions,
        } = opts;

        let builder = self
            .put_request(path, payload)
            .with_attributes(attributes)
            .with_extensions(extensions)
            .with_tags(tags);

        let builder = match &mode {
            PutMode::Overwrite => builder.idempotent(true),
            PutMode::Create => builder.header(&IF_NONE_MATCH, "*"),
            PutMode::Update(v) => {
                let etag = v.e_tag.as_ref().ok_or(Error::MissingETag)?;
                builder.header(&IF_MATCH, etag)
            }
        };

        let response = builder.header(&BLOB_TYPE, "BlockBlob").send().await?;
        Ok(get_put_result(response.headers(), VERSION_HEADER)
            .map_err(|source| Error::Metadata { source })?)
    }

    /// PUT a block <https://learn.microsoft.com/en-us/rest/api/storageservices/put-block>
    pub(crate) async fn put_block(
        &self,
        path: &Path,
        _part_idx: usize,
        payload: PutPayload,
    ) -> Result<PartId> {
        let part_idx = u128::from_be_bytes(rand::rng().random());
        let content_id = format!("{part_idx:032x}");
        let block_id = BASE64_STANDARD.encode(&content_id);

        self.put_request(path, payload)
            .query(&[("comp", "block"), ("blockid", &block_id)])
            .idempotent(true)
            .send()
            .await?;

        Ok(PartId { content_id })
    }

    /// PUT a block list <https://learn.microsoft.com/en-us/rest/api/storageservices/put-block-list>
    pub(crate) async fn put_block_list(
        &self,
        path: &Path,
        parts: Vec<PartId>,
        opts: PutMultipartOpts,
    ) -> Result<PutResult> {
        let PutMultipartOpts {
            tags,
            attributes,
            extensions,
        } = opts;

        let blocks = parts
            .into_iter()
            .map(|part| BlockId::from(part.content_id))
            .collect();

        let payload = BlockList { blocks }.to_xml().into();
        let response = self
            .put_request(path, payload)
            .with_attributes(attributes)
            .with_tags(tags)
            .with_extensions(extensions)
            .query(&[("comp", "blocklist")])
            .idempotent(true)
            .send()
            .await?;

        Ok(get_put_result(response.headers(), VERSION_HEADER)
            .map_err(|source| Error::Metadata { source })?)
    }

    /// Make an Azure Delete request <https://docs.microsoft.com/en-us/rest/api/storageservices/delete-blob>
    pub(crate) async fn delete_request<T: Serialize + ?Sized + Sync>(
        &self,
        path: &Path,
        query: &T,
    ) -> Result<()> {
        let credential = self.get_credential().await?;
        let url = self.config.path_url(path);

        let sensitive = credential
            .as_deref()
            .map(|c| c.sensitive_request())
            .unwrap_or_default();
        self.client
            .delete(url.as_str())
            .query(query)
            .header(&DELETE_SNAPSHOTS, "include")
            .with_azure_authorization(&credential, &self.config.account)
            .retryable(&self.config.retry_config)
            .sensitive(sensitive)
            .send()
            .await
            .map_err(|source| {
                let path = path.as_ref().into();
                Error::DeleteRequest { source, path }
            })?;

        Ok(())
    }

    fn build_bulk_delete_body(
        &self,
        boundary: &str,
        paths: &[Path],
        credential: &Option<Arc<AzureCredential>>,
    ) -> Vec<u8> {
        let mut body_bytes = Vec::with_capacity(paths.len() * 2048);

        for (idx, path) in paths.iter().enumerate() {
            let url = self.config.path_url(path);

            // Build subrequest with proper authorization
            let request = self
                .client
                .delete(url.as_str())
                .header(CONTENT_LENGTH, HeaderValue::from(0))
                // Each subrequest must be authorized individually [1] and we use
                // the CredentialExt for this.
                // [1]: https://learn.microsoft.com/en-us/rest/api/storageservices/blob-batch?tabs=microsoft-entra-id#request-body
                .with_azure_authorization(credential, &self.config.account)
                .into_parts()
                .1
                .unwrap();

            let url: Url = request.uri().to_string().parse().unwrap();

            // Url for part requests must be relative and without base
            let relative_url = self.config.service.make_relative(&url).unwrap();

            serialize_part_delete_request(&mut body_bytes, boundary, idx, request, relative_url)
        }

        // Encode end marker
        extend(&mut body_bytes, b"--");
        extend(&mut body_bytes, boundary.as_bytes());
        extend(&mut body_bytes, b"--");
        extend(&mut body_bytes, b"\r\n");
        body_bytes
    }

    pub(crate) async fn bulk_delete_request(&self, paths: Vec<Path>) -> Result<Vec<Result<Path>>> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }

        let credential = self.get_credential().await?;

        // https://www.ietf.org/rfc/rfc2046
        let random_bytes = rand::random::<[u8; 16]>(); // 128 bits
        let boundary = format!("batch_{}", BASE64_STANDARD_NO_PAD.encode(random_bytes));

        let body_bytes = self.build_bulk_delete_body(&boundary, &paths, &credential);

        // Send multipart request
        let url = self.config.path_url(&Path::from("/"));
        let batch_response = self
            .client
            .post(url.as_str())
            .query(&[("restype", "container"), ("comp", "batch")])
            .header(
                CONTENT_TYPE,
                HeaderValue::from_str(format!("multipart/mixed; boundary={}", boundary).as_str())
                    .unwrap(),
            )
            .header(CONTENT_LENGTH, HeaderValue::from(body_bytes.len()))
            .body(body_bytes)
            .with_azure_authorization(&credential, &self.config.account)
            .send_retry(&self.config.retry_config)
            .await
            .map_err(|source| Error::BulkDeleteRequest { source })?;

        let boundary = parse_multipart_response_boundary(&batch_response)?;

        let batch_body = batch_response
            .into_body()
            .bytes()
            .await
            .map_err(|source| Error::BulkDeleteRequestBody { source })?;

        let results = parse_blob_batch_delete_body(batch_body, boundary, &paths).await?;

        Ok(results)
    }

    /// Make an Azure Copy request <https://docs.microsoft.com/en-us/rest/api/storageservices/copy-blob>
    pub(crate) async fn copy_request(&self, from: &Path, to: &Path, overwrite: bool) -> Result<()> {
        let credential = self.get_credential().await?;
        let url = self.config.path_url(to);
        let mut source = self.config.path_url(from);

        // If using SAS authorization must include the headers in the URL
        // <https://docs.microsoft.com/en-us/rest/api/storageservices/copy-blob#request-headers>
        if let Some(AzureCredential::SASToken(pairs)) = credential.as_deref() {
            source.query_pairs_mut().extend_pairs(pairs);
        }

        let mut builder = self
            .client
            .request(Method::PUT, url.as_str())
            .header(&COPY_SOURCE, source.to_string())
            .header(CONTENT_LENGTH, HeaderValue::from_static("0"));

        if !overwrite {
            builder = builder.header(IF_NONE_MATCH, "*");
        }

        let sensitive = credential
            .as_deref()
            .map(|c| c.sensitive_request())
            .unwrap_or_default();
        builder
            .with_azure_authorization(&credential, &self.config.account)
            .retryable(&self.config.retry_config)
            .sensitive(sensitive)
            .idempotent(overwrite)
            .send()
            .await
            .map_err(|err| err.error(STORE, from.to_string()))?;

        Ok(())
    }

    /// Make a Get User Delegation Key request
    /// <https://docs.microsoft.com/en-us/rest/api/storageservices/get-user-delegation-key>
    async fn get_user_delegation_key(
        &self,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> Result<UserDelegationKey> {
        let credential = self.get_credential().await?;
        let url = self.config.service.clone();

        let start = start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let expiry = end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        let mut body = String::new();
        body.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<KeyInfo>\n");
        body.push_str(&format!(
            "\t<Start>{start}</Start>\n\t<Expiry>{expiry}</Expiry>\n"
        ));
        body.push_str("</KeyInfo>");

        let sensitive = credential
            .as_deref()
            .map(|c| c.sensitive_request())
            .unwrap_or_default();

        let response = self
            .client
            .post(url.as_str())
            .body(body)
            .query(&[("restype", "service"), ("comp", "userdelegationkey")])
            .with_azure_authorization(&credential, &self.config.account)
            .retryable(&self.config.retry_config)
            .sensitive(sensitive)
            .idempotent(true)
            .send()
            .await
            .map_err(|source| Error::DelegationKeyRequest { source })?
            .into_body()
            .bytes()
            .await
            .map_err(|source| Error::DelegationKeyResponseBody { source })?;

        let response: UserDelegationKey = quick_xml::de::from_reader(response.reader())
            .map_err(|source| Error::DelegationKeyResponse { source })?;

        Ok(response)
    }

    /// Creat an AzureSigner for generating SAS tokens (pre-signed urls).
    ///
    /// Depending on the type of credential, this will either use the account key or a user delegation key.
    /// Since delegation keys are acquired ad-hoc, the signer aloows for signing multiple urls with the same key.
    pub(crate) async fn signer(&self, expires_in: Duration) -> Result<AzureSigner> {
        let credential = self.get_credential().await?;
        let signed_start = chrono::Utc::now();
        let signed_expiry = signed_start + expires_in;
        match credential.as_deref() {
            Some(AzureCredential::BearerToken(_)) => {
                let key = self
                    .get_user_delegation_key(&signed_start, &signed_expiry)
                    .await?;
                let signing_key = AzureAccessKey::try_new(&key.value)?;
                Ok(AzureSigner::new(
                    signing_key,
                    self.config.account.clone(),
                    signed_start,
                    signed_expiry,
                    Some(key),
                ))
            }
            Some(AzureCredential::AccessKey(key)) => Ok(AzureSigner::new(
                key.to_owned(),
                self.config.account.clone(),
                signed_start,
                signed_expiry,
                None,
            )),
            None => Err(Error::SASwithSkipSignature.into()),
            _ => Err(Error::SASforSASNotSupported.into()),
        }
    }

    #[cfg(test)]
    pub(crate) async fn get_blob_tagging(&self, path: &Path) -> Result<HttpResponse> {
        let credential = self.get_credential().await?;
        let url = self.config.path_url(path);
        let sensitive = credential
            .as_deref()
            .map(|c| c.sensitive_request())
            .unwrap_or_default();
        let response = self
            .client
            .get(url.as_str())
            .query(&[("comp", "tags")])
            .with_azure_authorization(&credential, &self.config.account)
            .retryable(&self.config.retry_config)
            .sensitive(sensitive)
            .send()
            .await
            .map_err(|source| {
                let path = path.as_ref().into();
                Error::GetRequest { source, path }
            })?;

        Ok(response)
    }
}

#[async_trait]
impl GetClient for AzureClient {
    const STORE: &'static str = STORE;

    const HEADER_CONFIG: HeaderConfig = HeaderConfig {
        etag_required: true,
        last_modified_required: true,
        version_header: Some(VERSION_HEADER),
        user_defined_metadata_prefix: Some(USER_DEFINED_METADATA_HEADER_PREFIX),
    };

    /// Make an Azure GET request
    /// <https://docs.microsoft.com/en-us/rest/api/storageservices/get-blob>
    /// <https://docs.microsoft.com/en-us/rest/api/storageservices/get-blob-properties>
    async fn get_request(&self, path: &Path, options: GetOptions) -> Result<HttpResponse> {
        // As of 2024-01-02, Azure does not support suffix requests,
        // so we should fail fast here rather than sending one
        if let Some(GetRange::Suffix(_)) = options.range.as_ref() {
            return Err(crate::Error::NotSupported {
                source: "Azure does not support suffix range requests".into(),
            });
        }

        let credential = self.get_credential().await?;
        let url = self.config.path_url(path);
        let method = match options.head {
            true => Method::HEAD,
            false => Method::GET,
        };

        let mut builder = self
            .client
            .request(method, url.as_str())
            .header(CONTENT_LENGTH, HeaderValue::from_static("0"))
            .body(Bytes::new());

        if let Some(v) = &options.version {
            builder = builder.query(&[("versionid", v)])
        }

        let sensitive = credential
            .as_deref()
            .map(|c| c.sensitive_request())
            .unwrap_or_default();
        let response = builder
            .with_get_options(options)
            .with_azure_authorization(&credential, &self.config.account)
            .retryable(&self.config.retry_config)
            .sensitive(sensitive)
            .send()
            .await
            .map_err(|source| {
                let path = path.as_ref().into();
                Error::GetRequest { source, path }
            })?;

        match response.headers().get("x-ms-resource-type") {
            Some(resource) if resource.as_ref() != b"file" => Err(crate::Error::NotFound {
                path: path.to_string(),
                source: format!(
                    "Not a file, got x-ms-resource-type: {}",
                    String::from_utf8_lossy(resource.as_ref())
                )
                .into(),
            }),
            _ => Ok(response),
        }
    }
}

#[async_trait]
impl ListClient for Arc<AzureClient> {
    /// Make an Azure List request <https://docs.microsoft.com/en-us/rest/api/storageservices/list-blobs>
    async fn list_request(
        &self,
        prefix: Option<&str>,
        delimiter: bool,
        token: Option<&str>,
        offset: Option<&str>,
        max_keys: Option<usize>,
    ) -> Result<(ListResult, Option<String>)> {
        assert!(offset.is_none()); // Not yet supported

        let credential = self.get_credential().await?;
        let url = self.config.path_url(&Path::default());

        let mut query = Vec::with_capacity(5);
        query.push(("restype", "container"));
        query.push(("comp", "list"));

        if let Some(prefix) = prefix {
            query.push(("prefix", prefix))
        }

        if delimiter {
            query.push(("delimiter", DELIMITER))
        }

        if let Some(token) = token {
            query.push(("marker", token))
        }

        let max_keys = max_keys.map(|x| x.to_string());
        if let Some(max_keys) = &max_keys {
            query.push(("maxresults", max_keys))
        }

        let sensitive = credential
            .as_deref()
            .map(|c| c.sensitive_request())
            .unwrap_or_default();
        let response = self
            .client
            .get(url.as_str())
            .query(&query)
            .with_azure_authorization(&credential, &self.config.account)
            .retryable(&self.config.retry_config)
            .sensitive(sensitive)
            .send()
            .await
            .map_err(|source| Error::ListRequest { source })?
            .into_body()
            .bytes()
            .await
            .map_err(|source| Error::ListResponseBody { source })?;

        let mut response: ListResultInternal = quick_xml::de::from_reader(response.reader())
            .map_err(|source| Error::InvalidListResponse { source })?;

        let token = response.next_marker.take();

        Ok((to_list_result(response, prefix)?, token))
    }
}

/// Raw / internal response from list requests
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListResultInternal {
    pub prefix: Option<String>,
    pub max_results: Option<u32>,
    pub delimiter: Option<String>,
    pub next_marker: Option<String>,
    pub blobs: Blobs,
}

fn to_list_result(value: ListResultInternal, prefix: Option<&str>) -> Result<ListResult> {
    let prefix = prefix.unwrap_or_default();
    let common_prefixes = value
        .blobs
        .blob_prefix
        .into_iter()
        .map(|x| Ok(Path::parse(x.name)?))
        .collect::<Result<_>>()?;

    let objects = value
        .blobs
        .blobs
        .into_iter()
        // Note: Filters out directories from list results when hierarchical namespaces are
        // enabled. When we want directories, its always via the BlobPrefix mechanics,
        // and during lists we state that prefixes are evaluated on path segment basis.
        .filter(|blob| {
            !matches!(blob.properties.resource_type.as_ref(), Some(typ) if typ == "directory")
                && blob.name.len() > prefix.len()
        })
        .map(ObjectMeta::try_from)
        .collect::<Result<_>>()?;

    Ok(ListResult {
        common_prefixes,
        objects,
    })
}

/// Collection of blobs and potentially shared prefixes returned from list requests.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Blobs {
    #[serde(default)]
    pub blob_prefix: Vec<BlobPrefix>,
    #[serde(rename = "Blob", default)]
    pub blobs: Vec<Blob>,
}

/// Common prefix in list blobs response
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct BlobPrefix {
    pub name: String,
}

/// Details for a specific blob
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Blob {
    pub name: String,
    pub version_id: Option<String>,
    pub is_current_version: Option<bool>,
    pub deleted: Option<bool>,
    pub properties: BlobProperties,
    pub metadata: Option<HashMap<String, String>>,
}

impl TryFrom<Blob> for ObjectMeta {
    type Error = crate::Error;

    fn try_from(value: Blob) -> Result<Self> {
        Ok(Self {
            location: Path::parse(value.name)?,
            last_modified: value.properties.last_modified,
            size: value.properties.content_length,
            e_tag: value.properties.e_tag,
            version: None, // For consistency with S3 and GCP which don't include this
        })
    }
}

/// Properties associated with individual blobs. The actual list
/// of returned properties is much more exhaustive, but we limit
/// the parsed fields to the ones relevant in this crate.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct BlobProperties {
    #[serde(deserialize_with = "deserialize_rfc1123", rename = "Last-Modified")]
    pub last_modified: DateTime<Utc>,
    #[serde(rename = "Content-Length")]
    pub content_length: u64,
    #[serde(rename = "Content-Type")]
    pub content_type: String,
    #[serde(rename = "Content-Encoding")]
    pub content_encoding: Option<String>,
    #[serde(rename = "Content-Language")]
    pub content_language: Option<String>,
    #[serde(rename = "Etag")]
    pub e_tag: Option<String>,
    #[serde(rename = "ResourceType")]
    pub resource_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlockId(Bytes);

impl BlockId {
    pub(crate) fn new(block_id: impl Into<Bytes>) -> Self {
        Self(block_id.into())
    }
}

impl<B> From<B> for BlockId
where
    B: Into<Bytes>,
{
    fn from(v: B) -> Self {
        Self::new(v)
    }
}

impl AsRef<[u8]> for BlockId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlockList {
    pub blocks: Vec<BlockId>,
}

impl BlockList {
    pub(crate) fn to_xml(&self) -> String {
        let mut s = String::new();
        s.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<BlockList>\n");
        for block_id in &self.blocks {
            let node = format!(
                "\t<Uncommitted>{}</Uncommitted>\n",
                BASE64_STANDARD.encode(block_id)
            );
            s.push_str(&node);
        }

        s.push_str("</BlockList>");
        s
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct UserDelegationKey {
    pub signed_oid: String,
    pub signed_tid: String,
    pub signed_start: String,
    pub signed_expiry: String,
    pub signed_service: String,
    pub signed_version: String,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StaticCredentialProvider;
    use bytes::Bytes;
    use regex::bytes::Regex;
    use reqwest::Client;

    #[test]
    fn deserde_azure() {
        const S: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<EnumerationResults ServiceEndpoint=\"https://azureskdforrust.blob.core.windows.net/\" ContainerName=\"osa2\">
    <Blobs>
        <Blob>
            <Name>blob0.txt</Name>
            <Properties>
                <Creation-Time>Thu, 01 Jul 2021 10:44:59 GMT</Creation-Time>
                <Last-Modified>Thu, 01 Jul 2021 10:44:59 GMT</Last-Modified>
                <Expiry-Time>Thu, 07 Jul 2022 14:38:48 GMT</Expiry-Time>
                <Etag>0x8D93C7D4629C227</Etag>
                <Content-Length>8</Content-Length>
                <Content-Type>text/plain</Content-Type>
                <Content-Encoding />
                <Content-Language />
                <Content-CRC64 />
                <Content-MD5>rvr3UC1SmUw7AZV2NqPN0g==</Content-MD5>
                <Cache-Control />
                <Content-Disposition />
                <BlobType>BlockBlob</BlobType>
                <AccessTier>Hot</AccessTier>
                <AccessTierInferred>true</AccessTierInferred>
                <LeaseStatus>unlocked</LeaseStatus>
                <LeaseState>available</LeaseState>
                <ServerEncrypted>true</ServerEncrypted>
            </Properties>
            <Metadata><userkey>uservalue</userkey></Metadata>
            <OrMetadata />
        </Blob>
        <Blob>
            <Name>blob1.txt</Name>
            <Properties>
                <Creation-Time>Thu, 01 Jul 2021 10:44:59 GMT</Creation-Time>
                <Last-Modified>Thu, 01 Jul 2021 10:44:59 GMT</Last-Modified>
                <Etag>0x8D93C7D463004D6</Etag>
                <Content-Length>8</Content-Length>
                <Content-Type>text/plain</Content-Type>
                <Content-Encoding />
                <Content-Language />
                <Content-CRC64 />
                <Content-MD5>rvr3UC1SmUw7AZV2NqPN0g==</Content-MD5>
                <Cache-Control />
                <Content-Disposition />
                <BlobType>BlockBlob</BlobType>
                <AccessTier>Hot</AccessTier>
                <AccessTierInferred>true</AccessTierInferred>
                <LeaseStatus>unlocked</LeaseStatus>
                <LeaseState>available</LeaseState>
                <ServerEncrypted>true</ServerEncrypted>
            </Properties>
            <OrMetadata />
        </Blob>
        <Blob>
            <Name>blob2.txt</Name>
            <Properties>
                <Creation-Time>Thu, 01 Jul 2021 10:44:59 GMT</Creation-Time>
                <Last-Modified>Thu, 01 Jul 2021 10:44:59 GMT</Last-Modified>
                <Etag>0x8D93C7D4636478A</Etag>
                <Content-Length>8</Content-Length>
                <Content-Type>text/plain</Content-Type>
                <Content-Encoding />
                <Content-Language />
                <Content-CRC64 />
                <Content-MD5>rvr3UC1SmUw7AZV2NqPN0g==</Content-MD5>
                <Cache-Control />
                <Content-Disposition />
                <BlobType>BlockBlob</BlobType>
                <AccessTier>Hot</AccessTier>
                <AccessTierInferred>true</AccessTierInferred>
                <LeaseStatus>unlocked</LeaseStatus>
                <LeaseState>available</LeaseState>
                <ServerEncrypted>true</ServerEncrypted>
            </Properties>
            <OrMetadata />
        </Blob>
    </Blobs>
    <NextMarker />
</EnumerationResults>";

        let mut _list_blobs_response_internal: ListResultInternal =
            quick_xml::de::from_str(S).unwrap();
    }

    #[test]
    fn deserde_azurite() {
        const S: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<EnumerationResults ServiceEndpoint=\"http://127.0.0.1:10000/devstoreaccount1\" ContainerName=\"osa2\">
    <Prefix/>
    <Marker/>
    <MaxResults>5000</MaxResults>
    <Delimiter/>
    <Blobs>
        <Blob>
            <Name>blob0.txt</Name>
            <Properties>
                <Creation-Time>Thu, 01 Jul 2021 10:45:02 GMT</Creation-Time>
                <Last-Modified>Thu, 01 Jul 2021 10:45:02 GMT</Last-Modified>
                <Etag>0x228281B5D517B20</Etag>
                <Content-Length>8</Content-Length>
                <Content-Type>text/plain</Content-Type>
                <Content-MD5>rvr3UC1SmUw7AZV2NqPN0g==</Content-MD5>
                <BlobType>BlockBlob</BlobType>
                <LeaseStatus>unlocked</LeaseStatus>
                <LeaseState>available</LeaseState>
                <ServerEncrypted>true</ServerEncrypted>
                <AccessTier>Hot</AccessTier>
                <AccessTierInferred>true</AccessTierInferred>
                <AccessTierChangeTime>Thu, 01 Jul 2021 10:45:02 GMT</AccessTierChangeTime>
            </Properties>
        </Blob>
        <Blob>
            <Name>blob1.txt</Name>
            <Properties>
                <Creation-Time>Thu, 01 Jul 2021 10:45:02 GMT</Creation-Time>
                <Last-Modified>Thu, 01 Jul 2021 10:45:02 GMT</Last-Modified>
                <Etag>0x1DD959381A8A860</Etag>
                <Content-Length>8</Content-Length>
                <Content-Type>text/plain</Content-Type>
                <Content-MD5>rvr3UC1SmUw7AZV2NqPN0g==</Content-MD5>
                <BlobType>BlockBlob</BlobType>
                <LeaseStatus>unlocked</LeaseStatus>
                <LeaseState>available</LeaseState>
                <ServerEncrypted>true</ServerEncrypted>
                <AccessTier>Hot</AccessTier>
                <AccessTierInferred>true</AccessTierInferred>
                <AccessTierChangeTime>Thu, 01 Jul 2021 10:45:02 GMT</AccessTierChangeTime>
            </Properties>
        </Blob>
        <Blob>
            <Name>blob2.txt</Name>
            <Properties>
                <Creation-Time>Thu, 01 Jul 2021 10:45:02 GMT</Creation-Time>
                <Last-Modified>Thu, 01 Jul 2021 10:45:02 GMT</Last-Modified>
                <Etag>0x1FBE9C9B0C7B650</Etag>
                <Content-Length>8</Content-Length>
                <Content-Type>text/plain</Content-Type>
                <Content-MD5>rvr3UC1SmUw7AZV2NqPN0g==</Content-MD5>
                <BlobType>BlockBlob</BlobType>
                <LeaseStatus>unlocked</LeaseStatus>
                <LeaseState>available</LeaseState>
                <ServerEncrypted>true</ServerEncrypted>
                <AccessTier>Hot</AccessTier>
                <AccessTierInferred>true</AccessTierInferred>
                <AccessTierChangeTime>Thu, 01 Jul 2021 10:45:02 GMT</AccessTierChangeTime>
            </Properties>
        </Blob>
    </Blobs>
    <NextMarker/>
</EnumerationResults>";

        let _list_blobs_response_internal: ListResultInternal = quick_xml::de::from_str(S).unwrap();
    }

    #[test]
    fn to_xml() {
        const S: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<BlockList>
\t<Uncommitted>bnVtZXJvMQ==</Uncommitted>
\t<Uncommitted>bnVtZXJvMg==</Uncommitted>
\t<Uncommitted>bnVtZXJvMw==</Uncommitted>
</BlockList>";
        let mut blocks = BlockList { blocks: Vec::new() };
        blocks.blocks.push(Bytes::from_static(b"numero1").into());
        blocks.blocks.push("numero2".into());
        blocks.blocks.push("numero3".into());

        let res: &str = &blocks.to_xml();

        assert_eq!(res, S)
    }

    #[test]
    fn test_delegated_key_response() {
        const S: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<UserDelegationKey>
    <SignedOid>String containing a GUID value</SignedOid>
    <SignedTid>String containing a GUID value</SignedTid>
    <SignedStart>String formatted as ISO date</SignedStart>
    <SignedExpiry>String formatted as ISO date</SignedExpiry>
    <SignedService>b</SignedService>
    <SignedVersion>String specifying REST api version to use to create the user delegation key</SignedVersion>
    <Value>String containing the user delegation key</Value>
</UserDelegationKey>"#;

        let _delegated_key_response_internal: UserDelegationKey =
            quick_xml::de::from_str(S).unwrap();
    }

    #[tokio::test]
    async fn test_build_bulk_delete_body() {
        let credential_provider = Arc::new(StaticCredentialProvider::new(
            AzureCredential::BearerToken("static-token".to_string()),
        ));

        let config = AzureConfig {
            account: "testaccount".to_string(),
            container: "testcontainer".to_string(),
            credentials: credential_provider,
            service: "http://example.com".try_into().unwrap(),
            retry_config: Default::default(),
            is_emulator: false,
            skip_signature: false,
            disable_tagging: false,
            client_options: Default::default(),
        };

        let client = AzureClient::new(config, HttpClient::new(Client::new()));

        let credential = client.get_credential().await.unwrap();
        let paths = &[Path::from("a"), Path::from("b"), Path::from("c")];

        let boundary = "batch_statictestboundary".to_string();

        let body_bytes = client.build_bulk_delete_body(&boundary, paths, &credential);

        // Replace Date header value with a static date
        let re = Regex::new("Date:[^\r]+").unwrap();
        let body_bytes = re
            .replace_all(&body_bytes, b"Date: Tue, 05 Nov 2024 15:01:15 GMT")
            .to_vec();

        let expected_body = b"--batch_statictestboundary\r
Content-Type: application/http\r
Content-Transfer-Encoding: binary\r
Content-ID: 0\r
\r
DELETE /testcontainer/a HTTP/1.1\r
Content-Length: 0\r
Date: Tue, 05 Nov 2024 15:01:15 GMT\r
X-Ms-Version: 2023-11-03\r
Authorization: Bearer static-token\r
\r
\r
--batch_statictestboundary\r
Content-Type: application/http\r
Content-Transfer-Encoding: binary\r
Content-ID: 1\r
\r
DELETE /testcontainer/b HTTP/1.1\r
Content-Length: 0\r
Date: Tue, 05 Nov 2024 15:01:15 GMT\r
X-Ms-Version: 2023-11-03\r
Authorization: Bearer static-token\r
\r
\r
--batch_statictestboundary\r
Content-Type: application/http\r
Content-Transfer-Encoding: binary\r
Content-ID: 2\r
\r
DELETE /testcontainer/c HTTP/1.1\r
Content-Length: 0\r
Date: Tue, 05 Nov 2024 15:01:15 GMT\r
X-Ms-Version: 2023-11-03\r
Authorization: Bearer static-token\r
\r
\r
--batch_statictestboundary--\r\n"
            .to_vec();

        assert_eq!(expected_body, body_bytes);
    }

    #[tokio::test]
    async fn test_parse_blob_batch_delete_body() {
        let response_body = b"--batchresponse_66925647-d0cb-4109-b6d3-28efe3e1e5ed\r
Content-Type: application/http\r
Content-ID: 0\r
\r
HTTP/1.1 202 Accepted\r
x-ms-delete-type-permanent: true\r
x-ms-request-id: 778fdc83-801e-0000-62ff-0334671e284f\r
x-ms-version: 2018-11-09\r
\r
--batchresponse_66925647-d0cb-4109-b6d3-28efe3e1e5ed\r
Content-Type: application/http\r
Content-ID: 1\r
\r
HTTP/1.1 202 Accepted\r
x-ms-delete-type-permanent: true\r
x-ms-request-id: 778fdc83-801e-0000-62ff-0334671e2851\r
x-ms-version: 2018-11-09\r
\r
--batchresponse_66925647-d0cb-4109-b6d3-28efe3e1e5ed\r
Content-Type: application/http\r
Content-ID: 2\r
\r
HTTP/1.1 404 The specified blob does not exist.\r
x-ms-error-code: BlobNotFound\r
x-ms-request-id: 778fdc83-801e-0000-62ff-0334671e2852\r
x-ms-version: 2018-11-09\r
Content-Length: 216\r
Content-Type: application/xml\r
\r
<?xml version=\"1.0\" encoding=\"utf-8\"?>
<Error><Code>BlobNotFound</Code><Message>The specified blob does not exist.
RequestId:778fdc83-801e-0000-62ff-0334671e2852
Time:2018-06-14T16:46:54.6040685Z</Message></Error>\r
--batchresponse_66925647-d0cb-4109-b6d3-28efe3e1e5ed--\r\n";

        let response: HttpResponse = http::Response::builder()
            .status(202)
            .header("Transfer-Encoding", "chunked")
            .header(
                "Content-Type",
                "multipart/mixed; boundary=batchresponse_66925647-d0cb-4109-b6d3-28efe3e1e5ed",
            )
            .header("x-ms-request-id", "778fdc83-801e-0000-62ff-033467000000")
            .header("x-ms-version", "2018-11-09")
            .body(Bytes::from(response_body.as_slice()).into())
            .unwrap();

        let boundary = parse_multipart_response_boundary(&response).unwrap();
        let body = response.into_body().bytes().await.unwrap();

        let paths = &[Path::from("a"), Path::from("b"), Path::from("c")];

        let results = parse_blob_batch_delete_body(body, boundary, paths)
            .await
            .unwrap();

        assert!(results[0].is_ok());
        assert_eq!(&paths[0], results[0].as_ref().unwrap());

        assert!(results[1].is_ok());
        assert_eq!(&paths[1], results[1].as_ref().unwrap());

        assert!(results[2].is_err());
        let err = results[2].as_ref().unwrap_err();
        let crate::Error::NotFound { source, .. } = err else {
            unreachable!("must be not found")
        };
        let Some(Error::DeleteFailed { path, code, reason }) = source.downcast_ref::<Error>()
        else {
            unreachable!("must be client error")
        };

        assert_eq!(paths[2].as_ref(), path);
        assert_eq!("404", code);
        assert_eq!("The specified blob does not exist.", reason);
    }
}
