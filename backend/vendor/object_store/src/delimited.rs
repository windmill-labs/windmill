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

//! Utility for streaming newline delimited files from object storage

use std::collections::VecDeque;

use bytes::Bytes;
use futures::{Stream, StreamExt};

use super::Result;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("encountered unterminated string")]
    UnterminatedString,

    #[error("encountered trailing escape character")]
    TrailingEscape,
}

impl From<Error> for super::Error {
    fn from(err: Error) -> Self {
        Self::Generic {
            store: "LineDelimiter",
            source: Box::new(err),
        }
    }
}

/// The ASCII encoding of `"`
const QUOTE: u8 = b'"';

/// The ASCII encoding of `\n`
const NEWLINE: u8 = b'\n';

/// The ASCII encoding of `\`
const ESCAPE: u8 = b'\\';

/// [`LineDelimiter`] is provided with a stream of [`Bytes`] and returns an iterator
/// of [`Bytes`] containing a whole number of new line delimited records
#[derive(Debug, Default)]
struct LineDelimiter {
    /// Complete chunks of [`Bytes`]
    complete: VecDeque<Bytes>,
    /// Remainder bytes that form the next record
    remainder: Vec<u8>,
    /// True if the last character was the escape character
    is_escape: bool,
    /// True if currently processing a quoted string
    is_quote: bool,
}

impl LineDelimiter {
    /// Creates a new [`LineDelimiter`] with the provided delimiter
    fn new() -> Self {
        Self::default()
    }

    /// Adds the next set of [`Bytes`]
    fn push(&mut self, val: impl Into<Bytes>) {
        let val: Bytes = val.into();

        let is_escape = &mut self.is_escape;
        let is_quote = &mut self.is_quote;
        let mut record_ends = val.iter().enumerate().filter_map(|(idx, v)| {
            if *is_escape {
                *is_escape = false;
                None
            } else if *v == ESCAPE {
                *is_escape = true;
                None
            } else if *v == QUOTE {
                *is_quote = !*is_quote;
                None
            } else if *is_quote {
                None
            } else {
                (*v == NEWLINE).then_some(idx + 1)
            }
        });

        let start_offset = match self.remainder.is_empty() {
            true => 0,
            false => match record_ends.next() {
                Some(idx) => {
                    self.remainder.extend_from_slice(&val[0..idx]);
                    self.complete
                        .push_back(Bytes::from(std::mem::take(&mut self.remainder)));
                    idx
                }
                None => {
                    self.remainder.extend_from_slice(&val);
                    return;
                }
            },
        };
        let end_offset = record_ends.next_back().unwrap_or(start_offset);
        if start_offset != end_offset {
            self.complete.push_back(val.slice(start_offset..end_offset));
        }

        if end_offset != val.len() {
            self.remainder.extend_from_slice(&val[end_offset..])
        }
    }

    /// Marks the end of the stream, delimiting any remaining bytes
    ///
    /// Returns `true` if there is no remaining data to be read
    fn finish(&mut self) -> Result<bool> {
        if !self.remainder.is_empty() {
            if self.is_quote {
                Err(Error::UnterminatedString)?;
            }
            if self.is_escape {
                Err(Error::TrailingEscape)?;
            }

            self.complete
                .push_back(Bytes::from(std::mem::take(&mut self.remainder)))
        }
        Ok(self.complete.is_empty())
    }
}

impl Iterator for LineDelimiter {
    type Item = Bytes;

    fn next(&mut self) -> Option<Self::Item> {
        self.complete.pop_front()
    }
}

/// Given a [`Stream`] of [`Bytes`] returns a [`Stream`] where each
/// yielded [`Bytes`] contains a whole number of new line delimited records
/// accounting for `\` style escapes and `"` quotes
pub fn newline_delimited_stream<S>(s: S) -> impl Stream<Item = Result<Bytes>>
where
    S: Stream<Item = Result<Bytes>> + Unpin,
{
    let delimiter = LineDelimiter::new();

    futures::stream::unfold(
        (s, delimiter, false),
        |(mut s, mut delimiter, mut exhausted)| async move {
            loop {
                if let Some(next) = delimiter.next() {
                    return Some((Ok(next), (s, delimiter, exhausted)));
                } else if exhausted {
                    return None;
                }

                match s.next().await {
                    Some(Ok(bytes)) => delimiter.push(bytes),
                    Some(Err(e)) => return Some((Err(e), (s, delimiter, exhausted))),
                    None => {
                        exhausted = true;
                        match delimiter.finish() {
                            Ok(true) => return None,
                            Ok(false) => continue,
                            Err(e) => return Some((Err(e), (s, delimiter, exhausted))),
                        }
                    }
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use futures::stream::{BoxStream, TryStreamExt};

    use super::*;

    #[test]
    fn test_delimiter() {
        let mut delimiter = LineDelimiter::new();
        delimiter.push("hello\nworld");
        delimiter.push("\n\n");

        assert_eq!(delimiter.next().unwrap(), Bytes::from("hello\n"));
        assert_eq!(delimiter.next().unwrap(), Bytes::from("world\n"));
        assert_eq!(delimiter.next().unwrap(), Bytes::from("\n"));
        assert!(delimiter.next().is_none());
    }

    #[test]
    fn test_delimiter_escaped() {
        let mut delimiter = LineDelimiter::new();
        delimiter.push("");
        delimiter.push("fo\\\n\"foo");
        delimiter.push("bo\n\"bar\n");
        delimiter.push("\"he");
        delimiter.push("llo\"\n");
        assert_eq!(
            delimiter.next().unwrap(),
            Bytes::from("fo\\\n\"foobo\n\"bar\n")
        );
        assert_eq!(delimiter.next().unwrap(), Bytes::from("\"hello\"\n"));
        assert!(delimiter.next().is_none());

        // Verify can push further data
        delimiter.push("\"foo\nbar\",\"fiz\\\"inner\\\"\"\nhello");
        assert!(!delimiter.finish().unwrap());

        assert_eq!(
            delimiter.next().unwrap(),
            Bytes::from("\"foo\nbar\",\"fiz\\\"inner\\\"\"\n")
        );
        assert_eq!(delimiter.next().unwrap(), Bytes::from("hello"));
        assert!(delimiter.finish().unwrap());
        assert!(delimiter.next().is_none());
    }

    #[tokio::test]
    async fn test_delimiter_stream() {
        let input = vec!["hello\nworld\nbin", "go\ncup", "cakes"];
        let input_stream = futures::stream::iter(input.into_iter().map(|s| Ok(Bytes::from(s))));
        let stream = newline_delimited_stream(input_stream);

        let results: Vec<_> = stream.try_collect().await.unwrap();
        assert_eq!(
            results,
            vec![
                Bytes::from("hello\nworld\n"),
                Bytes::from("bingo\n"),
                Bytes::from("cupcakes")
            ]
        )
    }
    #[tokio::test]
    async fn test_delimiter_unfold_stream() {
        let input_stream: BoxStream<'static, Result<Bytes>> = futures::stream::unfold(
            VecDeque::from(["hello\nworld\nbin", "go\ncup", "cakes"]),
            |mut input| async move {
                if !input.is_empty() {
                    Some((Ok(Bytes::from(input.pop_front().unwrap())), input))
                } else {
                    None
                }
            },
        )
        .boxed();
        let stream = newline_delimited_stream(input_stream);

        let results: Vec<_> = stream.try_collect().await.unwrap();
        assert_eq!(
            results,
            vec![
                Bytes::from("hello\nworld\n"),
                Bytes::from("bingo\n"),
                Bytes::from("cupcakes")
            ]
        )
    }
}
