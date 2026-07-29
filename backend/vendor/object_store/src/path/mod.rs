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

//! Path abstraction for Object Storage

use itertools::Itertools;
use percent_encoding::percent_decode;
use std::fmt::Formatter;
#[cfg(not(target_arch = "wasm32"))]
use url::Url;

/// The delimiter to separate object namespaces, creating a directory structure.
pub const DELIMITER: &str = "/";

/// The path delimiter as a single byte
pub const DELIMITER_BYTE: u8 = DELIMITER.as_bytes()[0];

mod parts;

pub use parts::{InvalidPart, PathPart};

/// Error returned by [`Path::parse`]
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Error when there's an empty segment between two slashes `/` in the path
    #[error("Path \"{}\" contained empty path segment", path)]
    EmptySegment {
        /// The source path
        path: String,
    },

    /// Error when an invalid segment is encountered in the given path
    #[error("Error parsing Path \"{}\": {}", path, source)]
    BadSegment {
        /// The source path
        path: String,
        /// The part containing the error
        source: InvalidPart,
    },

    /// Error when path cannot be canonicalized
    #[error("Failed to canonicalize path \"{}\": {}", path.display(), source)]
    Canonicalize {
        /// The source path
        path: std::path::PathBuf,
        /// The underlying error
        source: std::io::Error,
    },

    /// Error when the path is not a valid URL
    #[error("Unable to convert path \"{}\" to URL", path.display())]
    InvalidPath {
        /// The source path
        path: std::path::PathBuf,
    },

    /// Error when a path contains non-unicode characters
    #[error("Path \"{}\" contained non-unicode characters: {}", path, source)]
    NonUnicode {
        /// The source path
        path: String,
        /// The underlying `UTF8Error`
        source: std::str::Utf8Error,
    },

    /// Error when the a path doesn't start with given prefix
    #[error("Path {} does not start with prefix {}", path, prefix)]
    PrefixMismatch {
        /// The source path
        path: String,
        /// The mismatched prefix
        prefix: String,
    },
}

/// A parsed path representation that can be safely written to object storage
///
/// A [`Path`] maintains the following invariants:
///
/// * Paths are delimited by `/`
/// * Paths do not contain leading or trailing `/`
/// * Paths do not contain relative path segments, i.e. `.` or `..`
/// * Paths do not contain empty path segments
/// * Paths do not contain any ASCII control characters
///
/// There are no enforced restrictions on path length, however, it should be noted that most
/// object stores do not permit paths longer than 1024 bytes, and many filesystems do not
/// support path segments longer than 255 bytes.
///
/// # Encode
///
/// In theory object stores support any UTF-8 character sequence, however, certain character
/// sequences cause compatibility problems with some applications and protocols. Additionally
/// some filesystems may impose character restrictions, see [`LocalFileSystem`]. As such the
/// naming guidelines for [S3], [GCS] and [Azure Blob Storage] all recommend sticking to a
/// limited character subset.
///
/// [S3]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/object-keys.html
/// [GCS]: https://cloud.google.com/storage/docs/naming-objects
/// [Azure Blob Storage]: https://docs.microsoft.com/en-us/rest/api/storageservices/Naming-and-Referencing-Containers--Blobs--and-Metadata#blob-names
///
/// A string containing potentially problematic path segments can therefore be encoded to a [`Path`]
/// using [`Path::from`] or [`Path::from_iter`]. This will percent encode any problematic
/// segments according to [RFC 1738].
///
/// ```
/// # use object_store::path::Path;
/// assert_eq!(Path::from("foo/bar").as_ref(), "foo/bar");
/// assert_eq!(Path::from("foo//bar").as_ref(), "foo/bar");
/// assert_eq!(Path::from("foo/../bar").as_ref(), "foo/%2E%2E/bar");
/// assert_eq!(Path::from("/").as_ref(), "");
/// assert_eq!(Path::from_iter(["foo", "foo/bar"]).as_ref(), "foo/foo%2Fbar");
/// ```
///
/// Note: if provided with an already percent encoded string, this will encode it again
///
/// ```
/// # use object_store::path::Path;
/// assert_eq!(Path::from("foo/foo%2Fbar").as_ref(), "foo/foo%252Fbar");
/// ```
///
/// # Parse
///
/// Alternatively a [`Path`] can be parsed from an existing string, returning an
/// error if it is invalid. Unlike the encoding methods above, this will permit
/// arbitrary unicode, including percent encoded sequences.
///
/// ```
/// # use object_store::path::Path;
/// assert_eq!(Path::parse("/foo/foo%2Fbar").unwrap().as_ref(), "foo/foo%2Fbar");
/// Path::parse("..").unwrap_err(); // Relative path segments are disallowed
/// Path::parse("/foo//").unwrap_err(); // Empty path segments are disallowed
/// Path::parse("\x00").unwrap_err(); // ASCII control characters are disallowed
/// ```
///
/// [RFC 1738]: https://www.ietf.org/rfc/rfc1738.txt
/// [`LocalFileSystem`]: crate::local::LocalFileSystem
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Path {
    /// The raw path with no leading or trailing delimiters
    raw: String,
}

impl Path {
    /// Parse a string as a [`Path`], returning a [`Error`] if invalid,
    /// as defined on the docstring for [`Path`]
    ///
    /// Note: this will strip any leading `/` or trailing `/`
    pub fn parse(path: impl AsRef<str>) -> Result<Self, Error> {
        let path = path.as_ref();

        let stripped = path.strip_prefix(DELIMITER).unwrap_or(path);
        if stripped.is_empty() {
            return Ok(Default::default());
        }

        let stripped = stripped.strip_suffix(DELIMITER).unwrap_or(stripped);

        for segment in stripped.split(DELIMITER) {
            if segment.is_empty() {
                return Err(Error::EmptySegment { path: path.into() });
            }

            PathPart::parse(segment).map_err(|source| {
                let path = path.into();
                Error::BadSegment { source, path }
            })?;
        }

        Ok(Self {
            raw: stripped.to_string(),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Convert a filesystem path to a [`Path`] relative to the filesystem root
    ///
    /// This will return an error if the path contains illegal character sequences
    /// as defined on the docstring for [`Path`] or does not exist
    ///
    /// Note: this will canonicalize the provided path, resolving any symlinks
    pub fn from_filesystem_path(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let absolute = std::fs::canonicalize(&path).map_err(|source| {
            let path = path.as_ref().into();
            Error::Canonicalize { source, path }
        })?;

        Self::from_absolute_path(absolute)
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Convert an absolute filesystem path to a [`Path`] relative to the filesystem root
    ///
    /// This will return an error if the path contains illegal character sequences,
    /// as defined on the docstring for [`Path`], or `base` is not an absolute path
    pub fn from_absolute_path(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        Self::from_absolute_path_with_base(path, None)
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Convert a filesystem path to a [`Path`] relative to the provided base
    ///
    /// This will return an error if the path contains illegal character sequences,
    /// as defined on the docstring for [`Path`], or `base` does not refer to a parent
    /// path of `path`, or `base` is not an absolute path
    pub(crate) fn from_absolute_path_with_base(
        path: impl AsRef<std::path::Path>,
        base: Option<&Url>,
    ) -> Result<Self, Error> {
        let url = absolute_path_to_url(path)?;
        let path = match base {
            Some(prefix) => {
                url.path()
                    .strip_prefix(prefix.path())
                    .ok_or_else(|| Error::PrefixMismatch {
                        path: url.path().to_string(),
                        prefix: prefix.to_string(),
                    })?
            }
            None => url.path(),
        };

        // Reverse any percent encoding performed by conversion to URL
        Self::from_url_path(path)
    }

    /// Parse a url encoded string as a [`Path`], returning a [`Error`] if invalid
    ///
    /// This will return an error if the path contains illegal character sequences
    /// as defined on the docstring for [`Path`]
    pub fn from_url_path(path: impl AsRef<str>) -> Result<Self, Error> {
        let path = path.as_ref();
        let decoded = percent_decode(path.as_bytes())
            .decode_utf8()
            .map_err(|source| {
                let path = path.into();
                Error::NonUnicode { source, path }
            })?;

        Self::parse(decoded)
    }

    /// Returns the [`PathPart`] of this [`Path`]
    pub fn parts(&self) -> impl Iterator<Item = PathPart<'_>> {
        self.raw
            .split_terminator(DELIMITER)
            .map(|s| PathPart { raw: s.into() })
    }

    /// Returns the last path segment containing the filename stored in this [`Path`]
    pub fn filename(&self) -> Option<&str> {
        match self.raw.is_empty() {
            true => None,
            false => self.raw.rsplit(DELIMITER).next(),
        }
    }

    /// Returns the extension of the file stored in this [`Path`], if any
    pub fn extension(&self) -> Option<&str> {
        self.filename()
            .and_then(|f| f.rsplit_once('.'))
            .and_then(|(_, extension)| {
                if extension.is_empty() {
                    None
                } else {
                    Some(extension)
                }
            })
    }

    /// Returns an iterator of the [`PathPart`] of this [`Path`] after `prefix`
    ///
    /// Returns `None` if the prefix does not match
    pub fn prefix_match(&self, prefix: &Self) -> Option<impl Iterator<Item = PathPart<'_>> + '_> {
        let mut stripped = self.raw.strip_prefix(&prefix.raw)?;
        if !stripped.is_empty() && !prefix.raw.is_empty() {
            stripped = stripped.strip_prefix(DELIMITER)?;
        }
        let iter = stripped
            .split_terminator(DELIMITER)
            .map(|x| PathPart { raw: x.into() });
        Some(iter)
    }

    /// Returns true if this [`Path`] starts with `prefix`
    pub fn prefix_matches(&self, prefix: &Self) -> bool {
        self.prefix_match(prefix).is_some()
    }

    /// Creates a new child of this [`Path`]
    pub fn child<'a>(&self, child: impl Into<PathPart<'a>>) -> Self {
        let raw = match self.raw.is_empty() {
            true => format!("{}", child.into().raw),
            false => format!("{}{}{}", self.raw, DELIMITER, child.into().raw),
        };

        Self { raw }
    }
}

impl AsRef<str> for Path {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

impl From<&str> for Path {
    fn from(path: &str) -> Self {
        Self::from_iter(path.split(DELIMITER))
    }
}

impl From<String> for Path {
    fn from(path: String) -> Self {
        Self::from_iter(path.split(DELIMITER))
    }
}

impl From<Path> for String {
    fn from(path: Path) -> Self {
        path.raw
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.raw.fmt(f)
    }
}

impl<'a, I> FromIterator<I> for Path
where
    I: Into<PathPart<'a>>,
{
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        let raw = T::into_iter(iter)
            .map(|s| s.into())
            .filter(|s| !s.raw.is_empty())
            .map(|s| s.raw)
            .join(DELIMITER);

        Self { raw }
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// Given an absolute filesystem path convert it to a URL representation without canonicalization
pub(crate) fn absolute_path_to_url(path: impl AsRef<std::path::Path>) -> Result<Url, Error> {
    Url::from_file_path(&path).map_err(|_| Error::InvalidPath {
        path: path.as_ref().into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_prefix_with_trailing_delimiter() {
        // Use case: files exist in object storage named `foo/bar.json` and
        // `foo_test.json`. A search for the prefix `foo/` should return
        // `foo/bar.json` but not `foo_test.json'.
        let prefix = Path::from_iter(["test"]);
        assert_eq!(prefix.as_ref(), "test");
    }

    #[test]
    fn push_encodes() {
        let location = Path::from_iter(["foo/bar", "baz%2Ftest"]);
        assert_eq!(location.as_ref(), "foo%2Fbar/baz%252Ftest");
    }

    #[test]
    fn test_parse() {
        assert_eq!(Path::parse("/").unwrap().as_ref(), "");
        assert_eq!(Path::parse("").unwrap().as_ref(), "");

        let err = Path::parse("//").unwrap_err();
        assert!(matches!(err, Error::EmptySegment { .. }));

        assert_eq!(Path::parse("/foo/bar/").unwrap().as_ref(), "foo/bar");
        assert_eq!(Path::parse("foo/bar/").unwrap().as_ref(), "foo/bar");
        assert_eq!(Path::parse("foo/bar").unwrap().as_ref(), "foo/bar");

        let err = Path::parse("foo///bar").unwrap_err();
        assert!(matches!(err, Error::EmptySegment { .. }));
    }

    #[test]
    fn convert_raw_before_partial_eq() {
        // dir and file_name
        let cloud = Path::from("test_dir/test_file.json");
        let built = Path::from_iter(["test_dir", "test_file.json"]);

        assert_eq!(built, cloud);

        // dir and file_name w/o dot
        let cloud = Path::from("test_dir/test_file");
        let built = Path::from_iter(["test_dir", "test_file"]);

        assert_eq!(built, cloud);

        // dir, no file
        let cloud = Path::from("test_dir/");
        let built = Path::from_iter(["test_dir"]);
        assert_eq!(built, cloud);

        // file_name, no dir
        let cloud = Path::from("test_file.json");
        let built = Path::from_iter(["test_file.json"]);
        assert_eq!(built, cloud);

        // empty
        let cloud = Path::from("");
        let built = Path::from_iter(["", ""]);

        assert_eq!(built, cloud);
    }

    #[test]
    fn parts_after_prefix_behavior() {
        let existing_path = Path::from("apple/bear/cow/dog/egg.json");

        // Prefix with one directory
        let prefix = Path::from("apple");
        let expected_parts: Vec<PathPart<'_>> = vec!["bear", "cow", "dog", "egg.json"]
            .into_iter()
            .map(Into::into)
            .collect();
        let parts: Vec<_> = existing_path.prefix_match(&prefix).unwrap().collect();
        assert_eq!(parts, expected_parts);

        // Prefix with two directories
        let prefix = Path::from("apple/bear");
        let expected_parts: Vec<PathPart<'_>> = vec!["cow", "dog", "egg.json"]
            .into_iter()
            .map(Into::into)
            .collect();
        let parts: Vec<_> = existing_path.prefix_match(&prefix).unwrap().collect();
        assert_eq!(parts, expected_parts);

        // Not a prefix
        let prefix = Path::from("cow");
        assert!(existing_path.prefix_match(&prefix).is_none());

        // Prefix with a partial directory
        let prefix = Path::from("ap");
        assert!(existing_path.prefix_match(&prefix).is_none());

        // Prefix matches but there aren't any parts after it
        let existing = Path::from("apple/bear/cow/dog");

        assert_eq!(existing.prefix_match(&existing).unwrap().count(), 0);
        assert_eq!(Path::default().parts().count(), 0);
    }

    #[test]
    fn prefix_matches() {
        let haystack = Path::from_iter(["foo/bar", "baz%2Ftest", "something"]);
        // self starts with self
        assert!(
            haystack.prefix_matches(&haystack),
            "{haystack:?} should have started with {haystack:?}"
        );

        // a longer prefix doesn't match
        let needle = haystack.child("longer now");
        assert!(
            !haystack.prefix_matches(&needle),
            "{haystack:?} shouldn't have started with {needle:?}"
        );

        // one dir prefix matches
        let needle = Path::from_iter(["foo/bar"]);
        assert!(
            haystack.prefix_matches(&needle),
            "{haystack:?} should have started with {needle:?}"
        );

        // two dir prefix matches
        let needle = needle.child("baz%2Ftest");
        assert!(
            haystack.prefix_matches(&needle),
            "{haystack:?} should have started with {needle:?}"
        );

        // partial dir prefix doesn't match
        let needle = Path::from_iter(["f"]);
        assert!(
            !haystack.prefix_matches(&needle),
            "{haystack:?} should not have started with {needle:?}"
        );

        // one dir and one partial dir doesn't match
        let needle = Path::from_iter(["foo/bar", "baz"]);
        assert!(
            !haystack.prefix_matches(&needle),
            "{haystack:?} should not have started with {needle:?}"
        );

        // empty prefix matches
        let needle = Path::from("");
        assert!(
            haystack.prefix_matches(&needle),
            "{haystack:?} should have started with {needle:?}"
        );
    }

    #[test]
    fn prefix_matches_with_file_name() {
        let haystack = Path::from_iter(["foo/bar", "baz%2Ftest", "something", "foo.segment"]);

        // All directories match and file name is a prefix
        let needle = Path::from_iter(["foo/bar", "baz%2Ftest", "something", "foo"]);

        assert!(
            !haystack.prefix_matches(&needle),
            "{haystack:?} should not have started with {needle:?}"
        );

        // All directories match but file name is not a prefix
        let needle = Path::from_iter(["foo/bar", "baz%2Ftest", "something", "e"]);

        assert!(
            !haystack.prefix_matches(&needle),
            "{haystack:?} should not have started with {needle:?}"
        );

        // Not all directories match; file name is a prefix of the next directory; this
        // does not match
        let needle = Path::from_iter(["foo/bar", "baz%2Ftest", "s"]);

        assert!(
            !haystack.prefix_matches(&needle),
            "{haystack:?} should not have started with {needle:?}"
        );

        // Not all directories match; file name is NOT a prefix of the next directory;
        // no match
        let needle = Path::from_iter(["foo/bar", "baz%2Ftest", "p"]);

        assert!(
            !haystack.prefix_matches(&needle),
            "{haystack:?} should not have started with {needle:?}"
        );
    }

    #[test]
    fn path_containing_spaces() {
        let a = Path::from_iter(["foo bar", "baz"]);
        let b = Path::from("foo bar/baz");
        let c = Path::parse("foo bar/baz").unwrap();

        assert_eq!(a.raw, "foo bar/baz");
        assert_eq!(a.raw, b.raw);
        assert_eq!(b.raw, c.raw);
    }

    #[test]
    fn from_url_path() {
        let a = Path::from_url_path("foo%20bar").unwrap();
        let b = Path::from_url_path("foo/%2E%2E/bar").unwrap_err();
        let c = Path::from_url_path("foo%2F%252E%252E%2Fbar").unwrap();
        let d = Path::from_url_path("foo/%252E%252E/bar").unwrap();
        let e = Path::from_url_path("%48%45%4C%4C%4F").unwrap();
        let f = Path::from_url_path("foo/%FF/as").unwrap_err();

        assert_eq!(a.raw, "foo bar");
        assert!(matches!(b, Error::BadSegment { .. }));
        assert_eq!(c.raw, "foo/%2E%2E/bar");
        assert_eq!(d.raw, "foo/%2E%2E/bar");
        assert_eq!(e.raw, "HELLO");
        assert!(matches!(f, Error::NonUnicode { .. }));
    }

    #[test]
    fn filename_from_path() {
        let a = Path::from("foo/bar");
        let b = Path::from("foo/bar.baz");
        let c = Path::from("foo.bar/baz");

        assert_eq!(a.filename(), Some("bar"));
        assert_eq!(b.filename(), Some("bar.baz"));
        assert_eq!(c.filename(), Some("baz"));
    }

    #[test]
    fn file_extension() {
        let a = Path::from("foo/bar");
        let b = Path::from("foo/bar.baz");
        let c = Path::from("foo.bar/baz");
        let d = Path::from("foo.bar/baz.qux");

        assert_eq!(a.extension(), None);
        assert_eq!(b.extension(), Some("baz"));
        assert_eq!(c.extension(), None);
        assert_eq!(d.extension(), Some("qux"));
    }
}
