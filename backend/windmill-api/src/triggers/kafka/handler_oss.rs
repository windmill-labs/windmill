#[allow(unused)]

#[cfg(feature = "private")]
pub use super::handler_ee::*;

#[cfg(not(feature = "private"))]
use {
    super::{EditKafkaConfig, KafkaConfig, NewKafkaConfig, TestKafkaConfig},
    crate::{
        db::{ApiAuthed, DB},
        triggers::{CreateTrigger, EditTrigger, Trigger, TriggerCrud},
    },
    axum::async_trait,
    sqlx::PgConnection,
    windmill_common::error::{Error, Result},
    windmill_git_sync::DeployedObject,
};

#[cfg(not(feature = "private"))]
#[async_trait]
impl TriggerCrud for KafkaTrigger {
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfig = KafkaConfig;
    type EditTriggerConfig = EditKafkaConfig;
    type NewTriggerConfig = NewKafkaConfig;
    type TestConnectionConfig = TestKafkaConfig;

    const TABLE_NAME: &'static str = "kafka_trigger";
    const TRIGGER_TYPE: &'static str = "kafka";
    const SUPPORTS_ENABLED: bool = true;
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str = "/kafka_triggers";
    const DEPLOYMENT_NAME: &'static str = "Kafka trigger";

    fn get_deployed_object(path: String) -> DeployedObject {
        DeployedObject::KafkaTrigger { path }
    }

    fn additional_select_fields(&self) -> Vec<&'static str> {
        vec!["kafka_resource_path", "group_id", "topics"]
    }

    async fn validate_new(&self, _workspace_id: &str, _new: &Self::NewTriggerConfig) -> Result<()> {
        Err(Error::BadRequest(
            "Kafka triggers are not available in open source version".to_string(),
        ))
    }

    async fn validate_edit(
        &self,
        _workspace_id: &str,
        _path: &str,
        _edit: &Self::EditTriggerConfig,
    ) -> Result<()> {
        Err(Error::BadRequest(
            "Kafka triggers are not available in open source version".to_string(),
        ))
    }

    async fn create_trigger(
        &self,
        _db: &DB,
        _tx: &mut PgConnection,
        _authed: &ApiAuthed,
        _w_id: &str,
        _trigger: CreateTrigger<Self::NewTriggerConfig>,
    ) -> Result<()> {
        Err(Error::BadRequest(
            "Kafka triggers are not available in open source version".to_string(),
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
            "Kafka triggers are not available in open source version".to_string(),
        ))
    }
}
