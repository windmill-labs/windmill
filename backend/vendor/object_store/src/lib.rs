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

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(rustdoc::broken_intra_doc_links, rustdoc::bare_urls, rust_2018_idioms)]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    clippy::explicit_iter_loop,
    clippy::future_not_send,
    clippy::use_self,
    clippy::clone_on_ref_ptr,
    unreachable_pub
)]

//! # object_store
//!
//! This crate provides a uniform API for interacting with object
//! storage services and local files via the [`ObjectStore`]
//! trait.
//!
//! Using this crate, the same binary and code can run in multiple
//! clouds and local test environments, via a simple runtime
//! configuration change.
//!
//! # Highlights
//!
//! 1. A high-performance async API focused on providing a consistent interface
//!    mirroring that of object stores such as [S3]
//!
//! 2. Production quality, leading this crate to be used in large
//!    scale production systems, such as [crates.io] and [InfluxDB IOx]
//!
//! 3. Support for advanced functionality, including atomic, conditional reads
//!    and writes, vectored IO, bulk deletion, and more...
//!
//! 4. Stable and predictable governance via the [Apache Arrow] project
//!
//! 5. Small dependency footprint, depending on only a small number of common crates
//!
//! Originally developed by [InfluxData] and subsequently donated
//! to [Apache Arrow].
//!
//! [Apache Arrow]: https://arrow.apache.org/
//! [InfluxData]: https://www.influxdata.com/
//! [crates.io]: https://github.com/rust-lang/crates.io
//! [ACID]: https://en.wikipedia.org/wiki/ACID
//! [S3]: https://aws.amazon.com/s3/
//!
//! # Available [`ObjectStore`] Implementations
//!
//! By default, this crate provides the following implementations:
//!
//! * Memory: [`InMemory`](memory::InMemory)
//!
//! Feature flags are used to enable support for other implementations:
//!
#![cfg_attr(
    feature = "fs",
    doc = "* Local filesystem: [`LocalFileSystem`](local::LocalFileSystem)"
)]
#![cfg_attr(
    feature = "gcp",
    doc = "* [`gcp`]: [Google Cloud Storage](https://cloud.google.com/storage/) support. See [`GoogleCloudStorageBuilder`](gcp::GoogleCloudStorageBuilder)"
)]
#![cfg_attr(
    feature = "aws",
    doc = "* [`aws`]: [Amazon S3](https://aws.amazon.com/s3/). See [`AmazonS3Builder`](aws::AmazonS3Builder)"
)]
#![cfg_attr(
    feature = "azure",
    doc = "* [`azure`]: [Azure Blob Storage](https://azure.microsoft.com/en-gb/services/storage/blobs/). See [`MicrosoftAzureBuilder`](azure::MicrosoftAzureBuilder)"
)]
#![cfg_attr(
    feature = "http",
    doc = "* [`http`]: [HTTP/WebDAV Storage](https://datatracker.ietf.org/doc/html/rfc2518). See [`HttpBuilder`](http::HttpBuilder)"
)]
//!
//! # Why not a Filesystem Interface?
//!
//! The [`ObjectStore`] interface is designed to mirror the APIs
//! of object stores and *not* filesystems, and thus has stateless APIs instead
//! of cursor based interfaces such as [`Read`] or [`Seek`] available in filesystems.
//!
//! This design provides the following advantages:
//!
//! * All operations are atomic, and readers cannot observe partial and/or failed writes
//! * Methods map directly to object store APIs, providing both efficiency and predictability
//! * Abstracts away filesystem and operating system specific quirks, ensuring portability
//! * Allows for functionality not native to filesystems, such as operation preconditions
//!   and atomic multipart uploads
//!
//! This crate does provide [`BufReader`] and [`BufWriter`] adapters
//! which provide a more filesystem-like API for working with the
//! [`ObjectStore`] trait, however, they should be used with care
//!
//! [`BufReader`]: buffered::BufReader
//! [`BufWriter`]: buffered::BufWriter
//!
//! # Adapters
//!
//! [`ObjectStore`] instances can be composed with various adapters
//! which add additional functionality:
//!
//! * Rate Throttling: [`ThrottleConfig`](throttle::ThrottleConfig)
//! * Concurrent Request Limit: [`LimitStore`](limit::LimitStore)
//!
//! # Configuration System
//!
//! This crate provides a configuration system inspired by the APIs exposed by [fsspec],
//! [PyArrow FileSystem], and [Hadoop FileSystem], allowing creating a [`DynObjectStore`]
//! from a URL and an optional list of key value pairs. This provides a flexible interface
//! to support a wide variety of user-defined store configurations, with minimal additional
//! application complexity.
//!
//! ```no_run
//! # #[cfg(feature = "aws")] {
//! # use url::Url;
//! # use object_store::{parse_url, parse_url_opts};
//! # use object_store::aws::{AmazonS3, AmazonS3Builder};
//! #
//! #
//! // Can manually create a specific store variant using the appropriate builder
//! let store: AmazonS3 = AmazonS3Builder::from_env()
//!     .with_bucket_name("my-bucket").build().unwrap();
//!
//! // Alternatively can create an ObjectStore from an S3 URL
//! let url = Url::parse("s3://bucket/path").unwrap();
//! let (store, path) = parse_url(&url).unwrap();
//! assert_eq!(path.as_ref(), "path");
//!
//! // Potentially with additional options
//! let (store, path) = parse_url_opts(&url, vec![("aws_access_key_id", "...")]).unwrap();
//!
//! // Or with URLs that encode the bucket name in the URL path
//! let url = Url::parse("https://ACCOUNT_ID.r2.cloudflarestorage.com/bucket/path").unwrap();
//! let (store, path) = parse_url(&url).unwrap();
//! assert_eq!(path.as_ref(), "path");
//! # }
//! ```
//!
//! [PyArrow FileSystem]: https://arrow.apache.org/docs/python/generated/pyarrow.fs.FileSystem.html#pyarrow.fs.FileSystem.from_uri
//! [fsspec]: https://filesystem-spec.readthedocs.io/en/latest/api.html#fsspec.filesystem
//! [Hadoop FileSystem]: https://hadoop.apache.org/docs/r3.0.0/api/org/apache/hadoop/fs/FileSystem.html#get-java.net.URI-org.apache.hadoop.conf.Configuration-
//!
//! # List objects
//!
//! Use the [`ObjectStore::list`] method to iterate over objects in
//! remote storage or files in the local filesystem:
//!
//! ```
//! # use object_store::local::LocalFileSystem;
//! # use std::sync::Arc;
//! # use object_store::{path::Path, ObjectStore};
//! # use futures::stream::StreamExt;
//! # // use LocalFileSystem for example
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! #
//! # async fn example() {
//! #
//! // create an ObjectStore
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//!
//! // Recursively list all files below the 'data' path.
//! // 1. On AWS S3 this would be the 'data/' prefix
//! // 2. On a local filesystem, this would be the 'data' directory
//! let prefix = Path::from("data");
//!
//! // Get an `async` stream of Metadata objects:
//! let mut list_stream = object_store.list(Some(&prefix));
//!
//! // Print a line about each object
//! while let Some(meta) = list_stream.next().await.transpose().unwrap() {
//!     println!("Name: {}, size: {}", meta.location, meta.size);
//! }
//! # }
//! ```
//!
//! Which will print out something like the following:
//!
//! ```text
//! Name: data/file01.parquet, size: 112832
//! Name: data/file02.parquet, size: 143119
//! Name: data/child/file03.parquet, size: 100
//! ...
//! ```
//!
//! # Fetch objects
//!
//! Use the [`ObjectStore::get`] method to fetch the data bytes
//! from remote storage or files in the local filesystem as a stream.
//!
//! ```
//! # use futures::TryStreamExt;
//! # use object_store::local::LocalFileSystem;
//! # use std::sync::Arc;
//! #  use bytes::Bytes;
//! # use object_store::{path::Path, ObjectStore, GetResult};
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! #
//! # async fn example() {
//! #
//! // Create an ObjectStore
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//!
//! // Retrieve a specific file
//! let path = Path::from("data/file01.parquet");
//!
//! // Fetch just the file metadata
//! let meta = object_store.head(&path).await.unwrap();
//! println!("{meta:?}");
//!
//! // Fetch the object including metadata
//! let result: GetResult = object_store.get(&path).await.unwrap();
//! assert_eq!(result.meta, meta);
//!
//! // Buffer the entire object in memory
//! let object: Bytes = result.bytes().await.unwrap();
//! assert_eq!(object.len() as u64, meta.size);
//!
//! // Alternatively stream the bytes from object storage
//! let stream = object_store.get(&path).await.unwrap().into_stream();
//!
//! // Count the '0's using `try_fold` from `TryStreamExt` trait
//! let num_zeros = stream
//!     .try_fold(0, |acc, bytes| async move {
//!         Ok(acc + bytes.iter().filter(|b| **b == 0).count())
//!     }).await.unwrap();
//!
//! println!("Num zeros in {} is {}", path, num_zeros);
//! # }
//! ```
//!
//! # Put Object
//!
//! Use the [`ObjectStore::put`] method to atomically write data.
//!
//! ```
//! # use object_store::local::LocalFileSystem;
//! # use object_store::{ObjectStore, PutPayload};
//! # use std::sync::Arc;
//! # use object_store::path::Path;
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! # async fn put() {
//! #
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//! let path = Path::from("data/file1");
//! let payload = PutPayload::from_static(b"hello");
//! object_store.put(&path, payload).await.unwrap();
//! # }
//! ```
//!
//! # Multipart Upload
//!
//! Use the [`ObjectStore::put_multipart`] method to atomically write a large amount of data
//!
//! ```
//! # use object_store::local::LocalFileSystem;
//! # use object_store::{ObjectStore, WriteMultipart};
//! # use std::sync::Arc;
//! # use bytes::Bytes;
//! # use tokio::io::AsyncWriteExt;
//! # use object_store::path::Path;
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! # async fn multi_upload() {
//! #
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//! let path = Path::from("data/large_file");
//! let upload =  object_store.put_multipart(&path).await.unwrap();
//! let mut write = WriteMultipart::new(upload);
//! write.write(b"hello");
//! write.finish().await.unwrap();
//! # }
//! ```
//!
//! # Vectored Read
//!
//! A common pattern, especially when reading structured datasets, is to need to fetch
//! multiple, potentially non-contiguous, ranges of a particular object.
//!
//! [`ObjectStore::get_ranges`] provides an efficient way to perform such vectored IO, and will
//! automatically coalesce adjacent ranges into an appropriate number of parallel requests.
//!
//! ```
//! # use object_store::local::LocalFileSystem;
//! # use object_store::ObjectStore;
//! # use std::sync::Arc;
//! # use bytes::Bytes;
//! # use tokio::io::AsyncWriteExt;
//! # use object_store::path::Path;
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! # async fn multi_upload() {
//! #
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//! let path = Path::from("data/large_file");
//! let ranges = object_store.get_ranges(&path, &[90..100, 400..600, 0..10]).await.unwrap();
//! assert_eq!(ranges.len(), 3);
//! assert_eq!(ranges[0].len(), 10);
//! # }
//! ```
//!
//! # Vectored Write
//!
//! When writing data it is often the case that the size of the output is not known ahead of time.
//!
//! A common approach to handling this is to bump-allocate a `Vec`, whereby the underlying
//! allocation is repeatedly reallocated, each time doubling the capacity. The performance of
//! this is suboptimal as reallocating memory will often involve copying it to a new location.
//!
//! Fortunately, as [`PutPayload`] does not require memory regions to be contiguous, it is
//! possible to instead allocate memory in chunks and avoid bump allocating. [`PutPayloadMut`]
//! encapsulates this approach
//!
//! ```
//! # use object_store::local::LocalFileSystem;
//! # use object_store::{ObjectStore, PutPayloadMut};
//! # use std::sync::Arc;
//! # use bytes::Bytes;
//! # use tokio::io::AsyncWriteExt;
//! # use object_store::path::Path;
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! # async fn multi_upload() {
//! #
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//! let path = Path::from("data/large_file");
//! let mut buffer = PutPayloadMut::new().with_block_size(8192);
//! for _ in 0..22 {
//!     buffer.extend_from_slice(&[0; 1024]);
//! }
//! let payload = buffer.freeze();
//!
//! // Payload consists of 3 separate 8KB allocations
//! assert_eq!(payload.as_ref().len(), 3);
//! assert_eq!(payload.as_ref()[0].len(), 8192);
//! assert_eq!(payload.as_ref()[1].len(), 8192);
//! assert_eq!(payload.as_ref()[2].len(), 6144);
//!
//! object_store.put(&path, payload).await.unwrap();
//! # }
//! ```
//!
//! # Conditional Fetch
//!
//! More complex object retrieval can be supported by [`ObjectStore::get_opts`].
//!
//! For example, efficiently refreshing a cache without re-fetching the entire object
//! data if the object hasn't been modified.
//!
//! ```
//! # use std::collections::btree_map::Entry;
//! # use std::collections::HashMap;
//! # use object_store::{GetOptions, GetResult, ObjectStore, Result, Error};
//! # use std::sync::Arc;
//! # use std::time::{Duration, Instant};
//! # use bytes::Bytes;
//! # use tokio::io::AsyncWriteExt;
//! # use object_store::path::Path;
//! struct CacheEntry {
//!     /// Data returned by last request
//!     data: Bytes,
//!     /// ETag identifying the object returned by the server
//!     e_tag: String,
//!     /// Instant of last refresh
//!     refreshed_at: Instant,
//! }
//!
//! /// Example cache that checks entries after 10 seconds for a new version
//! struct Cache {
//!     entries: HashMap<Path, CacheEntry>,
//!     store: Arc<dyn ObjectStore>,
//! }
//!
//! impl Cache {
//!     pub async fn get(&mut self, path: &Path) -> Result<Bytes> {
//!         Ok(match self.entries.get_mut(path) {
//!             Some(e) => match e.refreshed_at.elapsed() < Duration::from_secs(10) {
//!                 true => e.data.clone(), // Return cached data
//!                 false => { // Check if remote version has changed
//!                     let opts = GetOptions {
//!                         if_none_match: Some(e.e_tag.clone()),
//!                         ..GetOptions::default()
//!                     };
//!                     match self.store.get_opts(&path, opts).await {
//!                         Ok(d) => e.data = d.bytes().await?,
//!                         Err(Error::NotModified { .. }) => {} // Data has not changed
//!                         Err(e) => return Err(e),
//!                     };
//!                     e.refreshed_at = Instant::now();
//!                     e.data.clone()
//!                 }
//!             },
//!             None => { // Not cached, fetch data
//!                 let get = self.store.get(&path).await?;
//!                 let e_tag = get.meta.e_tag.clone();
//!                 let data = get.bytes().await?;
//!                 if let Some(e_tag) = e_tag {
//!                     let entry = CacheEntry {
//!                         e_tag,
//!                         data: data.clone(),
//!                         refreshed_at: Instant::now(),
//!                     };
//!                     self.entries.insert(path.clone(), entry);
//!                 }
//!                 data
//!             }
//!         })
//!     }
//! }
//! ```
//!
//! # Conditional Put
//!
//! The default behaviour when writing data is to upsert any existing object at the given path,
//! overwriting any previous value. More complex behaviours can be achieved using [`PutMode`], and
//! can be used to build [Optimistic Concurrency Control] based transactions. This facilitates
//! building metadata catalogs, such as [Apache Iceberg] or [Delta Lake], directly on top of object
//! storage, without relying on a separate DBMS.
//!
//! ```
//! # use object_store::{Error, ObjectStore, PutMode, UpdateVersion};
//! # use std::sync::Arc;
//! # use bytes::Bytes;
//! # use tokio::io::AsyncWriteExt;
//! # use object_store::memory::InMemory;
//! # use object_store::path::Path;
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(InMemory::new())
//! # }
//! # fn do_update(b: Bytes) -> Bytes {b}
//! # async fn conditional_put() {
//! let store = get_object_store();
//! let path = Path::from("test");
//!
//! // Perform a conditional update on path
//! loop {
//!     // Perform get request
//!     let r = store.get(&path).await.unwrap();
//!
//!     // Save version information fetched
//!     let version = UpdateVersion {
//!         e_tag: r.meta.e_tag.clone(),
//!         version: r.meta.version.clone(),
//!     };
//!
//!     // Compute new version of object contents
//!     let new = do_update(r.bytes().await.unwrap());
//!
//!     // Attempt to commit transaction
//!     match store.put_opts(&path, new.into(), PutMode::Update(version).into()).await {
//!         Ok(_) => break, // Successfully committed
//!         Err(Error::Precondition { .. }) => continue, // Object has changed, try again
//!         Err(e) => panic!("{e}")
//!     }
//! }
//! # }
//! ```
//!
//! [Optimistic Concurrency Control]: https://en.wikipedia.org/wiki/Optimistic_concurrency_control
//! [Apache Iceberg]: https://iceberg.apache.org/
//! [Delta Lake]: https://delta.io/
//!
//! # TLS Certificates
//!
//! Stores that use HTTPS/TLS (this is true for most cloud stores) can choose the source of their [CA]
//! certificates. By default the system-bundled certificates are used (see
//! [`rustls-native-certs`]). The `tls-webpki-roots` feature switch can be used to also bundle Mozilla's
//! root certificates with the library/application (see [`webpki-roots`]).
//!
//! [CA]: https://en.wikipedia.org/wiki/Certificate_authority
//! [`rustls-native-certs`]: https://crates.io/crates/rustls-native-certs/
//! [`webpki-roots`]: https://crates.io/crates/webpki-roots
//!
//! # Customizing HTTP Clients
//!
//! Many [`ObjectStore`] implementations permit customization of the HTTP client via
//! the [`HttpConnector`] trait and utilities in the [`client`] module.
//! Examples include injecting custom HTTP headers or using an alternate
//! tokio Runtime I/O requests.
//!
//! [`HttpConnector`]: client::HttpConnector

#[cfg(feature = "aws")]
pub mod aws;
#[cfg(feature = "azure")]
pub mod azure;
pub mod buffered;
#[cfg(not(target_arch = "wasm32"))]
pub mod chunked;
pub mod delimited;
#[cfg(feature = "gcp")]
pub mod gcp;
#[cfg(feature = "http")]
pub mod http;
pub mod limit;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub mod local;
pub mod memory;
pub mod path;
pub mod prefix;
#[cfg(feature = "cloud")]
pub mod signer;
pub mod throttle;

#[cfg(feature = "cloud")]
pub mod client;

#[cfg(feature = "cloud")]
pub use client::{
    backoff::BackoffConfig, retry::RetryConfig, ClientConfigKey, ClientOptions, CredentialProvider,
    StaticCredentialProvider,
};

#[cfg(all(feature = "cloud", not(target_arch = "wasm32")))]
pub use client::Certificate;

#[cfg(feature = "cloud")]
mod config;

mod tags;

pub use tags::TagSet;

pub mod multipart;
mod parse;
mod payload;
mod upload;
mod util;

mod attributes;

#[cfg(any(feature = "integration", test))]
pub mod integration;

pub use attributes::*;

pub use parse::{parse_url, parse_url_opts, ObjectStoreScheme};
pub use payload::*;
pub use upload::*;
pub use util::{coalesce_ranges, collect_bytes, GetRange, OBJECT_STORE_COALESCE_DEFAULT};

use crate::path::Path;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
use crate::util::maybe_spawn_blocking;
use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use std::fmt::{Debug, Formatter};
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use std::sync::Arc;

/// An alias for a dynamically dispatched object store implementation.
pub type DynObjectStore = dyn ObjectStore;

/// Id type for multipart uploads.
pub type MultipartId = String;

/// Universal API to multiple object store services.
#[async_trait]
pub trait ObjectStore: std::fmt::Display + Send + Sync + Debug + 'static {
    /// Save the provided bytes to the specified location
    ///
    /// The operation is guaranteed to be atomic, it will either successfully
    /// write the entirety of `payload` to `location`, or fail. No clients
    /// should be able to observe a partially written object
    async fn put(&self, location: &Path, payload: PutPayload) -> Result<PutResult> {
        self.put_opts(location, payload, PutOptions::default())
            .await
    }

    /// Save the provided `payload` to `location` with the given options
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult>;

    /// Perform a multipart upload
    ///
    /// Client should prefer [`ObjectStore::put`] for small payloads, as streaming uploads
    /// typically require multiple separate requests. See [`MultipartUpload`] for more information
    async fn put_multipart(&self, location: &Path) -> Result<Box<dyn MultipartUpload>> {
        self.put_multipart_opts(location, PutMultipartOpts::default())
            .await
    }

    /// Perform a multipart upload with options
    ///
    /// Client should prefer [`ObjectStore::put`] for small payloads, as streaming uploads
    /// typically require multiple separate requests. See [`MultipartUpload`] for more information
    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOpts,
    ) -> Result<Box<dyn MultipartUpload>>;

    /// Return the bytes that are stored at the specified location.
    async fn get(&self, location: &Path) -> Result<GetResult> {
        self.get_opts(location, GetOptions::default()).await
    }

    /// Perform a get request with options
    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult>;

    /// Return the bytes that are stored at the specified location
    /// in the given byte range.
    ///
    /// See [`GetRange::Bounded`] for more details on how `range` gets interpreted
    async fn get_range(&self, location: &Path, range: Range<u64>) -> Result<Bytes> {
        let options = GetOptions {
            range: Some(range.into()),
            ..Default::default()
        };
        self.get_opts(location, options).await?.bytes().await
    }

    /// Return the bytes that are stored at the specified location
    /// in the given byte ranges
    async fn get_ranges(&self, location: &Path, ranges: &[Range<u64>]) -> Result<Vec<Bytes>> {
        coalesce_ranges(
            ranges,
            |range| self.get_range(location, range),
            OBJECT_STORE_COALESCE_DEFAULT,
        )
        .await
    }

    /// Return the metadata for the specified location
    async fn head(&self, location: &Path) -> Result<ObjectMeta> {
        let options = GetOptions {
            head: true,
            ..Default::default()
        };
        Ok(self.get_opts(location, options).await?.meta)
    }

    /// Delete the object at the specified location.
    async fn delete(&self, location: &Path) -> Result<()>;

    /// Delete all the objects at the specified locations
    ///
    /// When supported, this method will use bulk operations that delete more
    /// than one object per a request. The default implementation will call
    /// the single object delete method for each location, but with up to 10
    /// concurrent requests.
    ///
    /// The returned stream yields the results of the delete operations in the
    /// same order as the input locations. However, some errors will be from
    /// an overall call to a bulk delete operation, and not from a specific
    /// location.
    ///
    /// If the object did not exist, the result may be an error or a success,
    /// depending on the behavior of the underlying store. For example, local
    /// filesystems, GCP, and Azure return an error, while S3 and in-memory will
    /// return Ok. If it is an error, it will be [`Error::NotFound`].
    ///
    /// ```
    /// # use futures::{StreamExt, TryStreamExt};
    /// # use object_store::local::LocalFileSystem;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let root = tempfile::TempDir::new().unwrap();
    /// # let store = LocalFileSystem::new_with_prefix(root.path()).unwrap();
    /// # use object_store::{ObjectStore, ObjectMeta};
    /// # use object_store::path::Path;
    /// # use futures::{StreamExt, TryStreamExt};
    /// #
    /// // Create two objects
    /// store.put(&Path::from("foo"), "foo".into()).await?;
    /// store.put(&Path::from("bar"), "bar".into()).await?;
    ///
    /// // List object
    /// let locations = store.list(None).map_ok(|m| m.location).boxed();
    ///
    /// // Delete them
    /// store.delete_stream(locations).try_collect::<Vec<Path>>().await?;
    /// # Ok(())
    /// # }
    /// # let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
    /// # rt.block_on(example()).unwrap();
    /// ```
    fn delete_stream<'a>(
        &'a self,
        locations: BoxStream<'a, Result<Path>>,
    ) -> BoxStream<'a, Result<Path>> {
        locations
            .map(|location| async {
                let location = location?;
                self.delete(&location).await?;
                Ok(location)
            })
            .buffered(10)
            .boxed()
    }

    /// List all the objects with the given prefix.
    ///
    /// Prefixes are evaluated on a path segment basis, i.e. `foo/bar` is a prefix of `foo/bar/x` but not of
    /// `foo/bar_baz/x`. List is recursive, i.e. `foo/bar/more/x` will be included.
    ///
    /// Note: the order of returned [`ObjectMeta`] is not guaranteed
    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>>;

    /// List all the objects with the given prefix and a location greater than `offset`
    ///
    /// Some stores, such as S3 and GCS, may be able to push `offset` down to reduce
    /// the number of network requests required
    ///
    /// Note: the order of returned [`ObjectMeta`] is not guaranteed
    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        let offset = offset.clone();
        self.list(prefix)
            .try_filter(move |f| futures::future::ready(f.location > offset))
            .boxed()
    }

    /// List objects with the given prefix and an implementation specific
    /// delimiter. Returns common prefixes (directories) in addition to object
    /// metadata.
    ///
    /// Prefixes are evaluated on a path segment basis, i.e. `foo/bar` is a prefix of `foo/bar/x` but not of
    /// `foo/bar_baz/x`. List is not recursive, i.e. `foo/bar/more/x` will not be included.
    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult>;

    /// One bounded page of a delimited listing: at most `max_keys` entries
    /// (common prefixes and objects together), resumable via the returned
    /// opaque token.
    ///
    /// Stores with native delimited pagination (S3, GCS, Azure) serve this in
    /// a single request. The default implementation delegates to
    /// [`ObjectStore::list_with_delimiter`] — which enumerates the whole level
    /// — and slices a page out of it, resuming after the entry named by the
    /// token; that is only suitable for stores where listing is cheap, such as
    /// in-memory or local-filesystem implementations.
    async fn list_delimited_page(
        &self,
        prefix: Option<&Path>,
        token: Option<&str>,
        max_keys: Option<usize>,
    ) -> Result<ListPage> {
        let listed = self.list_with_delimiter(prefix).await?;
        // Order prefixes and objects together so pagination is deterministic
        // whatever order the store returned, then resume strictly after the
        // token. Entries are (raw path, is_prefix); the token carries the
        // discriminator too, since an object and a common prefix can share a
        // name (object `foo` next to object `foo/bar`) yet must page as two
        // distinct entries.
        let mut entries: Vec<(String, bool, Option<ObjectMeta>)> = listed
            .common_prefixes
            .into_iter()
            .map(|p| (p.to_string(), true, None))
            .chain(
                listed
                    .objects
                    .into_iter()
                    .map(|o| (o.location.to_string(), false, Some(o))),
            )
            .collect();
        entries.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));

        fn encode_token(raw: &str, is_prefix: bool) -> String {
            format!("{}:{}", if is_prefix { 'p' } else { 'o' }, raw)
        }
        let start = match token {
            Some(token) => {
                let (t_raw, t_prefix) = match token.split_at_checked(2) {
                    Some(("o:", raw)) => (raw, false),
                    Some(("p:", raw)) => (raw, true),
                    // Unknown token shape: treat it as a bare path and resume
                    // past both entries of that name.
                    _ => (token, true),
                };
                entries.partition_point(|(raw, is_prefix, _)| {
                    (raw.as_str(), *is_prefix) <= (t_raw, t_prefix)
                })
            }
            None => 0,
        };
        let max_keys = max_keys.unwrap_or(usize::MAX).max(1);
        let end = start.saturating_add(max_keys).min(entries.len());
        let next_token = (end < entries.len())
            .then(|| {
                entries[start..end]
                    .last()
                    .map(|(raw, is_prefix, _)| encode_token(raw, *is_prefix))
            })
            .flatten();
        let mut common_prefixes = Vec::new();
        let mut objects = Vec::new();
        for (raw, is_prefix, meta) in entries.drain(..end).skip(start) {
            if is_prefix {
                common_prefixes.push(Path::from(raw.as_str()));
            } else if let Some(o) = meta {
                objects.push(o);
            }
        }
        Ok(ListPage { common_prefixes, objects, next_token })
    }

    /// Copy an object from one path to another in the same object store.
    ///
    /// If there exists an object at the destination, it will be overwritten.
    async fn copy(&self, from: &Path, to: &Path) -> Result<()>;

    /// Move an object from one path to another in the same object store.
    ///
    /// By default, this is implemented as a copy and then delete source. It may not
    /// check when deleting source that it was the same object that was originally copied.
    ///
    /// If there exists an object at the destination, it will be overwritten.
    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        self.copy(from, to).await?;
        self.delete(from).await
    }

    /// Copy an object from one path to another, only if destination is empty.
    ///
    /// Will return an error if the destination already has an object.
    ///
    /// Performs an atomic operation if the underlying object storage supports it.
    /// If atomic operations are not supported by the underlying object storage (like S3)
    /// it will return an error.
    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()>;

    /// Move an object from one path to another in the same object store.
    ///
    /// Will return an error if the destination already has an object.
    async fn rename_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        self.copy_if_not_exists(from, to).await?;
        self.delete(from).await
    }
}

macro_rules! as_ref_impl {
    ($type:ty) => {
        #[async_trait]
        impl ObjectStore for $type {
            async fn put(&self, location: &Path, payload: PutPayload) -> Result<PutResult> {
                self.as_ref().put(location, payload).await
            }

            async fn put_opts(
                &self,
                location: &Path,
                payload: PutPayload,
                opts: PutOptions,
            ) -> Result<PutResult> {
                self.as_ref().put_opts(location, payload, opts).await
            }

            async fn put_multipart(&self, location: &Path) -> Result<Box<dyn MultipartUpload>> {
                self.as_ref().put_multipart(location).await
            }

            async fn put_multipart_opts(
                &self,
                location: &Path,
                opts: PutMultipartOpts,
            ) -> Result<Box<dyn MultipartUpload>> {
                self.as_ref().put_multipart_opts(location, opts).await
            }

            async fn get(&self, location: &Path) -> Result<GetResult> {
                self.as_ref().get(location).await
            }

            async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
                self.as_ref().get_opts(location, options).await
            }

            async fn get_range(&self, location: &Path, range: Range<u64>) -> Result<Bytes> {
                self.as_ref().get_range(location, range).await
            }

            async fn get_ranges(
                &self,
                location: &Path,
                ranges: &[Range<u64>],
            ) -> Result<Vec<Bytes>> {
                self.as_ref().get_ranges(location, ranges).await
            }

            async fn head(&self, location: &Path) -> Result<ObjectMeta> {
                self.as_ref().head(location).await
            }

            async fn delete(&self, location: &Path) -> Result<()> {
                self.as_ref().delete(location).await
            }

            fn delete_stream<'a>(
                &'a self,
                locations: BoxStream<'a, Result<Path>>,
            ) -> BoxStream<'a, Result<Path>> {
                self.as_ref().delete_stream(locations)
            }

            fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
                self.as_ref().list(prefix)
            }

            fn list_with_offset(
                &self,
                prefix: Option<&Path>,
                offset: &Path,
            ) -> BoxStream<'static, Result<ObjectMeta>> {
                self.as_ref().list_with_offset(prefix, offset)
            }

            async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
                self.as_ref().list_with_delimiter(prefix).await
            }

            async fn list_delimited_page(
                &self,
                prefix: Option<&Path>,
                token: Option<&str>,
                max_keys: Option<usize>,
            ) -> Result<ListPage> {
                self.as_ref().list_delimited_page(prefix, token, max_keys).await
            }

            async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
                self.as_ref().copy(from, to).await
            }

            async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
                self.as_ref().rename(from, to).await
            }

            async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
                self.as_ref().copy_if_not_exists(from, to).await
            }

            async fn rename_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
                self.as_ref().rename_if_not_exists(from, to).await
            }
        }
    };
}

as_ref_impl!(Arc<dyn ObjectStore>);
as_ref_impl!(Box<dyn ObjectStore>);

/// Result of a list call that includes objects, prefixes (directories) and a
/// token for the next set of results. Individual result sets may be limited to
/// 1,000 objects based on the underlying object storage's limitations.
#[derive(Debug)]
pub struct ListResult {
    /// Prefixes that are common (like directories)
    pub common_prefixes: Vec<Path>,
    /// Object metadata for the listing
    pub objects: Vec<ObjectMeta>,
}

/// One page of a delimited listing, as returned by
/// [`ObjectStore::list_delimited_page`]
#[derive(Debug, Clone)]
pub struct ListPage {
    /// Prefixes that are common (like directories)
    pub common_prefixes: Vec<Path>,
    /// Object metadata for the listing
    pub objects: Vec<ObjectMeta>,
    /// Opaque token resuming the listing where this page ended, if more
    /// entries remain at this level
    pub next_token: Option<String>,
}

/// The metadata that describes an object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMeta {
    /// The full path to the object
    pub location: Path,
    /// The last modified time
    pub last_modified: DateTime<Utc>,
    /// The size in bytes of the object.
    ///
    /// Note this is not `usize` as `object_store` supports 32-bit architectures such as WASM
    pub size: u64,
    /// The unique identifier for the object
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#name-etag>
    pub e_tag: Option<String>,
    /// A version indicator for this object
    pub version: Option<String>,
}

/// Options for a get request, such as range
#[derive(Debug, Default, Clone)]
pub struct GetOptions {
    /// Request will succeed if the `ObjectMeta::e_tag` matches
    /// otherwise returning [`Error::Precondition`]
    ///
    /// See <https://datatracker.ietf.org/doc/html/rfc9110#name-if-match>
    ///
    /// Examples:
    ///
    /// ```text
    /// If-Match: "xyzzy"
    /// If-Match: "xyzzy", "r2d2xxxx", "c3piozzzz"
    /// If-Match: *
    /// ```
    pub if_match: Option<String>,
    /// Request will succeed if the `ObjectMeta::e_tag` does not match
    /// otherwise returning [`Error::NotModified`]
    ///
    /// See <https://datatracker.ietf.org/doc/html/rfc9110#section-13.1.2>
    ///
    /// Examples:
    ///
    /// ```text
    /// If-None-Match: "xyzzy"
    /// If-None-Match: "xyzzy", "r2d2xxxx", "c3piozzzz"
    /// If-None-Match: *
    /// ```
    pub if_none_match: Option<String>,
    /// Request will succeed if the object has been modified since
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#section-13.1.3>
    pub if_modified_since: Option<DateTime<Utc>>,
    /// Request will succeed if the object has not been modified since
    /// otherwise returning [`Error::Precondition`]
    ///
    /// Some stores, such as S3, will only return `NotModified` for exact
    /// timestamp matches, instead of for any timestamp greater than or equal.
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#section-13.1.4>
    pub if_unmodified_since: Option<DateTime<Utc>>,
    /// Request transfer of only the specified range of bytes
    /// otherwise returning [`Error::NotModified`]
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#name-range>
    pub range: Option<GetRange>,
    /// Request a particular object version
    pub version: Option<String>,
    /// Request transfer of no content
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#name-head>
    pub head: bool,
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to pass context-specific information (like tracing spans) via trait methods.
    ///
    /// These extensions are ignored entirely by backends offered through this crate.
    pub extensions: ::http::Extensions,
}

impl GetOptions {
    /// Returns an error if the modification conditions on this request are not satisfied
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc7232#section-6>
    pub fn check_preconditions(&self, meta: &ObjectMeta) -> Result<()> {
        // The use of the invalid etag "*" means no ETag is equivalent to never matching
        let etag = meta.e_tag.as_deref().unwrap_or("*");
        let last_modified = meta.last_modified;

        if let Some(m) = &self.if_match {
            if m != "*" && m.split(',').map(str::trim).all(|x| x != etag) {
                return Err(Error::Precondition {
                    path: meta.location.to_string(),
                    source: format!("{etag} does not match {m}").into(),
                });
            }
        } else if let Some(date) = self.if_unmodified_since {
            if last_modified > date {
                return Err(Error::Precondition {
                    path: meta.location.to_string(),
                    source: format!("{date} < {last_modified}").into(),
                });
            }
        }

        if let Some(m) = &self.if_none_match {
            if m == "*" || m.split(',').map(str::trim).any(|x| x == etag) {
                return Err(Error::NotModified {
                    path: meta.location.to_string(),
                    source: format!("{etag} matches {m}").into(),
                });
            }
        } else if let Some(date) = self.if_modified_since {
            if last_modified <= date {
                return Err(Error::NotModified {
                    path: meta.location.to_string(),
                    source: format!("{date} >= {last_modified}").into(),
                });
            }
        }
        Ok(())
    }
}

/// Result for a get request
#[derive(Debug)]
pub struct GetResult {
    /// The [`GetResultPayload`]
    pub payload: GetResultPayload,
    /// The [`ObjectMeta`] for this object
    pub meta: ObjectMeta,
    /// The range of bytes returned by this request
    ///
    /// Note this is not `usize` as `object_store` supports 32-bit architectures such as WASM
    pub range: Range<u64>,
    /// Additional object attributes
    pub attributes: Attributes,
}

/// The kind of a [`GetResult`]
///
/// This special cases the case of a local file, as some systems may
/// be able to optimise the case of a file already present on local disk
pub enum GetResultPayload {
    /// The file, path
    #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
    File(std::fs::File, std::path::PathBuf),
    /// An opaque stream of bytes
    Stream(BoxStream<'static, Result<Bytes>>),
}

impl Debug for GetResultPayload {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
            Self::File(_, _) => write!(f, "GetResultPayload(File)"),
            Self::Stream(_) => write!(f, "GetResultPayload(Stream)"),
        }
    }
}

impl GetResult {
    /// Collects the data into a [`Bytes`]
    pub async fn bytes(self) -> Result<Bytes> {
        let len = self.range.end - self.range.start;
        match self.payload {
            #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
            GetResultPayload::File(mut file, path) => {
                maybe_spawn_blocking(move || {
                    file.seek(SeekFrom::Start(self.range.start as _))
                        .map_err(|source| local::Error::Seek {
                            source,
                            path: path.clone(),
                        })?;

                    let mut buffer = if let Ok(len) = len.try_into() {
                        Vec::with_capacity(len)
                    } else {
                        Vec::new()
                    };
                    file.take(len as _)
                        .read_to_end(&mut buffer)
                        .map_err(|source| local::Error::UnableToReadBytes { source, path })?;

                    Ok(buffer.into())
                })
                .await
            }
            GetResultPayload::Stream(s) => collect_bytes(s, Some(len)).await,
        }
    }

    /// Converts this into a byte stream
    ///
    /// If the `self.kind` is [`GetResultPayload::File`] will perform chunked reads of the file,
    /// otherwise will return the [`GetResultPayload::Stream`].
    ///
    /// # Tokio Compatibility
    ///
    /// Tokio discourages performing blocking IO on a tokio worker thread, however,
    /// no major operating systems have stable async file APIs. Therefore if called from
    /// a tokio context, this will use [`tokio::runtime::Handle::spawn_blocking`] to dispatch
    /// IO to a blocking thread pool, much like `tokio::fs` does under-the-hood.
    ///
    /// If not called from a tokio context, this will perform IO on the current thread with
    /// no additional complexity or overheads
    pub fn into_stream(self) -> BoxStream<'static, Result<Bytes>> {
        match self.payload {
            #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
            GetResultPayload::File(file, path) => {
                const CHUNK_SIZE: usize = 8 * 1024;
                local::chunked_stream(file, path, self.range, CHUNK_SIZE)
            }
            GetResultPayload::Stream(s) => s,
        }
    }
}

/// Configure preconditions for the put operation
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PutMode {
    /// Perform an atomic write operation, overwriting any object present at the provided path
    #[default]
    Overwrite,
    /// Perform an atomic write operation, returning [`Error::AlreadyExists`] if an
    /// object already exists at the provided path
    Create,
    /// Perform an atomic write operation if the current version of the object matches the
    /// provided [`UpdateVersion`], returning [`Error::Precondition`] otherwise
    Update(UpdateVersion),
}

/// Uniquely identifies a version of an object to update
///
/// Stores will use differing combinations of `e_tag` and `version` to provide conditional
/// updates, and it is therefore recommended applications preserve both
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateVersion {
    /// The unique identifier for the newly created object
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#name-etag>
    pub e_tag: Option<String>,
    /// A version indicator for the newly created object
    pub version: Option<String>,
}

impl From<PutResult> for UpdateVersion {
    fn from(value: PutResult) -> Self {
        Self {
            e_tag: value.e_tag,
            version: value.version,
        }
    }
}

/// Options for a put request
#[derive(Debug, Clone, Default)]
pub struct PutOptions {
    /// Configure the [`PutMode`] for this operation
    pub mode: PutMode,
    /// Provide a [`TagSet`] for this object
    ///
    /// Implementations that don't support object tagging should ignore this
    pub tags: TagSet,
    /// Provide a set of [`Attributes`]
    ///
    /// Implementations that don't support an attribute should return an error
    pub attributes: Attributes,
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to pass context-specific information (like tracing spans) via trait methods.
    ///
    /// These extensions are ignored entirely by backends offered through this crate.
    ///
    /// They are also eclused from [`PartialEq`] and [`Eq`].
    pub extensions: ::http::Extensions,
}

impl PartialEq<Self> for PutOptions {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            mode,
            tags,
            attributes,
            extensions: _,
        } = self;
        let Self {
            mode: other_mode,
            tags: other_tags,
            attributes: other_attributes,
            extensions: _,
        } = other;
        (mode == other_mode) && (tags == other_tags) && (attributes == other_attributes)
    }
}

impl Eq for PutOptions {}

impl From<PutMode> for PutOptions {
    fn from(mode: PutMode) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }
}

impl From<TagSet> for PutOptions {
    fn from(tags: TagSet) -> Self {
        Self {
            tags,
            ..Default::default()
        }
    }
}

impl From<Attributes> for PutOptions {
    fn from(attributes: Attributes) -> Self {
        Self {
            attributes,
            ..Default::default()
        }
    }
}

/// Options for [`ObjectStore::put_multipart_opts`]
#[derive(Debug, Clone, Default)]
pub struct PutMultipartOpts {
    /// Provide a [`TagSet`] for this object
    ///
    /// Implementations that don't support object tagging should ignore this
    pub tags: TagSet,
    /// Provide a set of [`Attributes`]
    ///
    /// Implementations that don't support an attribute should return an error
    pub attributes: Attributes,
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to pass context-specific information (like tracing spans) via trait methods.
    ///
    /// These extensions are ignored entirely by backends offered through this crate.
    ///
    /// They are also eclused from [`PartialEq`] and [`Eq`].
    pub extensions: ::http::Extensions,
}

impl PartialEq<Self> for PutMultipartOpts {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            tags,
            attributes,
            extensions: _,
        } = self;
        let Self {
            tags: other_tags,
            attributes: other_attributes,
            extensions: _,
        } = other;
        (tags == other_tags) && (attributes == other_attributes)
    }
}

impl Eq for PutMultipartOpts {}

impl From<TagSet> for PutMultipartOpts {
    fn from(tags: TagSet) -> Self {
        Self {
            tags,
            ..Default::default()
        }
    }
}

impl From<Attributes> for PutMultipartOpts {
    fn from(attributes: Attributes) -> Self {
        Self {
            attributes,
            ..Default::default()
        }
    }
}

/// Result for a put request
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutResult {
    /// The unique identifier for the newly created object
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#name-etag>
    pub e_tag: Option<String>,
    /// A version indicator for the newly created object
    pub version: Option<String>,
}

/// A specialized `Result` for object store-related errors
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A specialized `Error` for object store-related errors
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A fallback error type when no variant matches
    #[error("Generic {} error: {}", store, source)]
    Generic {
        /// The store this error originated from
        store: &'static str,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when the object is not found at given location
    #[error("Object at location {} not found: {}", path, source)]
    NotFound {
        /// The path to file
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error for invalid path
    #[error("Encountered object with invalid path: {}", source)]
    InvalidPath {
        /// The wrapped error
        #[from]
        source: path::Error,
    },

    /// Error when `tokio::spawn` failed
    #[error("Error joining spawned task: {}", source)]
    JoinError {
        /// The wrapped error
        #[from]
        source: tokio::task::JoinError,
    },

    /// Error when the attempted operation is not supported
    #[error("Operation not supported: {}", source)]
    NotSupported {
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when the object already exists
    #[error("Object at location {} already exists: {}", path, source)]
    AlreadyExists {
        /// The path to the
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when the required conditions failed for the operation
    #[error("Request precondition failure for path {}: {}", path, source)]
    Precondition {
        /// The path to the file
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when the object at the location isn't modified
    #[error("Object at location {} not modified: {}", path, source)]
    NotModified {
        /// The path to the file
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when an operation is not implemented
    #[error("Operation not yet implemented.")]
    NotImplemented,

    /// Error when the used credentials don't have enough permission
    /// to perform the requested operation
    #[error(
        "The operation lacked the necessary privileges to complete for path {}: {}",
        path,
        source
    )]
    PermissionDenied {
        /// The path to the file
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when the used credentials lack valid authentication
    #[error(
        "The operation lacked valid authentication credentials for path {}: {}",
        path,
        source
    )]
    Unauthenticated {
        /// The path to the file
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when a configuration key is invalid for the store used
    #[error("Configuration key: '{}' is not valid for store '{}'.", key, store)]
    UnknownConfigurationKey {
        /// The object store used
        store: &'static str,
        /// The configuration key used
        key: String,
    },
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> Self {
        let kind = match &e {
            Error::NotFound { .. } => std::io::ErrorKind::NotFound,
            _ => std::io::ErrorKind::Other,
        };
        Self::new(kind, e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffered::BufWriter;
    use chrono::TimeZone;
    use tokio::io::AsyncWriteExt;

    macro_rules! maybe_skip_integration {
        () => {
            if std::env::var("TEST_INTEGRATION").is_err() {
                eprintln!("Skipping integration test - set TEST_INTEGRATION");
                return;
            }
        };
    }
    pub(crate) use maybe_skip_integration;

    #[tokio::test]
    async fn test_list_delimited_page_default_impl() {
        let store = memory::InMemory::new();
        for key in [
            "a/1", "a/2", "a/3", "a/sub/x", "a/sub/y", "b/1", "root1", "root2",
        ] {
            store.put(&Path::from(key), vec![0u8].into()).await.unwrap();
        }

        // Page through the root level two entries at a time, collecting
        // everything; prefixes and objects both count toward max_keys.
        let mut prefixes = Vec::new();
        let mut objects = Vec::new();
        let mut token: Option<String> = None;
        let mut pages = 0;
        loop {
            let page = store
                .list_delimited_page(None, token.as_deref(), Some(2))
                .await
                .unwrap();
            assert!(page.common_prefixes.len() + page.objects.len() <= 2);
            prefixes.extend(page.common_prefixes);
            objects.extend(page.objects.into_iter().map(|o| o.location));
            pages += 1;
            match page.next_token {
                Some(t) => token = Some(t),
                None => break,
            }
            assert!(pages < 10);
        }
        assert_eq!(prefixes, vec![Path::from("a"), Path::from("b")]);
        assert_eq!(objects, vec![Path::from("root1"), Path::from("root2")]);
        assert_eq!(pages, 2);

        // A single unbounded page returns the whole level with no token.
        let page = store
            .list_delimited_page(Some(&Path::from("a")), None, None)
            .await
            .unwrap();
        assert_eq!(page.common_prefixes, vec![Path::from("a/sub")]);
        assert_eq!(
            page.objects.iter().map(|o| &o.location).collect::<Vec<_>>(),
            vec![&Path::from("a/1"), &Path::from("a/2"), &Path::from("a/3")]
        );
        assert_eq!(page.next_token, None);

        // Unbounded max_keys on a later page must not overflow the slice bound.
        let first = store
            .list_delimited_page(None, None, Some(1))
            .await
            .unwrap();
        let rest = store
            .list_delimited_page(None, first.next_token.as_deref(), None)
            .await
            .unwrap();
        assert_eq!(rest.next_token, None);
        assert_eq!(
            first.common_prefixes.len()
                + first.objects.len()
                + rest.common_prefixes.len()
                + rest.objects.len(),
            4
        );
    }

    #[tokio::test]
    async fn test_list_delimited_page_name_collision() {
        // Object `dup` and prefix `dup/` coexist at the root level; a page
        // boundary landing between them must not drop either.
        let store = memory::InMemory::new();
        for key in ["dup", "dup/nested", "last"] {
            store.put(&Path::from(key), vec![0u8].into()).await.unwrap();
        }
        let mut prefixes = Vec::new();
        let mut objects = Vec::new();
        let mut token: Option<String> = None;
        let mut pages = 0;
        loop {
            let page = store
                .list_delimited_page(None, token.as_deref(), Some(1))
                .await
                .unwrap();
            prefixes.extend(page.common_prefixes);
            objects.extend(page.objects.into_iter().map(|o| o.location));
            pages += 1;
            match page.next_token {
                Some(t) => token = Some(t),
                None => break,
            }
            assert!(pages < 10);
        }
        assert_eq!(objects, vec![Path::from("dup"), Path::from("last")]);
        assert_eq!(prefixes, vec![Path::from("dup")]);
        assert_eq!(pages, 3);
    }

    /// Test that the returned stream does not borrow the lifetime of Path
    fn list_store<'a>(
        store: &'a dyn ObjectStore,
        path_str: &str,
    ) -> BoxStream<'a, Result<ObjectMeta>> {
        let path = Path::from(path_str);
        store.list(Some(&path))
    }

    #[cfg(any(feature = "azure", feature = "aws"))]
    pub(crate) async fn signing<T>(integration: &T)
    where
        T: ObjectStore + signer::Signer,
    {
        use reqwest::Method;
        use std::time::Duration;

        let data = Bytes::from("hello world");
        let path = Path::from("file.txt");
        integration.put(&path, data.clone().into()).await.unwrap();

        let signed = integration
            .signed_url(Method::GET, &path, Duration::from_secs(60))
            .await
            .unwrap();

        let resp = reqwest::get(signed).await.unwrap();
        let loaded = resp.bytes().await.unwrap();

        assert_eq!(data, loaded);
    }

    #[cfg(any(feature = "aws", feature = "azure"))]
    pub(crate) async fn tagging<F, Fut>(storage: Arc<dyn ObjectStore>, validate: bool, get_tags: F)
    where
        F: Fn(Path) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<client::HttpResponse>> + Send,
    {
        use bytes::Buf;
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct Tagging {
            #[serde(rename = "TagSet")]
            list: TagList,
        }

        #[derive(Debug, Deserialize)]
        struct TagList {
            #[serde(rename = "Tag")]
            tags: Vec<Tag>,
        }

        #[derive(Debug, Deserialize, Eq, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Tag {
            key: String,
            value: String,
        }

        let tags = vec![
            Tag {
                key: "foo.com=bar/s".to_string(),
                value: "bananas/foo.com-_".to_string(),
            },
            Tag {
                key: "namespace/key.foo".to_string(),
                value: "value with a space".to_string(),
            },
        ];
        let mut tag_set = TagSet::default();
        for t in &tags {
            tag_set.push(&t.key, &t.value)
        }

        let path = Path::from("tag_test");
        storage
            .put_opts(&path, "test".into(), tag_set.clone().into())
            .await
            .unwrap();

        let multi_path = Path::from("tag_test_multi");
        let mut write = storage
            .put_multipart_opts(&multi_path, tag_set.clone().into())
            .await
            .unwrap();

        write.put_part("foo".into()).await.unwrap();
        write.complete().await.unwrap();

        let buf_path = Path::from("tag_test_buf");
        let mut buf = BufWriter::new(storage, buf_path.clone()).with_tags(tag_set);
        buf.write_all(b"foo").await.unwrap();
        buf.shutdown().await.unwrap();

        // Write should always succeed, but certain configurations may simply ignore tags
        if !validate {
            return;
        }

        for path in [path, multi_path, buf_path] {
            let resp = get_tags(path.clone()).await.unwrap();
            let body = resp.into_body().bytes().await.unwrap();

            let mut resp: Tagging = quick_xml::de::from_reader(body.reader()).unwrap();
            resp.list.tags.sort_by(|a, b| a.key.cmp(&b.key));
            assert_eq!(resp.list.tags, tags);
        }
    }

    #[tokio::test]
    async fn test_list_lifetimes() {
        let store = memory::InMemory::new();
        let mut stream = list_store(&store, "path");
        assert!(stream.next().await.is_none());
    }

    #[test]
    fn test_preconditions() {
        let mut meta = ObjectMeta {
            location: Path::from("test"),
            last_modified: Utc.timestamp_nanos(100),
            size: 100,
            e_tag: Some("123".to_string()),
            version: None,
        };

        let mut options = GetOptions::default();
        options.check_preconditions(&meta).unwrap();

        options.if_modified_since = Some(Utc.timestamp_nanos(50));
        options.check_preconditions(&meta).unwrap();

        options.if_modified_since = Some(Utc.timestamp_nanos(100));
        options.check_preconditions(&meta).unwrap_err();

        options.if_modified_since = Some(Utc.timestamp_nanos(101));
        options.check_preconditions(&meta).unwrap_err();

        options = GetOptions::default();

        options.if_unmodified_since = Some(Utc.timestamp_nanos(50));
        options.check_preconditions(&meta).unwrap_err();

        options.if_unmodified_since = Some(Utc.timestamp_nanos(100));
        options.check_preconditions(&meta).unwrap();

        options.if_unmodified_since = Some(Utc.timestamp_nanos(101));
        options.check_preconditions(&meta).unwrap();

        options = GetOptions::default();

        options.if_match = Some("123".to_string());
        options.check_preconditions(&meta).unwrap();

        options.if_match = Some("123,354".to_string());
        options.check_preconditions(&meta).unwrap();

        options.if_match = Some("354, 123,".to_string());
        options.check_preconditions(&meta).unwrap();

        options.if_match = Some("354".to_string());
        options.check_preconditions(&meta).unwrap_err();

        options.if_match = Some("*".to_string());
        options.check_preconditions(&meta).unwrap();

        // If-Match takes precedence
        options.if_unmodified_since = Some(Utc.timestamp_nanos(200));
        options.check_preconditions(&meta).unwrap();

        options = GetOptions::default();

        options.if_none_match = Some("123".to_string());
        options.check_preconditions(&meta).unwrap_err();

        options.if_none_match = Some("*".to_string());
        options.check_preconditions(&meta).unwrap_err();

        options.if_none_match = Some("1232".to_string());
        options.check_preconditions(&meta).unwrap();

        options.if_none_match = Some("23, 123".to_string());
        options.check_preconditions(&meta).unwrap_err();

        // If-None-Match takes precedence
        options.if_modified_since = Some(Utc.timestamp_nanos(10));
        options.check_preconditions(&meta).unwrap_err();

        // Check missing ETag
        meta.e_tag = None;
        options = GetOptions::default();

        options.if_none_match = Some("*".to_string()); // Fails if any file exists
        options.check_preconditions(&meta).unwrap_err();

        options = GetOptions::default();
        options.if_match = Some("*".to_string()); // Passes if file exists
        options.check_preconditions(&meta).unwrap();
    }
}
