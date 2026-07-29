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
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use std::time::Duration;

use humantime::{format_duration, parse_duration};
use reqwest::header::HeaderValue;

use crate::{Error, Result};

/// Provides deferred parsing of a value
///
/// This allows builders to defer fallibility to build
#[derive(Debug, Clone)]
pub(crate) enum ConfigValue<T> {
    Parsed(T),
    Deferred(String),
}

impl<T: Display> Display for ConfigValue<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parsed(v) => write!(f, "{v}"),
            Self::Deferred(v) => write!(f, "{v}"),
        }
    }
}

impl<T> From<T> for ConfigValue<T> {
    fn from(value: T) -> Self {
        Self::Parsed(value)
    }
}

impl<T: Parse + Clone> ConfigValue<T> {
    pub(crate) fn parse(&mut self, v: impl Into<String>) {
        *self = Self::Deferred(v.into())
    }

    pub(crate) fn get(&self) -> Result<T> {
        match self {
            Self::Parsed(v) => Ok(v.clone()),
            Self::Deferred(v) => T::parse(v),
        }
    }
}

impl<T: Default> Default for ConfigValue<T> {
    fn default() -> Self {
        Self::Parsed(T::default())
    }
}

/// A value that can be stored in [`ConfigValue`]
pub(crate) trait Parse: Sized {
    fn parse(v: &str) -> Result<Self>;
}

impl Parse for bool {
    fn parse(v: &str) -> Result<Self> {
        let lower = v.to_ascii_lowercase();
        match lower.as_str() {
            "1" | "true" | "on" | "yes" | "y" => Ok(true),
            "0" | "false" | "off" | "no" | "n" => Ok(false),
            _ => Err(Error::Generic {
                store: "Config",
                source: format!("failed to parse \"{v}\" as boolean").into(),
            }),
        }
    }
}

impl Parse for Duration {
    fn parse(v: &str) -> Result<Self> {
        parse_duration(v).map_err(|_| Error::Generic {
            store: "Config",
            source: format!("failed to parse \"{v}\" as Duration").into(),
        })
    }
}

impl Parse for usize {
    fn parse(v: &str) -> Result<Self> {
        Self::from_str(v).map_err(|_| Error::Generic {
            store: "Config",
            source: format!("failed to parse \"{v}\" as usize").into(),
        })
    }
}

impl Parse for u32 {
    fn parse(v: &str) -> Result<Self> {
        Self::from_str(v).map_err(|_| Error::Generic {
            store: "Config",
            source: format!("failed to parse \"{v}\" as u32").into(),
        })
    }
}

impl Parse for HeaderValue {
    fn parse(v: &str) -> Result<Self> {
        Self::from_str(v).map_err(|_| Error::Generic {
            store: "Config",
            source: format!("failed to parse \"{v}\" as HeaderValue").into(),
        })
    }
}

pub(crate) fn fmt_duration(duration: &ConfigValue<Duration>) -> String {
    match duration {
        ConfigValue::Parsed(v) => format_duration(*v).to_string(),
        ConfigValue::Deferred(v) => v.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_parse_duration() {
        let duration = Duration::from_secs(60);
        assert_eq!(Duration::parse("60 seconds").unwrap(), duration);
        assert_eq!(Duration::parse("60 s").unwrap(), duration);
        assert_eq!(Duration::parse("60s").unwrap(), duration)
    }
}
