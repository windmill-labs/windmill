#[cfg(feature = "private")]
use crate::teams_approvals_ee;

use hyper::StatusCode;

use windmill_common::error::Error;

pub async fn request_teams_approval() -> Result<StatusCode, Error> {
    #[cfg(feature = "private")]
    {
        return teams_approvals_ee::request_teams_approval().await;
    }
    #[cfg(not(feature = "private"))]
    {
        Err(Error::InternalErr("enterprise feature only".to_string()))
    }
}
