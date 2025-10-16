#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::git_sync_ee::*;
use url::Url;
#[cfg(not(feature = "private"))]
use sqlx::{Pool, Postgres};

#[cfg(not(feature = "private"))]
pub async fn get_github_app_token_internal(
    _db: &Pool<Postgres>,
    _job_token: &str,
) -> crate::error::Result<String> {
    return Err(crate::error::Error::BadRequest("Github app authentication is not available on the open source build".to_string()))
}

pub fn prepend_token_to_github_url(
    github_url: &str,
    installation_token: &str,
) -> crate::error::Result<String> {
    let url = Url::parse(github_url)?;

    if url.host_str() != Some("github.com") {
        return Err(crate::error::Error::BadRequest("Invalid: not a github URL".to_string()));
    }

    Ok(format!(
        "https://x-access-token:{}@github.com{}",
        installation_token,
        url.path()
    ))
}
