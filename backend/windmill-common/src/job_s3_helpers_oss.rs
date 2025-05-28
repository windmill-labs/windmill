use crate::{error::Result, object_store::ObjectStoreResource, DB};

pub enum TokenGenerator<'c> {
    AsClient(&'c crate::client::AuthedClient),
    AsServerInstance(),
}

impl<'c> TokenGenerator<'c> {
    pub async fn gen_token(&self, _audience: &str, _db: Option<&DB>) -> anyhow::Result<String> {
        crate::job_s3_helpers_ee::TokenGenerator::gen_token(self, _audience, _db).await
    }
}

pub async fn get_s3_resource_internal(
    _token_generator: &TokenGenerator<'_>,
    _audience: &str,
    _workspace_id: &str,
    _resource_path: &str,
    _db: Option<&DB>,
) -> Result<ObjectStoreResource> {
    crate::job_s3_helpers_ee::get_s3_resource_internal(_token_generator, _audience, _workspace_id, _resource_path, _db).await
}

pub(crate) async fn generate_s3_aws_oidc_resource(
    _token_generator: &TokenGenerator<'_>,
    _audience: &str,
    _role_arn: &str,
    _region: &str,
    _db: Option<&DB>,
) -> Result<ObjectStoreResource> {
    crate::job_s3_helpers_ee::generate_s3_aws_oidc_resource(_token_generator, _audience, _role_arn, _region, _db).await
}