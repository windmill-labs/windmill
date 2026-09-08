#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::git_sync_ee::*;
#[cfg(not(feature = "private"))]
use sqlx::{Pool, Postgres};
use url::Url;

#[cfg(not(feature = "private"))]
pub async fn get_github_app_token_internal(
    _db: &Pool<Postgres>,
    _job_token: &str,
) -> crate::error::Result<String> {
    return Err(crate::error::Error::BadRequest(
        "Github app authentication is not available on the open source build".to_string(),
    ));
}

lazy_static::lazy_static! {
    /// Matches a `user:password@` (or `user@`) userinfo component right after the URL scheme.
    static ref GIT_URL_USERINFO_RE: regex::Regex =
        regex::Regex::new(r"://[^/@]+@").unwrap();
}

/// Strip embedded credentials (the `user:password@` userinfo component) from a git URL so it can be
/// safely included in error messages and logs. Falls back to a regex when the URL does not parse.
pub fn sanitize_git_url(url: &str) -> String {
    if let Ok(mut parsed) = Url::parse(url) {
        if !parsed.username().is_empty() || parsed.password().is_some() {
            // These setters only fail for cannot-be-a-base URLs, in which case we keep the parsed
            // string as-is and let the regex fallback below handle stripping.
            let _ = parsed.set_username("");
            let _ = parsed.set_password(None);
        }
        return GIT_URL_USERINFO_RE
            .replace(parsed.as_str(), "://***@")
            .into_owned();
    }
    GIT_URL_USERINFO_RE.replace(url, "://***@").into_owned()
}

/// Validate a user-supplied git remote URL before it is handed to `git` (`clone`,
/// `ls-remote`, `remote add`, `fetch`, ...). Two classes of abuse are rejected:
///  - Argument injection: a URL that git parses as a command-line option (e.g.
///    `--upload-pack=<cmd>`) turns `git ls-remote <url> HEAD` into arbitrary command
///    execution on the worker host, outside any job sandbox.
///  - Dangerous transports: git's remote-helper syntax (`ext::sh -c ...`, `fd::...`) runs
///    arbitrary programs, and `file://` / local paths read host files — both escape the
///    intended network-only fetch.
///
/// Only the standard network transports are allowed: `http(s)`, `ssh`, `git`, and the
/// scp-like `[user@]host:path` shorthand. Validation is transport-syntax based (not git
/// version dependent) so it holds regardless of git's own option/protocol handling.
pub fn validate_git_repo_url(url: &str) -> crate::error::Result<()> {
    let reject =
        |msg: &str| crate::error::Error::BadRequest(format!("Invalid git repository URL: {msg}"));

    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(reject("the URL is empty"));
    }
    // Leading '-' makes git parse the URL as an option (argument injection).
    if trimmed.starts_with('-') {
        return Err(reject("the URL must not start with '-'"));
    }
    // `<helper>::<address>` remote-helper transports execute arbitrary programs.
    if trimmed.contains("::") {
        return Err(reject("remote-helper transports (`::`) are not allowed"));
    }

    if let Some((scheme, _rest)) = trimmed.split_once("://") {
        // A real scheme is ASCII-alnum plus `+ - .` and holds no slash (a slash means the
        // `://` came from the path, so there is no scheme and this is not a valid URL).
        let is_scheme = !scheme.is_empty()
            && !scheme.contains('/')
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
        if !is_scheme {
            return Err(reject("malformed URL scheme"));
        }
        match scheme.to_ascii_lowercase().as_str() {
            "http" | "https" | "ssh" | "git" => Ok(()),
            other => Err(reject(&format!(
                "scheme `{other}` is not allowed (use http(s), ssh, or git)"
            ))),
        }
    } else {
        // No scheme: accept only the scp-like `[user@]host:path` shorthand. The host (the
        // part before the first `:`) must be non-empty and slash-free; a slash there means a
        // local path (`./repo`, `/abs/repo`), and a single-letter host is a Windows drive.
        let Some((host, _path)) = trimmed.split_once(':') else {
            return Err(reject(
                "local paths are not allowed; use an http(s), ssh, or git URL",
            ));
        };
        let bad_host = host.is_empty()
            || host.contains('/')
            || (host.len() == 1 && host.chars().all(|c| c.is_ascii_alphabetic()));
        if bad_host {
            return Err(reject(
                "local paths are not allowed; use an http(s), ssh, or git URL",
            ));
        }
        Ok(())
    }
}

pub fn prepend_token_to_github_url(
    github_url: &str,
    installation_token: &str,
) -> crate::error::Result<String> {
    let url = Url::parse(github_url)?;

    let host = url.host_str().ok_or_else(|| {
        crate::error::Error::BadRequest("Invalid GitHub URL: no host".to_string())
    })?;

    Ok(format!(
        "https://x-access-token:{}@{}{}",
        installation_token,
        host,
        url.path()
    ))
}

#[cfg(test)]
mod tests {
    use super::sanitize_git_url;

    #[test]
    fn strips_username_and_password() {
        assert_eq!(
            sanitize_git_url("https://user:p4ssw0rd@github.com/org/repo.git"),
            "https://github.com/org/repo.git"
        );
    }

    #[test]
    fn strips_token_only_userinfo() {
        assert_eq!(
            sanitize_git_url("https://ghp_secrettoken@github.com/org/repo.git"),
            "https://github.com/org/repo.git"
        );
    }

    #[test]
    fn leaves_credential_free_url_untouched() {
        assert_eq!(
            sanitize_git_url("https://github.com/org/repo.git"),
            "https://github.com/org/repo.git"
        );
    }

    #[test]
    fn strips_credentials_from_unparseable_url() {
        // scp-like syntax that `url::Url` cannot parse
        assert_eq!(
            sanitize_git_url("not a url://user:secret@host/repo"),
            "not a url://***@host/repo"
        );
    }

    use super::validate_git_repo_url;

    #[test]
    fn accepts_standard_transports() {
        for url in [
            "https://github.com/org/repo.git",
            "http://internal.example/org/repo.git",
            "https://user:token@github.com/org/repo.git",
            "ssh://git@github.com/org/repo.git",
            "ssh://git@github.com:2222/org/repo.git",
            "git://github.com/org/repo.git",
            "git@github.com:org/repo.git",
            "user@host.example:path/to/repo",
        ] {
            assert!(validate_git_repo_url(url).is_ok(), "should accept {url}");
        }
    }

    #[test]
    fn rejects_argument_injection() {
        for url in [
            "--upload-pack=touch /tmp/pwned",
            "-oProxyCommand=touch /tmp/pwned",
            "--config=core.fsmonitor=touch /tmp/pwned",
        ] {
            assert!(validate_git_repo_url(url).is_err(), "should reject {url}");
        }
    }

    #[test]
    fn rejects_remote_helpers_and_local_transports() {
        for url in [
            "ext::sh -c 'id > /tmp/pwned'",
            "fd::17/foo",
            "file:///etc/passwd",
            "/etc/passwd",
            "./local/repo",
            "../local/repo",
            "ftp://host/repo",
            "C:\\path\\to\\repo",
            "",
            "   ",
        ] {
            assert!(validate_git_repo_url(url).is_err(), "should reject {url:?}");
        }
    }
}
