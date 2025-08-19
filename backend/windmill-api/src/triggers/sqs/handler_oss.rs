#[cfg(feature = "private")]
pub use super::handler_ee::*;

#[cfg(not(feature = "private"))]
use {
    super::{EditSqsConfig, NewSqsConfig, SqsConfig, TestSqsConfig, TriggerCrud},
    crate::{
        db::{ApiAuthed, DB},
        triggers::{CreateTrigger, EditTrigger, Trigger},
    },
    axum::async_trait,
    sqlx::PgConnection,
    windmill_common::error::{Error, Result},
    windmill_git_sync::DeployedObject,
};

#[cfg(not(feature = "private"))]
pub struct SqsTrigger;

#[cfg(not(feature = "private"))]
#[async_trait]
#[cfg(not(feature = "private"))]
impl TriggerCrud for SqsTrigger {
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfig = SqsConfig;
    type EditTriggerConfig = EditSqsConfig;
    type NewTriggerConfig = NewSqsConfig;
    type TestConnectionConfig = TestSqsConfig;

    const TABLE_NAME: &'static str = "sqs_trigger";
    const TRIGGER_TYPE: &'static str = "sqs";
    const SUPPORTS_ENABLED: bool = true;
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str = "/sqs_triggers";
    const DEPLOYMENT_NAME: &'static str = "SQS trigger";

    fn get_deployed_object(path: String) -> DeployedObject {
        DeployedObject::SqsTrigger { path }
    }

    fn additional_select_fields(&self) -> Vec<&'static str> {
        vec![
            "queue_url",
            "aws_resource_path",
            "message_attributes",
            "aws_auth_resource_type",
        ]
    }

    async fn validate_new(&self, _workspace_id: &str, _new: &Self::NewTriggerConfig) -> Result<()> {
        Err(Error::BadRequest(
            "SQS triggers are not available in open source version".to_string(),
        ))
    }

    async fn validate_edit(
        &self,
        _workspace_id: &str,
        _path: &str,
        _edit: &Self::EditTriggerConfig,
    ) -> Result<()> {
        Err(Error::BadRequest(
            "SQS triggers are not available in open source version".to_string(),
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
            "SQS triggers are not available in open source version".to_string(),
        ))
    }

    async fn update_trigger(
        &self,
        _db: &DB,
        _tx: &mut PgConnection,
        _authed: &ApiAuthed,
        _workspace_id: &str,
        _path: &str,
        _trigger: EditTrigger<Self::EditTriggerConfig>,
    ) -> Result<()> {
        Err(Error::BadRequest(
            "SQS triggers are not available in open source version".to_string(),
        ))
    }
}
