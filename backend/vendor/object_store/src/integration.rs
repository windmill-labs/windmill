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

//! Integration tests for custom object store implementations
//!
//! NB: These tests will delete everything present in the provided [`DynObjectStore`].
//!
//! These tests are not a stable part of the public API and breaking changes may be made
//! in patch releases.
//!
//! They are intended solely for testing purposes.

use core::str;

use crate::multipart::MultipartStore;
use crate::path::Path;
use crate::{
    Attribute, Attributes, DynObjectStore, Error, GetOptions, GetRange, MultipartUpload,
    ObjectStore, PutMode, PutPayload, UpdateVersion, WriteMultipart,
};
use bytes::Bytes;
use futures::stream::FuturesUnordered;
use futures::{StreamExt, TryStreamExt};
use rand::distr::Alphanumeric;
use rand::{rng, Rng};

pub(crate) async fn flatten_list_stream(
    storage: &DynObjectStore,
    prefix: Option<&Path>,
) -> crate::Result<Vec<Path>> {
    storage
        .list(prefix)
        .map_ok(|meta| meta.location)
        .try_collect::<Vec<Path>>()
        .await
}

/// Tests basic read/write and listing operations
pub async fn put_get_delete_list(storage: &DynObjectStore) {
    delete_fixtures(storage).await;

    let content_list = flatten_list_stream(storage, None).await.unwrap();
    assert!(
        content_list.is_empty(),
        "Expected list to be empty; found: {content_list:?}"
    );

    let location = Path::from("test_dir/test_file.json");

    let data = Bytes::from("arbitrary data");
    storage.put(&location, data.clone().into()).await.unwrap();

    let root = Path::from("/");

    // List everything
    let content_list = flatten_list_stream(storage, None).await.unwrap();
    assert_eq!(content_list, &[location.clone()]);

    // Should behave the same as no prefix
    let content_list = flatten_list_stream(storage, Some(&root)).await.unwrap();
    assert_eq!(content_list, &[location.clone()]);

    // List with delimiter
    let result = storage.list_with_delimiter(None).await.unwrap();
    assert_eq!(&result.objects, &[]);
    assert_eq!(result.common_prefixes.len(), 1);
    assert_eq!(result.common_prefixes[0], Path::from("test_dir"));

    // Should behave the same as no prefix
    let result = storage.list_with_delimiter(Some(&root)).await.unwrap();
    assert!(result.objects.is_empty());
    assert_eq!(result.common_prefixes.len(), 1);
    assert_eq!(result.common_prefixes[0], Path::from("test_dir"));

    // Should return not found
    let err = storage.get(&Path::from("test_dir")).await.unwrap_err();
    assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);

    // Should return not found
    let err = storage.head(&Path::from("test_dir")).await.unwrap_err();
    assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);

    // List everything starting with a prefix that should return results
    let prefix = Path::from("test_dir");
    let content_list = flatten_list_stream(storage, Some(&prefix)).await.unwrap();
    assert_eq!(content_list, &[location.clone()]);

    // List everything starting with a prefix that shouldn't return results
    let prefix = Path::from("something");
    let content_list = flatten_list_stream(storage, Some(&prefix)).await.unwrap();
    assert!(content_list.is_empty());

    let read_data = storage.get(&location).await.unwrap().bytes().await.unwrap();
    assert_eq!(&*read_data, data);

    // Test range request
    let range = 3..7;
    let range_result = storage.get_range(&location, range.clone()).await;

    let bytes = range_result.unwrap();
    assert_eq!(bytes, data.slice(range.start as usize..range.end as usize));

    let opts = GetOptions {
        range: Some(GetRange::Bounded(2..5)),
        ..Default::default()
    };
    let result = storage.get_opts(&location, opts).await.unwrap();
    // Data is `"arbitrary data"`, length 14 bytes
    assert_eq!(result.meta.size, 14); // Should return full object size (#5272)
    assert_eq!(result.range, 2..5);
    let bytes = result.bytes().await.unwrap();
    assert_eq!(bytes, b"bit".as_ref());

    let out_of_range = 200..300;
    let out_of_range_result = storage.get_range(&location, out_of_range).await;

    // Should be a non-fatal error
    out_of_range_result.unwrap_err();

    let opts = GetOptions {
        range: Some(GetRange::Bounded(2..100)),
        ..Default::default()
    };
    let result = storage.get_opts(&location, opts).await.unwrap();
    assert_eq!(result.range, 2..14);
    assert_eq!(result.meta.size, 14);
    let bytes = result.bytes().await.unwrap();
    assert_eq!(bytes, b"bitrary data".as_ref());

    let opts = GetOptions {
        range: Some(GetRange::Suffix(2)),
        ..Default::default()
    };
    match storage.get_opts(&location, opts).await {
        Ok(result) => {
            assert_eq!(result.range, 12..14);
            assert_eq!(result.meta.size, 14);
            let bytes = result.bytes().await.unwrap();
            assert_eq!(bytes, b"ta".as_ref());
        }
        Err(Error::NotSupported { .. }) => {}
        Err(e) => panic!("{e}"),
    }

    let opts = GetOptions {
        range: Some(GetRange::Suffix(100)),
        ..Default::default()
    };
    match storage.get_opts(&location, opts).await {
        Ok(result) => {
            assert_eq!(result.range, 0..14);
            assert_eq!(result.meta.size, 14);
            let bytes = result.bytes().await.unwrap();
            assert_eq!(bytes, b"arbitrary data".as_ref());
        }
        Err(Error::NotSupported { .. }) => {}
        Err(e) => panic!("{e}"),
    }

    let opts = GetOptions {
        range: Some(GetRange::Offset(3)),
        ..Default::default()
    };
    let result = storage.get_opts(&location, opts).await.unwrap();
    assert_eq!(result.range, 3..14);
    assert_eq!(result.meta.size, 14);
    let bytes = result.bytes().await.unwrap();
    assert_eq!(bytes, b"itrary data".as_ref());

    let opts = GetOptions {
        range: Some(GetRange::Offset(100)),
        ..Default::default()
    };
    storage.get_opts(&location, opts).await.unwrap_err();

    let ranges = vec![0..1, 2..3, 0..5];
    let bytes = storage.get_ranges(&location, &ranges).await.unwrap();
    for (range, bytes) in ranges.iter().zip(bytes) {
        assert_eq!(bytes, data.slice(range.start as usize..range.end as usize));
    }

    let head = storage.head(&location).await.unwrap();
    assert_eq!(head.size, data.len() as u64);

    storage.delete(&location).await.unwrap();

    let content_list = flatten_list_stream(storage, None).await.unwrap();
    assert!(content_list.is_empty());

    let err = storage.get(&location).await.unwrap_err();
    assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);

    let err = storage.head(&location).await.unwrap_err();
    assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);

    // Test handling of paths containing an encoded delimiter

    let file_with_delimiter = Path::from_iter(["a", "b/c", "foo.file"]);
    storage
        .put(&file_with_delimiter, "arbitrary".into())
        .await
        .unwrap();

    let files = flatten_list_stream(storage, None).await.unwrap();
    assert_eq!(files, vec![file_with_delimiter.clone()]);

    let files = flatten_list_stream(storage, Some(&Path::from("a/b")))
        .await
        .unwrap();
    assert!(files.is_empty());

    let files = storage
        .list_with_delimiter(Some(&Path::from("a/b")))
        .await
        .unwrap();
    assert!(files.common_prefixes.is_empty());
    assert!(files.objects.is_empty());

    let files = storage
        .list_with_delimiter(Some(&Path::from("a")))
        .await
        .unwrap();
    assert_eq!(files.common_prefixes, vec![Path::from_iter(["a", "b/c"])]);
    assert!(files.objects.is_empty());

    let files = storage
        .list_with_delimiter(Some(&Path::from_iter(["a", "b/c"])))
        .await
        .unwrap();
    assert!(files.common_prefixes.is_empty());
    assert_eq!(files.objects.len(), 1);
    assert_eq!(files.objects[0].location, file_with_delimiter);

    storage.delete(&file_with_delimiter).await.unwrap();

    // Test handling of paths containing non-ASCII characters, e.g. emoji

    let emoji_prefix = Path::from("🙀");
    let emoji_file = Path::from("🙀/😀.parquet");
    storage.put(&emoji_file, "arbitrary".into()).await.unwrap();

    storage.head(&emoji_file).await.unwrap();
    storage
        .get(&emoji_file)
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();

    let files = flatten_list_stream(storage, Some(&emoji_prefix))
        .await
        .unwrap();

    assert_eq!(files, vec![emoji_file.clone()]);

    let dst = Path::from("foo.parquet");
    storage.copy(&emoji_file, &dst).await.unwrap();
    let mut files = flatten_list_stream(storage, None).await.unwrap();
    files.sort_unstable();
    assert_eq!(files, vec![emoji_file.clone(), dst.clone()]);

    let dst2 = Path::from("new/nested/foo.parquet");
    storage.copy(&emoji_file, &dst2).await.unwrap();
    let mut files = flatten_list_stream(storage, None).await.unwrap();
    files.sort_unstable();
    assert_eq!(files, vec![emoji_file.clone(), dst.clone(), dst2.clone()]);

    let dst3 = Path::from("new/nested2/bar.parquet");
    storage.rename(&dst, &dst3).await.unwrap();
    let mut files = flatten_list_stream(storage, None).await.unwrap();
    files.sort_unstable();
    assert_eq!(files, vec![emoji_file.clone(), dst2.clone(), dst3.clone()]);

    let err = storage.head(&dst).await.unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }));

    storage.delete(&emoji_file).await.unwrap();
    storage.delete(&dst3).await.unwrap();
    storage.delete(&dst2).await.unwrap();
    let files = flatten_list_stream(storage, Some(&emoji_prefix))
        .await
        .unwrap();
    assert!(files.is_empty());

    // Test handling of paths containing percent-encoded sequences

    // "HELLO" percent encoded
    let hello_prefix = Path::parse("%48%45%4C%4C%4F").unwrap();
    let path = hello_prefix.child("foo.parquet");

    storage.put(&path, vec![0, 1].into()).await.unwrap();
    let files = flatten_list_stream(storage, Some(&hello_prefix))
        .await
        .unwrap();
    assert_eq!(files, vec![path.clone()]);

    // Cannot list by decoded representation
    let files = flatten_list_stream(storage, Some(&Path::from("HELLO")))
        .await
        .unwrap();
    assert!(files.is_empty());

    // Cannot access by decoded representation
    let err = storage
        .head(&Path::from("HELLO/foo.parquet"))
        .await
        .unwrap_err();
    assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);

    storage.delete(&path).await.unwrap();

    // Test handling of unicode paths
    let path = Path::parse("🇦🇺/$shenanigans@@~.txt").unwrap();
    storage.put(&path, "test".into()).await.unwrap();

    let r = storage.get(&path).await.unwrap();
    assert_eq!(r.bytes().await.unwrap(), "test");

    let dir = Path::parse("🇦🇺").unwrap();
    let r = storage.list_with_delimiter(None).await.unwrap();
    assert!(r.common_prefixes.contains(&dir));

    let r = storage.list_with_delimiter(Some(&dir)).await.unwrap();
    assert_eq!(r.objects.len(), 1);
    assert_eq!(r.objects[0].location, path);

    storage.delete(&path).await.unwrap();

    // Can also write non-percent encoded sequences
    let path = Path::parse("%Q.parquet").unwrap();
    storage.put(&path, vec![0, 1].into()).await.unwrap();

    let files = flatten_list_stream(storage, None).await.unwrap();
    assert_eq!(files, vec![path.clone()]);

    storage.delete(&path).await.unwrap();

    let path = Path::parse("foo bar/I contain spaces.parquet").unwrap();
    storage.put(&path, vec![0, 1].into()).await.unwrap();
    storage.head(&path).await.unwrap();

    let files = flatten_list_stream(storage, Some(&Path::from("foo bar")))
        .await
        .unwrap();
    assert_eq!(files, vec![path.clone()]);

    storage.delete(&path).await.unwrap();

    let files = flatten_list_stream(storage, None).await.unwrap();
    assert!(files.is_empty(), "{files:?}");

    // Test list order
    let files = vec![
        Path::from("a a/b.file"),
        Path::parse("a%2Fa.file").unwrap(),
        Path::from("a/😀.file"),
        Path::from("a/a file"),
        Path::parse("a/a%2F.file").unwrap(),
        Path::from("a/a.file"),
        Path::from("a/a/b.file"),
        Path::from("a/b.file"),
        Path::from("aa/a.file"),
        Path::from("ab/a.file"),
    ];

    for file in &files {
        storage.put(file, "foo".into()).await.unwrap();
    }

    let cases = [
        (None, Path::from("a")),
        (None, Path::from("a/a file")),
        (None, Path::from("a/a/b.file")),
        (None, Path::from("ab/a.file")),
        (None, Path::from("a%2Fa.file")),
        (None, Path::from("a/😀.file")),
        (Some(Path::from("a")), Path::from("")),
        (Some(Path::from("a")), Path::from("a")),
        (Some(Path::from("a")), Path::from("a/😀")),
        (Some(Path::from("a")), Path::from("a/😀.file")),
        (Some(Path::from("a")), Path::from("a/b")),
        (Some(Path::from("a")), Path::from("a/a/b.file")),
    ];

    for (prefix, offset) in cases {
        let s = storage.list_with_offset(prefix.as_ref(), &offset);
        let mut actual: Vec<_> = s.map_ok(|x| x.location).try_collect().await.unwrap();

        actual.sort_unstable();

        let expected: Vec<_> = files
            .iter()
            .filter(|x| {
                let prefix_match = prefix.as_ref().map(|p| x.prefix_matches(p)).unwrap_or(true);
                prefix_match && *x > &offset
            })
            .cloned()
            .collect();

        assert_eq!(actual, expected, "{prefix:?} - {offset:?}");
    }

    // Test bulk delete
    let paths = vec![
        Path::from("a/a.file"),
        Path::from("a/a/b.file"),
        Path::from("aa/a.file"),
        Path::from("does_not_exist"),
        Path::from("I'm a < & weird path"),
        Path::from("ab/a.file"),
        Path::from("a/😀.file"),
    ];

    storage.put(&paths[4], "foo".into()).await.unwrap();

    let out_paths = storage
        .delete_stream(futures::stream::iter(paths.clone()).map(Ok).boxed())
        .collect::<Vec<_>>()
        .await;

    assert_eq!(out_paths.len(), paths.len());

    let expect_errors = [3];

    for (i, input_path) in paths.iter().enumerate() {
        let err = storage.head(input_path).await.unwrap_err();
        assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);

        if expect_errors.contains(&i) {
            // Some object stores will report NotFound, but others (such as S3) will
            // report success regardless.
            match &out_paths[i] {
                Err(Error::NotFound { path: out_path, .. }) => {
                    assert!(out_path.ends_with(&input_path.to_string()));
                }
                Ok(out_path) => {
                    assert_eq!(out_path, input_path);
                }
                _ => panic!("unexpected error"),
            }
        } else {
            assert_eq!(out_paths[i].as_ref().unwrap(), input_path);
        }
    }

    delete_fixtures(storage).await;

    let path = Path::from("empty");
    storage.put(&path, PutPayload::default()).await.unwrap();
    let meta = storage.head(&path).await.unwrap();
    assert_eq!(meta.size, 0);
    let data = storage.get(&path).await.unwrap().bytes().await.unwrap();
    assert_eq!(data.len(), 0);

    storage.delete(&path).await.unwrap();
}

/// Tests the ability to read and write [`Attributes`]
pub async fn put_get_attributes(integration: &dyn ObjectStore) {
    // Test handling of attributes
    let attributes = Attributes::from_iter([
        (Attribute::CacheControl, "max-age=604800"),
        (
            Attribute::ContentDisposition,
            r#"attachment; filename="test.html""#,
        ),
        (Attribute::ContentEncoding, "gzip"),
        (Attribute::ContentLanguage, "en-US"),
        (Attribute::ContentType, "text/html; charset=utf-8"),
        (Attribute::Metadata("test_key".into()), "test_value"),
    ]);

    let path = Path::from("attributes");
    let opts = attributes.clone().into();
    match integration.put_opts(&path, "foo".into(), opts).await {
        Ok(_) => {
            let r = integration.get(&path).await.unwrap();
            assert_eq!(r.attributes, attributes);
        }
        Err(Error::NotImplemented) => {}
        Err(e) => panic!("{e}"),
    }

    let opts = attributes.clone().into();
    match integration.put_multipart_opts(&path, opts).await {
        Ok(mut w) => {
            w.put_part("foo".into()).await.unwrap();
            w.complete().await.unwrap();

            let r = integration.get(&path).await.unwrap();
            assert_eq!(r.attributes, attributes);
        }
        Err(Error::NotImplemented) => {}
        Err(e) => panic!("{e}"),
    }
}

/// Tests conditional read requests
pub async fn get_opts(storage: &dyn ObjectStore) {
    let path = Path::from("test");
    storage.put(&path, "foo".into()).await.unwrap();
    let meta = storage.head(&path).await.unwrap();

    let options = GetOptions {
        if_unmodified_since: Some(meta.last_modified),
        ..GetOptions::default()
    };
    match storage.get_opts(&path, options).await {
        Ok(_) | Err(Error::NotSupported { .. }) => {}
        Err(e) => panic!("{e}"),
    }

    let options = GetOptions {
        if_unmodified_since: Some(meta.last_modified + chrono::Duration::try_hours(10).unwrap()),
        ..GetOptions::default()
    };
    match storage.get_opts(&path, options).await {
        Ok(_) | Err(Error::NotSupported { .. }) => {}
        Err(e) => panic!("{e}"),
    }

    let options = GetOptions {
        if_unmodified_since: Some(meta.last_modified - chrono::Duration::try_hours(10).unwrap()),
        ..GetOptions::default()
    };
    match storage.get_opts(&path, options).await {
        Err(Error::Precondition { .. } | Error::NotSupported { .. }) => {}
        d => panic!("{d:?}"),
    }

    let options = GetOptions {
        if_modified_since: Some(meta.last_modified),
        ..GetOptions::default()
    };
    match storage.get_opts(&path, options).await {
        Err(Error::NotModified { .. } | Error::NotSupported { .. }) => {}
        d => panic!("{d:?}"),
    }

    let options = GetOptions {
        if_modified_since: Some(meta.last_modified - chrono::Duration::try_hours(10).unwrap()),
        ..GetOptions::default()
    };
    match storage.get_opts(&path, options).await {
        Ok(_) | Err(Error::NotSupported { .. }) => {}
        Err(e) => panic!("{e}"),
    }

    let tag = meta.e_tag.unwrap();
    let options = GetOptions {
        if_match: Some(tag.clone()),
        ..GetOptions::default()
    };
    storage.get_opts(&path, options).await.unwrap();

    let options = GetOptions {
        if_match: Some("invalid".to_string()),
        ..GetOptions::default()
    };
    let err = storage.get_opts(&path, options).await.unwrap_err();
    assert!(matches!(err, Error::Precondition { .. }), "{err}");

    let options = GetOptions {
        if_none_match: Some(tag.clone()),
        ..GetOptions::default()
    };
    let err = storage.get_opts(&path, options).await.unwrap_err();
    assert!(matches!(err, Error::NotModified { .. }), "{err}");

    let options = GetOptions {
        if_none_match: Some("invalid".to_string()),
        ..GetOptions::default()
    };
    storage.get_opts(&path, options).await.unwrap();

    let result = storage.put(&path, "test".into()).await.unwrap();
    let new_tag = result.e_tag.unwrap();
    assert_ne!(tag, new_tag);

    let meta = storage.head(&path).await.unwrap();
    assert_eq!(meta.e_tag.unwrap(), new_tag);

    let options = GetOptions {
        if_match: Some(new_tag),
        ..GetOptions::default()
    };
    storage.get_opts(&path, options).await.unwrap();

    let options = GetOptions {
        if_match: Some(tag),
        ..GetOptions::default()
    };
    let err = storage.get_opts(&path, options).await.unwrap_err();
    assert!(matches!(err, Error::Precondition { .. }), "{err}");

    if let Some(version) = meta.version {
        storage.put(&path, "bar".into()).await.unwrap();

        let options = GetOptions {
            version: Some(version),
            ..GetOptions::default()
        };

        // Can retrieve previous version
        let get_opts = storage.get_opts(&path, options).await.unwrap();
        let old = get_opts.bytes().await.unwrap();
        assert_eq!(old, b"test".as_slice());

        // Current version contains the updated data
        let current = storage.get(&path).await.unwrap().bytes().await.unwrap();
        assert_eq!(&current, b"bar".as_slice());
    }
}

/// Tests conditional writes
pub async fn put_opts(storage: &dyn ObjectStore, supports_update: bool) {
    // When using DynamoCommit repeated runs of this test will produce the same sequence of records in DynamoDB
    // As a result each conditional operation will need to wait for the lease to timeout before proceeding
    // One solution would be to clear DynamoDB before each test, but this would require non-trivial additional code
    // so we instead just generate a random suffix for the filenames
    let rng = rng();
    let suffix = String::from_utf8(rng.sample_iter(Alphanumeric).take(32).collect()).unwrap();

    delete_fixtures(storage).await;
    let path = Path::from(format!("put_opts_{suffix}"));
    let v1 = storage
        .put_opts(&path, "a".into(), PutMode::Create.into())
        .await
        .unwrap();

    let err = storage
        .put_opts(&path, "b".into(), PutMode::Create.into())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::AlreadyExists { .. }), "{err}");

    let b = storage.get(&path).await.unwrap().bytes().await.unwrap();
    assert_eq!(b.as_ref(), b"a");

    if !supports_update {
        let err = storage
            .put_opts(&path, "c".into(), PutMode::Update(v1.clone().into()).into())
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotImplemented), "{err}");

        return;
    }

    let v2 = storage
        .put_opts(&path, "c".into(), PutMode::Update(v1.clone().into()).into())
        .await
        .unwrap();

    let b = storage.get(&path).await.unwrap().bytes().await.unwrap();
    assert_eq!(b.as_ref(), b"c");

    let err = storage
        .put_opts(&path, "d".into(), PutMode::Update(v1.into()).into())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Precondition { .. }), "{err}");

    storage
        .put_opts(&path, "e".into(), PutMode::Update(v2.clone().into()).into())
        .await
        .unwrap();

    let b = storage.get(&path).await.unwrap().bytes().await.unwrap();
    assert_eq!(b.as_ref(), b"e");

    // Update not exists
    let path = Path::from("I don't exist");
    let err = storage
        .put_opts(&path, "e".into(), PutMode::Update(v2.into()).into())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Precondition { .. }), "{err}");

    const NUM_WORKERS: usize = 5;
    const NUM_INCREMENTS: usize = 10;

    let path = Path::from(format!("RACE-{suffix}"));
    let mut futures: FuturesUnordered<_> = (0..NUM_WORKERS)
        .map(|_| async {
            for _ in 0..NUM_INCREMENTS {
                loop {
                    match storage.get(&path).await {
                        Ok(r) => {
                            let mode = PutMode::Update(UpdateVersion {
                                e_tag: r.meta.e_tag.clone(),
                                version: r.meta.version.clone(),
                            });

                            let b = r.bytes().await.unwrap();
                            let v: usize = std::str::from_utf8(&b).unwrap().parse().unwrap();
                            let new = (v + 1).to_string();

                            match storage.put_opts(&path, new.into(), mode.into()).await {
                                Ok(_) => break,
                                Err(Error::Precondition { .. }) => continue,
                                Err(e) => return Err(e),
                            }
                        }
                        Err(Error::NotFound { .. }) => {
                            let mode = PutMode::Create;
                            match storage.put_opts(&path, "1".into(), mode.into()).await {
                                Ok(_) => break,
                                Err(Error::AlreadyExists { .. }) => continue,
                                Err(e) => return Err(e),
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            Ok(())
        })
        .collect();

    while futures.next().await.transpose().unwrap().is_some() {}
    let b = storage.get(&path).await.unwrap().bytes().await.unwrap();
    let v = std::str::from_utf8(&b).unwrap().parse::<usize>().unwrap();
    assert_eq!(v, NUM_WORKERS * NUM_INCREMENTS);
}

/// Returns a chunk of length `chunk_length`
fn get_chunk(chunk_length: usize) -> Bytes {
    let mut data = vec![0_u8; chunk_length];
    let mut rng = rng();
    // Set a random selection of bytes
    for _ in 0..1000 {
        data[rng.random_range(0..chunk_length)] = rng.random();
    }
    data.into()
}

/// Returns `num_chunks` of length `chunks`
fn get_chunks(chunk_length: usize, num_chunks: usize) -> Vec<Bytes> {
    (0..num_chunks).map(|_| get_chunk(chunk_length)).collect()
}

/// Tests the ability to perform multipart writes
pub async fn stream_get(storage: &DynObjectStore) {
    let location = Path::from("test_dir/test_upload_file.txt");

    // Can write to storage
    let data = get_chunks(5 * 1024 * 1024, 3);
    let bytes_expected = data.concat();
    let mut upload = storage.put_multipart(&location).await.unwrap();
    let uploads = data.into_iter().map(|x| upload.put_part(x.into()));
    futures::future::try_join_all(uploads).await.unwrap();

    // Object should not yet exist in store
    let meta_res = storage.head(&location).await;
    assert!(meta_res.is_err());
    assert!(matches!(
        meta_res.unwrap_err(),
        crate::Error::NotFound { .. }
    ));

    let files = flatten_list_stream(storage, None).await.unwrap();
    assert_eq!(&files, &[]);

    let result = storage.list_with_delimiter(None).await.unwrap();
    assert_eq!(&result.objects, &[]);

    upload.complete().await.unwrap();

    let bytes_written = storage.get(&location).await.unwrap().bytes().await.unwrap();
    assert_eq!(bytes_expected, bytes_written);

    // Can overwrite some storage
    // Sizes chosen to ensure we write three parts
    let data = get_chunks(3_200_000, 7);
    let bytes_expected = data.concat();
    let upload = storage.put_multipart(&location).await.unwrap();
    let mut writer = WriteMultipart::new(upload);
    for chunk in &data {
        writer.write(chunk)
    }
    writer.finish().await.unwrap();
    let bytes_written = storage.get(&location).await.unwrap().bytes().await.unwrap();
    assert_eq!(bytes_expected, bytes_written);

    let location = Path::from("test_dir/test_put_part.txt");
    let upload = storage.put_multipart(&location).await.unwrap();
    let mut write = WriteMultipart::new(upload);
    write.put(vec![0; 2].into());
    write.put(vec![3; 4].into());
    write.finish().await.unwrap();

    let meta = storage.head(&location).await.unwrap();
    assert_eq!(meta.size, 6);

    let location = Path::from("test_dir/test_put_part_mixed.txt");
    let upload = storage.put_multipart(&location).await.unwrap();
    let mut write = WriteMultipart::new(upload);
    write.put(vec![0; 2].into());
    write.write(&[1, 2, 3]);
    write.put(vec![4, 5, 6, 7].into());
    write.finish().await.unwrap();

    let r = storage.get(&location).await.unwrap();
    let r = r.bytes().await.unwrap();
    assert_eq!(r.as_ref(), &[0, 0, 1, 2, 3, 4, 5, 6, 7]);

    // We can abort an empty write
    let location = Path::from("test_dir/test_abort_upload.txt");
    let mut upload = storage.put_multipart(&location).await.unwrap();
    upload.abort().await.unwrap();
    let get_res = storage.get(&location).await;
    assert!(get_res.is_err());
    assert!(matches!(
        get_res.unwrap_err(),
        crate::Error::NotFound { .. }
    ));

    // We can abort an in-progress write
    let mut upload = storage.put_multipart(&location).await.unwrap();
    upload
        .put_part(data.first().unwrap().clone().into())
        .await
        .unwrap();

    upload.abort().await.unwrap();
    let get_res = storage.get(&location).await;
    assert!(get_res.is_err());
    assert!(matches!(get_res.unwrap_err(), Error::NotFound { .. }));
}

/// Tests that directories are transparent
pub async fn list_uses_directories_correctly(storage: &DynObjectStore) {
    delete_fixtures(storage).await;

    let content_list = flatten_list_stream(storage, None).await.unwrap();
    assert!(
        content_list.is_empty(),
        "Expected list to be empty; found: {content_list:?}"
    );

    let location1 = Path::from("foo/x.json");
    let location2 = Path::from("foo.bar/y.json");

    let data = PutPayload::from("arbitrary data");
    storage.put(&location1, data.clone()).await.unwrap();
    storage.put(&location2, data).await.unwrap();

    let prefix = Path::from("foo");
    let content_list = flatten_list_stream(storage, Some(&prefix)).await.unwrap();
    assert_eq!(content_list, &[location1.clone()]);

    let result = storage.list_with_delimiter(Some(&prefix)).await.unwrap();
    assert_eq!(result.objects.len(), 1);
    assert_eq!(result.objects[0].location, location1);
    assert_eq!(result.common_prefixes, &[]);

    // Listing an existing path (file) should return an empty list:
    // https://github.com/apache/arrow-rs/issues/3712
    let content_list = flatten_list_stream(storage, Some(&location1))
        .await
        .unwrap();
    assert_eq!(content_list, &[]);

    let list = storage.list_with_delimiter(Some(&location1)).await.unwrap();
    assert_eq!(list.objects, &[]);
    assert_eq!(list.common_prefixes, &[]);

    let prefix = Path::from("foo/x");
    let content_list = flatten_list_stream(storage, Some(&prefix)).await.unwrap();
    assert_eq!(content_list, &[]);

    let list = storage.list_with_delimiter(Some(&prefix)).await.unwrap();
    assert_eq!(list.objects, &[]);
    assert_eq!(list.common_prefixes, &[]);
}

/// Tests listing with delimiter
pub async fn list_with_delimiter(storage: &DynObjectStore) {
    delete_fixtures(storage).await;

    // ==================== check: store is empty ====================
    let content_list = flatten_list_stream(storage, None).await.unwrap();
    assert!(content_list.is_empty());

    // ==================== do: create files ====================
    let data = Bytes::from("arbitrary data");

    let files: Vec<_> = [
        "test_file",
        "mydb/wb/000/000/000.segment",
        "mydb/wb/000/000/001.segment",
        "mydb/wb/000/000/002.segment",
        "mydb/wb/001/001/000.segment",
        "mydb/wb/foo.json",
        "mydb/wbwbwb/111/222/333.segment",
        "mydb/data/whatevs",
    ]
    .iter()
    .map(|&s| Path::from(s))
    .collect();

    for f in &files {
        storage.put(f, data.clone().into()).await.unwrap();
    }

    // ==================== check: prefix-list `mydb/wb` (directory) ====================
    let prefix = Path::from("mydb/wb");

    let expected_000 = Path::from("mydb/wb/000");
    let expected_001 = Path::from("mydb/wb/001");
    let expected_location = Path::from("mydb/wb/foo.json");

    let result = storage.list_with_delimiter(Some(&prefix)).await.unwrap();

    assert_eq!(result.common_prefixes, vec![expected_000, expected_001]);
    assert_eq!(result.objects.len(), 1);

    let object = &result.objects[0];

    assert_eq!(object.location, expected_location);
    assert_eq!(object.size, data.len() as u64);

    // ==================== check: prefix-list `mydb/wb/000/000/001` (partial filename doesn't match) ====================
    let prefix = Path::from("mydb/wb/000/000/001");

    let result = storage.list_with_delimiter(Some(&prefix)).await.unwrap();
    assert!(result.common_prefixes.is_empty());
    assert_eq!(result.objects.len(), 0);

    // ==================== check: prefix-list `not_there` (non-existing prefix) ====================
    let prefix = Path::from("not_there");

    let result = storage.list_with_delimiter(Some(&prefix)).await.unwrap();
    assert!(result.common_prefixes.is_empty());
    assert!(result.objects.is_empty());

    // ==================== do: remove all files ====================
    for f in &files {
        storage.delete(f).await.unwrap();
    }

    // ==================== check: store is empty ====================
    let content_list = flatten_list_stream(storage, None).await.unwrap();
    assert!(content_list.is_empty());
}

/// Tests fetching a non-existent object returns a not found error
pub async fn get_nonexistent_object(
    storage: &DynObjectStore,
    location: Option<Path>,
) -> crate::Result<Bytes> {
    let location = location.unwrap_or_else(|| Path::from("this_file_should_not_exist"));

    let err = storage.head(&location).await.unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }));

    storage.get(&location).await?.bytes().await
}

/// Tests copying
pub async fn rename_and_copy(storage: &DynObjectStore) {
    // Create two objects
    let path1 = Path::from("test1");
    let path2 = Path::from("test2");
    let contents1 = Bytes::from("cats");
    let contents2 = Bytes::from("dogs");

    // copy() make both objects identical
    storage.put(&path1, contents1.clone().into()).await.unwrap();
    storage.put(&path2, contents2.clone().into()).await.unwrap();
    storage.copy(&path1, &path2).await.unwrap();
    let new_contents = storage.get(&path2).await.unwrap().bytes().await.unwrap();
    assert_eq!(&new_contents, &contents1);

    // rename() copies contents and deletes original
    storage.put(&path1, contents1.clone().into()).await.unwrap();
    storage.put(&path2, contents2.clone().into()).await.unwrap();
    storage.rename(&path1, &path2).await.unwrap();
    let new_contents = storage.get(&path2).await.unwrap().bytes().await.unwrap();
    assert_eq!(&new_contents, &contents1);
    let result = storage.get(&path1).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::NotFound { .. }));

    // Clean up
    storage.delete(&path2).await.unwrap();
}

/// Tests copy if not exists
pub async fn copy_if_not_exists(storage: &DynObjectStore) {
    // Create two objects
    let path1 = Path::from("test1");
    let path2 = Path::from("not_exists_nested/test2");
    let contents1 = Bytes::from("cats");
    let contents2 = Bytes::from("dogs");

    // copy_if_not_exists() errors if destination already exists
    storage.put(&path1, contents1.clone().into()).await.unwrap();
    storage.put(&path2, contents2.clone().into()).await.unwrap();
    let result = storage.copy_if_not_exists(&path1, &path2).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::Error::AlreadyExists { .. }
    ));

    // copy_if_not_exists() copies contents and allows deleting original
    storage.delete(&path2).await.unwrap();
    storage.copy_if_not_exists(&path1, &path2).await.unwrap();
    storage.delete(&path1).await.unwrap();
    let new_contents = storage.get(&path2).await.unwrap().bytes().await.unwrap();
    assert_eq!(&new_contents, &contents1);
    let result = storage.get(&path1).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), crate::Error::NotFound { .. }));

    // Clean up
    storage.delete(&path2).await.unwrap();
}

/// Tests copy and renaming behaviour of non-existent objects
pub async fn copy_rename_nonexistent_object(storage: &DynObjectStore) {
    // Create empty source object
    let path1 = Path::from("test1");

    // Create destination object
    let path2 = Path::from("test2");
    storage.put(&path2, "hello".into()).await.unwrap();

    // copy() errors if source does not exist
    let result = storage.copy(&path1, &path2).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), crate::Error::NotFound { .. }));

    // rename() errors if source does not exist
    let result = storage.rename(&path1, &path2).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), crate::Error::NotFound { .. }));

    // copy_if_not_exists() errors if source does not exist
    let result = storage.copy_if_not_exists(&path1, &path2).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), crate::Error::NotFound { .. }));

    // Clean up
    storage.delete(&path2).await.unwrap();
}

/// Tests [`MultipartStore`]
pub async fn multipart(storage: &dyn ObjectStore, multipart: &dyn MultipartStore) {
    let path = Path::from("test_multipart");
    let chunk_size = 5 * 1024 * 1024;

    let chunks = get_chunks(chunk_size, 2);

    let id = multipart.create_multipart(&path).await.unwrap();

    let parts: Vec<_> = futures::stream::iter(chunks)
        .enumerate()
        .map(|(idx, b)| multipart.put_part(&path, &id, idx, b.into()))
        .buffered(2)
        .try_collect()
        .await
        .unwrap();

    multipart
        .complete_multipart(&path, &id, parts)
        .await
        .unwrap();

    let meta = storage.head(&path).await.unwrap();
    assert_eq!(meta.size, chunk_size as u64 * 2);

    // Empty case
    let path = Path::from("test_empty_multipart");

    let id = multipart.create_multipart(&path).await.unwrap();

    let parts = vec![];

    multipart
        .complete_multipart(&path, &id, parts)
        .await
        .unwrap();

    let meta = storage.head(&path).await.unwrap();
    assert_eq!(meta.size, 0);
}

async fn delete_fixtures(storage: &DynObjectStore) {
    let paths = storage.list(None).map_ok(|meta| meta.location).boxed();
    storage
        .delete_stream(paths)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
}

/// Tests a race condition where 2 threads are performing multipart writes to the same path
pub async fn multipart_race_condition(storage: &dyn ObjectStore, last_writer_wins: bool) {
    let path = Path::from("test_multipart_race_condition");

    let mut multipart_upload_1 = storage.put_multipart(&path).await.unwrap();
    let mut multipart_upload_2 = storage.put_multipart(&path).await.unwrap();

    multipart_upload_1
        .put_part(Bytes::from(format!("1:{:0width$},", 0, width = 5_300_000)).into())
        .await
        .unwrap();
    multipart_upload_2
        .put_part(Bytes::from(format!("2:{:0width$},", 0, width = 5_300_000)).into())
        .await
        .unwrap();

    multipart_upload_2
        .put_part(Bytes::from(format!("2:{:0width$},", 1, width = 5_300_000)).into())
        .await
        .unwrap();
    multipart_upload_1
        .put_part(Bytes::from(format!("1:{:0width$},", 1, width = 5_300_000)).into())
        .await
        .unwrap();

    multipart_upload_1
        .put_part(Bytes::from(format!("1:{:0width$},", 2, width = 5_300_000)).into())
        .await
        .unwrap();
    multipart_upload_2
        .put_part(Bytes::from(format!("2:{:0width$},", 2, width = 5_300_000)).into())
        .await
        .unwrap();

    multipart_upload_2
        .put_part(Bytes::from(format!("2:{:0width$},", 3, width = 5_300_000)).into())
        .await
        .unwrap();
    multipart_upload_1
        .put_part(Bytes::from(format!("1:{:0width$},", 3, width = 5_300_000)).into())
        .await
        .unwrap();

    multipart_upload_1
        .put_part(Bytes::from(format!("1:{:0width$},", 4, width = 5_300_000)).into())
        .await
        .unwrap();
    multipart_upload_2
        .put_part(Bytes::from(format!("2:{:0width$},", 4, width = 5_300_000)).into())
        .await
        .unwrap();

    multipart_upload_1.complete().await.unwrap();

    if last_writer_wins {
        multipart_upload_2.complete().await.unwrap();
    } else {
        let err = multipart_upload_2.complete().await.unwrap_err();

        assert!(matches!(err, crate::Error::Generic { .. }), "{err}");
    }

    let get_result = storage.get(&path).await.unwrap();
    let bytes = get_result.bytes().await.unwrap();
    let string_contents = str::from_utf8(&bytes).unwrap();

    if last_writer_wins {
        assert!(string_contents.starts_with(
            format!(
                "2:{:0width$},2:{:0width$},2:{:0width$},2:{:0width$},2:{:0width$},",
                0, 1, 2, 3, 4,
                width = 5_300_000
            )
            .as_str()
        ));
    } else {
        assert!(string_contents.starts_with(
            format!(
                "1:{:0width$},1:{:0width$},1:{:0width$},1:{:0width$},1:{:0width$},",
                0, 1, 2, 3, 4,
                width = 5_300_000
            )
            .as_str()
        ));
    }
}

/// Tests performing out of order multipart uploads
pub async fn multipart_out_of_order(storage: &dyn ObjectStore) {
    let path = Path::from("test_multipart_out_of_order");
    let mut multipart_upload = storage.put_multipart(&path).await.unwrap();

    let part1 = std::iter::repeat(b'1')
        .take(5 * 1024 * 1024)
        .collect::<Bytes>();
    let part2 = std::iter::repeat(b'2')
        .take(5 * 1024 * 1024)
        .collect::<Bytes>();
    let part3 = std::iter::repeat(b'3')
        .take(5 * 1024 * 1024)
        .collect::<Bytes>();
    let full = [part1.as_ref(), part2.as_ref(), part3.as_ref()].concat();

    let fut1 = multipart_upload.put_part(part1.into());
    let fut2 = multipart_upload.put_part(part2.into());
    let fut3 = multipart_upload.put_part(part3.into());
    // note order is 2,3,1 , different than the parts were created in
    fut2.await.unwrap();
    fut3.await.unwrap();
    fut1.await.unwrap();

    multipart_upload.complete().await.unwrap();

    let result = storage.get(&path).await.unwrap();
    let bytes = result.bytes().await.unwrap();
    assert_eq!(bytes, full);
}
