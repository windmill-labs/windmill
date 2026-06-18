use thiserror::Error;

/**
* This implementation is inspired by Postgres replication functionality
* from https://github.com/supabase/pg_replicate
*
* Original implementation:
* - https://github.dev/supabase/pg_replicate/blob/main/pg_replicate/src/conversions/bool.rs
*
*/

#[derive(Debug, Error)]
pub enum ParseBoolError {
    #[error("invalid input value: {0}")]
    InvalidInput(String),
}

pub fn parse_bool(s: &str) -> Result<bool, ParseBoolError> {
    match s {
        "t" => Ok(true),
        "f" => Ok(false),
        _ => Err(ParseBoolError::InvalidInput(s.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_true() {
        assert_eq!(parse_bool("t").unwrap(), true);
    }

    #[test]
    fn test_parse_false() {
        assert_eq!(parse_bool("f").unwrap(), false);
    }

    #[test]
    fn test_invalid_true_string() {
        assert!(matches!(
            parse_bool("true"),
            Err(ParseBoolError::InvalidInput(s)) if s == "true"
        ));
    }

    #[test]
    fn test_invalid_empty() {
        assert!(parse_bool("").is_err());
    }

    #[test]
    fn test_invalid_uppercase() {
        assert!(parse_bool("T").is_err());
    }
}
