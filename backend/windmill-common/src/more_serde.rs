/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! helpers for serde + serde derive attributes

use crate::{
    error::{self, to_anyhow},
    utils::rd_string,
};
use bytes::Bytes;
use futures::TryStreamExt;
use serde::{de::DeserializeSeed, Deserialize, Deserializer};
use serde_json::{value::RawValue, Value};
use std::{fmt::Display, str::FromStr};
use tokio::{
    sync::mpsc::Sender,
    task::{self},
};
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

// Takes a json stream and returns a stream of json values, without loading the
// entire input into memory.
pub async fn json_stream_values<
    'a,
    D: DeserializeSeed<'a> + 'static + Send,
    F: FnOnce(Sender<Value>) -> D,
    E: Display,
>(
    mut stream: impl TryStreamExt<Item = Result<Bytes, E>> + Send + Unpin + 'static,
    mpsc_deserializer_factory: F,
) -> error::Result<impl StreamExt<Item = serde_json::Value>> {
    const MAX_MPSC_SIZE: usize = 1000;

    use std::path::PathBuf;
    use tokio::io::AsyncWriteExt;

    let tmp_filename = format!("tmp_json_stream_{}", rd_string(8));

    // Start by writing the async stream (from network) to a file.
    let mut path = PathBuf::from(std::env::temp_dir());
    path.push(tmp_filename);
    let mut file: tokio::fs::File = tokio::fs::File::create(&path).await.map_err(to_anyhow)?;
    while let Some(chunk) = stream.next().await {
        let chunk: Bytes = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                std::fs::remove_file(&path)?;
                return Err(error::Error::ExecutionErr(format!(
                    "Error reading stream: {}",
                    e
                )));
            }
        };
        file.write_all(&chunk).await?;
    }

    let (tx, rx) = tokio::sync::mpsc::channel(MAX_MPSC_SIZE);
    // Takes ownership of tx
    let mpsc_deserializer = mpsc_deserializer_factory(tx);

    // We read the file and pipe each element to the channel in a blocking task.
    let _ = task::spawn_blocking::<_, anyhow::Result<()>>(move || {
        let sync_file = std::fs::File::open(&path).map_err(to_anyhow)?;
        let mut buf_reader = std::io::BufReader::new(sync_file);

        let mut deserializer = serde_json::Deserializer::from_reader(&mut buf_reader);
        // This deserializer will read the file and send each row to the channel
        let _ = mpsc_deserializer.deserialize(&mut deserializer)?;

        std::fs::remove_file(&path)?;
        Ok(())
        // tx drops with mpsc_deserializer so the stream ends
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    Ok(stream)
}
