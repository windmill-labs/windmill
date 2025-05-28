use axum::Router;

pub struct ServiceProviderExt();

pub async fn build_sp_extension() -> anyhow::Result<ServiceProviderExt> {
    crate::saml_ee::build_sp_extension().await
}

pub fn global_service() -> Router {
    crate::saml_ee::global_service()
}

pub async fn acs() -> String {
    crate::saml_ee::acs().await
}