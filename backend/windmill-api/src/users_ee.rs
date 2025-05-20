#[cfg(feature = "private")]
use crate::users_ee;

use std::sync::Arc;

use crate::db::ApiAuthed;

use crate::users::{EditPassword, NewUser};
use crate::{db::DB, webhook_util::WebhookShared};
use argon2::Argon2;

use http::StatusCode;

use windmill_common::error::{Error, Result};

pub async fn create_user(
    authed: ApiAuthed,
    db: DB,
    webhook: WebhookShared,
    argon2: Arc<Argon2<'_>>,
    mut nu: NewUser,
) -> Result<(StatusCode, String)> {
    #[cfg(feature = "private")]
    {
        return users_ee::create_user(authed, db, webhook, argon2, nu).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (authed, db, webhook, argon2, nu);
        Err(Error::internal_err(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }
}

pub async fn set_password(
    db: DB,
    argon2: Arc<Argon2<'_>>,
    authed: ApiAuthed,
    user_email: &str,
    ep: EditPassword,
) -> Result<String> {
    #[cfg(feature = "private")]
    {
        return users_ee::set_password(db, argon2, authed, user_email, ep).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (db, argon2, authed, user_email, ep);
        Err(Error::internal_err(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }
}

pub fn send_email_if_possible(subject: &str, content: &str, to: &str) {
    #[cfg(feature = "private")]
    {
        users_ee::send_email_if_possible(subject, content, to);
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (subject, content, to);
        tracing::warn!(
            "send_email_if_possible is not implemented in Windmill's Open Source repository"
        );
    }
}
