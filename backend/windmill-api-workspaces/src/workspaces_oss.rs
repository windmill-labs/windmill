#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::workspaces_ee::*;

#[cfg(not(feature = "private"))]
use crate::workspaces::{EditAutoInvite, NewServiceAccount};
#[cfg(not(feature = "private"))]
use http::StatusCode;
#[cfg(not(feature = "private"))]
use windmill_api_auth::ApiAuthed;
#[cfg(not(feature = "private"))]
use windmill_common::DB;

#[cfg(not(feature = "private"))]
pub async fn edit_auto_invite(
    _authed: ApiAuthed,
    _db: DB,
    _w_id: String,
    _ea: EditAutoInvite,
) -> windmill_common::error::Result<String> {
    Err(windmill_common::error::Error::internal_err(
        "Not implemented on OSS".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
pub async fn create_service_account(
    _authed: ApiAuthed,
    _db: DB,
    _w_id: String,
    _nu: NewServiceAccount,
) -> windmill_common::error::Result<(StatusCode, String)> {
    Err(windmill_common::error::Error::BadRequest(
        "Service accounts require Windmill Enterprise Edition".to_string(),
    ))
}
