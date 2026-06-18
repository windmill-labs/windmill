//! helpers for serde + serde derive attributes

use rand::distr::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;
use std::{fmt::Display, str::FromStr};

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

fn rd_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
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
