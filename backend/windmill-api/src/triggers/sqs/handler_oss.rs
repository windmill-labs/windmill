#[cfg(feature = "private")]
#[allow(unused)]
pub use super::handler_ee::*;

#[cfg(not(feature = "private"))]
use {
    super::SqsTrigger,
    crate::{
        db::{ApiAuthed, DB},
        triggers::{Trigger, TriggerCrud, TriggerData},
    },
    axum::async_trait,
    sqlx::PgConnection,
    windmill_common::error::{Error, Result},
    windmill_git_sync::DeployedObject,
};

#[cfg(not(feature = "private"))]
#[async_trait]
#[cfg(not(feature = "private"))]
impl TriggerCrud for SqsTrigger {
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfig = ();
    type TriggerConfigRequest = ();
    type TestConnectionConfig = ();

    const TABLE_NAME: &'static str = "";
    const TRIGGER_TYPE: &'static str = "";
    const SUPPORTS_ENABLED: bool = false;
    const SUPPORTS_SERVER_STATE: bool = false;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str = "/sqs_triggers";
    const DEPLOYMENT_NAME: &'static str = "";
    const IS_CLOUD_HOSTED: bool = false;

    fn get_deployed_object(path: String) -> DeployedObject {
        DeployedObject::SqsTrigger { path }
    }

    async fn create_trigger(
        &self,
        _db: &DB,
        _tx: &mut PgConnection,
        _authed: &ApiAuthed,
        _w_id: &str,
        _trigger: TriggerData<Self::TriggerConfigRequest>,
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
        _trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        Err(Error::BadRequest(
            "SQS triggers are not available in open source version".to_string(),
        ))
    }
}
