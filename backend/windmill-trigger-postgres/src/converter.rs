use core::str;
use std::{
    num::{ParseFloatError, ParseIntError},
    str::FromStr,
};

use super::{
    bool::{parse_bool, ParseBoolError},
    hex::{from_bytea_hex, ByteaHexParseError},
};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use rust_decimal::Decimal;
use rust_postgres::types::Type;
use serde_json::{to_value, Number, Value};
use thiserror::Error;
use uuid::Uuid;

/**
* This implementation is inspired by Postgres replication functionality
* from https://github.com/supabase/pg_replicate
*
* Original implementation:
* - https://github.com/supabase/pg_replicate/blob/main/pg_replicate/src/conversions/text.rs
*
*/

#[derive(Debug, Error)]
pub enum ConverterError {
    #[error("invalid bool value")]
    InvalidBool(#[from] ParseBoolError),

    #[error("invalid int value")]
    InvalidInt(#[from] ParseIntError),

    #[error("invalid float value")]
    InvalidFloat(#[from] ParseFloatError),

    #[error("invalid numeric: {0}")]
    InvalidNumeric(#[from] rust_decimal::Error),

    #[error("invalid bytea: {0}")]
    InvalidBytea(#[from] ByteaHexParseError),

    #[error("invalid uuid: {0}")]
    InvalidUuid(#[from] uuid::Error),

    #[error("invalid json: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("invalid timestamp: {0} ")]
    InvalidTimestamp(#[from] chrono::ParseError),

    #[error("invalid array: {0}")]
    InvalidArray(#[from] ArrayParseError),

    #[error("{0}")]
    Custom(String),
}

fn convert_into<T>(number: T) -> Number
where
    T: Sized,
    serde_json::Number: From<T>,
{
    serde_json::Number::from(number)
}

pub struct Converter;

#[derive(Debug, Error)]
pub enum ArrayParseError {
    #[error("input too short")]
    InputTooShort,

    #[error("missing braces")]
    MissingBraces,
}

fn f64_to_json_number(raw_val: f64) -> Result<Value, ConverterError> {
    let temp = serde_json::Number::from_f64(raw_val.into())
        .ok_or(ConverterError::Custom("invalid json-float".to_string()))?;
    Ok(Value::Number(temp))
}

impl Converter {
    pub fn try_from_str(typ: Option<Type>, str: &str) -> Result<Value, ConverterError> {
        let value = match typ.unwrap_or(Type::TEXT) {
            Type::BOOL => Value::Bool(parse_bool(str)?),
            Type::BOOL_ARRAY => {
                Converter::parse_array(str, |str| Ok(Value::Bool(parse_bool(str)?)))?
            }
            Type::CHAR | Type::BPCHAR | Type::VARCHAR | Type::NAME | Type::TEXT => {
                Value::String(str.to_string())
            }
            Type::CHAR_ARRAY
            | Type::BPCHAR_ARRAY
            | Type::VARCHAR_ARRAY
            | Type::NAME_ARRAY
            | Type::TEXT_ARRAY => {
                Converter::parse_array(str, |str| Ok(Value::String(str.to_string())))?
            }
            Type::INT2 => Value::Number(convert_into(str.parse::<i16>()?)),
            Type::INT2_ARRAY => Converter::parse_array(str, |str| {
                Ok(Value::Number(convert_into(str.parse::<i16>()?)))
            })?,
            Type::INT4 => Value::Number(convert_into(str.parse::<i32>()?)),
            Type::INT4_ARRAY => Converter::parse_array(str, |str| {
                Ok(Value::Number(convert_into(str.parse::<i32>()?)))
            })?,
            Type::INT8 => Value::Number(convert_into(str.parse::<i64>()?)),
            Type::INT8_ARRAY => Converter::parse_array(str, |str| {
                Ok(Value::Number(convert_into(str.parse::<i64>()?)))
            })?,
            Type::FLOAT4 => f64_to_json_number(str.parse::<f64>()?)?,
            Type::FLOAT4_ARRAY => {
                Converter::parse_array(str, |str| f64_to_json_number(str.parse::<f64>()?))?
            }
            Type::FLOAT8 => f64_to_json_number(str.parse::<f64>()?)?,
            Type::FLOAT8_ARRAY => {
                Converter::parse_array(str, |str| f64_to_json_number(str.parse::<f64>()?))?
            }
            Type::NUMERIC => serde_json::json!(Decimal::from_str(str)?),
            Type::NUMERIC_ARRAY => {
                Converter::parse_array(str, |str| Ok(serde_json::json!(Decimal::from_str(str)?)))?
            }
            Type::BYTEA => to_value(from_bytea_hex(str)?).unwrap(),
            Type::BYTEA_ARRAY => {
                Converter::parse_array(str, |str| Ok(to_value(from_bytea_hex(str)?).unwrap()))?
            }
            Type::DATE => {
                let date = NaiveDate::parse_from_str(str, "%Y-%m-%d")?;
                Value::String(date.to_string())
            }
            Type::DATE_ARRAY => Converter::parse_array(str, |str| {
                let date = NaiveDate::parse_from_str(str, "%Y-%m-%d")?;
                Ok(Value::String(date.to_string()))
            })?,
            Type::TIME => {
                let time = NaiveTime::parse_from_str(str, "%H:%M:%S%.f")?;
                Value::String(time.to_string())
            }
            Type::TIME_ARRAY => Converter::parse_array(str, |str| {
                let time = NaiveTime::parse_from_str(str, "%H:%M:%S%.f")?;
                Ok(Value::String(time.to_string()))
            })?,
            Type::TIMESTAMP => {
                let timestamp = NaiveDateTime::parse_from_str(str, "%Y-%m-%d %H:%M:%S%.f")?;
                Value::String(timestamp.to_string())
            }
            Type::TIMESTAMP_ARRAY => Converter::parse_array(str, |str| {
                let timestamp = NaiveDateTime::parse_from_str(str, "%Y-%m-%d %H:%M:%S%.f")?;
                Ok(Value::String(timestamp.to_string()))
            })?,
            Type::TIMESTAMPTZ => {
                let val =
                    match DateTime::<FixedOffset>::parse_from_str(str, "%Y-%m-%d %H:%M:%S%.f%#z") {
                        Ok(val) => val,
                        Err(_) => {
                            DateTime::<FixedOffset>::parse_from_str(str, "%Y-%m-%d %H:%M:%S%.f%:z")?
                        }
                    };
                let utc: DateTime<Utc> = val.into();
                Value::String(utc.to_string())
            }
            Type::TIMESTAMPTZ_ARRAY => {
                match Converter::parse_array(str, |str| {
                    let utc: DateTime<Utc> =
                        DateTime::<FixedOffset>::parse_from_str(str, "%Y-%m-%d %H:%M:%S%.f%#z")?
                            .into();
                    Ok(Value::String(utc.to_string()))
                }) {
                    Ok(val) => val,
                    Err(_) => Converter::parse_array(str, |str| {
                        let utc: DateTime<Utc> = DateTime::<FixedOffset>::parse_from_str(
                            str,
                            "%Y-%m-%d %H:%M:%S%.f%#z",
                        )?
                        .into();
                        Ok(Value::String(utc.to_string()))
                    })?,
                }
            }
            Type::UUID => Value::String(Uuid::parse_str(str)?.to_string()),
            Type::UUID_ARRAY => Converter::parse_array(str, |str| {
                Ok(Value::String(Uuid::parse_str(str)?.to_string()))
            })?,
            Type::JSON | Type::JSONB => serde_json::from_str::<serde_json::Value>(str)?,
            Type::JSON_ARRAY | Type::JSONB_ARRAY => Converter::parse_array(str, |str| {
                Ok(serde_json::from_str::<serde_json::Value>(str)?)
            })?,
            Type::OID => Value::Number(convert_into(str.parse::<u32>()?)),
            Type::OID_ARRAY => Converter::parse_array(str, |str| {
                Ok(Value::Number(convert_into(str.parse::<u32>()?)))
            })?,
            _ => Value::String(str.to_string()),
        };

        Ok(value)
    }

    fn parse_array<P>(str: &str, mut parse: P) -> Result<Value, ConverterError>
    where
        P: FnMut(&str) -> Result<Value, ConverterError>,
    {
        if str.len() < 2 {
            return Err(ArrayParseError::InputTooShort.into());
        }

        if !str.starts_with('{') || !str.ends_with('}') {
            return Err(ArrayParseError::MissingBraces.into());
        }

        let mut res = vec![];
        let str = &str[1..(str.len() - 1)];
        let mut val_str = String::with_capacity(10);
        let mut in_quotes = false;
        let mut in_escape = false;
        let mut chars = str.chars();
        let mut done = str.is_empty();

        while !done {
            loop {
                match chars.next() {
                    Some(c) => match c {
                        c if in_escape => {
                            val_str.push(c);
                            in_escape = false;
                        }
                        '"' => in_quotes = !in_quotes,
                        '\\' => in_escape = true,
                        ',' if !in_quotes => {
                            break;
                        }
                        c => {
                            val_str.push(c);
                        }
                    },
                    None => {
                        done = true;
                        break;
                    }
                }
            }
            let val = if val_str.to_lowercase() == "null" {
                Value::Null
            } else {
                parse(&val_str)?
            };
            res.push(val);
            val_str.clear();
        }
        let arr = Value::Array(res);
        Ok(arr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- Scalar type conversions ---

    #[test]
    fn test_bool_true() {
        let result = Converter::try_from_str(Some(Type::BOOL), "t").unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_bool_false() {
        let result = Converter::try_from_str(Some(Type::BOOL), "f").unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_text() {
        let result = Converter::try_from_str(Some(Type::TEXT), "hello world").unwrap();
        assert_eq!(result, Value::String("hello world".to_string()));
    }

    #[test]
    fn test_varchar() {
        let result = Converter::try_from_str(Some(Type::VARCHAR), "test").unwrap();
        assert_eq!(result, Value::String("test".to_string()));
    }

    #[test]
    fn test_int2() {
        let result = Converter::try_from_str(Some(Type::INT2), "42").unwrap();
        assert_eq!(result, json!(42));
    }

    #[test]
    fn test_int4() {
        let result = Converter::try_from_str(Some(Type::INT4), "-100").unwrap();
        assert_eq!(result, json!(-100));
    }

    #[test]
    fn test_int8() {
        let result = Converter::try_from_str(Some(Type::INT8), "9999999999").unwrap();
        assert_eq!(result, json!(9999999999i64));
    }

    #[test]
    fn test_float4() {
        let result = Converter::try_from_str(Some(Type::FLOAT4), "3.14").unwrap();
        assert!(result.as_f64().unwrap() - 3.14 < 0.001);
    }

    #[test]
    fn test_float8() {
        let result = Converter::try_from_str(Some(Type::FLOAT8), "2.718281828").unwrap();
        assert!(result.as_f64().unwrap() - 2.718281828 < 0.0001);
    }

    #[test]
    fn test_numeric() {
        let result = Converter::try_from_str(Some(Type::NUMERIC), "123.456").unwrap();
        assert_eq!(result.to_string(), "123.456");
    }

    #[test]
    fn test_uuid() {
        let result =
            Converter::try_from_str(Some(Type::UUID), "550e8400-e29b-41d4-a716-446655440000")
                .unwrap();
        assert_eq!(
            result,
            Value::String("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_json() {
        let result =
            Converter::try_from_str(Some(Type::JSON), r#"{"key": "value", "n": 1}"#).unwrap();
        assert_eq!(result, json!({"key": "value", "n": 1}));
    }

    #[test]
    fn test_jsonb() {
        let result = Converter::try_from_str(Some(Type::JSONB), r#"[1,2,3]"#).unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_date() {
        let result = Converter::try_from_str(Some(Type::DATE), "2024-01-15").unwrap();
        assert_eq!(result, Value::String("2024-01-15".to_string()));
    }

    #[test]
    fn test_time() {
        let result = Converter::try_from_str(Some(Type::TIME), "14:30:00.0").unwrap();
        assert_eq!(result, Value::String("14:30:00".to_string()));
    }

    #[test]
    fn test_timestamp() {
        let result =
            Converter::try_from_str(Some(Type::TIMESTAMP), "2024-01-15 14:30:00.0").unwrap();
        assert_eq!(result, Value::String("2024-01-15 14:30:00".to_string()));
    }

    #[test]
    fn test_timestamptz() {
        let result =
            Converter::try_from_str(Some(Type::TIMESTAMPTZ), "2024-01-15 14:30:00.0+00").unwrap();
        assert!(result.as_str().unwrap().contains("2024-01-15"));
    }

    #[test]
    fn test_bytea() {
        let result = Converter::try_from_str(Some(Type::BYTEA), "\\x48656c6c6f").unwrap();
        assert_eq!(result, json!([72, 101, 108, 108, 111]));
    }

    #[test]
    fn test_oid() {
        let result = Converter::try_from_str(Some(Type::OID), "12345").unwrap();
        assert_eq!(result, json!(12345u32));
    }

    #[test]
    fn test_none_type_defaults_to_text() {
        let result = Converter::try_from_str(None, "anything").unwrap();
        assert_eq!(result, Value::String("anything".to_string()));
    }

    // --- Array type conversions ---

    #[test]
    fn test_int4_array() {
        let result = Converter::try_from_str(Some(Type::INT4_ARRAY), "{1,2,3}").unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_text_array() {
        let result = Converter::try_from_str(Some(Type::TEXT_ARRAY), "{hello,world}").unwrap();
        assert_eq!(result, json!(["hello", "world"]));
    }

    #[test]
    fn test_bool_array() {
        let result = Converter::try_from_str(Some(Type::BOOL_ARRAY), "{t,f,t}").unwrap();
        assert_eq!(result, json!([true, false, true]));
    }

    #[test]
    fn test_array_with_null() {
        let result = Converter::try_from_str(Some(Type::INT4_ARRAY), "{1,NULL,3}").unwrap();
        assert_eq!(result, json!([1, null, 3]));
    }

    #[test]
    fn test_array_with_quoted_strings() {
        let result =
            Converter::try_from_str(Some(Type::TEXT_ARRAY), r#"{"hello, world","test"}"#).unwrap();
        assert_eq!(result, json!(["hello, world", "test"]));
    }

    #[test]
    fn test_empty_array() {
        let result = Converter::try_from_str(Some(Type::INT4_ARRAY), "{}").unwrap();
        assert_eq!(result, json!([]));
    }

    #[test]
    fn test_uuid_array() {
        let result = Converter::try_from_str(
            Some(Type::UUID_ARRAY),
            "{550e8400-e29b-41d4-a716-446655440000,6ba7b810-9dad-11d1-80b4-00c04fd430c8}",
        )
        .unwrap();
        assert_eq!(
            result,
            json!([
                "550e8400-e29b-41d4-a716-446655440000",
                "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
            ])
        );
    }

    // --- Error cases ---

    #[test]
    fn test_invalid_int() {
        assert!(Converter::try_from_str(Some(Type::INT4), "not_a_number").is_err());
    }

    #[test]
    fn test_invalid_bool() {
        assert!(Converter::try_from_str(Some(Type::BOOL), "yes").is_err());
    }

    #[test]
    fn test_invalid_uuid() {
        assert!(Converter::try_from_str(Some(Type::UUID), "not-a-uuid").is_err());
    }

    #[test]
    fn test_invalid_json() {
        assert!(Converter::try_from_str(Some(Type::JSON), "not json").is_err());
    }

    #[test]
    fn test_array_missing_braces() {
        assert!(Converter::try_from_str(Some(Type::INT4_ARRAY), "1,2,3").is_err());
    }

    #[test]
    fn test_array_too_short() {
        assert!(Converter::try_from_str(Some(Type::INT4_ARRAY), "{").is_err());
    }

    #[test]
    fn test_array_with_escaped_backslash() {
        let result = Converter::try_from_str(Some(Type::TEXT_ARRAY), r#"{"a\\b","c"}"#).unwrap();
        assert_eq!(result, json!(["a\\b", "c"]));
    }

    #[test]
    fn test_float_nan_rejected() {
        assert!(Converter::try_from_str(Some(Type::FLOAT4), "NaN").is_err());
    }
}
