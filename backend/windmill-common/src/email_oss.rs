use crate::server::Smtp;

pub async fn send_email(
    _subject: &str,
    _content: &str,
    _to: Vec<String>,
    _smtp: Smtp,
    _client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    Ok(())
}