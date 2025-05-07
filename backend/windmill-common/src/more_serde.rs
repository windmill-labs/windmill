/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! helpers for serde + serde derive attributes

use crate::{error::to_anyhow, utils::rd_string};
use futures::TryStreamExt;
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;
use std::{fmt::Display, str::FromStr};
use tokio::task::{self, JoinHandle};
use tokio_stream::StreamExt;

pub fn default_true() -> bool {
    true
}

pub fn default_false() -> bool {
    false
}

pub fn default_null() -> Box<RawValue> {
    RawValue::from_string("null".to_string()).unwrap()
}

pub fn default_empty_string() -> String {
    String::new()
}

pub fn default_id() -> String {
    rd_string(6)
}

pub fn is_default<T: Default + std::cmp::PartialEq>(t: &T) -> bool {
    &T::default() == t
}

pub fn maybe_number_opt<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + serde::Deserialize<'de>,
    <T as FromStr>::Err: Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumericOrNull<'a, T> {
        String(String),
        Str(&'a str),
        RawT(T),
        Null,
    }

    match NumericOrNull::<T>::deserialize(deserializer)? {
        NumericOrNull::String(s) => match s.as_str() {
            "" => Ok(None),
            _ => T::from_str(&s).map(Some).map_err(serde::de::Error::custom),
        },
        NumericOrNull::Str(s) => match s {
            "" => Ok(None),
            _ => T::from_str(s).map(Some).map_err(serde::de::Error::custom),
        },
        NumericOrNull::RawT(i) => Ok(Some(i)),
        NumericOrNull::Null => Ok(None),
    }
}

struct SerdeArrMpscDeserializer<'a> {
    sender: &'a tokio::sync::mpsc::Sender<serde_json::Value>,
}

// This visitor takes a JSON array and sends each element to a channel.
// It doesn't hold the entire array in memory at once but rather
// sends each element to the channel as it is deserialized.
impl<'de, 'a> serde::de::DeserializeSeed<'de> for SerdeArrMpscDeserializer<'a> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SeqVisitor<'a> {
            sender: &'a tokio::sync::mpsc::Sender<serde_json::Value>,
        }

        impl<'de, 'a> serde::de::Visitor<'de> for SeqVisitor<'a> {
            type Value = ();

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of rows")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                while let Some(elem) = seq.next_element::<serde_json::Value>()? {
                    // Push each item into the queue
                    self.sender.blocking_send(elem).map_err(|e| {
                        serde::de::Error::custom(format!("Queue send failed: {}", e))
                    })?;
                }

                Ok(())
            }
        }

        deserializer.deserialize_seq(SeqVisitor { sender: &self.sender })
    }
}

// Takes in a stream of a json array and returns a stream of each value in the array.
//
// Main reason for this is that we need to tranform a huge json (from bigquery)
// into csv or parquet format. But this requires parsing the json, which may
// be too big to fit in memory.
pub async fn json_stream_arr_values<S, T, E: Display>(
    mut stream: S,
) -> anyhow::Result<impl StreamExt<Item = serde_json::Value>>
where
    S: TryStreamExt<Item = Result<T, E>> + Send + Unpin + 'static,
    bytes::Bytes: From<T>,
{
    const MAX_MPSC_SIZE: usize = 1000;

    use bytes::Bytes;
    use serde::de::DeserializeSeed;
    use std::path::PathBuf;
    use tokio::io::AsyncWriteExt;

    let tmp_filename = format!("tmp_json_stream_{}", rd_string(8));

    // Start by writing the async stream (from network) to a file.
    let mut path = PathBuf::from(std::env::temp_dir());
    path.push(tmp_filename);
    let mut file: tokio::fs::File = tokio::fs::File::create(&path).await.map_err(to_anyhow)?;
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                std::fs::remove_file(&path)?;
                return Err(anyhow::anyhow!("Error reading stream: {}", e));
            }
        };
        let b: Bytes = chunk.into();
        file.write_all(&b).await?;
    }

    let (tx, rx) = tokio::sync::mpsc::channel(MAX_MPSC_SIZE);

    // Then we read the file and pipe each element to the channel in a blocking task.
    let _: JoinHandle<anyhow::Result<()>> = task::spawn_blocking(move || {
        let sync_file = std::fs::File::open(&path).map_err(to_anyhow)?;
        let mut buf_reader = std::io::BufReader::new(sync_file);

        let mut deserializer = serde_json::Deserializer::from_reader(&mut buf_reader);
        // This deserializer will read the file and send each row to the channel
        let () = SerdeArrMpscDeserializer { sender: &tx }.deserialize(&mut deserializer)?;

        drop(tx); // Drops out of scope anyway but important to signal the end of the stream
        std::fs::remove_file(&path)?;
        Ok(())
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    Ok(stream)
}
