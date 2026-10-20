/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::{Authable, UserDB};
use crate::error::{self, Error};
use crate::scripts::ScriptHash;
use crate::secret_backend::{get_secret_value, is_external_stored_value};
use crate::utils::WarnAfterExt;
use crate::worker::Connection;
use crate::workspace_crypt::CipherFormat;
pub use crate::workspace_crypt::WorkspaceCrypt;
use crate::{worker::WORKER_GROUP, BASE_URL, DB};
use chrono::{SecondsFormat, Utc};
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

lazy_static::lazy_static! {
    pub static ref SECRET_SALT: Option<String> = std::env::var("SECRET_SALT").ok();
    static ref RESERVED_WM_VAR_NAME: regex::Regex = regex::Regex::new(r"^WM_[A-Z_]+$").unwrap();
    static ref RE_RES_VAR: regex::Regex =
        regex::Regex::new(r#"\$(?:var|jsonvar|res|encrypted)\:"#).unwrap();
}

/// Whether a job argument, as JSON text, holds a `$var:`/`$jsonvar:`/`$res:`/`$encrypted:`
/// reference, which makes the worker interpolate the whole argument with the job's token,
/// `$`-prefixed reserved variables included.
pub fn arg_needs_interpolation(json: &str) -> bool {
    RE_RES_VAR.is_match(json)
}

/// Escape a string so it can be safely embedded inside a single-quoted JS
/// string literal in a generated NativeTS/Bun prologue.
pub fn escape_js_single_quoted(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// True when `name` is a plain ASCII JS identifier and can therefore sit in a
/// single-quoted string literal or identifier context of a generated prologue
/// without smuggling statement terminators, quotes, or comments. Custom
/// workspace env-var names are attacker-controllable (any workspace admin sets
/// them), so a non-identifier name must never reach an identifier position in
/// generated JS.
pub fn is_valid_js_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c == '_' || c == '$' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c == '$' || c.is_ascii_alphanumeric())
}

/// Names that cannot be used as a binding — `const {word} = ...` is a
/// SyntaxError. The generated prologue runs as an ES module (always strict), so
/// this includes the strict-mode reserved words and the two names that strict
/// mode additionally forbids as bindings: `eval` and `arguments`.
fn is_js_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "arguments"
            | "eval"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
            | "let"
            | "static"
            | "await"
            | "implements"
            | "interface"
            | "package"
            | "private"
            | "protected"
            | "public"
    )
}

/// Identifiers the generated prologue already binds; a second `const {name}`
/// for them would be a redeclaration SyntaxError. Keep in sync with the prologue
/// head emitted in worker.rs and bun_executor.rs (`build_nativets_env_code`).
const PROLOGUE_RESERVED_BINDINGS: &[&str] = &["process", "BASE_URL", "BASE_INTERNAL_URL"];

/// True when `name` can be emitted as a `const {name}` binding in the NativeTS/
/// Bun prologue: a valid identifier that is neither a JS reserved word nor a
/// name the prologue already binds. Names failing this are still exposed via
/// `process.env['{name}']` (with the name escaped), so nothing is lost — a
/// `const` for such a name would only ever be a SyntaxError that breaks every
/// NativeTS run in the workspace.
pub fn can_bind_as_prologue_const(name: &str) -> bool {
    is_valid_js_identifier(name)
        && !is_js_reserved_word(name)
        && !PROLOGUE_RESERVED_BINDINGS.contains(&name)
}

#[derive(Serialize, Clone)]

pub struct ContextualVariable {
    pub name: String,
    pub value: String,
    pub description: String,
    pub is_custom: bool,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone)]

pub struct ListableVariable {
    pub workspace_id: String,
    pub path: String,
    pub value: Option<String>,
    pub is_secret: bool,
    pub description: String,
    pub extra_perms: serde_json::Value,
    pub account: Option<i32>,
    pub is_oauth: Option<bool>,
    pub is_expired: Option<bool>,
    pub is_refreshed: Option<bool>,
    pub refresh_error: Option<String>,
    pub is_linked: Option<bool>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    /// Labels inherited from the parent folder, computed at read time.
    #[sqlx(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherited_labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_specific: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<chrono::DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_by: Option<String>,
    /// True when this row is a per-user draft with no deployed variable
    /// at the same path. Surfaced by `include_draft_only` so the frontend
    /// can render a "Draft" badge and the editor can open from the draft
    /// alone. `None`/omitted on rows fetched from the `variable` table.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(default)]
    pub draft_only: Option<bool>,
    /// True when the authed user has a per-user draft at this path —
    /// layered over a deployed variable or a synthesized draft-only row.
    /// Drives the `*` suffix on the variables page.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(default)]
    pub is_draft: Option<bool>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ExportableListableVariable {
    pub workspace_id: String,
    pub path: String,
    pub value: Option<String>,
    pub is_secret: bool,
    pub description: String,
    pub extra_perms: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<i32>,
    #[serde(skip_serializing_if = "is_none_or_false")]
    pub is_oauth: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<chrono::DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

fn is_none_or_false(b: &Option<bool>) -> bool {
    b.is_none() || !b.unwrap()
}

#[derive(Deserialize)]
pub struct CreateVariable {
    pub path: String,
    pub value: String,
    pub is_secret: bool,
    pub description: String,
    pub account: Option<i32>,
    pub is_oauth: Option<bool>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    #[serde(default)]
    pub ws_specific: Option<bool>,
}

pub async fn build_crypt(db: &DB, w_id: &str) -> crate::error::Result<WorkspaceCrypt> {
    // Check cache first (300-second staleness)
    let cached_key_o = WORKSPACE_CRYPT_CACHE.get(w_id).and_then(|(ts, key)| {
        if ts > chrono::Utc::now().timestamp() - 300 {
            Some(key)
        } else {
            None
        }
    });

    let crypt = if let Some(cached_key) = cached_key_o {
        cached_key
    } else {
        let key = get_workspace_key(w_id, db).await?;
        tracing::info!(
            "crypt for workspace {} with key {}*** expired, refetching",
            w_id,
            &key[..key.len().min(8)]
        );
        let crypt_key = if let Some(ref salt) = SECRET_SALT.as_ref() {
            format!("{}{}", key, salt)
        } else {
            key
        };
        let ncrypt = WorkspaceCrypt::new(&crypt_key);
        WORKSPACE_CRYPT_CACHE.insert(
            w_id.to_string(),
            (chrono::Utc::now().timestamp(), ncrypt.clone()),
        );
        ncrypt
    };

    Ok(crypt)
}

pub async fn build_crypt_with_key_suffix(
    db: &DB,
    w_id: &str,
    key_suffix: &str,
) -> crate::error::Result<WorkspaceCrypt> {
    let key = get_workspace_key(w_id, db).await?;
    Ok(crypt_from_key_with_suffix(&key, key_suffix))
}

/// The cipher `build_crypt_with_key_suffix` builds, from a key string in hand.
pub fn crypt_from_key_with_suffix(key: &str, key_suffix: &str) -> WorkspaceCrypt {
    let crypt_key = if let Some(ref salt) = SECRET_SALT.as_ref() {
        format!("{}{}{}", key, salt, key_suffix)
    } else {
        format!("{}{}", key, key_suffix)
    };
    WorkspaceCrypt::new(&crypt_key)
}

pub async fn get_workspace_key(w_id: &str, db: &DB) -> crate::error::Result<String> {
    let key = sqlx::query_scalar!(
        "SELECT key FROM workspace_key WHERE workspace_id = $1 AND kind = 'cloud'",
        w_id
    )
    .fetch_one(db)
    .warn_after_seconds(5)
    .await
    .map_err(|e| crate::Error::internal_err(format!("fetching workspace key: {e:#}")))?;

    Ok(key)
}

/// Generate a stateless approval token from workspace key + job_id.
/// This token grants access to view approval info and attempt to resume,
/// but cannot be reversed to obtain the HMAC resume secret.
pub async fn generate_approval_token(
    w_id: &str,
    job_id: uuid::Uuid,
    db: &DB,
) -> crate::error::Result<String> {
    let key = get_workspace_key(w_id, db).await?;
    approval_token_from_key(&key, job_id)
}

/// The token [`generate_approval_token`] mints, from a workspace key already in hand.
pub fn approval_token_from_key(key: &str, job_id: uuid::Uuid) -> crate::error::Result<String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())
        .map_err(|e| crate::Error::internal_err(format!("HMAC key error: {e}")))?;
    mac.update(job_id.as_bytes());
    mac.update(b"approval_token");
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// Domain separator of the member-audience view token (see [`generate_view_token`]).
pub const VIEW_TOKEN_DOMAIN: &[u8] = b"view_token";

/// Domain separator of the public-audience view token (see [`generate_view_token`]).
/// Distinct from [`VIEW_TOKEN_DOMAIN`] so that already-shared member links keep granting
/// only what they were minted for: going public is always an explicit new mint.
pub const PUBLIC_VIEW_TOKEN_DOMAIN: &[u8] = b"public_view_token";

/// Stateless read-share signature for a job: `HMAC(workspace_key, job_id || domain)`.
/// Mirrors [`generate_approval_token`] but in a distinct domain so an approval token can
/// never be used as a view token (or vice-versa). Used to build a "share read link" that
/// grants read access to a job (and its flow subtree) to someone who otherwise lacks ACL
/// on it: an authenticated workspace member under [`VIEW_TOKEN_DOMAIN`], anyone holding
/// the link (logged in or not) under [`PUBLIC_VIEW_TOKEN_DOMAIN`]. No expiry/revocation
/// (stateless), like the approval token.
pub async fn generate_view_token(
    w_id: &str,
    job_id: uuid::Uuid,
    domain: &[u8],
    db: &DB,
) -> crate::error::Result<String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let key = get_workspace_key(w_id, db).await?;
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())
        .map_err(|e| crate::Error::internal_err(format!("HMAC key error: {e}")))?;
    mac.update(job_id.as_bytes());
    mac.update(domain);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

pub async fn get_secret_value_as_admin(
    db: &DB,
    w_id: &str,
    path: &str,
) -> crate::error::Result<String> {
    let variable_o = sqlx::query!(
        "SELECT value, is_secret, path from variable WHERE variable.path = $1 AND variable.workspace_id = $2", path, w_id
    )
    .fetch_optional(db)
    .await?;

    let variable = if let Some(variable) = variable_o {
        variable
    } else {
        return Err(crate::Error::NotFound(format!(
            "variable {} not found in workspace {}",
            path, w_id
        )));
    };

    let r = if variable.is_secret {
        let value = variable.value;
        if !value.is_empty() {
            if is_external_stored_value(&value) {
                get_secret_value(db, w_id, &variable.path, &value).await?
            } else {
                let mc = build_crypt(db, w_id).await?;
                decrypt(&mc, value).map_err(|e| {
                    crate::error::Error::internal_err(format!(
                        "Error decrypting variable {}: {}",
                        variable.path, e
                    ))
                })?
            }
        } else {
            "".to_string()
        }
    } else {
        variable.value
    };

    Ok(r)
}

/// Marks a workspace setting's secret written into the settings document encrypted with the
/// workspace key, as `wmill sync pull` writes it.
pub const SETTINGS_ENCRYPTED_PREFIX: &str = "$encrypted:";

pub fn encrypt(mc: &WorkspaceCrypt, value: &str) -> String {
    mc.encrypt_str(value)
}

const DECRYPT_FAILED: &str =
    "Could not decrypt value. The value may have been encrypted with a different key.";

/// Every failure returns the same message so responses carry no signal about why decryption
/// failed (authentication, padding, base64, UTF-8); the kind of failure is only logged.
pub fn decrypt(mc: &WorkspaceCrypt, value: String) -> error::Result<String> {
    mc.decrypt_str(&value).map_err(|e| {
        tracing::warn!("decryption failed: {e}");
        error::Error::internal_err(DECRYPT_FAILED.to_string())
    })
}

static LEGACY_SECRET_UPGRADE_DONE: AtomicBool = AtomicBool::new(false);
static LEGACY_SECRET_UPGRADE_RUNNING: AtomicBool = AtomicBool::new(false);
const LEGACY_SECRET_UPGRADE_BATCH: i64 = 200;

/// Whether this process still has to run [`upgrade_legacy_secret_variables`]: once new values
/// are written in v2, until one pass has gone through every workspace.
pub fn legacy_secret_upgrade_pending() -> bool {
    CipherFormat::current() == CipherFormat::V2
        && !LEGACY_SECRET_UPGRADE_DONE.load(Ordering::Relaxed)
}

/// AES-256-GCM ciphertext is 32 bytes (magic, nonce, tag) longer than legacy CBC before base64,
/// so the longest CBC value that fits 15000 chars is exactly 15040 chars once re-encrypted.
const VARIABLE_VALUE_WIDTH: i32 = 15040;
static VARIABLE_VALUE_WIDENED: AtomicBool = AtomicBool::new(false);

/// Whether this process has seen `variable.value` wide enough for every v2 value. Only servers
/// look, so it stays false in a worker.
pub fn variable_value_widened() -> bool {
    VARIABLE_VALUE_WIDENED.load(Ordering::Relaxed)
}

/// Whether `variable.value` may still have to be widened to fit v2 values.
pub fn variable_value_widen_pending() -> bool {
    CipherFormat::current() == CipherFormat::V2 && !VARIABLE_VALUE_WIDENED.load(Ordering::Relaxed)
}

/// Widen `variable.value` to [`VARIABLE_VALUE_WIDTH`], or `false` when it is too early to.
/// Not a migration: changing a column's type, even its length, fails every statement a live
/// process prepared on it until its connection closes. It waits for v2 writes, gated on a
/// worker version that retires such connections on the first failure, so that no process too
/// old to recover is still running.
async fn widen_variable_value(db: &DB) -> error::Result<bool> {
    if VARIABLE_VALUE_WIDENED.load(Ordering::Relaxed) {
        return Ok(true);
    }
    // varchar's typmod is its length + 4, -1 when unbounded.
    let typmod: i32 = sqlx::query_scalar(
        "SELECT atttypmod FROM pg_attribute WHERE attrelid = 'variable'::regclass AND attname = 'value'",
    )
    .fetch_one(db)
    .await?;
    if typmod >= 0 && typmod - 4 < VARIABLE_VALUE_WIDTH {
        if crate::db::process_uptime() < crate::db::PREVIOUS_RELEASE_ROLLOUT_UPTIME {
            return Ok(false);
        }
        tracing::info!("widening variable.value to VARCHAR({VARIABLE_VALUE_WIDTH})");
        let mut tx = db.begin().await?;
        // Queued behind a long transaction, the ALTER would block every variable read meanwhile.
        sqlx::query("SET LOCAL lock_timeout = '3s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query(&format!(
            "ALTER TABLE variable ALTER COLUMN value TYPE VARCHAR({VARIABLE_VALUE_WIDTH})"
        ))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        crate::db::retire_connections_opened_before_now();
    }
    VARIABLE_VALUE_WIDENED.store(true, Ordering::Relaxed);
    Ok(true)
}

/// Widen `variable.value` for v2 values, then re-encrypt the secret variables still in the
/// legacy format in v2, workspace by workspace.
/// Idempotent, and emits no git-sync event: the plaintexts do not change.
pub async fn upgrade_legacy_secret_variables(db: &DB) {
    struct Running;
    impl Drop for Running {
        fn drop(&mut self) {
            LEGACY_SECRET_UPGRADE_RUNNING.store(false, Ordering::Release);
        }
    }
    if !(variable_value_widen_pending() || legacy_secret_upgrade_pending())
        || LEGACY_SECRET_UPGRADE_RUNNING.swap(true, Ordering::AcqRel)
    {
        return;
    }
    // Cleared on drop, so a pass that panics does not stop every later one in this process.
    let _running = Running;
    match widen_variable_value(db).await {
        Ok(true) => {}
        Ok(false) => return,
        Err(e) => {
            tracing::warn!(
                "secret encryption upgrade: widening variable.value failed, retrying later: {e:#}"
            );
            return;
        }
    }
    if legacy_secret_upgrade_pending() {
        upgrade_legacy_secret_variables_pass(db).await;
    }
}

async fn upgrade_legacy_secret_variables_pass(db: &DB) {
    let workspaces: Vec<String> = match sqlx::query_scalar("SELECT id FROM workspace ORDER BY id")
        .fetch_all(db)
        .await
    {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("secret encryption upgrade: listing workspaces failed: {e:#}");
            return;
        }
    };
    tracing::info!(
        "secret encryption upgrade: re-encrypting legacy secret variables of {} workspaces in AES-256-GCM",
        workspaces.len()
    );
    let (mut upgraded, mut skipped, mut failed_workspaces) = (0, 0, 0);
    for w_id in workspaces {
        match upgrade_workspace_secret_variables(db, &w_id).await {
            Ok((u, s)) => {
                if u > 0 || s > 0 {
                    tracing::info!(
                        "secret encryption upgrade: workspace {w_id}: {u} re-encrypted, {s} skipped"
                    );
                }
                upgraded += u;
                skipped += s;
            }
            Err(e) => {
                failed_workspaces += 1;
                tracing::error!("secret encryption upgrade: workspace {w_id} failed: {e:#}");
            }
        }
    }
    tracing::info!(
        "secret encryption upgrade: done, {upgraded} re-encrypted, {skipped} skipped, {failed_workspaces} workspaces failed"
    );
    if failed_workspaces == 0 {
        LEGACY_SECRET_UPGRADE_DONE.store(true, Ordering::Relaxed);
    }
}

/// The number of values re-encrypted and of values skipped as they do not decrypt.
async fn upgrade_workspace_secret_variables(db: &DB, w_id: &str) -> error::Result<(usize, usize)> {
    let (mut upgraded, mut skipped) = (0, 0);
    let mut after = String::new();
    loop {
        let mut tx = db.begin().await?;
        // Shared, so a rotation (which locks the key row for update and rewrites every secret)
        // never interleaves with a batch, and the key read here is the one the values are under.
        let key: Option<String> = sqlx::query_scalar(
            "SELECT key FROM workspace_key WHERE workspace_id = $1 AND kind = 'cloud' FOR SHARE",
        )
        .bind(w_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(key) = key else {
            return Ok((upgraded, skipped));
        };
        let mc = crypt_from_key_with_suffix(&key, "");
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT path, value FROM variable WHERE workspace_id = $1 AND is_secret AND path > $2 \
             ORDER BY path LIMIT $3",
        )
        .bind(w_id)
        .bind(&after)
        .bind(LEGACY_SECRET_UPGRADE_BATCH)
        .fetch_all(&mut *tx)
        .await?;
        let last_page = (rows.len() as i64) < LEGACY_SECRET_UPGRADE_BATCH;
        for (path, value) in rows {
            if value.is_empty() || is_external_stored_value(&value) || mc.is_v2(&value) {
                after = path;
                continue;
            }
            match mc.upgrade(&value) {
                Some(new_value) => {
                    // In a savepoint, so a value the update rejects (v2 is up to 44 chars longer
                    // than CBC, which a column narrower than 15040 cannot hold) is skipped
                    // without losing the batch.
                    let mut sp = sqlx::Acquire::begin(&mut tx).await?;
                    // Only over the value it was computed from, so a write racing it wins.
                    match sqlx::query(
                        "UPDATE variable SET value = $1 \
                         WHERE workspace_id = $2 AND path = $3 AND value = $4",
                    )
                    .bind(new_value)
                    .bind(w_id)
                    .bind(&path)
                    .bind(&value)
                    .execute(&mut *sp)
                    .await
                    {
                        Ok(r) => {
                            sp.commit().await?;
                            upgraded += r.rows_affected() as usize;
                        }
                        Err(e) => {
                            sp.rollback().await?;
                            tracing::warn!(
                                "secret encryption upgrade: secret variable {path} of workspace {w_id} could not be stored re-encrypted, leaving it as is: {e}"
                            );
                            skipped += 1;
                        }
                    }
                }
                None => {
                    tracing::warn!(
                        "secret encryption upgrade: secret variable {path} of workspace {w_id} does not decrypt, leaving it as is"
                    );
                    skipped += 1;
                }
            }
            after = path;
        }
        tx.commit().await?;
        if last_page {
            return Ok((upgraded, skipped));
        }
    }
}

pub const WM_SCHEDULED_FOR: &str = "WM_SCHEDULED_FOR";

lazy_static::lazy_static! {
    pub static ref CUSTOM_ENVS_CACHE: Cache<String, (i64, Vec<(String, String)>)> = Cache::new(100);
    pub static ref WORKSPACE_CRYPT_CACHE: Cache<String, (i64, WorkspaceCrypt)> = Cache::new(1000);

}

pub async fn get_reserved_variables(
    conn: &Connection,
    w_id: &str,
    token: &str,
    email: &str,
    username: &str,
    job_id: &str,
    permissioned_as: &str,
    path: Option<String>,
    flow_id: Option<String>,
    flow_path: Option<String>,
    schedule_path: Option<String>,
    step_id: Option<String>,
    flow_innermost_root_job: Option<String>,
    root_job_id: Option<String>,
    scheduled_for: Option<chrono::DateTime<Utc>>,
    runnable_id: Option<ScriptHash>,
    end_user_email: Option<String>,
    tested_runnable: Option<String>,
) -> Vec<ContextualVariable> {
    let state_path = {
        let trigger = if schedule_path.is_some() {
            username.to_string()
        } else {
            "user".to_string()
        };

        if let Some(flow_path) = flow_path.clone() {
            format!(
                "{flow_path}/{}_{trigger}",
                step_id.clone().unwrap_or_else(|| "nostep".to_string())
            )
        } else if let Some(script_path) = path.clone() {
            let script_path = if script_path.ends_with("/") {
                format!("{script_path}state")
            } else {
                script_path
            };
            format!("{script_path}/{trigger}")
        } else {
            format!("u/{username}/tmp_state")
        }
    };

    let custom_envs = get_cached_workspace_envs(conn, w_id).await;
    let root_workspace = crate::workspaces::root_workspace_id(conn, w_id).await;

    let joined_schedule_path = schedule_path
        .clone()
        .unwrap_or("manual".to_string())
        .split("/")
        .collect::<Vec<&str>>()
        .join("_");
    let ts = chrono::Utc::now().timestamp_millis();
    let object_path = if let Some(flow_path) = flow_path.clone() {
        let flow_path = flow_path.split("/").collect::<Vec<&str>>().join("_");
        let step_id = step_id.clone().unwrap_or("".to_string());
        format!("{flow_path}/{joined_schedule_path}/{step_id}/{ts}_{job_id}")
    } else {
        let joined_script_path = path
            .clone()
            .unwrap_or("".to_string())
            .split("/")
            .collect::<Vec<&str>>()
            .join("_");
        format!("{joined_script_path}/{joined_schedule_path}/{ts}_{job_id}")
    };

    vec![
    ContextualVariable {
        name: "WM_WORKSPACE".to_string(),
        value: w_id.to_string(),
        description: "Workspace id of the current script".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_ROOT_WORKSPACE".to_string(),
        value: root_workspace,
        description: "Workspace id of the nearest dev or prod workspace at or above the current \
                      one. Equal to WM_WORKSPACE unless the script runs in a fork, in which case \
                      it is the closest dev or prod workspace the fork descends from - not \
                      necessarily its direct parent, since a fork of a fork skips past it".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_TOKEN".to_string(),
        value: token.to_string(),
        description: "Token ephemeral to the current script with equal permission to the \
                      permission of the run (Usable as a bearer token)"
            .to_string(),
            is_custom: false,
    },
    ContextualVariable {
        name: "WM_EMAIL".to_string(),
        value: email.to_string(),
        description: "Email of the user that executed the current script".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_USERNAME".to_string(),
        value: username.to_string(),
        description: "Username of the user that executed the current script".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_BASE_URL".to_string(),
        value: (**BASE_URL.load()).clone(),
        description: "base url of this instance".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_JOB_ID".to_string(),
        value: job_id.to_string(),
        description: "Job id of the current script".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: WM_SCHEDULED_FOR.to_string(),
        value: scheduled_for
            .map(|ts| ts.to_rfc3339_opts(SecondsFormat::Secs, true))
            .unwrap_or_else(|| "".to_string()),
        description: "date-time in UTC (e.g: 2014-11-28T12:45:59.324310806Z) of when the job was scheduled".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_JOB_PATH".to_string(),
        value: path.unwrap_or_else(|| "".to_string()),
        description: "Path of the script or flow being run if any".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_FLOW_JOB_ID".to_string(),
        value: flow_id.unwrap_or_else(|| "".to_string()),
        description: "Job id of the encapsulating flow if the job is a flow step".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_ROOT_FLOW_JOB_ID".to_string(),
        value: flow_innermost_root_job.unwrap_or_else(|| "".to_string()),
        description: "Job id of the innermost root flow if the job is a flow step".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_ROOT_JOB_ID".to_string(),
        value: root_job_id.unwrap_or_else(|| "".to_string()),
        description: "Job id of the root job".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_FLOW_PATH".to_string(),
        value: flow_path.unwrap_or_else(|| "".to_string()),
        description: "Path of the encapsulating flow if the job is a flow step".to_string(),
        is_custom: false,
    },

    ContextualVariable {
        name: "WM_SCHEDULE_PATH".to_string(),
        value: schedule_path.unwrap_or_else(|| "".to_string()),
        description: "Path of the schedule if the job of the step or encapsulating step has \
                      been triggered by a schedule"
            .to_string(),
            is_custom: false,
    },
    ContextualVariable {
        name: "WM_PERMISSIONED_AS".to_string(),
        value: permissioned_as.to_string(),
        description: "Fully Qualified (u/g) owner name of executor of the job".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_STATE_PATH".to_string(),
        value: state_path.clone(),
        description: "State resource path unique to a script and its trigger".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_FLOW_STEP_ID".to_string(),
        value: step_id.unwrap_or_else(|| "".to_string()),
        description: "The node id in a flow (like 'a', 'b', or 'f')".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_OBJECT_PATH".to_string(),
        value: object_path,
        description: "Script or flow step execution unique path, useful for storing results in an external service".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_WORKER_GROUP".to_string(),
        value: WORKER_GROUP.clone(),
        description: "Name of the worker group the job is running on".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_RUNNABLE_ID".to_string(),
        value: runnable_id.map(|x| x.to_string()).unwrap_or_else(|| "".to_string()),
        description: "Hash of the script. Useful as cache key for cache that should be runnable specific.".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_END_USER_EMAIL".to_string(),
        value: end_user_email.unwrap_or_else(|| "".to_string()),
        description: "Email of the end user that executed the current script, even when it runs on behalf of another identity, propagated to flow steps and child jobs. Set for app and API runs (for a webhook, the owner of the token it was called with), empty for runs fired by a schedule or by a trigger such as an HTTP route or Kafka.".to_string(),
        is_custom: false,
    },
    ContextualVariable {
        name: "WM_TESTED_RUNNABLE".to_string(),
        value: tested_runnable.unwrap_or_else(|| "".to_string()),
        description: "Qualified path ({kind}/{path}) of the runnable that triggered this CI test. Empty string unless the job was triggered by a CI test annotation.".to_string(),
        is_custom: false,
    },
].into_iter().chain(custom_envs.into_iter().map(|(name, value)| ContextualVariable {
    name,
    value,
    description: "Custom workspace environment variable".to_string(),
    is_custom: true,
})).collect()
}

async fn get_cached_workspace_envs(conn: &Connection, w_id: &str) -> Vec<(String, String)> {
    let cached_envs_o = CUSTOM_ENVS_CACHE.get(w_id).and_then(|(ts, envs)| {
        if ts > chrono::Utc::now().timestamp() - (60 * 15) {
            Some(envs)
        } else {
            None
        }
    });

    let custom_envs = if let Some(cached_envs) = cached_envs_o {
        cached_envs
    } else {
        let raw_envs = match conn {
            Connection::Sql(db) => sqlx::query_as::<_, (String, String)>(
                "SELECT name, value FROM workspace_env WHERE workspace_id = $1",
            )
            .bind(w_id)
            .fetch_all(db)
            .await
            .unwrap_or_default(),
            Connection::Http(client) => client
                .get(&format!("/api/w/{w_id}/agent_workers/custom_envs"))
                .await
                .unwrap_or_default(),
        };
        // Applied here (not in the SQL branch alone) so agent workers going
        // through `Connection::Http` are covered too — drop any name that
        // would shadow a built-in `%%WM_*%%` contextual var.
        let custom_envs: Vec<(String, String)> = raw_envs
            .into_iter()
            .filter(|(name, _)| !RESERVED_WM_VAR_NAME.is_match(name))
            .collect();
        CUSTOM_ENVS_CACHE.insert(
            w_id.to_string(),
            (chrono::Utc::now().timestamp(), custom_envs.clone()),
        );
        custom_envs
    };
    custom_envs
}

pub async fn get_variable_or_self(
    path: String,
    db: &DB,
    w_id: &str,
) -> crate::error::Result<String> {
    if !path.starts_with("$var:") {
        return Ok(path);
    }
    let path = path.strip_prefix("$var:").unwrap().to_string();

    let record = sqlx::query!(
        "SELECT value, is_secret
         FROM variable
         WHERE path = $1 AND workspace_id = $2",
        &path,
        &w_id
    )
    .fetch_optional(db)
    .await?;

    if let Some(record) = record {
        let mut value = record.value;
        if record.is_secret {
            if is_external_stored_value(&value) {
                value = get_secret_value(db, w_id, &path, &value).await?;
            } else {
                let mc = build_crypt(db, w_id).await?;
                value = decrypt(&mc, value).map_err(|e| {
                    Error::internal_err(format!("Error decrypting variable {}: {}", path, e))
                })?;
            }
        }

        Ok(value)
    } else {
        Err(Error::NotFound(format!(
            "Variable not found when resolving `$var:{}`",
            path
        )))
    }
}

/// Like `get_variable_or_self`, but uses an RLS-scoped connection to enforce
/// that the caller has read access to the referenced variable.
pub async fn get_variable_or_self_as<T: Authable + Sync>(
    path: String,
    db: &DB,
    user_db: &UserDB,
    authed: &T,
    w_id: &str,
) -> crate::error::Result<String> {
    if !path.starts_with("$var:") {
        return Ok(path);
    }
    let var_path = path.strip_prefix("$var:").unwrap().to_string();

    // Use an RLS-scoped transaction so the query respects row-level security
    let mut tx = user_db.clone().begin(authed).await?;
    let record = sqlx::query!(
        "SELECT value, is_secret
         FROM variable
         WHERE path = $1 AND workspace_id = $2",
        &var_path,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    if let Some(record) = record {
        let mut value = record.value;
        if record.is_secret {
            if is_external_stored_value(&value) {
                value = get_secret_value(db, w_id, &var_path, &value).await?;
            } else {
                let mc = build_crypt(db, w_id).await?;
                value = decrypt(&mc, value).map_err(|e| {
                    Error::internal_err(format!("Error decrypting variable {}: {}", var_path, e))
                })?;
            }
        }

        Ok(value)
    } else {
        Err(Error::NotFound(format!(
            "Variable not found when resolving `$var:{}`",
            var_path
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        can_bind_as_prologue_const, decrypt, escape_js_single_quoted, is_valid_js_identifier,
        DECRYPT_FAILED,
    };
    use crate::error::Error;
    use crate::workspace_crypt::WorkspaceCrypt;
    use base64::Engine;
    use magic_crypt::MagicCryptTrait;

    #[test]
    fn decrypt_failures_all_return_the_same_message() {
        let mc = WorkspaceCrypt::new("workspace-key");
        let legacy = magic_crypt::new_magic_crypt!("workspace-key", 256);
        let other = WorkspaceCrypt::new("another-key");
        let b64 = base64::engine::general_purpose::STANDARD;
        let v2 = mc.upgrade(&legacy.encrypt_str_to_base64("secret")).unwrap();
        let mut tampered = b64.decode(v2).unwrap();
        *tampered.last_mut().unwrap() ^= 1;
        let failures = [
            "not base64 at all!".to_string(),
            legacy.encrypt_bytes_to_base64(&[0xff, 0xfe, 0xfd]),
            other.encrypt_str_to_base64("secret"),
            b64.encode(&tampered),
        ];
        for value in failures {
            match decrypt(&mc, value.clone()) {
                Err(Error::InternalErrLoc { message, .. }) => {
                    assert_eq!(message, DECRYPT_FAILED, "for {value:?}")
                }
                other => panic!("expected the uniform decrypt error for {value:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn is_valid_js_identifier_gates_prologue_injection() {
        for ok in ["FOO", "_bar", "$x", "a1_2", "WM_CUSTOM"] {
            assert!(
                is_valid_js_identifier(ok),
                "{ok} should be a valid identifier"
            );
        }
        // Custom workspace env-var names are attacker-controllable; none of these
        // may reach `const {name}` position in the NativeTS/Bun prologue.
        for bad in [
            "",
            "1BAD",
            "has space",
            "dotted.name",
            "kebab-case",
            "_x=1;globalThis.PWNED=1//",
            "x'];globalThis.PWNED=1;process.env['y",
        ] {
            assert!(
                !is_valid_js_identifier(bad),
                "{bad:?} must not be a valid identifier"
            );
        }
    }

    #[test]
    fn can_bind_as_prologue_const_excludes_reserved_and_owned_names() {
        // `async` is a contextual keyword, not reserved — `const async` is valid.
        for ok in ["FOO", "_bar", "$x", "myVar", "async"] {
            assert!(can_bind_as_prologue_const(ok), "{ok} should be bindable");
        }
        // Valid identifiers that would still break `const {name}`: reserved words
        // (SyntaxError), the two names strict mode forbids as bindings (`eval`,
        // `arguments`), and names the prologue already binds (redeclaration).
        for bad in [
            "class",
            "const",
            "await",
            "return",
            "eval",
            "arguments",
            "process",
            "BASE_URL",
            "BASE_INTERNAL_URL",
        ] {
            assert!(is_valid_js_identifier(bad), "{bad} is a valid identifier");
            assert!(
                !can_bind_as_prologue_const(bad),
                "{bad} must not bind as a prologue const"
            );
        }
    }

    #[test]
    fn escape_js_single_quoted_neutralizes_string_breakout() {
        assert_eq!(escape_js_single_quoted("a'b"), "a\\'b");
        assert_eq!(escape_js_single_quoted("a\\b"), "a\\\\b");
        assert_eq!(escape_js_single_quoted("a\nb\rc"), "a\\nb\\rc");
        // A key crafted to break out of process.env['...'] is fully contained.
        assert_eq!(
            escape_js_single_quoted("x'];danger();['y"),
            "x\\'];danger();[\\'y"
        );
    }
}
