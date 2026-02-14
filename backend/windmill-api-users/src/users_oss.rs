use std::sync::Arc;

use windmill_api_auth::ApiAuthed;

use crate::users::{EditPassword, NewUser};

use windmill_common::webhook::WebhookShared;
use windmill_common::DB;

use argon2::Argon2;

use axum::{extract::Extension, Json};

use http::StatusCode;

use serde::Deserialize;

use windmill_common::error::{Error, Result};

pub async fn create_user(
    _authed: ApiAuthed,
    _db: DB,
    _webhook: WebhookShared,
    _argon2: Arc<Argon2<'_>>,
    mut _nu: NewUser,
) -> Result<(StatusCode, String)> {
    Err(Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

pub async fn set_password(
    _db: DB,
    _argon2: Arc<Argon2<'_>>,
    _authed: ApiAuthed,
    _user_email: &str,
    _ep: EditPassword,
) -> Result<String> {
    Err(Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

pub fn hash_password(_argon2: Arc<Argon2<'_>>, _password: String) -> Result<String> {
    Err(Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct OnboardingData {
    pub touch_point: String,
    pub use_case: String,
}

pub async fn submit_onboarding_data(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Json(_data): Json<OnboardingData>,
) -> Result<String> {
    Err(Error::internal_err(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}
