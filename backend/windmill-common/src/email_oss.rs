#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::email_ee::*;

#[cfg(not(feature = "private"))]
use crate::server::Smtp;

/// Every send below is a no-op in this build, so callers that report success to a user (the
/// instance-settings SMTP test) have to say so instead of claiming the email went out.
#[cfg(not(feature = "private"))]
pub const SMTP_ENABLED: bool = false;

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

#[cfg(not(feature = "private"))]
pub async fn send_email_html(
    _subject: &str,
    _content: &str,
    _to: Vec<String>,
    _smtp: Smtp,
    _client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    Ok(())
}

#[cfg(not(feature = "private"))]
pub async fn send_email_plain_text(
    _subject: &str,
    _content: &str,
    _to: Vec<String>,
    _smtp: Smtp,
    _client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    Ok(())
}

#[cfg(not(feature = "private"))]
pub fn send_email_if_possible(_subject: &str, _content: &str, _to: &str) {
    tracing::warn!(
        "send_email_if_possible is not implemented in Windmill's Open Source repository"
    );
}
