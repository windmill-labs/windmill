use core::str;
use std::num::{ParseFloatError, ParseIntError};

use super::{
    bool::{parse_bool, ParseBoolError},
    hex::{from_bytea_hex, ByteaHexParseError},
    // numeric::PgNumeric,
};
//use bigdecimal::ParseBigDecimalError;
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use rust_postgres::types::Type;
use serde::Serialize;
use serde_json::value::RawValue;
use thiserror::Error;
use uuid::Uuid;
use windmill_common::worker::to_raw_value;

#[derive(Debug, Error)]
pub enum ConverterError {
    #[error("invalid bool value")]
    InvalidBool(#[from] ParseBoolError),

    #[error("invalid int value")]
    InvalidInt(#[from] ParseIntError),

    #[error("invalid float value")]
    InvalidFloat(#[from] ParseFloatError),

    /*#[error("invalid numeric: {0}")]
    InvalidNumeric(#[from] ParseBigDecimalError),*/
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
}

pub struct Converter;

#[derive(Debug, Error)]
pub enum ArrayParseError {
    #[error("input too short")]
    InputTooShort,

    #[error("missing braces")]
    MissingBraces,
}

impl Converter {
    pub fn try_from_str(typ: Option<&Type>, str: &str) -> Result<Box<RawValue>, ConverterError> {
        let value = match typ {
            Some(typ) => {
                match *typ {
                    Type::BOOL => to_raw_value(&parse_bool(str)?),
                    Type::BOOL_ARRAY => {
                        Converter::parse_array(str, |str| Ok(Some(parse_bool(str)?)))?
                    }
                    Type::CHAR | Type::BPCHAR | Type::VARCHAR | Type::NAME | Type::TEXT => {
                        to_raw_value(&str.to_string())
                    }
                    Type::CHAR_ARRAY
                    | Type::BPCHAR_ARRAY
                    | Type::VARCHAR_ARRAY
                    | Type::NAME_ARRAY
                    | Type::TEXT_ARRAY => {
                        Converter::parse_array(str, |str| Ok(Some(str.to_string())))?
                    }
                    Type::INT2 => to_raw_value(&str.parse::<i16>()?),
                    Type::INT2_ARRAY => {
                        Converter::parse_array(str, |str| Ok(Some(str.parse::<i16>()?)))?
                    }
                    Type::INT4 => to_raw_value(&str.parse::<i32>()?),
                    Type::INT4_ARRAY => {
                        Converter::parse_array(str, |str| Ok(Some(str.parse::<i32>()?)))?
                    }
                    Type::INT8 => to_raw_value(&str.parse::<i64>()?),
                    Type::INT8_ARRAY => {
                        Converter::parse_array(str, |str| Ok(Some(str.parse::<i64>()?)))?
                    }
                    Type::FLOAT4 => to_raw_value(&str.parse::<f32>()?),
                    Type::FLOAT4_ARRAY => {
                        Converter::parse_array(str, |str| Ok(Some(str.parse::<f32>()?)))?
                    }
                    Type::FLOAT8 => to_raw_value(&str.parse::<f64>()?),
                    Type::FLOAT8_ARRAY => {
                        Converter::parse_array(str, |str| Ok(Some(str.parse::<f64>()?)))?
                    }
                    //Type::NUMERIC => str.parse()?,
                    //Type::NUMERIC_ARRAY => Converter::parse_array(str, |str| Ok(Some(str.parse()?)))?,
                    Type::BYTEA => to_raw_value(&from_bytea_hex(str)?),
                    Type::BYTEA_ARRAY => {
                        Converter::parse_array(str, |str| Ok(Some(from_bytea_hex(str)?)))?
                    }
                    Type::DATE => to_raw_value(&NaiveDate::parse_from_str(str, "%Y-%m-%d")?),
                    Type::DATE_ARRAY => Converter::parse_array(str, |str| {
                        Ok(Some(NaiveDate::parse_from_str(str, "%Y-%m-%d")?))
                    })?,
                    Type::TIME => to_raw_value(&NaiveTime::parse_from_str(str, "%H:%M:%S%.f")?),
                    Type::TIME_ARRAY => Converter::parse_array(str, |str| {
                        Ok(Some(NaiveTime::parse_from_str(str, "%H:%M:%S%.f")?))
                    })?,
                    Type::TIMESTAMP => {
                        to_raw_value(&NaiveDateTime::parse_from_str(str, "%Y-%m-%d %H:%M:%S%.f")?)
                    }
                    Type::TIMESTAMP_ARRAY => Converter::parse_array(str, |str| {
                        Ok(Some(NaiveDateTime::parse_from_str(
                            str,
                            "%Y-%m-%d %H:%M:%S%.f",
                        )?))
                    })?,
                    Type::TIMESTAMPTZ => {
                        let val = match DateTime::<FixedOffset>::parse_from_str(
                            str,
                            "%Y-%m-%d %H:%M:%S%.f%#z",
                        ) {
                            Ok(val) => val,
                            Err(_) => DateTime::<FixedOffset>::parse_from_str(
                                str,
                                "%Y-%m-%d %H:%M:%S%.f%:z",
                            )?,
                        };
                        let utc: DateTime<Utc> = val.into();
                        to_raw_value(&utc)
                    }
                    Type::TIMESTAMPTZ_ARRAY => {
                        match Converter::parse_array(str, |str| {
                            let utc: DateTime<Utc> = DateTime::<FixedOffset>::parse_from_str(
                                str,
                                "%Y-%m-%d %H:%M:%S%.f%#z",
                            )?
                            .into();
                            Ok(Some(utc))
                        }) {
                            Ok(val) => val,
                            Err(_) => Converter::parse_array(str, |str| {
                                let utc: DateTime<Utc> = DateTime::<FixedOffset>::parse_from_str(
                                    str,
                                    "%Y-%m-%d %H:%M:%S%.f%#z",
                                )?
                                .into();
                                Ok(Some(utc))
                            })?,
                        }
                    }
                    Type::UUID => to_raw_value(&Uuid::parse_str(str)?),
                    Type::UUID_ARRAY => {
                        Converter::parse_array(str, |str| Ok(Some(Uuid::parse_str(str)?)))?
                    }
                    Type::JSON | Type::JSONB => {
                        to_raw_value(&serde_json::from_str::<serde_json::Value>(str)?)
                    }
                    Type::JSON_ARRAY | Type::JSONB_ARRAY => Converter::parse_array(str, |str| {
                        Ok(Some(serde_json::from_str::<serde_json::Value>(str)?))
                    })?,
                    Type::OID => to_raw_value(&str.parse::<u32>()?),
                    Type::OID_ARRAY => {
                        Converter::parse_array(str, |str| Ok(Some(str.parse::<u32>()?)))?
                    }
                    _ => to_raw_value(&str.to_string()),
                }
            }
            None => to_raw_value(&str.to_string()),
        };

        Ok(value)
    }

    fn parse_array<P, T>(str: &str, mut parse: P) -> Result<Box<RawValue>, ConverterError>
    where
        P: FnMut(&str) -> Result<Option<T>, ConverterError>,
        T: Serialize,
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
                None
            } else {
                parse(&val_str)?
            };
            res.push(val);
            val_str.clear();
        }

        Ok(to_raw_value(&res))
    }
}
