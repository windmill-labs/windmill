use std::num::ParseIntError;


/**
* This implementation is inspired by Postgres replication functionality
* from https://github.com/supabase/pg_replicate
* 
* Original implementation: 
* - https://github.dev/supabase/pg_replicate/blob/main/pg_replicate/src/conversions/hex.rs
* 
*/

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ByteaHexParseError {
    #[error("missing prefix '\\x'")]
    InvalidPrefix,

    #[error("invalid byte")]
    OddNumerOfDigits,

    #[error("parse int result: {0}")]
    ParseInt(#[from] ParseIntError),
}

pub fn from_bytea_hex(s: &str) -> Result<Vec<u8>, ByteaHexParseError> {
    if s.len() < 2 || &s[..2] != "\\x" {
        return Err(ByteaHexParseError::InvalidPrefix);
    }

    let mut result = Vec::with_capacity((s.len() - 2) / 2);
    let s = &s[2..];

    if s.len() % 2 != 0 {
        return Err(ByteaHexParseError::OddNumerOfDigits);
    }

    for i in (0..s.len()).step_by(2) {
        let val = u8::from_str_radix(&s[i..i + 2], 16)?;
        result.push(val);
    }

    Ok(result)
}
