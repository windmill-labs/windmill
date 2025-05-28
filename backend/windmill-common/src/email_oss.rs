use crate::{error::Result, DB};

pub async fn send_email(
    _to: &str,
    _subject: &str,
    _body: &str,
    _html_body: Option<&str>,
    _workspace_id: &str,
    _db: &DB,
) -> Result<()> {
    crate::email_ee::send_email(_to, _subject, _body, _html_body, _workspace_id, _db).await
}