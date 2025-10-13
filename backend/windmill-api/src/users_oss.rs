#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::users_ee::*;

#[cfg(not(feature = "private"))]
use std::sync::Arc;

#[cfg(not(feature = "private"))]
use crate::db::ApiAuthed;

#[cfg(not(feature = "private"))]
use crate::users::{EditPassword, NewUser};
#[cfg(not(feature = "private"))]
use crate::{db::DB, webhook_util::WebhookShared};
#[cfg(not(feature = "private"))]
use argon2::Argon2;

#[cfg(not(feature = "private"))]
use http::StatusCode;

#[cfg(not(feature = "private"))]
use windmill_common::error::{Error, Result};

#[cfg(not(feature = "private"))]
pub async fn create_user(
    _authed: ApiAuthed,
    _db: DB,
    _webhook: WebhookShared,
    _argon2: Arc<Argon2<'_>>,
    mut _nu: NewUser,
) -> Result<(StatusCode, String)> {
    Err(Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
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

#[cfg(not(feature = "private"))]
pub fn send_email_if_possible(_subject: &str, _content: &str, _to: &str) {
    tracing::warn!(
        "send_email_if_possible is not implemented in Windmill's Open Source repository"
    );
}
