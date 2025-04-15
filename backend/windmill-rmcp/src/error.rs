use std::fmt::Display;

use crate::model::ErrorData;

pub type Error = ErrorData;

impl Display for ErrorData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.0, self.message)?;
        if let Some(data) = &self.data {
            write!(f, "({})", data)?;
        }
        Ok(())
    }
}

impl std::error::Error for ErrorData {}
