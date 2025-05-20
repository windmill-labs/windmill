#[cfg(feature = "private")]
use crate::email_ee;

use crate::server::Smtp;

pub async fn send_email(
    subject: &str,
    content: &str,
    to: Vec<String>,
    smtp: Smtp,
    client_timeout: Option<tokio::time::Duration>,
) -> crate::error::Result<()> {
    #[cfg(feature = "private")]
    {
        return email_ee::send_email(subject, content, to, smtp, client_timeout).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (subject, content, to, smtp, client_timeout);
        Ok(())
    }
}
