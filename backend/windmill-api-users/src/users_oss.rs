#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::users_ee::*;

#[cfg(not(feature = "private"))]
use crate::users::ImpersonateServiceAccountRequest;
#[cfg(not(feature = "private"))]
use http::StatusCode;
#[cfg(not(feature = "private"))]
use tower_cookies::Cookies;
#[cfg(not(feature = "private"))]
use windmill_api_auth::ApiAuthed;
#[cfg(not(feature = "private"))]
use windmill_common::DB;

#[cfg(not(feature = "private"))]
pub async fn impersonate_service_account(
    _db: DB,
    _authed: ApiAuthed,
    _cookies: Cookies,
    _current_token: String,
    _w_id: String,
    _req: ImpersonateServiceAccountRequest,
) -> windmill_common::error::Result<(StatusCode, String)> {
    Err(windmill_common::error::Error::BadRequest(
        "Service accounts require Windmill Enterprise Edition".to_string(),
    ))
}
