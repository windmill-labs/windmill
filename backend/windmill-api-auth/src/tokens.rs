/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sqlx::PgConnection;
use tracing::Instrument;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    error::{Error, Result},
    utils::rd_string,
    worker::CLOUD_HOSTED,
    DB,
};

use crate::ApiAuthed;

#[derive(serde::Deserialize)]
pub struct NewToken {
    pub label: Option<String>,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
    pub impersonate_email: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub workspace_id: Option<String>,
}

impl NewToken {
    pub fn new(
        label: Option<String>,
        expiration: Option<chrono::DateTime<chrono::Utc>>,
        impersonate_email: Option<String>,
        scopes: Option<Vec<String>>,
        workspace_id: Option<String>,
    ) -> NewToken {
        NewToken { label, expiration, impersonate_email, scopes, workspace_id }
    }
}

pub async fn create_token_internal(
    tx: &mut PgConnection,
    db: &DB,
    authed: &ApiAuthed,
    token_config: NewToken,
) -> Result<String> {
    let token = rd_string(32);

    let is_super_admin = sqlx::query_scalar!(
        "SELECT super_admin FROM password WHERE email = $1",
        authed.email
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(false);
    if *CLOUD_HOSTED {
        let nb_tokens =
            sqlx::query_scalar!("SELECT COUNT(*) FROM token WHERE email = $1", &authed.email)
                .fetch_one(db)
                .await?;
        if nb_tokens.unwrap_or(0) >= 10000 {
            return Err(Error::BadRequest(
                "You have reached the maximum number of tokens (10000) on cloud. Contact support@windmill.dev to increase the limit"
                    .to_string(),
            ));
        }
    }
    sqlx::query!(
        "INSERT INTO token
            (token, email, label, expiration, super_admin, scopes, workspace_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
        token,
        authed.email,
        token_config.label,
        token_config.expiration,
        is_super_admin,
        token_config.scopes.as_ref().map(|x| x.as_slice()),
        token_config.workspace_id,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        authed,
        "users.token.create",
        ActionKind::Create,
        &"global",
        Some(&token[0..10]),
        None,
    )
    .instrument(tracing::info_span!("token", email = &authed.email))
    .await?;

    Ok(token)
}
