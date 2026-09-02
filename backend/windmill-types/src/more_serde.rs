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

pub fn maybe_number<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + serde::Deserialize<'de>,
    <T as FromStr>::Err: Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumericOrString<T> {
        String(String),
        RawT(T),
    }

    match NumericOrString::<T>::deserialize(deserializer)? {
        NumericOrString::String(s) => T::from_str(&s).map_err(serde::de::Error::custom),
        NumericOrString::RawT(i) => Ok(i),
    }
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

/// Deserializer for a doubly-optional field, so a struct can tell an absent key
/// (`None`) from an explicit `null` (`Some(None)`).
///
/// Plain serde collapses both into the outer `None`, which makes the distinction
/// unusable exactly where it matters: a payload that omits a field means "leave it
/// alone", while one that sends `null` means "clear it".
///
/// ```ignore
/// #[serde(default, deserialize_with = "double_option", skip_serializing_if = "Option::is_none")]
/// pub field: Option<Option<String>>,
/// ```
pub fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct WithMaybeNumber {
        #[serde(deserialize_with = "super::maybe_number")]
        n: i64,
    }

    #[test]
    fn maybe_number_accepts_number() {
        let v: WithMaybeNumber = serde_json::from_value(serde_json::json!({ "n": 12345 })).unwrap();
        assert_eq!(v.n, 12345);
    }

    #[test]
    fn maybe_number_accepts_string() {
        let v: WithMaybeNumber =
            serde_json::from_value(serde_json::json!({ "n": "12345" })).unwrap();
        assert_eq!(v.n, 12345);
    }

    #[test]
    fn maybe_number_rejects_non_numeric_string() {
        assert!(
            serde_json::from_value::<WithMaybeNumber>(serde_json::json!({ "n": "abc" })).is_err()
        );
    }

    #[test]
    fn maybe_number_rejects_null() {
        assert!(
            serde_json::from_value::<WithMaybeNumber>(serde_json::json!({ "n": null })).is_err()
        );
    }
}
