/*
 * Author: Vladislav Kuzmin (https://github.com/principalwater)
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Apple Keychain backend for secret storage
//!
//! Stores each secret as a generic password item in a macOS keychain. The
//! database keeps no ciphertext for these variables, only the row that says the
//! variable exists, so a stolen database dump does not contain the secrets.
//!
//! Two transports, because Windmill is frequently not the process that can talk
//! to a keychain:
//!
//! * `Native` calls the Security framework directly. Only available when the
//!   Windmill process itself runs on macOS.
//! * `Helper` runs an operator-provided command. This is what makes the backend
//!   usable when Windmill runs in a Linux container on a Mac host: the container
//!   cannot load `Security.framework` or reach the host keychain, but it can run
//!   a command that does.
//!
//! The helper contract is deliberately small — two appended arguments and the
//! value on a stream:
//!
//! ```text
//! <helper...> get <service>              -> value on stdout, exit 0
//!                                        -> exit 44 when the item is absent
//! <helper...> set <service>              <- value on stdin, exit 0
//! <helper...> delete <service>           -> exit 0, also 0 when already absent
//! ```
//!
//! Anything else on exit is an error and its stderr is surfaced verbatim, so a
//! locked keychain is distinguishable from a missing item.

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use crate::error::{Error, Result};

use super::{KeychainSettings, KeychainTransport, SecretBackend};

/// Exit code the helper uses to report "no such item". Any other non-zero exit
/// is an error: conflating the two would let a locked keychain look like an
/// empty one, and callers would silently read nothing instead of failing.
const HELPER_NOT_FOUND: i32 = 44;

pub struct KeychainBackend {
    settings: KeychainSettings,
}

impl KeychainBackend {
    pub fn new(settings: KeychainSettings) -> Self {
        Self { settings }
    }

    /// Service name of a secret.
    ///
    /// The workspace id is part of the name rather than of the account, so one
    /// keychain can hold several workspaces without collisions, and an operator
    /// scanning the keychain can tell at a glance which workspace an item
    /// belongs to.
    fn service(&self, workspace_id: &str, path: &str) -> String {
        format!("{}/{}/{}", self.settings.service_prefix, workspace_id, path)
    }

    fn account(&self) -> &str {
        self.settings.account.as_deref().unwrap_or("windmill")
    }

    async fn helper(&self, cmd: &[String], op: &str, service: &str, value: Option<&str>)
        -> Result<Option<String>> {
        let (program, rest) = cmd
            .split_first()
            .ok_or_else(|| Error::internal_err("keychain helper command is empty".to_string()))?;

        let mut command = tokio::process::Command::new(program);
        command
            .args(rest)
            .arg(op)
            .arg(service)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|e| Error::internal_err(format!("keychain helper {program}: {e}")))?;

        // stdin is always closed, even with nothing to send: a helper that reads
        // to EOF would otherwise wait forever on a get.
        if let Some(stdin) = child.stdin.as_mut() {
            if let Some(value) = value {
                stdin
                    .write_all(value.as_bytes())
                    .await
                    .map_err(|e| Error::internal_err(format!("keychain helper stdin: {e}")))?;
            }
        }
        drop(child.stdin.take());

        let out = child
            .wait_with_output()
            .await
            .map_err(|e| Error::internal_err(format!("keychain helper {program}: {e}")))?;

        if out.status.code() == Some(HELPER_NOT_FOUND) {
            return Ok(None);
        }
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(Error::internal_err(format!(
                "keychain helper {op} {service} failed ({}): {}",
                out.status,
                stderr.trim()
            )));
        }
        // A trailing newline is an artifact of writing to a pipe, not part of
        // the secret. Only one is trimmed: a secret may legitimately end in a
        // blank line and we would otherwise corrupt it.
        let mut value = String::from_utf8(out.stdout)
            .map_err(|_| Error::internal_err("keychain helper returned non-UTF-8".to_string()))?;
        if value.ends_with('\n') {
            value.pop();
            if value.ends_with('\r') {
                value.pop();
            }
        }
        Ok(Some(value))
    }
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use security_framework::passwords::{
        delete_generic_password, get_generic_password, set_generic_password,
    };

    /// The Security framework calls block. They are short, but a blocking call
    /// on a runtime worker stalls unrelated jobs, and a locked keychain can
    /// block for as long as a dialog is up.
    pub async fn get(service: String, account: String) -> Result<Option<String>> {
        tokio::task::spawn_blocking(move || match get_generic_password(&service, &account) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| Error::internal_err("keychain item is not UTF-8".to_string())),
            Err(e) if e.code() == ITEM_NOT_FOUND => Ok(None),
            Err(e) => Err(Error::internal_err(format!("keychain read failed: {e}"))),
        })
        .await
        .map_err(|e| Error::internal_err(format!("keychain task: {e}")))?
    }

    pub async fn set(service: String, account: String, value: String) -> Result<()> {
        tokio::task::spawn_blocking(move || {
            set_generic_password(&service, &account, value.as_bytes())
                .map_err(|e| Error::internal_err(format!("keychain write failed: {e}")))
        })
        .await
        .map_err(|e| Error::internal_err(format!("keychain task: {e}")))?
    }

    pub async fn delete(service: String, account: String) -> Result<()> {
        tokio::task::spawn_blocking(move || match delete_generic_password(&service, &account) {
            Ok(()) => Ok(()),
            // Deleting what is already gone is the desired end state.
            Err(e) if e.code() == ITEM_NOT_FOUND => Ok(()),
            Err(e) => Err(Error::internal_err(format!("keychain delete failed: {e}"))),
        })
        .await
        .map_err(|e| Error::internal_err(format!("keychain task: {e}")))?
    }

    /// errSecItemNotFound. Spelled out rather than matched on the message: the
    /// message is localised and would stop matching on a non-English system.
    const ITEM_NOT_FOUND: i32 = -25300;
}

#[async_trait]
impl SecretBackend for KeychainBackend {
    async fn get_secret(&self, workspace_id: &str, path: &str) -> Result<String> {
        let service = self.service(workspace_id, path);
        let found = match &self.settings.transport {
            KeychainTransport::Helper { command } => {
                self.helper(command, "get", &service, None).await?
            }
            KeychainTransport::Native => self.native_get(&service).await?,
        };
        found.ok_or_else(|| {
            Error::NotFound(format!(
                "Secret variable {} not found in keychain for workspace {}",
                path, workspace_id
            ))
        })
    }

    async fn set_secret(&self, workspace_id: &str, path: &str, value: &str) -> Result<()> {
        let service = self.service(workspace_id, path);
        match &self.settings.transport {
            KeychainTransport::Helper { command } => {
                self.helper(command, "set", &service, Some(value)).await?;
                Ok(())
            }
            KeychainTransport::Native => self.native_set(&service, value).await,
        }
    }

    async fn delete_secret(&self, workspace_id: &str, path: &str) -> Result<()> {
        let service = self.service(workspace_id, path);
        match &self.settings.transport {
            KeychainTransport::Helper { command } => {
                self.helper(command, "delete", &service, None).await?;
                Ok(())
            }
            KeychainTransport::Native => self.native_delete(&service).await,
        }
    }

    fn backend_name(&self) -> &'static str {
        "apple_keychain"
    }
}

/// Native transport, kept behind small helpers so the trait implementation above
/// reads the same on every platform. On non-macOS the calls fail closed instead
/// of falling back to the database: a silent fallback would put plaintext where
/// the operator asked for it not to be.
impl KeychainBackend {
    #[cfg(target_os = "macos")]
    async fn native_get(&self, service: &str) -> Result<Option<String>> {
        native::get(service.to_string(), self.account().to_string()).await
    }

    #[cfg(not(target_os = "macos"))]
    async fn native_get(&self, _service: &str) -> Result<Option<String>> {
        Err(unsupported())
    }

    #[cfg(target_os = "macos")]
    async fn native_set(&self, service: &str, value: &str) -> Result<()> {
        native::set(
            service.to_string(),
            self.account().to_string(),
            value.to_string(),
        )
        .await
    }

    #[cfg(not(target_os = "macos"))]
    async fn native_set(&self, _service: &str, _value: &str) -> Result<()> {
        Err(unsupported())
    }

    #[cfg(target_os = "macos")]
    async fn native_delete(&self, service: &str) -> Result<()> {
        native::delete(service.to_string(), self.account().to_string()).await
    }

    #[cfg(not(target_os = "macos"))]
    async fn native_delete(&self, _service: &str) -> Result<()> {
        Err(unsupported())
    }
}

#[cfg(not(target_os = "macos"))]
fn unsupported() -> Error {
    Error::internal_err(
        "the Apple Keychain backend in `native` mode requires Windmill to run on macOS; \
         use `helper` mode to reach a keychain on the host"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_backend::{KeychainSettings, KeychainTransport};

    /// Backend whose helper is a shell snippet. Exercising the real contract —
    /// arguments, streams and exit codes — is the point: a mock of our own
    /// parsing would pass no matter what the parsing did.
    fn backend(script: &str) -> KeychainBackend {
        KeychainBackend::new(KeychainSettings {
            service_prefix: "windmill".to_string(),
            account: None,
            transport: KeychainTransport::Helper {
                // Заполнитель после скрипта не косметика: `sh -c script a b`
                // кладёт первый аргумент в $0, а не в $1. Без него операция и
                // имя службы съезжали бы на одну позицию.
                command: vec![
                    "/bin/sh".to_string(),
                    "-c".to_string(),
                    script.to_string(),
                    "helper".to_string(),
                ],
            },
        })
    }

    #[tokio::test]
    async fn get_trims_one_trailing_newline() {
        // Writing to a pipe adds the newline; the secret does not contain it.
        let b = backend("printf 'hunter2\\n'");
        assert_eq!(b.get_secret("ws", "u/admin/token").await.unwrap(), "hunter2");
    }

    #[tokio::test]
    async fn get_keeps_a_deliberate_blank_line() {
        // Only one newline is an artifact. Trimming more would corrupt a secret
        // that legitimately ends in a blank line.
        let b = backend("printf 'multi\\nline\\n\\n'");
        assert_eq!(
            b.get_secret("ws", "u/admin/token").await.unwrap(),
            "multi\nline\n"
        );
    }

    #[tokio::test]
    async fn absent_item_is_not_found_rather_than_empty() {
        // The distinction matters: an empty string would be accepted by callers
        // as a valid secret and fail much later, somewhere unrelated.
        let b = backend("exit 44");
        let err = b.get_secret("ws", "u/admin/token").await.unwrap_err();
        assert!(
            matches!(err, Error::NotFound(_)),
            "expected NotFound, got {err:?}"
        );
    }

    #[tokio::test]
    async fn other_failures_surface_stderr() {
        // A locked keychain must be distinguishable from a missing item, so the
        // helper's own words are carried through instead of being flattened.
        let b = backend("echo 'keychain is locked' >&2; exit 1");
        let err = b.get_secret("ws", "u/admin/token").await.unwrap_err();
        let text = err.to_string();
        assert!(text.contains("keychain is locked"), "got: {text}");
        assert!(!matches!(err, Error::NotFound(_)));
    }

    #[tokio::test]
    async fn set_passes_the_value_on_stdin_and_the_service_in_argv() {
        // Both halves of the contract at once: the value must not appear in argv
        // (any process can read argv) and the service name must.
        let b = backend(
            "read -r got; \
             test \"$got\" = 'sekret' || { echo 'value not on stdin' >&2; exit 1; }; \
             case \"$2\" in windmill/ws/u/admin/token) exit 0 ;; \
               *) echo \"unexpected service: $2\" >&2; exit 1 ;; esac",
        );
        b.set_secret("ws", "u/admin/token", "sekret").await.unwrap();
    }

    #[tokio::test]
    async fn get_does_not_hang_when_the_helper_reads_stdin() {
        // stdin is closed even with nothing to send; a helper that reads to EOF
        // would otherwise wait forever and take the request with it.
        let b = backend("cat >/dev/null; printf 'value'");
        assert_eq!(b.get_secret("ws", "u/admin/token").await.unwrap(), "value");
    }
}
