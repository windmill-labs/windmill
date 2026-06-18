use crate::db::Authable;

#[derive(Clone)]
pub struct AuditAuthor {
    pub username: String,
    pub email: String,
    pub username_override: Option<String>,
    pub token_prefix: Option<String>,
}

impl<T: Authable> From<&T> for AuditAuthor {
    fn from(value: &T) -> Self {
        Self {
            username: value.username().to_string(),
            email: value.email().to_string(),
            username_override: None,
            token_prefix: None,
        }
    }
}
