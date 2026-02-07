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
