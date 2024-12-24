use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseBoolError {
    #[error("invalid input value: {0}")]
    InvalidInput(String),
}

pub fn parse_bool(s: &str) -> Result<bool, ParseBoolError> {
    if s == "t" {
        Ok(true)
    } else if s == "f" {
        Ok(false)
    } else {
        Err(ParseBoolError::InvalidInput(s.to_string()))
    }
}
