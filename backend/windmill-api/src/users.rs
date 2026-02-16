/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

// Re-export everything from windmill-api-users
pub use windmill_api_users::users::*;

use std::sync::Arc;

use crate::db::ApiAuthed;
use crate::secret_backend_ext::rename_vault_secrets_with_prefix;
use argon2::Argon2;
use axum::{
    extract::{Extension, Path},
    routing::post,
    Json, Router,
};
use hyper::StatusCode;
use serde::Deserialize;
use windmill_api_auth::require_super_admin;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::audit::AuditAuthor;
use windmill_common::usernames::VALID_USERNAME;
use windmill_common::{
    error::{self, Error, Result},
    DB,
};

/// Wraps the subcrate's global_service with routes that depend on windmill-api internals.
pub fn global_service() -> Router {
    windmill_api_users::users::global_service()
        .route("/setpassword", post(set_password))
        .route("/set_password_of/:user", post(set_password_of_user))
        .route("/create", post(create_user))
        .route("/rename/:user", post(rename_user))
        .route("/onboarding", post(submit_onboarding_data))
}

/// Wraps the subcrate's make_unauthed_service with routes that depend on windmill-api internals.
pub fn make_unauthed_service() -> Router {
    windmill_api_users::users::make_unauthed_service()
        .route("/reset_password", post(reset_password))
}

async fn create_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<windmill_common::webhook::WebhookShared>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Json(nu): Json<NewUser>,
) -> Result<(StatusCode, String)> {
    crate::users_oss::create_user(authed, db, webhook, argon2, nu).await
}

async fn submit_onboarding_data(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(data): Json<crate::users_oss::OnboardingData>,
) -> Result<String> {
    crate::users_oss::submit_onboarding_data(authed, Extension(db), Json(data)).await
}

async fn set_password(
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    authed: ApiAuthed,
    Json(ep): Json<EditPassword>,
) -> Result<String> {
    let email = authed.email.clone();
    crate::users_oss::set_password(db, argon2, authed, &email, ep).await
}

async fn set_password_of_user(
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Path(email): Path<String>,
    authed: ApiAuthed,
    Json(ep): Json<EditPassword>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    crate::users_oss::set_password(db, argon2, authed, &email, ep).await
}

#[derive(Deserialize)]
struct RenameUser {
    new_username: String,
}

async fn rename_user(
    authed: ApiAuthed,
    Path(user_email): Path<String>,
    Extension(db): Extension<DB>,
    Json(ru): Json<RenameUser>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;

    let mut tx = db.begin().await?;

    let username_conflict = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 and email != $2 UNION SELECT 1 FROM password WHERE username = $1 UNION SELECT 1 FROM pending_user WHERE username = $1)",
        &ru.new_username,
        &user_email
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if username_conflict {
        return Err(Error::BadRequest(format!(
            "username {} already used by another user",
            &ru.new_username
        )));
    }

    if !VALID_USERNAME.is_match(&ru.new_username) {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Usermame can only contain alphanumeric characters and underscores and must start with a letter"
        )));
    }

    sqlx::query!(
        "UPDATE password SET username = $1 WHERE email = $2",
        ru.new_username,
        user_email
    )
    .execute(&mut *tx)
    .await?;

    let workspace_usernames = sqlx::query!(
        "SELECT workspace_id, username FROM usr WHERE email = $1",
        &user_email
    )
    .fetch_all(&mut *tx)
    .await?;

    for w_u in workspace_usernames {
        if ru.new_username == w_u.username {
            continue;
        }
        update_username_in_workpsace(
            &mut tx,
            &db,
            &user_email,
            &w_u.username,
            &ru.new_username,
            &w_u.workspace_id,
        )
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "users.rename",
        ActionKind::Update,
        "global",
        Some(&user_email),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!(
        "updated username of user {} to {}",
        &user_email, &ru.new_username
    ))
}

async fn update_username_in_workpsace<'c>(
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    db: &DB,
    email: &str,
    old_username: &str,
    new_username: &str,
    w_id: &str,
) -> error::Result<()> {
    // ---- instance and workspace users ----
    sqlx::query!(
        "UPDATE usr SET username = $1 WHERE email = $2",
        new_username,
        email
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE usr_to_group SET usr = $1 WHERE usr = $2",
        new_username,
        old_username
    )
    .execute(&mut **tx)
    .await?;

    // ---- v2_job ----
    sqlx::query!(
        r#"UPDATE v2_job SET runnable_path = REGEXP_REPLACE(runnable_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE runnable_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE v2_job SET trigger = REGEXP_REPLACE(trigger,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE trigger LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job SET permissioned_as = ('u/' || $1) WHERE permissioned_as = ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- v2_job_queue ----
    sqlx::query!(
        "UPDATE v2_job_queue SET canceled_by = $1 WHERE canceled_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- v2_job_completed ----
    sqlx::query!(
        "UPDATE v2_job_completed SET canceled_by = $1 WHERE canceled_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- resources----
    sqlx::query!(
        r#"UPDATE resource SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE resource_type SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE resource SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE resource SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- variables ----

    // Handle Vault secret renames before updating paths in DB
    let old_prefix = format!("u/{}/", old_username);
    let new_prefix = format!("u/{}/", new_username);

    // Fetch all Vault-stored secret variables under this user's path
    let vault_secrets: Vec<(String, String)> = sqlx::query!(
        r#"SELECT path, value FROM variable
           WHERE path LIKE ('u/' || $1 || '/%')
           AND workspace_id = $2
           AND is_secret = true
           AND value LIKE '$vault:%'"#,
        old_username,
        w_id
    )
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|r| (r.path, r.value))
    .collect();

    // Rename secrets in Vault and get the new values
    let vault_updates =
        rename_vault_secrets_with_prefix(db, w_id, &old_prefix, &new_prefix, vault_secrets).await?;

    // Update the values in the DB for renamed Vault secrets (using OLD path, before path update)
    for (old_path, new_value) in vault_updates {
        sqlx::query!(
            "UPDATE variable SET value = $1 WHERE path = $2 AND workspace_id = $3",
            new_value,
            old_path,
            w_id
        )
        .execute(&mut **tx)
        .await?;
    }

    // Now update the paths in the database
    sqlx::query!(
        r#"UPDATE variable SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE workspace_integrations SET resource_path = REGEXP_REPLACE(resource_path, 'u/' || $2 || '/(.*)', 'u/' || $1 || '/\1') WHERE resource_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE variable SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- scripts ----
    sqlx::query!(
        r#"UPDATE script SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE script SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE script SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- flows ----
    sqlx::query!(
        r#"INSERT INTO flow
            (workspace_id, path, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at)
        SELECT workspace_id, REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1'), summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at
            FROM flow
            WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE flow_version SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET flow_path = REGEXP_REPLACE(flow_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE flow_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET app_path = REGEXP_REPLACE(app_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE app_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET runnable_path = REGEXP_REPLACE(runnable_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE runnable_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE asset SET usage_path = REGEXP_REPLACE(usage_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE usage_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE flow_node SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "DELETE FROM flow WHERE path LIKE ('u/' || $1 || '/%') AND workspace_id = $2",
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE flow SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- draft ----
    sqlx::query!(
        r#"UPDATE draft SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE draft SET value = to_json(jsonb_set(to_jsonb(value), ARRAY['path'], to_jsonb(REGEXP_REPLACE(value->>'path','u/' || $2 || '/(.*)','u/' || $1 || '/\1')))) WHERE value->>'path' LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    // ---- app ----
    sqlx::query!(
        r#"UPDATE app SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE app SET policy = jsonb_set(policy, ARRAY['on_behalf_of'], to_jsonb('u/' || $1)) WHERE policy->>'on_behalf_of' = ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE app SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- app_version ----

    sqlx::query!(
        "UPDATE app_version SET created_by = $1 WHERE created_by = $2 AND EXISTS (SELECT 1 FROM app WHERE workspace_id = $3 AND app.id = app_version.app_id)",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- schedules ----

    sqlx::query!(
        r#"UPDATE schedule SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE schedule SET script_path = REGEXP_REPLACE(script_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE script_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE schedule SET edited_by = $1 WHERE edited_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE schedule SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- group_ ----

    sqlx::query!(
        "UPDATE group_ SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- folders ----

    sqlx::query!(
        "UPDATE folder SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE folder SET owners = ARRAY_REPLACE(owners, 'u/' || $2, 'u/' || $1) WHERE  ('u/' || $2) = ANY(owners) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE folder SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- input ----

    sqlx::query!(
        "UPDATE input SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    // ---- favorite ----

    sqlx::query!(
        "UPDATE favorite SET usr = $1 WHERE usr = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    // ---- capture ----

    sqlx::query!(
        "UPDATE capture SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    // ---- token ----

    sqlx::query!(
        "UPDATE token SET owner = ('u/' || $1) WHERE owner = ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    sqlx::query!(
        r#"UPDATE token SET scopes = array(select regexp_replace(unnest(scopes), 'run:([^/]+)/u/' || $2 || '/(.+)', 'run:\1/u/' || $1 || '/\2')) WHERE EXISTS (SELECT 1 FROM UNNEST(scopes) scope WHERE scope LIKE ('run:%/u/' || $2 || '/%')) AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- raw_app ----

    sqlx::query!(
        "UPDATE raw_app SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Reset password using a token
async fn reset_password(
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Json(req): Json<ResetPassword>,
) -> Result<Json<PasswordResetResponse>> {
    let mut tx = db.begin().await?;

    // Find the token and verify it's not expired
    let magic_link = sqlx::query!(
        "SELECT email FROM magic_link WHERE token = $1 AND expiration > NOW()",
        &req.token
    )
    .fetch_optional(&mut *tx)
    .await?;

    let email = match magic_link {
        Some(link) => link.email,
        None => {
            return Err(Error::BadRequest(
                "Invalid or expired password reset token".to_string(),
            ))
        }
    };

    // Hash the new password
    let password_hash = crate::users_oss::hash_password(argon2, req.new_password)?;

    // Update the password
    let rows_updated = sqlx::query!(
        "UPDATE password SET password_hash = $1 WHERE email = $2 AND login_type = 'password'",
        &password_hash,
        &email
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if rows_updated == 0 {
        return Err(Error::BadRequest(
            "Unable to update password. User may not exist or may use a different login method."
                .to_string(),
        ));
    }

    // Delete the used token and any other tokens for this email
    sqlx::query!("DELETE FROM magic_link WHERE email = $1", &email)
        .execute(&mut *tx)
        .await?;

    // Invalidate all existing sessions for this user
    sqlx::query!(
        "DELETE FROM token WHERE email = $1 AND label = 'session'",
        &email
    )
    .execute(&mut *tx)
    .await?;

    // Audit log
    let audit_author = AuditAuthor {
        email: email.clone(),
        username: email.clone(),
        username_override: None,
        token_prefix: None,
    };

    audit_log(
        &mut *tx,
        &audit_author,
        "users.password_reset",
        ActionKind::Update,
        "global",
        Some(&email),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(Json(PasswordResetResponse {
        message: "Password has been reset successfully. You can now log in with your new password."
            .to_string(),
    }))
}
