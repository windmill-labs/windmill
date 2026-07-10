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
}
