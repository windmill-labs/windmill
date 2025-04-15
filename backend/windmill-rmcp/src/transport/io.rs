use std::marker::PhantomData;

// use crate::schema::*;
use futures::{Sink, SinkExt, Stream, StreamExt};
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::{
    bytes::{Buf, BufMut, BytesMut},
    codec::{Decoder, Encoder, FramedRead, FramedWrite},
};

use super::IntoTransport;
use crate::service::{RxJsonRpcMessage, ServiceRole, TxJsonRpcMessage};

#[cfg(feature = "transport-io")]
/// # StdIO Transport
///
/// Create a pair of [`tokio::io::Stdin`] and [`tokio::io::Stdout`].
pub fn stdio() -> (tokio::io::Stdin, tokio::io::Stdout) {
    (tokio::io::stdin(), tokio::io::stdout())
}

pub enum TransportAdapterAsyncRW {}

impl<Role, R, W> IntoTransport<Role, std::io::Error, TransportAdapterAsyncRW> for (R, W)
where
    Role: ServiceRole,
    R: AsyncRead + Send + 'static,
    W: AsyncWrite + Send + 'static,
{
    fn into_transport(
        self,
    ) -> (
        impl Sink<TxJsonRpcMessage<Role>, Error = std::io::Error> + Send + 'static,
        impl Stream<Item = RxJsonRpcMessage<Role>> + Send + 'static,
    ) {
        (from_async_write(self.1), from_async_read(self.0))
    }
}

pub enum TransportAdapterAsyncCombinedRW {}
impl<Role, S> IntoTransport<Role, std::io::Error, TransportAdapterAsyncCombinedRW> for S
where
    Role: ServiceRole,
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    fn into_transport(
        self,
    ) -> (
        impl Sink<TxJsonRpcMessage<Role>, Error = std::io::Error> + Send + 'static,
        impl Stream<Item = RxJsonRpcMessage<Role>> + Send + 'static,
    ) {
        IntoTransport::<Role, std::io::Error, TransportAdapterAsyncRW>::into_transport(
            tokio::io::split(self),
        )
    }
}

pub fn from_async_read<T: DeserializeOwned, R: AsyncRead>(reader: R) -> impl Stream<Item = T> {
    FramedRead::new(reader, JsonRpcMessageCodec::<T>::default()).filter_map(|result| {
        if let Err(e) = &result {
            tracing::error!("Error reading from stream: {}", e);
        }
        futures::future::ready(result.ok())
    })
}

pub fn from_async_write<T: Serialize, W: AsyncWrite>(
    writer: W,
) -> impl Sink<T, Error = std::io::Error> {
    FramedWrite::new(writer, JsonRpcMessageCodec::<T>::default()).sink_map_err(Into::into)
}

#[derive(Debug, Clone)]
pub struct JsonRpcMessageCodec<T> {
    _marker: PhantomData<fn() -> T>,
    next_index: usize,
    max_length: usize,
    is_discarding: bool,
}

impl<T> Default for JsonRpcMessageCodec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> JsonRpcMessageCodec<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
            next_index: 0,
            max_length: usize::MAX,
            is_discarding: false,
        }
    }

    pub fn new_with_max_length(max_length: usize) -> Self {
        Self {
            max_length,
            ..Self::new()
        }
    }

    pub fn max_length(&self) -> usize {
        self.max_length
    }
}

fn without_carriage_return(s: &[u8]) -> &[u8] {
    if let Some(&b'\r') = s.last() {
        &s[..s.len() - 1]
    } else {
        s
    }
}

#[derive(Debug, Error)]
pub enum JsonRpcMessageCodecError {
    #[error("max line length exceeded")]
    MaxLineLengthExceeded,
    #[error("serde error {0}")]
    Serde(#[from] serde_json::Error),
    #[error("io error {0}")]
    Io(#[from] std::io::Error),
}

impl From<JsonRpcMessageCodecError> for std::io::Error {
    fn from(value: JsonRpcMessageCodecError) -> Self {
        match value {
            JsonRpcMessageCodecError::MaxLineLengthExceeded => {
                std::io::Error::new(std::io::ErrorKind::InvalidData, value)
            }
            JsonRpcMessageCodecError::Serde(e) => e.into(),
            JsonRpcMessageCodecError::Io(e) => e,
        }
    }
}

impl<T: DeserializeOwned> Decoder for JsonRpcMessageCodec<T> {
    type Item = T;

    type Error = JsonRpcMessageCodecError;

    fn decode(
        &mut self,
        buf: &mut BytesMut,
    ) -> Result<Option<Self::Item>, JsonRpcMessageCodecError> {
        loop {
            // Determine how far into the buffer we'll search for a newline. If
            // there's no max_length set, we'll read to the end of the buffer.
            let read_to = std::cmp::min(self.max_length.saturating_add(1), buf.len());

            let newline_offset = buf[self.next_index..read_to]
                .iter()
                .position(|b| *b == b'\n');

            match (self.is_discarding, newline_offset) {
                (true, Some(offset)) => {
                    // If we found a newline, discard up to that offset and
                    // then stop discarding. On the next iteration, we'll try
                    // to read a line normally.
                    buf.advance(offset + self.next_index + 1);
                    self.is_discarding = false;
                    self.next_index = 0;
                }
                (true, None) => {
                    // Otherwise, we didn't find a newline, so we'll discard
                    // everything we read. On the next iteration, we'll continue
                    // discarding up to max_len bytes unless we find a newline.
                    buf.advance(read_to);
                    self.next_index = 0;
                    if buf.is_empty() {
                        return Ok(None);
                    }
                }
                (false, Some(offset)) => {
                    // Found a line!
                    let newline_index = offset + self.next_index;
                    self.next_index = 0;
                    let line = buf.split_to(newline_index + 1);
                    let line = &line[..line.len() - 1];
                    let line = without_carriage_return(line);
                    let item =
                        serde_json::from_slice(line).map_err(JsonRpcMessageCodecError::Serde)?;
                    return Ok(Some(item));
                }
                (false, None) if buf.len() > self.max_length => {
                    // Reached the maximum length without finding a
                    // newline, return an error and start discarding on the
                    // next call.
                    self.is_discarding = true;
                    return Err(JsonRpcMessageCodecError::MaxLineLengthExceeded);
                }
                (false, None) => {
                    // We didn't find a line or reach the length limit, so the next
                    // call will resume searching at the current offset.
                    self.next_index = read_to;
                    return Ok(None);
                }
            }
        }
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<T>, JsonRpcMessageCodecError> {
        Ok(match self.decode(buf)? {
            Some(frame) => Some(frame),
            None => {
                self.next_index = 0;
                // No terminating newline - return remaining data, if any
                if buf.is_empty() || buf == &b"\r"[..] {
                    None
                } else {
                    let line = buf.split_to(buf.len());
                    let line = without_carriage_return(&line);
                    let item =
                        serde_json::from_slice(line).map_err(JsonRpcMessageCodecError::Serde)?;
                    Some(item)
                }
            }
        })
    }
}

impl<T: Serialize> Encoder<T> for JsonRpcMessageCodec<T> {
    type Error = JsonRpcMessageCodecError;

    fn encode(&mut self, item: T, buf: &mut BytesMut) -> Result<(), JsonRpcMessageCodecError> {
        serde_json::to_writer(buf.writer(), &item)?;
        buf.put_u8(b'\n');
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    async fn test_decode() {
        use futures::StreamExt;
        use tokio::io::BufReader;

        let data = r#"{"jsonrpc":"2.0","method":"subtract","params":[42,23],"id":1}
    {"jsonrpc":"2.0","method":"subtract","params":[23,42],"id":2}
    {"jsonrpc":"2.0","method":"subtract","params":[42,23],"id":3}
    {"jsonrpc":"2.0","method":"subtract","params":[23,42],"id":4}
    {"jsonrpc":"2.0","method":"subtract","params":[42,23],"id":5}
    {"jsonrpc":"2.0","method":"subtract","params":[23,42],"id":6}
    {"jsonrpc":"2.0","method":"subtract","params":[42,23],"id":7}
    {"jsonrpc":"2.0","method":"subtract","params":[23,42],"id":8}
    {"jsonrpc":"2.0","method":"subtract","params":[42,23],"id":9}
    {"jsonrpc":"2.0","method":"subtract","params":[23,42],"id":10}
    
    "#;

        let mut cursor = BufReader::new(data.as_bytes());
        let mut stream = from_async_read::<serde_json::Value, _>(&mut cursor);

        for i in 1..=10 {
            let item = stream.next().await.unwrap();
            assert_eq!(
                item,
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "subtract",
                    "params": if i % 2 != 0 { [42, 23] } else { [23, 42] },
                    "id": i,
                })
            );
        }
    }

    #[tokio::test]
    async fn test_encode() {
        let test_messages = vec![
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "subtract",
                "params": [42, 23],
                "id": 1,
            }),
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "subtract",
                "params": [23, 42],
                "id": 2,
            }),
        ];

        // Create a buffer to write to
        let mut buffer = Vec::new();
        let mut writer = from_async_write(&mut buffer);

        // Write the test messages
        for message in test_messages.iter() {
            writer.send(message.clone()).await.unwrap();
        }
        writer.close().await.unwrap();
        drop(writer);
        // Parse the buffer back into lines and check each one
        let output = String::from_utf8_lossy(&buffer);
        let mut lines = output.lines();

        for expected_message in test_messages {
            let line = lines.next().unwrap();
            let parsed_message: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(parsed_message, expected_message);
        }

        // Make sure there are no extra lines
        assert!(lines.next().is_none());
    }
}
