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

use percent_encoding::{percent_encode, AsciiSet, CONTROLS};
use std::borrow::Cow;

use crate::path::DELIMITER_BYTE;

/// Error returned by [`PathPart::parse`]
#[derive(Debug, thiserror::Error)]
#[error(
    "Encountered illegal character sequence \"{}\" whilst parsing path segment \"{}\"",
    illegal,
    segment
)]
#[allow(missing_copy_implementations)]
pub struct InvalidPart {
    segment: String,
    illegal: String,
}

/// The PathPart type exists to validate the directory/file names that form part
/// of a path.
///
/// A [`PathPart`] is guaranteed to:
///
/// * Contain no ASCII control characters or `/`
/// * Not be a relative path segment, i.e. `.` or `..`
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct PathPart<'a> {
    pub(super) raw: Cow<'a, str>,
}

impl<'a> PathPart<'a> {
    /// Parse the provided path segment as a [`PathPart`] returning an error if invalid
    pub fn parse(segment: &'a str) -> Result<Self, InvalidPart> {
        if segment == "." || segment == ".." {
            return Err(InvalidPart {
                segment: segment.to_string(),
                illegal: segment.to_string(),
            });
        }

        for c in segment.chars() {
            if c.is_ascii_control() || c == '/' {
                return Err(InvalidPart {
                    segment: segment.to_string(),
                    // This is correct as only single byte characters up to this point
                    illegal: c.to_string(),
                });
            }
        }

        Ok(Self {
            raw: segment.into(),
        })
    }
}

/// Characters we want to encode.
const INVALID: &AsciiSet = &CONTROLS
    // The delimiter we are reserving for internal hierarchy
    .add(DELIMITER_BYTE)
    // Characters AWS recommends avoiding for object keys
    // https://docs.aws.amazon.com/AmazonS3/latest/dev/UsingMetadata.html
    .add(b'\\')
    .add(b'{')
    .add(b'^')
    .add(b'}')
    .add(b'%')
    .add(b'`')
    .add(b']')
    .add(b'"') // " <-- my editor is confused about double quotes within single quotes
    .add(b'>')
    .add(b'[')
    .add(b'~')
    .add(b'<')
    .add(b'#')
    .add(b'|')
    // Characters Google Cloud Storage recommends avoiding for object names
    // https://cloud.google.com/storage/docs/naming-objects
    .add(b'\r')
    .add(b'\n')
    .add(b'*')
    .add(b'?');

impl<'a> From<&'a [u8]> for PathPart<'a> {
    fn from(v: &'a [u8]) -> Self {
        let inner = match v {
            // We don't want to encode `.` generally, but we do want to disallow parts of paths
            // to be equal to `.` or `..` to prevent file system traversal shenanigans.
            b"." => "%2E".into(),
            b".." => "%2E%2E".into(),
            other => percent_encode(other, INVALID).into(),
        };
        Self { raw: inner }
    }
}

impl<'a> From<&'a str> for PathPart<'a> {
    fn from(v: &'a str) -> Self {
        Self::from(v.as_bytes())
    }
}

impl From<String> for PathPart<'static> {
    fn from(s: String) -> Self {
        Self {
            raw: Cow::Owned(PathPart::from(s.as_str()).raw.into_owned()),
        }
    }
}

impl AsRef<str> for PathPart<'_> {
    fn as_ref(&self) -> &str {
        self.raw.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_part_delimiter_gets_encoded() {
        let part: PathPart<'_> = "foo/bar".into();
        assert_eq!(part.raw, "foo%2Fbar");
    }

    #[test]
    fn path_part_given_already_encoded_string() {
        let part: PathPart<'_> = "foo%2Fbar".into();
        assert_eq!(part.raw, "foo%252Fbar");
    }

    #[test]
    fn path_part_cant_be_one_dot() {
        let part: PathPart<'_> = ".".into();
        assert_eq!(part.raw, "%2E");
    }

    #[test]
    fn path_part_cant_be_two_dots() {
        let part: PathPart<'_> = "..".into();
        assert_eq!(part.raw, "%2E%2E");
    }

    #[test]
    fn path_part_parse() {
        PathPart::parse("foo").unwrap();
        PathPart::parse("foo/bar").unwrap_err();

        // Test percent-encoded path
        PathPart::parse("foo%2Fbar").unwrap();
        PathPart::parse("L%3ABC.parquet").unwrap();

        // Test path containing bad escape sequence
        PathPart::parse("%Z").unwrap();
        PathPart::parse("%%").unwrap();
    }
}
