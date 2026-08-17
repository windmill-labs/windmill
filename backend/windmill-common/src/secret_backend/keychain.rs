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
//! Anything else on exit is an error. The helper's stderr is deliberately not
//! propagated: a faulty helper may echo the value it was handed, and error text
//! reaches both the log and the API client.

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use crate::error::{Error, Result};

use super::{KeychainSettings, KeychainTransport, SecretBackend};

/// Exit code the helper uses to report "no such item". Any other non-zero exit
/// is an error: conflating the two would let a locked keychain look like an
/// empty one, and callers would silently read nothing instead of failing.
const HELPER_NOT_FOUND: i32 = 44;

/// Upper bound for one operation. A helper that hangs — waiting for interactive
/// input, or on a keychain that never answers — must not hold a request open
/// indefinitely.
const HELPER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Upper bound on what a helper may return. Without it a helper that prints
/// without end is collected into memory in full.
const MAX_SECRET_BYTES: usize = 1 << 20;

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

    /// Run the helper for one operation.
    ///
    /// `Ok(None)` means the helper reported "no such item" and is only produced
    /// for `get`: treating exit 44 as a benign outcome of `set` would turn a
    /// failed write into a reported success.
    ///
    /// stdout is returned byte-for-byte. The protocol adds no delimiter on the
    /// way in, so the backend must not remove one on the way out — a secret that
    /// legitimately ends in a newline would otherwise come back changed.
    async fn helper(
        &self,
        cmd: &[String],
        op: &str,
        service: &str,
        value: Option<&str>,
    ) -> Result<Option<String>> {
        let (program, rest) = cmd
            .split_first()
            .ok_or_else(|| Error::internal_err("keychain helper command is empty"))?;

        let mut command = tokio::process::Command::new(program);
        command
            .args(rest)
            .arg(op)
            .arg(service)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            // A cancelled request must not leave a live process holding the
            // secret; Tokio does not kill children on drop by default.
            .kill_on_drop(true);

        let mut child = command
            .spawn()
            .map_err(|_| Error::internal_err(format!("keychain helper {program} failed to start")))?;

        // stdin is written by a separate task while the output pipes are being
        // drained. Doing it in sequence deadlocks: a helper that fills its
        // stdout or stderr pipe before reading stdin waits for us to read while
        // we wait for it to read, and neither ever proceeds.
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::internal_err("keychain helper has no stdin"))?;
        let payload = value.map(|v| v.as_bytes().to_vec());
        let writer = tokio::spawn(async move {
            if let Some(bytes) = payload {
                stdin.write_all(&bytes).await?;
            }
            // Closing stdin is not optional: a helper that reads to EOF would
            // wait forever on a get, where there is nothing to send.
            stdin.shutdown().await
        });

        let out = tokio::time::timeout(HELPER_TIMEOUT, child.wait_with_output())
            .await
            .map_err(|_| {
                Error::internal_err(format!(
                    "keychain helper timed out after {}s on {op} {service}",
                    HELPER_TIMEOUT.as_secs()
                ))
            })?
            .map_err(|_| Error::internal_err(format!("keychain helper {program} failed")))?;

        // A write error is reported, but only after the process has been reaped:
        // a helper that legitimately ignores stdin closes it early, and that is
        // not a failure of the operation.
        let wrote = writer.await;

        let code = out.status.code();
        if code == Some(HELPER_NOT_FOUND) {
            if op == "get" {
                return Ok(None);
            }
            return Err(Error::internal_err(format!(
                "keychain helper reported no-such-item ({HELPER_NOT_FOUND}) for \
                 {op} {service}, which is only meaningful for get"
            )));
        }
        if !out.status.success() {
            // The helper's stderr is deliberately not included. A faulty helper
            // may echo the value it was given, and this message reaches both the
            // log and the API client.
            return Err(Error::internal_err(format!(
                "keychain helper failed on {op} {service}: {}",
                out.status
            )));
        }
        if let Ok(Err(_)) = wrote {
            return Err(Error::internal_err(format!(
                "keychain helper exited successfully but its stdin could not be \
                 written for {op} {service}"
            )));
        }
        if out.stdout.len() > MAX_SECRET_BYTES {
            return Err(Error::internal_err(format!(
                "keychain helper returned more than {MAX_SECRET_BYTES} bytes for {op} {service}"
            )));
        }

        String::from_utf8(out.stdout)
            .map(Some)
            .map_err(|_| Error::internal_err("keychain helper returned non-UTF-8"))
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
                // The placeholder after the script is not cosmetic: `sh -c script
                // a b` puts the first argument in $0, not $1, so the operation
                // and the service name would otherwise be off by one.
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
    async fn get_returns_stdout_byte_for_byte() {
        // The protocol adds no delimiter on the way in, so nothing may be
        // removed on the way out. Trimming here silently changed any secret that
        // legitimately ends in a newline — a PEM block, for one.
        for (script, want) in [
            (r"printf 'hunter2'", "hunter2"),
            (r"printf 'hunter2\n'", "hunter2\n"),
            (r"printf 'hunter2\r\n'", "hunter2\r\n"),
            (r"printf 'multi\nline\n\n'", "multi\nline\n\n"),
        ] {
            let got = backend(script).get_secret("ws", "u/admin/token").await;
            assert_eq!(got.unwrap(), want, "round trip changed the value");
        }
    }

    #[tokio::test]
    async fn absent_item_is_not_found_rather_than_empty() {
        // The distinction matters: an empty string would be accepted by callers
        // as a valid secret and fail much later, somewhere unrelated.
        let err = backend("exit 44")
            .get_secret("ws", "u/admin/token")
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::NotFound(_)),
            "expected NotFound, got {err:?}"
        );
    }

    #[tokio::test]
    async fn not_found_code_does_not_excuse_a_failed_set() {
        // Exit 44 means "no such item", which says nothing useful about a write.
        // Accepting it as success reported a stored secret that was never stored.
        let err = backend("exit 44")
            .set_secret("ws", "u/admin/token", "sekret")
            .await
            .unwrap_err();
        assert!(
            !matches!(err, Error::NotFound(_)),
            "a failed set must not be NotFound"
        );
    }

    #[tokio::test]
    async fn failures_are_errors_but_do_not_carry_the_helper_stderr() {
        // A faulty helper may echo the value it was handed to stderr, and error
        // text reaches both the log and the API client. So the failure must be
        // reported without repeating whatever the helper said.
        let err = backend("cat >&2; exit 1")
            .set_secret("ws", "u/admin/token", "sekret")
            .await
            .unwrap_err();
        let text = err.to_string();
        assert!(!text.contains("sekret"), "the value leaked into the error: {text}");
        assert!(text.contains("u/admin/token"), "the error names no secret: {text}");
        assert!(!matches!(err, Error::NotFound(_)));
    }

    #[tokio::test]
    async fn set_passes_the_value_on_stdin_and_the_service_in_argv() {
        // Both halves of the contract at once: the value must not appear in argv
        // (any process can read argv) and the service name must.
        let script = concat!(
            "read -r got; ",
            "test \"$got\" = 'sekret' || { echo 'value not on stdin' >&2; exit 1; }; ",
            "case \"$2\" in windmill/ws/u/admin/token) exit 0 ;; ",
            "*) echo 'unexpected service' >&2; exit 1 ;; esac"
        );
        backend(script)
            .set_secret("ws", "u/admin/token", "sekret")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn get_does_not_hang_when_the_helper_reads_stdin() {
        // stdin is closed even with nothing to send; a helper that reads to EOF
        // would otherwise wait forever and take the request with it.
        let got = backend("cat >/dev/null; printf 'value'")
            .get_secret("ws", "u/admin/token")
            .await;
        assert_eq!(got.unwrap(), "value");
    }

    #[tokio::test]
    async fn helper_filling_its_output_pipe_before_reading_stdin_does_not_deadlock() {
        // The regression this guards: writing all of stdin before reading any
        // output. A helper that fills its stdout pipe first waits for us to read
        // while we wait for it to read, and neither ever proceeds. Both sides are
        // deliberately larger than a pipe buffer.
        let b = backend("dd if=/dev/zero bs=1024 count=300 2>/dev/null | tr '\\0' 'x'; cat >/dev/null");
        let big = "s".repeat(300 * 1024);
        tokio::time::timeout(
            std::time::Duration::from_secs(20),
            b.set_secret("ws", "u/admin/token", &big),
        )
        .await
        .expect("set deadlocked on the helper's pipes")
        .expect("set failed");
    }

    #[tokio::test]
    async fn a_hanging_helper_is_not_waited_on_forever() {
        // Without a bound a helper that waits for interactive input holds the
        // request open until something else gives up.
        let b = KeychainBackend::new(KeychainSettings {
            service_prefix: "windmill".to_string(),
            account: None,
            transport: KeychainTransport::Helper {
                command: vec!["/bin/sh".to_string(), "-c".to_string(),
                              "sleep 600".to_string(), "helper".to_string()],
            },
        });
        // The real timeout is 30s; this only checks the bound exists and that the
        // call returns an error rather than never returning.
        let outcome = tokio::time::timeout(
            HELPER_TIMEOUT + std::time::Duration::from_secs(15),
            b.get_secret("ws", "u/admin/token"),
        )
        .await;
        assert!(outcome.is_ok(), "the helper was waited on past its own timeout");
        assert!(outcome.unwrap().is_err(), "a timed-out helper must be an error");
    }
}
