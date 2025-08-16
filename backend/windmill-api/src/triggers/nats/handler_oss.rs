#[cfg(feature = "private")]
pub use super::handler_ee::*;

#[cfg(not(feature = "private"))]
use {
    super::{EditNatsConfig, NatsConfig, NewNatsConfig, TestNatsConfig},
    crate::{
        db::{ApiAuthed, DB},
        triggers::{CreateTrigger, EditTrigger, Trigger, TriggerCrud},
    },
    axum::async_trait,
    sqlx::PgConnection,
    windmill_common::{
        error::{Error, Result},
        jobs::JobTriggerKind,
    },
    windmill_git_sync::DeployedObject,
};

#[cfg(not(feature = "private"))]
pub struct NatsTriggerHandler;

#[cfg(not(feature = "private"))]
#[async_trait]
impl TriggerCrud for NatsTriggerHandler {
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfig = NatsConfig;
    type EditTriggerConfig = EditNatsConfig;
    type NewTriggerConfig = NewNatsConfig;
    type TestConnectionConfig = TestNatsConfig;

    const TABLE_NAME: &'static str = "nats_trigger";
    const TRIGGER_TYPE: &'static str = "nats";
    const TRIGGER_KIND: JobTriggerKind = JobTriggerKind::Nats;
    const SUPPORTS_ENABLED: bool = true;
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str = "/nats_triggers";
    const DEPLOYMENT_NAME: &'static str = "NATS trigger";

    fn get_deployed_object(path: String) -> DeployedObject {
        DeployedObject::NatsTrigger { path }
    }

    fn additional_select_fields(&self) -> Vec<&'static str> {
        vec![
            "nats_resource_path",
            "subjects",
            "stream_name",
            "consumer_name",
            "use_jetstream",
        ]
    }

    async fn validate_new(&self, _workspace_id: &str, _new: &Self::NewTriggerConfig) -> Result<()> {
        Err(Error::BadRequest(
            "NATS triggers are not available in open source version".to_string(),
        ))
    }

    async fn validate_edit(
        &self,
        _workspace_id: &str,
        _path: &str,
        _edit: &Self::EditTriggerConfig,
    ) -> Result<()> {
        Err(Error::BadRequest(
            "NATS triggers are not available in open source version".to_string(),
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
            "NATS triggers are not available in open source version".to_string(),
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
            "NATS triggers are not available in open source version".to_string(),
        ))
    }
}
