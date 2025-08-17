#[cfg(not(feature = "private"))]
use windmill_common::triggers::TriggerKind;

#[cfg(feature = "private")]
pub use super::handler_ee::*;

#[cfg(not(feature = "private"))]
use {
    super::{EditGcpConfig, GcpConfig, NewGcpConfig, TestGcpConfig},
    crate::db::{ApiAuthed, DB},
    axum::async_trait,
    sqlx::PgConnection,
    windmill_common::error::{Error, Result},
    windmill_git_sync::DeployedObject,
};

#[cfg(not(feature = "private"))]
pub struct GcpTriggerHandler;

#[cfg(not(feature = "private"))]
#[async_trait]
impl TriggerCrud for GcpTriggerHandler {
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfig = GcpConfig;
    type EditTriggerConfig = EditGcpConfig;
    type NewTriggerConfig = NewGcpConfig;
    type TestConnectionConfig = TestGcpConfig;

    const TABLE_NAME: &'static str = "gcp_trigger";
    const TRIGGER_TYPE: &'static str = "gcp";
    const TRIGGER_KIND: TriggerKind = TriggerKind::Gcp;
    const SUPPORTS_ENABLED: bool = true;
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str = "/gcp_triggers";
    const DEPLOYMENT_NAME: &'static str = "GCP trigger";

    fn get_deployed_object(path: String) -> DeployedObject {
        DeployedObject::GcpTrigger { path }
    }

    fn additional_select_fields(&self) -> Vec<&'static str> {
        vec![
            "gcp_resource_path",
            "topic_id",
            "subscription_id",
            "delivery_type",
            "delivery_config",
            "subscription_mode",
        ]
    }

    async fn validate_new(&self, _workspace_id: &str, _new: &Self::NewTriggerConfig) -> Result<()> {
        Err(Error::BadRequest(
            "GCP triggers are not available in open source version".to_string(),
        ))
    }

    async fn validate_edit(
        &self,
        _workspace_id: &str,
        _path: &str,
        _edit: &Self::EditTriggerConfig,
    ) -> Result<()> {
        Err(Error::BadRequest(
            "GCP triggers are not available in open source version".to_string(),
        ))
    }

    async fn create_trigger(
        &self,
        _db: &DB,
        _executor: &mut PgConnection,
        _authed: &ApiAuthed,
        _w_id: &str,
        _trigger: CreateTrigger<Self::NewTriggerConfig>,
    ) -> Result<()> {
        Err(Error::BadRequest(
            "GCP triggers are not available in open source version".to_string(),
        ))
    }

    async fn update_trigger(
        &self,
        _db: &DB,
        _executor: &mut PgConnection,
        _authed: &ApiAuthed,
        _workspace_id: &str,
        _path: &str,
        _trigger: EditTrigger<Self::EditTriggerConfig>,
    ) -> Result<()> {
        Err(Error::BadRequest(
            "GCP triggers are not available in open source version".to_string(),
        ))
    }
}
