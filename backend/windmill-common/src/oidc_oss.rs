use crate::{error::Result as WindmillResult, DB};
use serde::{Deserialize, Serialize};

pub trait AdditionalClaims: Serialize + for<'de> Deserialize<'de> {}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceClaim {
    pub workspace: String,
}

impl AdditionalClaims for WorkspaceClaim {}

#[derive(Serialize, Deserialize)]
pub struct InstanceClaim {}

impl AdditionalClaims for InstanceClaim {}

#[derive(Serialize, Deserialize)]
pub struct JobClaim {
    pub workspace: String,
    pub job: String,
    pub path: Option<String>,
    pub groups: Vec<String>,
    pub email: String,
    pub username: String,
    pub is_operator: bool,
    pub is_admin: bool,
    pub is_super_admin: bool,
    pub folders: Vec<String>,
}

impl AdditionalClaims for JobClaim {}

pub struct WindmillIdToken;

pub async fn generate_id_token<T: AdditionalClaims>(
    _additional_claims: T,
    _audience: &str,
    _db: Option<&DB>,
) -> anyhow::Result<WindmillIdToken> {
    crate::oidc_ee::generate_id_token(_additional_claims, _audience, _db).await
}

pub async fn get_private_key(db: Option<&DB>) -> anyhow::Result<String> {
    crate::oidc_ee::get_private_key(db).await
}