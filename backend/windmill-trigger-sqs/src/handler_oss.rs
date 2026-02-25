#[cfg(feature = "private")]
#[allow(unused)]
pub use super::handler_ee::*;

#[cfg(not(feature = "private"))]
use {
    super::SqsTrigger,
    axum::async_trait,
    sqlx::PgConnection,
    windmill_api_auth::ApiAuthed,
    windmill_common::{
        error::{Error, Result},
        DB,
    },
    windmill_git_sync::DeployedObject,
    windmill_trigger::{Trigger, TriggerCrud, TriggerData},
};

#[cfg(not(feature = "private"))]
#[async_trait]
impl TriggerCrud for SqsTrigger {
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfig = ();
    type TriggerConfigRequest = ();
    type TestConnectionConfig = ();

    const TABLE_NAME: &'static str = "";
    const TRIGGER_TYPE: &'static str = "";
    const SUPPORTS_SERVER_STATE: bool = false;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str = "/sqs_triggers";
    const DEPLOYMENT_NAME: &'static str = "";
    const IS_ALLOWED_ON_CLOUD: bool = false;

    fn get_deployed_object(path: String, parent_path: Option<String>) -> DeployedObject {
        DeployedObject::SqsTrigger { path, parent_path }
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
