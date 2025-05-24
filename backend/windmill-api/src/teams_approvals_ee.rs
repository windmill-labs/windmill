use hyper::StatusCode;

use windmill_common::error::Error;

pub async fn request_teams_approval() -> Result<StatusCode, Error> {
    Err(Error::InternalErr("enterprise feature only".to_string()))
}