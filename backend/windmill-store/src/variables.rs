/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_api_auth::{
    build_scope_path_predicate, check_scopes, maybe_refresh_folders, require_owner_of_path,
    ApiAuthed,
};
use windmill_common::db::DB;
use windmill_common::workspaces::{check_deploy_rules, RuleCheckResult};

use crate::secret_backend_ext::{
    delete_secret_from_backend, get_secret_value, is_external_stored_value, is_vault_stored_value,
    rename_vault_secret, store_secret_value,
};
use windmill_common::utils::{escape_ilike_pattern, BulkDeleteRequest};
use windmill_common::webhook::{WebhookMessage, WebhookShared};

use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures::future::try_join_all;
use hyper::StatusCode;
use serde_json::Value;

use windmill_audit::audit_oss::{audit_log, AuditAuthorable};
use windmill_audit::ActionKind;
use windmill_common::{
    db::{DbWithOptAuthed, UserDB},
    error::{Error, JsonResult, Result},
    scripts::ScriptHash,
    user_drafts::{
        decrypt_draft_secret_value, delete_all_drafts_for_path, delete_own_draft_for_path,
        fetch_draft_only, fetch_draft_only_list_rows, maybe_overlay_draft, UserDraftItemKind,
        WithDraftOverlay, ENCRYPTED_DRAFT_PREFIX,
    },
    utils::{not_found_if_none, paginate, Pagination, StripPath, WarnAfterExt},
    variables::{
        build_crypt, get_reserved_variables, ContextualVariable, CreateVariable, ListableVariable,
    },
    worker::CLOUD_HOSTED,
};

use crate::var_resource_cache::{
    auth_identity, cache_variable, get_cached_variable, CachedVariable,
};
use lazy_static::lazy_static;
use serde::Deserialize;
use sqlx::{Acquire, Postgres, Transaction};
use windmill_common::variables::encrypt;
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

lazy_static! {
    pub static ref SECRET_SALT: Option<String> = std::env::var("SECRET_SALT").ok();
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_variables))
        .route("/list_contextual", get(list_contextual_variables))
        .route("/get/{*path}", get(get_variable))
        .route("/get_value/{*path}", get(get_value))
        .route("/exists/{*path}", get(exists_variable))
        .route("/update/{*path}", post(update_variable))
        .route("/delete/{*path}", delete(delete_variable))
        .route("/delete_bulk", delete(delete_variables_bulk))
        .route("/create", post(create_variable))
        .route("/encrypt", post(encrypt_value))
}

async fn list_contextual_variables(
    Path(w_id): Path<String>,
    ApiAuthed { username, email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<Vec<ContextualVariable>> {
    Ok(Json(
        get_reserved_variables(
            &db.into(),
            &w_id,
            "q1A0qcPuO00yxioll7iph76N9CJDqn",
            &email,
            &username,
            "017e0ad5-f499-73b6-5488-92a61c5196dd",
            format!("u/{username}").as_str(),
            Some("u/user/script_path".to_string()),
            Some("017e0ad5-f499-73b6-5488-92a61c5196dd".to_string()),
            Some("u/user/encapsulating_flow_path".to_string()),
            Some("u/user/triggering_flow_path".to_string()),
            Some("c".to_string()),
            Some("017e0ad5-f499-73b6-5488-92a61c5196dd".to_string()),
            Some("017e0ad5-f499-73b6-5488-92a61c5196dd".to_string()),
            Some(chrono::offset::Utc::now()),
            Some(ScriptHash(1234567890)),
            None,
            None,
        )
        .await
        .to_vec(),
    ))
}

#[derive(Deserialize)]
struct ListVariableQuery {
    pub path_start: Option<String>,
    pub path: Option<String>,
    pub description: Option<String>,
    // filter by matching the non-encrypted value (for non-secrets only)
    pub value: Option<String>,
    pub broad_filter: Option<String>,
    pub label: Option<String>,
    /// When true, append per-user draft-only rows; picker callers leave it off
    /// to stay deployed-only. See list synthesis in scripts.rs.
    pub include_draft_only: Option<bool>,
}

async fn list_variables(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(lq): Query<ListVariableQuery>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<ListableVariable>> {
    use sql_builder::{bind::Bind, SqlBuilder};

    let (per_page, offset) = paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("variable")
        .fields(&[
            "variable.workspace_id",
            "variable.path",
            "CASE WHEN is_secret IS TRUE THEN null ELSE variable.value::text END as value",
            "is_secret",
            "variable.description",
            "variable.extra_perms",
            "account",
            "is_oauth",
            "(now() > account.expires_at) as is_expired",
            "account.refresh_error",
            "resource.path IS NOT NULL as is_linked",
            "account.refresh_token != '' as is_refreshed",
            "variable.expires_at",
            "variable.labels",
            "folder_labels(variable.workspace_id, variable.path) as inherited_labels",
            "ws_specific.path IS NOT NULL as ws_specific",
            "variable.edited_at",
            "variable.edited_by",
        ])
        // Scalar EXISTS flags the authed user's per-user draft; see resources.rs.
        .field(
            &"EXISTS(SELECT 1 FROM draft WHERE draft.workspace_id = variable.workspace_id \
              AND draft.path = variable.path AND draft.typ = 'variable' \
              AND draft.email = ?) as is_draft"
                .bind(&authed.email),
        )
        .left()
        .join("account")
        .on("variable.account = account.id AND account.workspace_id = ?".bind(&w_id))
        .left()
        .join("resource")
        .on("resource.path = variable.path AND resource.workspace_id = ?".bind(&w_id))
        .left()
        .join("ws_specific")
        .on("ws_specific.path = variable.path AND ws_specific.workspace_id = variable.workspace_id AND ws_specific.item_kind = 'variable'")
        .and_where("variable.workspace_id = ?".bind(&w_id))
        .and_where("variable.path NOT LIKE 'u/' || ? || '/secret_arg/%'".bind(&authed.username))
        .order_by("path", false)
        .limit(per_page)
        .offset(offset)
        .clone();

    if let Some(path_start) = &lq.path_start {
        sqlb.and_where_like_left("variable.path", path_start);
    }

    if let Some(path) = &lq.path {
        sqlb.and_where_eq("variable.path", "?".bind(path));
    }

    if let Some(description) = &lq.description {
        let pat = format!("%{}%", escape_ilike_pattern(description));
        sqlb.and_where("variable.description ILIKE ?".bind(&pat));
    }

    if let Some(value) = &lq.value {
        // Only filter on non-secret variables' value
        let pat = format!("%{}%", escape_ilike_pattern(value));
        sqlb.and_where("(is_secret = FALSE AND variable.value ILIKE ?)".bind(&pat));
    }

    if let Some(broad_filter) = &lq.broad_filter {
        let pat = format!("%{}%", escape_ilike_pattern(broad_filter));
        sqlb.and_where(
            "(variable.path ILIKE ? OR variable.description ILIKE ? OR (is_secret = FALSE AND variable.value ILIKE ?))"
                .bind(&pat).bind(&pat).bind(&pat)
        );
    }

    if let Some(label) = &lq.label {
        for l in label.split(',') {
            sqlb.and_where(
                "(variable.labels @> ARRAY[?] OR folder_labels(variable.workspace_id, variable.path) @> ARRAY[?])"
                    .bind(&l.trim())
                    .bind(&l.trim()),
            );
        }
    }

    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let allowed = build_scope_path_predicate(&authed, "variables", "read");
    let mut rows = sqlx::query_as::<_, ListableVariable>(&sql)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .filter(|r| allowed(&r.path))
        .collect::<Vec<_>>();

    tx.commit().await?;

    // Append the authed user's draft-only variables; see scripts.rs.
    if lq.include_draft_only.unwrap_or(false)
        && !authed.is_operator
        && offset == 0
        && lq.path_start.is_none()
        && lq.path.is_none()
        && lq.description.is_none()
        && lq.value.is_none()
        && lq.broad_filter.is_none()
        && lq.label.is_none()
    {
        let draft_only_rows =
            fetch_draft_only_list_rows(&db, &w_id, &authed.email, UserDraftItemKind::Variable)
                .await?;

        for row in draft_only_rows {
            let v: serde_json::Value =
                serde_json::from_str(row.value.0.get()).unwrap_or(serde_json::Value::Null);
            // VariableEditor's `VariableState`: { path, variable: { value, is_secret, description }, labels?, wsSpecific }
            let path = v
                .get("path")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            if path.is_empty() || !allowed(&path) {
                continue;
            }
            let variable = v
                .get("variable")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let is_secret = variable
                .get("is_secret")
                .and_then(|x| x.as_bool())
                .unwrap_or(false);
            let description = variable
                .get("description")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            // Secret variables never expose their value in the list response, even from a draft.
            let value = if is_secret {
                None
            } else {
                variable
                    .get("value")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
            };
            let labels = v.get("labels").and_then(|x| {
                x.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
            });
            let ws_specific = v.get("wsSpecific").and_then(|x| x.as_bool());

            rows.push(ListableVariable {
                workspace_id: w_id.clone(),
                path,
                value,
                is_secret,
                description,
                extra_perms: serde_json::Value::Object(serde_json::Map::new()),
                account: None,
                is_oauth: None,
                is_expired: None,
                is_refreshed: None,
                refresh_error: None,
                is_linked: None,
                expires_at: None,
                labels,
                // No deployed row to inherit folder labels from.
                inherited_labels: None,
                ws_specific,
                edited_at: Some(row.created_at),
                edited_by: None,
                draft_only: Some(true),
                // Synthesized rows are the authed user's draft.
                is_draft: Some(true),
            });
        }
    }

    Ok(Json(rows))
}

// `get_draft` inlined rather than flattened (axum query bool quirk); see GetScriptByPathQuery in scripts.rs.
#[derive(Deserialize)]
struct GetVariableQuery {
    decrypt_secret: Option<bool>,
    include_encrypted: Option<bool>,
    #[serde(default)]
    get_draft: bool,
}

async fn get_variable(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Query(q): Query<GetVariableQuery>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<WithDraftOverlay> {
    let path = path.to_path();
    check_scopes(&authed, || format!("variables:read:{}", path))?;

    let mut tx = user_db.begin(&authed).await?;

    let variable_o = sqlx::query_as::<_, ListableVariable>(
        "SELECT variable.workspace_id, variable.path, variable.value, variable.is_secret,
        variable.description, variable.extra_perms, variable.account, variable.is_oauth,
        variable.expires_at, variable.labels,
        folder_labels(variable.workspace_id, variable.path) as inherited_labels,
        variable.edited_at, variable.edited_by,
        (now() > account.expires_at) as is_expired, account.refresh_error,
        resource.path IS NOT NULL as is_linked,
        account.refresh_token != '' as is_refreshed,
        ws_specific.path IS NOT NULL as ws_specific
        FROM variable
        LEFT JOIN account ON variable.account = account.id
        LEFT JOIN resource ON resource.path = variable.path AND resource.workspace_id = $2
        LEFT JOIN ws_specific ON ws_specific.path = variable.path AND ws_specific.workspace_id = $2 AND ws_specific.item_kind = 'variable'
        WHERE variable.path = $1 AND variable.workspace_id = $2
        LIMIT 1",
    )
    .bind(&path)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;

    let variable = if let Some(variable) = variable_o {
        variable
    } else if q.get_draft {
        // No deployed row + `get_draft`: fall back to the draft (see scripts.rs).
        // Drop the user_db tx first since `fetch_draft_only` runs on `db`.
        tx.commit().await?;
        if let Some(overlay) =
            fetch_draft_only(&db, &w_id, &authed.email, UserDraftItemKind::Variable, path).await?
        {
            return Ok(Json(overlay));
        }
        explain_variable_perm_error(&path, &w_id, &db).await?;
        unreachable!()
    } else {
        explain_variable_perm_error(&path, &w_id, &db).await?;
        unreachable!()
    };

    let decrypt_secret = q.decrypt_secret.unwrap_or(true);

    let r = if variable.is_secret {
        if decrypt_secret {
            audit_log(
                &mut *tx,
                &authed,
                "variables.decrypt_secret",
                ActionKind::Execute,
                &w_id,
                Some(&variable.path),
                None,
            )
            .await?;
        }

        let value = variable.value.unwrap_or_else(|| "".to_string());
        ListableVariable {
            value: if variable.is_expired.unwrap_or(false) && variable.account.is_some() {
                #[cfg(feature = "oauth2")]
                {
                    let refresh_tx = db
                        .begin()
                        .await
                        .map_err(|e| Error::InternalErr(e.to_string()))?;
                    Some(
                        crate::oauth_refresh_oss::_refresh_token(
                            refresh_tx,
                            &variable.path,
                            &w_id,
                            variable.account.unwrap(),
                            &db,
                        )
                        .await?,
                    )
                }
                #[cfg(not(feature = "oauth2"))]
                return Err(Error::internal_err("Require oauth2 feature".to_string()));
            } else if !value.is_empty() && decrypt_secret {
                let _ = tx.commit().await;
                // Use secret backend for decryption (supports both DB and Vault)
                Some(get_secret_value(&db, &w_id, &variable.path, &value).await?)
            } else if q.include_encrypted.unwrap_or(false) {
                Some(value)
            } else {
                None
            },
            ..variable
        }
    } else {
        variable
    };

    let overlay = maybe_overlay_draft(
        &db,
        &w_id,
        &authed.email,
        UserDraftItemKind::Variable,
        path,
        q.get_draft,
        r,
    )
    .await?;
    Ok(Json(overlay))
}

#[derive(Deserialize)]
struct GetValueQuery {
    allow_cache: Option<bool>,
}
async fn get_value(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(q): Query<GetValueQuery>,
) -> JsonResult<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("variables:read:{}", path))?;
    let userdb_authed = DbWithOptAuthed::from_authed(&authed, db.clone(), Some(user_db.clone()));

    return get_value_internal(&userdb_authed, &w_id, &path, q.allow_cache.unwrap_or(false))
        .warn_after_seconds(10)
        .await
        .map(Json);
}

async fn explain_variable_perm_error(
    path: &str,
    w_id: &str,
    db: &sqlx::Pool<Postgres>,
) -> windmill_common::error::Result<()> {
    let extra_perms = sqlx::query_scalar!(
        "SELECT extra_perms from variable WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Variable {} not found", path)))?;
    if path.starts_with("f/") {
        let folder = path.split("/").nth(1).ok_or_else(|| {
            Error::BadRequest(format!(
                "path {} should have at least 2 components separated by /",
                path
            ))
        })?;
        let folder_extra_perms = sqlx::query_scalar!(
            "SELECT extra_perms from folder WHERE name = $1 AND workspace_id = $2",
            folder,
            w_id
        )
        .fetch_optional(db)
        .await?;
        return Err(Error::NotAuthorized(format!(
            "Variable exists but you don't have access to it:\nvariable perms: {}\nfolder perms: {}",
            serde_json::to_string_pretty(&extra_perms).unwrap_or_default(), serde_json::to_string_pretty(&folder_extra_perms).unwrap_or_default()
        )));
    } else {
        return Err(Error::NotAuthorized(format!(
            "Variable exists but you don't have access to it:\nvariable perms: {}",
            serde_json::to_string_pretty(&extra_perms).unwrap_or_default()
        )));
    }
}

async fn exists_variable(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM variable WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn check_path_conflict(db: &DB, w_id: &str, path: &str) -> Result<()> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM variable WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);
    if exists {
        return Err(Error::BadRequest(format!(
            "Variable {} already exists",
            path
        )));
    }
    return Ok(());
}

/// Reject a secret value flagged as already-encrypted (`already_encrypted=true`)
/// that is not actually workspace-key ciphertext — e.g. plaintext mistakenly
/// pushed as encrypted. Storing plaintext in the encrypted `value` column
/// silently bricks the variable: every later read fails to decrypt it.
///
/// The check is purely structural and never decrypts, so it cannot act as a
/// decryption/padding oracle for a caller who can write but not read secrets.
/// `encrypt` (AES-256-CBC) always yields standard base64 decoding to a non-zero
/// multiple of the 16-byte block size; anything else cannot be our ciphertext.
/// Values stored by an external backend ($vault:/$aws_sm:/$azure_kv: markers)
/// are not workspace ciphertext and are passed through untouched.
fn validate_already_encrypted_secret(path: &str, value: &str) -> Result<()> {
    if is_external_stored_value(value) {
        return Ok(());
    }
    let looks_like_ciphertext = STANDARD
        .decode(value)
        .map(|bytes| !bytes.is_empty() && bytes.len() % 16 == 0)
        .unwrap_or(false);
    if !looks_like_ciphertext {
        return Err(Error::BadRequest(format!(
            "Variable {path} was sent as already-encrypted (already_encrypted=true) but its \
             value is not valid workspace-encrypted ciphertext. To push a plaintext secret, \
             send it without already_encrypted (CLI: use --plain-secrets) so it gets encrypted."
        )));
    }
    Ok(())
}

async fn create_variable(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Query(AlreadyEncrypted { already_encrypted }): Query<AlreadyEncrypted>,
    Json(variable): Json<CreateVariable>,
) -> Result<(StatusCode, String)> {
    check_scopes(&authed, || format!("variables:write:{}", variable.path))?;
    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    if *CLOUD_HOSTED {
        let nb_variables = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM variable WHERE workspace_id = $1",
            &w_id
        )
        .fetch_one(&db)
        .await?;
        if nb_variables.unwrap_or(0) >= 10000 {
            return Err(Error::BadRequest(
                    "You have reached the maximum number of variables (10000) on cloud. Check your usage in Workspace Settings > General > Cloud Quotas. Contact support@windmill.dev to increase the limit"
                        .to_string(),
                ));
        }
    }
    let authed = maybe_refresh_folders(&variable.path, &w_id, authed, &db).await;

    check_path_conflict(&db, &w_id, &variable.path).await?;
    let value = if variable.is_secret && !already_encrypted.unwrap_or(false) {
        // A restored draft sends the `$encrypted:` marker as-is; decrypt it back
        // (validating against the workspace key) before the secret backend re-stores it.
        let plain = if variable.value.starts_with(ENCRYPTED_DRAFT_PREFIX) {
            decrypt_draft_secret_value(&db, &w_id, &variable.value).await?
        } else {
            variable.value.clone()
        };
        // Use secret backend for encryption (supports both DB and Vault)
        store_secret_value(&db, &w_id, &variable.path, &plain).await?
    } else {
        if variable.is_secret {
            // already_encrypted == true: value is stored verbatim, so it must be
            // ciphertext and not plaintext mislabeled as encrypted.
            validate_already_encrypted_secret(&variable.path, &variable.value)?;
        }
        variable.value
    };

    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "INSERT INTO variable
            (workspace_id, path, value, is_secret, description, account, is_oauth, expires_at, labels, edited_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        &w_id,
        variable.path,
        value,
        variable.is_secret,
        variable.description,
        variable.account,
        variable.is_oauth.unwrap_or(false),
        variable.expires_at,
        variable.labels.as_deref() as Option<&[String]>,
        &authed.username
    )
    .execute(&mut *tx)
    .await?;

    if variable.ws_specific.unwrap_or(false) {
        sqlx::query!(
            "INSERT INTO ws_specific (workspace_id, item_kind, path) VALUES ($1, 'variable', $2) ON CONFLICT DO NOTHING",
            w_id,
            variable.path,
        )
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "variables.create",
        ActionKind::Create,
        &w_id,
        Some(&variable.path),
        None,
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Variable { path: variable.path.clone(), parent_path: None },
        Some(format!("Variable '{}' created", variable.path.clone())),
        true,
        None,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::CreateVariable { workspace: w_id, path: variable.path.clone() },
    );

    Ok((
        StatusCode::CREATED,
        format!("variable {} created", variable.path),
    ))
}

async fn encrypt_value(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(variable): Json<String>,
) -> Result<String> {
    let mc = build_crypt(&db, &w_id).await?;
    let value = encrypt(&mc, &variable);

    Ok(value)
}

async fn delete_variable(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();

    check_scopes(&authed, || format!("variables:write:{}", path))?;
    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    // Check if variable is a secret before deleting (for Vault cleanup)
    let is_secret = sqlx::query_scalar!(
        "SELECT is_secret FROM variable WHERE path = $1 AND workspace_id = $2",
        path,
        &w_id
    )
    .fetch_optional(&db)
    .await?
    .unwrap_or(false);

    let mut tx = user_db.begin(&authed).await?;

    // Capture data for trashbin before deleting
    let trash_var: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM variable t WHERE path = $1 AND workspace_id = $2",
    )
    .bind(path)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;

    let trash_linked_resource: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM resource t WHERE path = $1 AND workspace_id = $2",
    )
    .bind(path)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM variable WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .execute(&mut *tx)
    .await?;
    // Clean up any ws_specific row for the linked resource at the same path
    // before deleting the resource itself — symmetric with the cleanup
    // delete_resource does for linked variables. Without this, a resource
    // later recreated at the same path would inherit a stale ws_specific flag.
    sqlx::query!(
        "DELETE FROM ws_specific
         WHERE workspace_id = $1 AND item_kind = 'resource' AND path = $2",
        w_id,
        path
    )
    .execute(&mut *tx)
    .await?;
    let deleted_linked_resource = sqlx::query_scalar!(
        "DELETE FROM resource WHERE path = $1 AND workspace_id = $2 RETURNING path",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(var_data) = trash_var {
        let mut trash_data = serde_json::json!({"row": var_data});
        if let Some(linked) = trash_linked_resource {
            trash_data["linked_resource"] = linked;
        }
        windmill_common::trashbin::move_to_trash(
            &mut *tx,
            &w_id,
            "variable",
            path,
            trash_data,
            &authed.username,
        )
        .await?;
    }

    sqlx::query!(
        "DELETE FROM ws_specific WHERE workspace_id = $1 AND item_kind = 'variable' AND path = $2",
        w_id,
        path
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "variables.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    // Variable gone for everyone: wipe ALL users' drafts at this path (see resources.rs).
    // Resource included because variables cascade-delete the linked resource at the same path.
    delete_all_drafts_for_path(&db, &w_id, UserDraftItemKind::Variable, path).await?;
    if deleted_linked_resource.is_some() {
        delete_all_drafts_for_path(&db, &w_id, UserDraftItemKind::Resource, path).await?;
    }

    // If variable was a secret, also delete from Vault backend (if configured)
    if is_secret {
        delete_secret_from_backend(&db, &w_id, path).await?;
    }

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Variable { path: path.to_string(), parent_path: Some(path.to_string()) },
        Some(format!("Variable '{}' deleted", path)),
        true,
        None,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::DeleteVariable { workspace: w_id.clone(), path: path.to_owned() },
    );

    if deleted_linked_resource.is_some() {
        handle_deployment_metadata(
            &authed.email,
            &authed.username,
            &db,
            &w_id,
            DeployedObject::Resource {
                path: path.to_string(),
                parent_path: Some(path.to_string()),
            },
            Some(format!(
                "Resource '{}' deleted (linked variable deleted)",
                path
            )),
            true,
            None,
        )
        .await?;

        webhook.send_message(
            w_id.clone(),
            WebhookMessage::DeleteResource { workspace: w_id, path: path.to_owned() },
        );
    }

    Ok(format!("variable {} deleted", path))
}

async fn delete_variables_bulk(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(request): Json<BulkDeleteRequest>,
) -> JsonResult<Vec<String>> {
    for path in &request.paths {
        check_scopes(&authed, || format!("variables:write:{}", path))?;
    }

    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    // Query which paths are secrets before deletion (for Vault cleanup)
    let secret_paths: Vec<String> = sqlx::query_scalar!(
        "SELECT path FROM variable WHERE path = ANY($1) AND workspace_id = $2 AND is_secret = true",
        &request.paths,
        &w_id
    )
    .fetch_all(&db)
    .await?;

    let mut tx = user_db.begin(&authed).await?;

    // Capture variables for trashbin per path before bulk delete
    for path in &request.paths {
        let trash_var: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT to_jsonb(t) FROM variable t WHERE path = $1 AND workspace_id = $2",
        )
        .bind(path)
        .bind(&w_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(var_data) = trash_var {
            let trash_linked: Option<serde_json::Value> = sqlx::query_scalar(
                "SELECT to_jsonb(t) FROM resource t WHERE path = $1 AND workspace_id = $2",
            )
            .bind(path)
            .bind(&w_id)
            .fetch_optional(&mut *tx)
            .await?;

            let mut trash_data = serde_json::json!({"row": var_data});
            if let Some(linked) = trash_linked {
                trash_data["linked_resource"] = linked;
            }
            windmill_common::trashbin::move_to_trash(
                &mut *tx,
                &w_id,
                "variable",
                path,
                trash_data,
                &authed.username,
            )
            .await?;
        }
    }

    let deleted_paths = sqlx::query_scalar!(
        "DELETE FROM variable WHERE path = ANY($1) AND workspace_id = $2 RETURNING path",
        &request.paths,
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    // Mirror single delete_variable: clean the linked-resource ws_specific
    // markers BEFORE deleting the resource rows so they don't survive as
    // orphans. A resource later created at the same path would otherwise
    // inherit a stale ws_specific flag.
    sqlx::query!(
        "DELETE FROM ws_specific WHERE workspace_id = $1 AND item_kind = 'resource' AND path = ANY($2)",
        w_id,
        &deleted_paths
    )
    .execute(&mut *tx)
    .await?;
    let deleted_resource_paths = sqlx::query_scalar!(
        "DELETE FROM resource WHERE path = ANY($1) AND workspace_id = $2 RETURNING path",
        &deleted_paths,
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM ws_specific WHERE workspace_id = $1 AND item_kind = 'variable' AND path = ANY($2)",
        w_id,
        &deleted_paths
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "variables.delete_bulk",
        ActionKind::Delete,
        &w_id,
        Some(&deleted_paths.join(", ")),
        None,
    )
    .await?;

    tx.commit().await?;

    // Wipe ALL users' drafts at these paths (and linked resources); see delete_variable.
    for path in &deleted_paths {
        delete_all_drafts_for_path(&db, &w_id, UserDraftItemKind::Variable, path).await?;
    }
    for path in &deleted_resource_paths {
        delete_all_drafts_for_path(&db, &w_id, UserDraftItemKind::Resource, path).await?;
    }

    // Delete secrets from Vault backend (if configured)
    for path in &secret_paths {
        if deleted_paths.contains(path) {
            delete_secret_from_backend(&db, &w_id, path).await?;
        }
    }

    try_join_all(deleted_paths.iter().map(|path| {
        handle_deployment_metadata(
            &authed.email,
            &authed.username,
            &db,
            &w_id,
            DeployedObject::Variable {
                path: path.to_string(),
                parent_path: Some(path.to_string()),
            },
            Some(format!("Variable '{}' deleted", path)),
            true,
            None,
        )
    }))
    .await?;

    for path in &deleted_paths {
        webhook.send_message(
            w_id.clone(),
            WebhookMessage::DeleteVariable { workspace: w_id.clone(), path: path.to_owned() },
        );
    }

    Ok(Json(deleted_paths))
}

#[derive(Deserialize)]
struct EditVariable {
    path: Option<String>,
    value: Option<String>,
    is_secret: Option<bool>,
    description: Option<String>,
    account: Option<i32>,
    labels: Option<Vec<String>>,
    ws_specific: Option<bool>,
}

#[derive(Deserialize)]
struct AlreadyEncrypted {
    already_encrypted: Option<bool>,
}

async fn update_variable(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Query(AlreadyEncrypted { already_encrypted }): Query<AlreadyEncrypted>,
    Json(ns): Json<EditVariable>,
) -> Result<String> {
    use sql_builder::prelude::*;

    if let RuleCheckResult::Blocked(msg) = check_deploy_rules(
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.groups,
        authed.is_admin,
        &db,
    )
    .await?
    {
        return Err(Error::PermissionDenied(msg));
    }

    let path = path.to_path();
    check_scopes(&authed, || format!("variables:write:{}", path))?;
    // A rename moves the (possibly secret) variable to ns.path, so the
    // destination must also be within the token's write scope, not just the
    // source path.
    if let Some(npath) = ns.path.as_deref() {
        check_scopes(&authed, || format!("variables:write:{}", npath))?;
    }
    let authed = maybe_refresh_folders(&path, &w_id, authed, &db).await;

    let mut sqlb = SqlBuilder::update_table("variable");
    let mut has_sql_updates = false;
    sqlb.and_where_eq("path", "?".bind(&path));
    sqlb.and_where_eq("workspace_id", "?".bind(&w_id));

    if let Some(npath) = &ns.path {
        sqlb.set_str("path", npath);
        has_sql_updates = true;
    }
    let ns_value_is_none = ns.value.is_none();
    // Determine the target path for storing secrets (use new path if provided)
    let target_path = ns.path.as_deref().unwrap_or(path);
    if let Some(nvalue) = ns.value.clone() {
        let is_secret = if ns.is_secret.is_some() {
            ns.is_secret.unwrap()
        } else {
            sqlx::query_scalar!(
                "SELECT is_secret from variable WHERE path = $1 AND workspace_id = $2",
                &path,
                &w_id
            )
            .fetch_optional(&db)
            .await?
            .unwrap_or(false)
        };

        let value = if is_secret && !already_encrypted.unwrap_or(false) {
            // Decrypt a restored draft's `$encrypted:` marker before re-storing; see create_variable.
            let plain = if nvalue.starts_with(ENCRYPTED_DRAFT_PREFIX) {
                decrypt_draft_secret_value(&db, &w_id, &nvalue).await?
            } else {
                nvalue
            };
            // Use secret backend for encryption (supports both DB and Vault)
            // Store at target_path (new path if renaming, otherwise current path)
            store_secret_value(&db, &w_id, target_path, &plain).await?
        } else {
            if is_secret {
                // already_encrypted == true: value is stored verbatim, so it must
                // be ciphertext and not plaintext mislabeled as encrypted.
                validate_already_encrypted_secret(target_path, &nvalue)?;
            }
            nvalue
        };
        sqlb.set_str("value", &value);
        has_sql_updates = true;
    }

    if let Some(desc) = ns.description {
        sqlb.set_str("description", &desc);
        has_sql_updates = true;
    }

    if let Some(account_id) = ns.account {
        sqlb.set_str("account", account_id);
        has_sql_updates = true;
    }

    if let Some(nbool) = ns.is_secret {
        let old_secret = sqlx::query_scalar!(
            "SELECT is_secret from variable WHERE path = $1 AND workspace_id = $2",
            &path,
            &w_id
        )
        .fetch_optional(&db)
        .await?
        .unwrap_or(false);
        if old_secret != nbool && ns_value_is_none {
            return Err(Error::BadRequest(
                "cannot change is_secret without updating value too".to_string(),
            ));
        }
        sqlb.set_str("is_secret", nbool);
        has_sql_updates = true;
    }

    // Get old account_id if we're updating the account field
    let old_account_id = if ns.account.is_some() {
        sqlx::query_scalar!(
            "SELECT account FROM variable WHERE path = $1 AND workspace_id = $2",
            &path,
            &w_id
        )
        .fetch_optional(&db)
        .await?
        .flatten()
    } else {
        None
    };

    let mut tx: Transaction<'_, Postgres> = user_db.begin(&authed).await?;

    if let Some(npath) = ns.path.clone() {
        if npath != path {
            check_path_conflict(&db, &w_id, &npath).await?;
            require_owner_of_path(&authed, path)?;

            // Handle Vault secret rename if the variable is a secret stored in Vault
            let current_var = sqlx::query!(
                "SELECT value, is_secret FROM variable WHERE path = $1 AND workspace_id = $2",
                path,
                w_id
            )
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(var) = current_var {
                if var.is_secret && is_vault_stored_value(&var.value) {
                    if ns.value.is_some() {
                        // New value was provided and already stored at new path
                        // Just delete the old secret from Vault
                        delete_secret_from_backend(&db, &w_id, path).await?;
                    } else {
                        // No new value - rename the secret in Vault
                        if let Some(new_value) =
                            rename_vault_secret(&db, &w_id, path, &npath, &var.value).await?
                        {
                            // Update the variable's value to point to the new Vault path
                            sqlb.set_str("value", &new_value);
                            has_sql_updates = true;
                        }
                    }
                }
            }

            let mut v = sqlx::query_scalar!(
                "SELECT value FROM resource  WHERE path = $1 AND workspace_id = $2",
                path,
                w_id
            )
            .fetch_optional(&mut *tx)
            .await?
            .flatten();

            if let Some(old_v) = v {
                v = Some(replace_path(
                    old_v,
                    &format!("$var:{path}"),
                    &format!("$var:{npath}"),
                ))
            }

            sqlx::query!(
                "UPDATE resource SET path = $1, value = $2, edited_at = now() WHERE path = $3 AND workspace_id = $4",
                npath,
                v,
                path,
                w_id
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "UPDATE workspace_integrations SET resource_path = $1 WHERE workspace_id = $2 AND resource_path = $3",
                npath,
                w_id,
                path
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "UPDATE ws_specific SET path = $1 WHERE workspace_id = $2 AND item_kind = 'variable' AND path = $3",
                npath,
                w_id,
                path
            )
            .execute(&mut *tx)
            .await?;

            // The linked resource at the same path is renamed above; move
            // its ws_specific 'resource' marker too so an explicitly-flagged
            // resource doesn't lose its ws_specific status on rename and
            // doesn't leave a stale marker at the old path. Symmetric with
            // update_resource's rename block.
            sqlx::query!(
                "UPDATE ws_specific SET path = $1 WHERE workspace_id = $2 AND item_kind = 'resource' AND path = $3",
                npath,
                w_id,
                path
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    let npath = if has_sql_updates {
        sqlb.set("edited_at", "now()");
        sqlb.set_str("edited_by", &authed.username);
        sqlb.returning("path");
        let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
        let npath_o: Option<String> = sqlx::query_scalar(&sql).fetch_optional(&mut *tx).await?;
        not_found_if_none(npath_o, "Variable", path)?
    } else {
        // `has_sql_updates` is guaranteed true whenever `ns.path` is provided
        // (set unconditionally above when ns.path is Some). So this branch
        // only runs for non-rename edits (e.g. labels-only or ws_specific-only)
        // and the rename-side queries (secret table, variable_history,
        // ws_specific path UPDATE) in the rename block above are skipped.
        debug_assert!(
            ns.path.is_none(),
            "has_sql_updates should be true when ns.path is Some"
        );
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM variable WHERE path = $1 AND workspace_id = $2)",
        )
        .bind(path)
        .bind(&w_id)
        .fetch_one(&mut *tx)
        .await?;
        not_found_if_none(
            if exists { Some(path.to_string()) } else { None },
            "Variable",
            path,
        )?
    };

    if let Some(nlabels) = &ns.labels {
        sqlx::query!(
            "UPDATE variable SET labels = $1, edited_at = now(), edited_by = $4 WHERE path = $2 AND workspace_id = $3",
            nlabels as &[String],
            &npath,
            &w_id,
            &authed.username
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(ws_specific) = ns.ws_specific {
        if ws_specific {
            sqlx::query!(
                "INSERT INTO ws_specific (workspace_id, item_kind, path) VALUES ($1, 'variable', $2) ON CONFLICT DO NOTHING",
                w_id,
                &npath,
            )
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query!(
                "DELETE FROM ws_specific WHERE workspace_id = $1 AND item_kind = 'variable' AND path = $2",
                w_id,
                &npath,
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    audit_log(
        &mut *tx,
        &authed,
        "variables.update",
        ActionKind::Update,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    // Clean up old account if it's no longer referenced and different from new account
    if let Some(old_acc_id) = old_account_id {
        if ns.account.is_some() && ns.account != Some(old_acc_id) {
            // Check if old account is still referenced by other variables or resources
            let account_still_used = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM variable WHERE account = $1 AND workspace_id = $2)",
                old_acc_id,
                &w_id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(true);

            if !account_still_used {
                // Delete the orphaned account
                sqlx::query!(
                    "DELETE FROM account WHERE id = $1 AND workspace_id = $2",
                    old_acc_id,
                    &w_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    tx.commit().await?;

    // Detect if this was a rename operation
    let old_path_if_renamed = if npath != path { Some(path) } else { None };

    // On rename the old-path draft orphans (see resources.rs); the linked resource
    // renames alongside the variable, so its old-path draft orphans too.
    if let Some(old_path) = old_path_if_renamed {
        delete_own_draft_for_path(
            &db,
            &w_id,
            UserDraftItemKind::Variable,
            old_path,
            &authed.email,
        )
        .await?;
        delete_own_draft_for_path(
            &db,
            &w_id,
            UserDraftItemKind::Resource,
            old_path,
            &authed.email,
        )
        .await?;
    }

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Variable { path: npath.clone(), parent_path: Some(path.to_string()) },
        None,
        true,
        old_path_if_renamed,
    )
    .await?;

    webhook.send_message(
        w_id.clone(),
        WebhookMessage::UpdateVariable {
            workspace: w_id,
            old_path: path.to_owned(),
            new_path: npath.clone(),
        },
    );

    Ok(format!("variable {} updated (npath: {:?})", path, npath))
}

fn replace_path(v: serde_json::Value, path: &str, npath: &str) -> Value {
    match v {
        Value::Object(v) => Value::Object(
            v.into_iter()
                .map(|(k, v)| (k, replace_path(v, path, npath)))
                .collect(),
        ),
        Value::Array(arr) => Value::Array(
            arr.into_iter()
                .map(|v| replace_path(v, path, npath))
                .collect(),
        ),
        Value::String(s) if s == path => Value::String(npath.to_owned()),
        _ => v,
    }
}

/// Emit the `variables.decrypt_secret` audit event for a secret-variable read. Run on both
/// the cache-miss and cache-hit paths so `allow_cache` never skips secret-access auditing.
async fn audit_decrypt_secret(
    db_with_opt_authed: &DbWithOptAuthed<'_, ApiAuthed>,
    w_id: &str,
    path: &str,
) -> Result<()> {
    let mut tx = db_with_opt_authed.db().begin().await?;
    audit_log(
        &mut *tx,
        db_with_opt_authed,
        "variables.decrypt_secret",
        ActionKind::Execute,
        w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn get_value_internal<'a>(
    db_with_opt_authed: &'a DbWithOptAuthed<'a, ApiAuthed>,
    w_id: &str,
    path: &str,
    allow_cache: bool,
) -> Result<String> {
    // Scope the cache to the caller's full authorization identity (not just email): the
    // cached value is the decrypted variable, resolved under this caller's RLS context.
    let cache_identity = allow_cache.then(|| match db_with_opt_authed.authed() {
        Some(authed) => auth_identity(authed),
        None => format!("\0system:{}", db_with_opt_authed.email()),
    });

    if let Some(identity) = cache_identity.as_deref() {
        if let Some(cached) = get_cached_variable(&w_id, &path, identity) {
            // A cache hit must be observably equivalent to a miss: re-run the per-read side
            // effects a secret read performs (the `variables.decrypt_secret` audit and
            // running-job secret registration) so `allow_cache` never silently skips them.
            if cached.is_secret {
                audit_decrypt_secret(db_with_opt_authed, &w_id, &path).await?;
                if !cached.value.is_empty() {
                    windmill_common::sensitive_log_masks::register_secret_for_all_running_jobs(
                        &cached.value,
                    );
                }
            }
            return Ok(cached.value);
        }
    }

    let mut tx = db_with_opt_authed.begin().await?;
    let variable_o = sqlx::query!(
        "SELECT value, account, (now() > account.expires_at) as is_expired, is_secret, path from variable
        LEFT JOIN account ON variable.account = account.id WHERE variable.path = $1 AND variable.workspace_id = $2", path, w_id
    )
    .fetch_optional(&mut *tx)
    .warn_after_seconds(5)
    .await?;
    drop(tx);

    let variable = if let Some(variable) = variable_o {
        variable
    } else {
        explain_variable_perm_error(path, w_id, &db_with_opt_authed.db()).await?;
        unreachable!()
    };

    let r = if variable.is_secret {
        audit_decrypt_secret(db_with_opt_authed, &w_id, &variable.path).await?;

        let value = variable.value;
        if variable.is_expired.unwrap_or(false) && variable.account.is_some() {
            #[cfg(feature = "oauth2")]
            {
                let db = db_with_opt_authed.db();
                let refresh_tx = db
                    .begin()
                    .await
                    .map_err(|e| Error::InternalErr(e.to_string()))?;
                crate::oauth_refresh_oss::_refresh_token(
                    refresh_tx,
                    &variable.path,
                    &w_id,
                    variable.account.unwrap(),
                    db,
                )
                .await?
            }
            #[cfg(not(feature = "oauth2"))]
            return Err(Error::internal_err("Require oauth2 feature".to_string()));
        } else if !value.is_empty() {
            // Use secret backend for decryption (supports both DB and Vault)
            get_secret_value(db_with_opt_authed.db(), &w_id, &variable.path, &value).await?
        } else {
            "".to_string()
        }
    } else {
        variable.value
    };

    if variable.is_secret && !r.is_empty() {
        windmill_common::sensitive_log_masks::register_secret_for_all_running_jobs(&r);
    }

    // Cache the result when explicitly allowed. Secrets are cached too: their per-read side
    // effects (audit + running-job registration) are re-run on a hit (see the hit path above),
    // and `is_secret` is stored so the hit knows to do so.
    if let Some(identity) = cache_identity.as_deref() {
        cache_variable(
            &w_id,
            &path,
            identity,
            CachedVariable { value: r.clone(), is_secret: variable.is_secret },
        );
    }

    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use magic_crypt::MagicCryptTrait;

    #[test]
    fn accepts_real_workspace_ciphertext() {
        // The exact shape produced by `encrypt` (AES-256-CBC, base64).
        let mc = magic_crypt::new_magic_crypt!("a-test-workspace-key", 256);
        for plain in [
            "",
            "original-secret",
            "some: plaintext\n",
            "a".repeat(500).as_str(),
        ] {
            let ciphertext = mc.encrypt_str_to_base64(plain);
            assert!(
                validate_already_encrypted_secret("f/x/cfg", &ciphertext).is_ok(),
                "should accept genuine ciphertext for plaintext {plain:?}: {ciphertext}"
            );
        }
    }

    #[test]
    fn rejects_plaintext_mislabeled_as_encrypted() {
        // Plaintext mislabeled as encrypted: storing it verbatim would make the
        // variable undecryptable on every read, so it must be rejected.
        for plaintext in [
            "some: plaintext\n",
            "original-secret",
            "hunter2",
            "{\"a\": 1}",
            "not base64!!",
            " leading-space",
        ] {
            assert!(
                validate_already_encrypted_secret("f/x/cfg", plaintext).is_err(),
                "should reject plaintext mislabeled as encrypted: {plaintext:?}"
            );
        }
    }

    #[test]
    fn rejects_empty_and_non_block_aligned() {
        // Valid base64 but not a whole number of AES blocks -> cannot be our ciphertext.
        assert!(validate_already_encrypted_secret("p", "").is_err());
        assert!(validate_already_encrypted_secret("p", "dGVzdA==").is_err()); // "test" -> 4 bytes
    }

    #[test]
    fn passes_through_external_backend_markers() {
        // External secret backends store $-prefixed markers, not workspace ciphertext.
        for marker in ["$vault:f/x/cfg", "$aws_sm:f/x/cfg", "$azure_kv:f/x/cfg"] {
            assert!(validate_already_encrypted_secret("f/x/cfg", marker).is_ok());
        }
    }
}
