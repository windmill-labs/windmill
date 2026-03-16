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
