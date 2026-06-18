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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_bytea_hex() {
        assert_eq!(from_bytea_hex("\\x48656c6c6f").unwrap(), b"Hello");
    }

    #[test]
    fn test_empty_bytea_hex() {
        assert_eq!(from_bytea_hex("\\x").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_single_byte() {
        assert_eq!(from_bytea_hex("\\xff").unwrap(), vec![0xff]);
    }

    #[test]
    fn test_all_zeros() {
        assert_eq!(from_bytea_hex("\\x000000").unwrap(), vec![0, 0, 0]);
    }

    #[test]
    fn test_missing_prefix() {
        assert!(matches!(
            from_bytea_hex("48656c6c6f"),
            Err(ByteaHexParseError::InvalidPrefix)
        ));
    }

    #[test]
    fn test_wrong_prefix() {
        assert!(matches!(
            from_bytea_hex("0x48656c6c6f"),
            Err(ByteaHexParseError::InvalidPrefix)
        ));
    }

    #[test]
    fn test_too_short() {
        assert!(matches!(
            from_bytea_hex("\\"),
            Err(ByteaHexParseError::InvalidPrefix)
        ));
    }

    #[test]
    fn test_empty_string() {
        assert!(matches!(
            from_bytea_hex(""),
            Err(ByteaHexParseError::InvalidPrefix)
        ));
    }

    #[test]
    fn test_odd_digits() {
        assert!(matches!(
            from_bytea_hex("\\xabc"),
            Err(ByteaHexParseError::OddNumerOfDigits)
        ));
    }

    #[test]
    fn test_invalid_hex_chars() {
        assert!(matches!(
            from_bytea_hex("\\xzz"),
            Err(ByteaHexParseError::ParseInt(_))
        ));
    }

    #[test]
    fn test_uppercase_hex() {
        assert_eq!(from_bytea_hex("\\xABCD").unwrap(), vec![0xab, 0xcd]);
    }

    #[test]
    fn test_mixed_case_hex() {
        assert_eq!(from_bytea_hex("\\xAbCd").unwrap(), vec![0xab, 0xcd]);
    }
}
