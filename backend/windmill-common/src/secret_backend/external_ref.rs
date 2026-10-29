/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! References from a secret variable to a secret that already exists in an
//! external store: the variable stores `$external:<scheme>://<path>#<key>` and
//! the value is read from the store each time the variable is read. Unlike the
//! `$vault:`/`$aws_sm:`/`$azure_kv:` markers of a managed secret backend,
//! nothing is ever written to the store. Only `vault://` resolves, with `<path>`
//! written as `vault kv get` takes it.
//!
//! The instance's identity in the store can usually read far more than any one
//! workspace should, so a reference resolves only under a prefix a superadmin
//! allowed for that workspace in the `external_secrets` setting. The check runs
//! on save and again on every read, so narrowing the list takes effect on
//! existing references.

use serde::{Deserialize, Serialize};

use crate::{
    db::DB,
    error::{Error, Result},
    global_settings::{
        load_value_from_global_settings, EXTERNAL_SECRETS_SETTING, SECRET_BACKEND_SETTING,
    },
    secret_backend::{SecretBackendConfig, VaultSettings},
};

pub const EXTERNAL_REF_MARKER: &str = "$external:";

/// The `external_secrets` instance setting: the connections references are
/// read through, and where they may point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExternalSecretsSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultSettings>,
    #[serde(default)]
    pub allowed: Vec<ExternalSecretAllowedPrefix>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalSecretAllowedPrefix {
    /// `<scheme>://<path prefix>`, the path matched on whole segments.
    pub prefix: String,
    /// Workspaces that may reference under `prefix`; empty means all.
    #[serde(default)]
    pub workspaces: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalSecretBackend {
    Vault,
}

impl ExternalSecretBackend {
    fn scheme(self) -> &'static str {
        match self {
            Self::Vault => "vault",
        }
    }

    fn from_scheme(scheme: &str) -> Result<Self> {
        match scheme {
            "vault" => Ok(Self::Vault),
            _ => Err(Error::BadRequest(format!(
                "Unsupported external secret backend `{scheme}://`: only `vault://` is supported"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSecretRef {
    pub backend: ExternalSecretBackend,
    /// Normalized: no surrounding or doubled slashes.
    pub path: String,
    pub key: String,
}

pub fn is_external_ref_value(value: &str) -> bool {
    value.starts_with(EXTERNAL_REF_MARKER)
}

/// A stored secret value whose plaintext is not workspace ciphertext but lives
/// in an external store: a managed backend entry or an `$external:` reference.
pub fn resolves_externally(value: &str) -> bool {
    super::is_external_stored_value(value) || is_external_ref_value(value)
}

fn path_segments(path: &str) -> Result<Vec<&str>> {
    let segments: Vec<&str> = path.trim().trim_matches('/').split('/').collect();
    // The URL parser treats `\` as `/` and drops tabs and newlines, so either would let a
    // `..` segment slip past this check and the allow-list.
    if segments.iter().any(|s| {
        s.is_empty()
            || *s == "."
            || *s == ".."
            || s.contains(['?', '%', '#', '\\'])
            || s.contains(char::is_control)
    }) {
        return Err(Error::BadRequest(format!(
            "Invalid external secret path `{path}`: expected segments like `secret/team/app`"
        )));
    }
    Ok(segments)
}

fn split_scheme(uri: &str) -> Result<(ExternalSecretBackend, &str)> {
    let (scheme, rest) = uri.trim().split_once("://").ok_or_else(|| {
        Error::BadRequest(format!(
            "Invalid external secret reference `{uri}`: expected `<scheme>://<path>#<key>`, e.g. `vault://secret/team/app#token`"
        ))
    })?;
    Ok((ExternalSecretBackend::from_scheme(scheme)?, rest))
}

/// Parses `$external:<scheme>://<path>#<key>` (the marker is optional).
pub fn parse_external_ref(value: &str) -> Result<ExternalSecretRef> {
    let uri = value.strip_prefix(EXTERNAL_REF_MARKER).unwrap_or(value);
    let (backend, rest) = split_scheme(uri)?;
    let (path, key) = rest.split_once('#').ok_or_else(|| {
        Error::BadRequest(format!(
            "Invalid external secret reference `{uri}`: missing `#<key>` after the path"
        ))
    })?;
    let key = key.trim();
    if key.is_empty() {
        return Err(Error::BadRequest(format!(
            "Invalid external secret reference `{uri}`: the key after `#` is empty"
        )));
    }
    Ok(ExternalSecretRef { backend, path: path_segments(path)?.join("/"), key: key.to_string() })
}

fn is_allowed(settings: &ExternalSecretsSettings, w_id: &str, r: &ExternalSecretRef) -> bool {
    let path: Vec<&str> = r.path.split('/').collect();
    settings.allowed.iter().any(|a| {
        (a.workspaces.is_empty() || a.workspaces.iter().any(|w| w == w_id))
            && split_scheme(&a.prefix).is_ok_and(|(backend, prefix)| {
                backend == r.backend && path_segments(prefix).is_ok_and(|p| path.starts_with(&p))
            })
    })
}

async fn load_settings(db: &DB) -> Result<ExternalSecretsSettings> {
    match load_value_from_global_settings(db, EXTERNAL_SECRETS_SETTING).await? {
        Some(value) => serde_json::from_value(value).map_err(|e| {
            Error::internal_err(format!("Invalid {EXTERNAL_SECRETS_SETTING} setting: {e}"))
        }),
        None => Ok(ExternalSecretsSettings::default()),
    }
}

fn vault_segments(s: Option<&str>) -> Vec<&str> {
    s.unwrap_or_default()
        .split('/')
        .filter(|s| !s.trim().is_empty())
        .collect()
}

/// Whether `path`, read through `conn`, lands in the storage of the managed Vault
/// backend `managed`. The namespace is folded into the path because Vault also
/// takes a namespace as a path prefix, so `ns/kv/x` with no namespace header
/// reaches the same secret as `kv/x` under namespace `ns`.
fn is_managed_storage(managed: &VaultSettings, conn: &VaultSettings, path: &str) -> bool {
    let address = |s: &VaultSettings| s.address.trim().trim_end_matches('/').to_lowercase();
    let mount = vault_segments(Some(&managed.mount_path));
    if mount.is_empty() || address(managed) != address(conn) {
        return false;
    }
    let mut storage = vault_segments(managed.namespace.as_deref());
    storage.extend(mount);
    storage.extend(vault_segments(managed.kv_secret_path_prefix.as_deref()));
    let mut target = vault_segments(conn.namespace.as_deref());
    target.extend(vault_segments(Some(path)));
    target.starts_with(&storage)
}

/// Parses a reference and checks it is allowed for `w_id`.
async fn authorize(
    db: &DB,
    w_id: &str,
    value: &str,
) -> Result<(ExternalSecretsSettings, ExternalSecretRef)> {
    let r = parse_external_ref(value)?;
    let settings = load_settings(db).await?;
    if !is_allowed(&settings, w_id, &r) {
        return Err(Error::PermissionDenied(format!(
            "External secret `{}://{}` is outside the prefixes a superadmin allowed for workspace {w_id}",
            r.backend.scheme(),
            r.path
        )));
    }
    // The managed backend's storage holds every workspace's secrets, so no
    // allow-list entry may open it.
    if let Some(SecretBackendConfig::HashiCorpVault(managed)) =
        load_value_from_global_settings(db, SECRET_BACKEND_SETTING)
            .await?
            .and_then(|v| serde_json::from_value(v).ok())
    {
        if is_managed_storage(
            &managed,
            settings.vault.as_ref().unwrap_or(&managed),
            &r.path,
        ) {
            return Err(Error::PermissionDenied(format!(
                "External secret `vault://{}` points into the storage of Windmill's own Vault secret backend",
                r.path
            )));
        }
    }
    Ok((settings, r))
}

/// Write-time check of an `$external:` value. Does not contact the store, so a
/// reference may be saved before its secret exists.
pub async fn check_external_ref(db: &DB, w_id: &str, value: &str) -> Result<()> {
    authorize(db, w_id, value).await.map(|_| ())
}

pub(crate) async fn resolve_external_ref(
    db: &DB,
    w_id: &str,
    variable_path: &str,
    value: &str,
) -> Result<String> {
    let (settings, r) = authorize(db, w_id, value).await?;
    let read = match r.backend {
        ExternalSecretBackend::Vault => {
            let vault = settings.vault.as_ref().ok_or_else(|| {
                Error::BadRequest(
                    "No Vault connection is configured for external secrets (instance settings > Secret storage)"
                        .to_string(),
                )
            })?;
            super::read_vault_external_secret(db, vault, &r.path, &r.key).await
        }
    };
    read.map_err(|e| match e {
        Error::NotFound(m) => Error::NotFound(format!("{m} (variable {variable_path})")),
        Error::PermissionDenied(m) => {
            Error::PermissionDenied(format!("{m} (variable {variable_path})"))
        }
        e => e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(allowed: &[(&str, &[&str])]) -> ExternalSecretsSettings {
        serde_json::from_value(serde_json::json!({
            "allowed": allowed
                .iter()
                .map(|(p, w)| serde_json::json!({"prefix": p, "workspaces": w}))
                .collect::<Vec<_>>(),
        }))
        .unwrap()
    }

    fn allowed(s: &ExternalSecretsSettings, w_id: &str, value: &str) -> bool {
        is_allowed(s, w_id, &parse_external_ref(value).unwrap())
    }

    #[test]
    fn parses_references() {
        assert_eq!(
            parse_external_ref("$external:vault:///secret/team/app/#token").unwrap(),
            ExternalSecretRef {
                backend: ExternalSecretBackend::Vault,
                path: "secret/team/app".to_string(),
                key: "token".to_string()
            }
        );
        for bad in [
            "$external:secret/app#k",
            "$external:vault://secret/app",
            "$external:vault://secret/app#",
            "$external:vault://secret//app#k",
            "$external:vault://secret/../sys#k",
            "$external:vault://secret/team/..\\other/app#k",
            "$external:vault://secret/app?list=true#k",
            "$external:aws-sm://secret/app#k",
        ] {
            assert!(parse_external_ref(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn allow_list_matches_whole_segments_and_workspaces() {
        let s = settings(&[
            ("vault://secret/team", &["ws1"]),
            ("vault://kv/shared/", &[]),
        ]);
        assert!(allowed(&s, "ws1", "vault://secret/team/app#k"));
        assert!(allowed(&s, "ws1", "vault://secret/team#k"));
        assert!(!allowed(&s, "ws1", "vault://secret/team-other/app#k"));
        assert!(!allowed(&s, "ws2", "vault://secret/team/app#k"));
        assert!(allowed(&s, "ws2", "vault://kv/shared/x#k"));
        assert!(!allowed(&s, "ws2", "vault://kv#k"));
    }

    #[test]
    fn refuses_the_managed_backend_storage() {
        let vault = |address: &str, mount: &str, prefix: Option<&str>, ns: Option<&str>| {
            serde_json::from_value::<VaultSettings>(serde_json::json!({
                "address": address, "mount_path": mount,
                "kv_secret_path_prefix": prefix, "namespace": ns,
            }))
            .unwrap()
        };
        let managed = vault("https://vault:8200", "kv", Some("/apps/wm/"), Some("ns"));
        let same = vault("https://vault:8200/", "", None, Some("ns"));
        assert!(is_managed_storage(&managed, &same, "kv/apps/wm/ws1/f/x"));
        assert!(is_managed_storage(&managed, &same, "kv/apps/wm"));
        assert!(!is_managed_storage(&managed, &same, "kv/apps/other/x"));
        assert!(!is_managed_storage(&managed, &same, "secret/apps/wm/x"));
        let root = vault("https://vault:8200", "", None, None);
        assert!(is_managed_storage(&managed, &root, "ns/kv/apps/wm/ws1/x"));
        let other = vault("https://other:8200", "", None, Some("ns"));
        assert!(!is_managed_storage(&managed, &other, "kv/apps/wm/ws1/x"));
    }
}
