/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Filter wrappers that support an optional `!` negation prefix.
//!
//! - [`NegatedFilter<T>`] — a single value, e.g. `"schedule"` or `"!schedule"`.
//! - [`NegatedListFilter<T>`] — comma-separated values, e.g. `"!schedule,!email"` or `"http,webhook"`.
//!   Every item in the list shares the same negated/non-negated sense; mixing is not supported

use serde::{
    de::{self, DeserializeOwned},
    Deserializer,
};
use std::{fmt, marker::PhantomData};

// ── NegatedFilter<T> ──────────────────────────────────────────────────────────

/// A single filter value optionally prefixed with `!` to indicate negation.
///
/// Deserializes `"schedule"` → `NegatedFilter { value: Schedule, negated: false }`
/// Deserializes `"!schedule"` → `NegatedFilter { value: Schedule, negated: true }`
#[derive(Debug, Clone)]
pub struct NegatedFilter<T> {
    pub value: T,
    pub negated: bool,
}

impl<T> NegatedFilter<T> {
    pub fn positive(value: T) -> Self {
        Self { value, negated: false }
    }

    pub fn negated(value: T) -> Self {
        Self { value, negated: true }
    }
}

struct NegatedFilterVisitor<T>(PhantomData<T>);

impl<'de, T: DeserializeOwned> de::Visitor<'de> for NegatedFilterVisitor<T> {
    type Value = NegatedFilter<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a string optionally prefixed with '!'")
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
        let (negated, raw) = match s.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, s),
        };
        let value = serde_json::from_value(serde_json::Value::String(raw.to_owned()))
            .map_err(|e| E::custom(format!("invalid filter value {:?}: {}", raw, e)))?;
        Ok(NegatedFilter { value, negated })
    }
}

impl<'de, T: DeserializeOwned> de::Deserialize<'de> for NegatedFilter<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(NegatedFilterVisitor(PhantomData))
    }
}

// ── NegatedListFilter<T> ──────────────────────────────────────────────────────

/// A comma-separated list of filter values, all sharing the same negation sense.
///
/// Deserializes `"schedule,email"` → `NegatedListFilter { values: [Schedule, Email], negated: false }`
/// Deserializes `"!schedule,!email"` → `NegatedListFilter { values: [Schedule, Email], negated: true }`
///
/// The `!` is read from the **first** item only; subsequent items may or may not carry
/// `!` and it is stripped regardless, keeping the API forgiving.
#[derive(Debug, Clone)]
pub struct NegatedListFilter<T> {
    pub values: Vec<T>,
    pub negated: bool,
}

impl<T> NegatedListFilter<T> {
    pub fn positive(values: Vec<T>) -> Self {
        Self { values, negated: false }
    }
}

struct NegatedListFilterVisitor<T>(PhantomData<T>);

impl<'de, T: DeserializeOwned> de::Visitor<'de> for NegatedListFilterVisitor<T> {
    type Value = NegatedListFilter<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a comma-separated string optionally prefixed with '!'")
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
        let mut negated = false;
        let values = s
            .split(',')
            .enumerate()
            .map(|(i, item)| {
                let raw = match item.strip_prefix('!') {
                    Some(rest) => {
                        if i == 0 {
                            negated = true;
                        }
                        rest
                    }
                    None => item,
                };
                serde_json::from_value::<T>(serde_json::Value::String(raw.to_owned()))
                    .map_err(|e| E::custom(format!("invalid filter value {:?}: {}", raw, e)))
            })
            .collect::<Result<Vec<T>, E>>()?;
        Ok(NegatedListFilter { values, negated })
    }
}

impl<'de, T: DeserializeOwned> de::Deserialize<'de> for NegatedListFilter<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(NegatedListFilterVisitor(PhantomData))
    }
}
