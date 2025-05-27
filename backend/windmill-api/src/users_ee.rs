use std::sync::Arc;

use crate::db::ApiAuthed;

use crate::users::{EditPassword, NewUser};
use crate::{db::DB, webhook_util::WebhookShared};
use argon2::Argon2;

use http::StatusCode;

use windmill_common::error::{Error, Result};

pub async fn create_user(
    _authed: ApiAuthed,
    _db: DB,
    _webhook: WebhookShared,
    _argon2: Arc<Argon2<'_>>,
    mut _nu: NewUser,
) -> Result<(StatusCode, String)> {
    // The skip_email parameter is available in _nu.skip_email
    // This would be used to conditionally call send_email_if_possible
    // For now, the open source version still returns an error
    Err(Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

pub async fn set_password(
    _db: DB,
    _argon2: Arc<Argon2<'_>>,
    _authed: ApiAuthed,
    _user_email: &str,
    _ep: EditPassword,
) -> Result<String> {
    Err(Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

pub fn send_email_if_possible(_subject: &str, _content: &str, _to: &str) {
    tracing::warn!(
        "send_email_if_possible is not implemented in Windmill's Open Source repository"
    );
}

pub fn send_email_if_possible_with_skip(_subject: &str, _content: &str, _to: &str, _skip_email: bool) {
    if _skip_email {
        tracing::info!("Skipping email send to {} as requested", _to);
        return;
    }
    send_email_if_possible(_subject, _content, _to);
}
