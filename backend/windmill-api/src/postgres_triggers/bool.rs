use thiserror::Error;

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
