#[cfg(not(feature = "private"))]
use hyper::StatusCode;

#[cfg(not(feature = "private"))]
use windmill_common::error::Error;

#[cfg(not(feature = "private"))]
pub async fn request_teams_approval() -> Result<StatusCode, Error> {
    Err(Error::InternalErr("enterprise feature only".to_string()))
}
