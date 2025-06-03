#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::email_ee::*;

#[cfg(not(feature = "private"))]
use crate::server::Smtp;

#[cfg(not(feature = "private"))]
pub async fn send_email(
    _subject: &str,
    _content: &str,
    _to: Vec<String>,
    _smtp: Smtp,
    _client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    Ok(())
}
