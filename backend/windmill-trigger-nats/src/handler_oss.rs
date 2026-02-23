#[allow(unused)]
#[cfg(feature = "private")]
pub use super::handler_ee::*;

#[cfg(not(feature = "private"))]
use windmill_trigger::TriggerData;

#[cfg(not(feature = "private"))]
use {
    super::NatsTrigger,
    axum::async_trait,
    sqlx::PgConnection,
    windmill_api_auth::ApiAuthed,
    windmill_common::{
        db::DB,
        error::{Error, Result},
    },
    windmill_git_sync::DeployedObject,
    windmill_trigger::TriggerCrud,
};

#[cfg(not(feature = "private"))]
#[async_trait]
impl TriggerCrud for NatsTrigger {
    type Trigger = ();
    type TriggerConfig = ();
    type TriggerConfigRequest = ();
    type TestConnectionConfig = ();

    const TABLE_NAME: &'static str = "";
    const TRIGGER_TYPE: &'static str = "";
    const SUPPORTS_SERVER_STATE: bool = false;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str = "/nats_triggers";
    const DEPLOYMENT_NAME: &'static str = "";
    const IS_ALLOWED_ON_CLOUD: bool = false;

    fn get_deployed_object(path: String, parent_path: Option<String>) -> DeployedObject {
        DeployedObject::NatsTrigger { path, parent_path }
    }

    async fn create_trigger(
        &self,
        _db: &DB,
        _executor: &mut PgConnection,
        _authed: &ApiAuthed,
        _w_id: &str,
        _trigger: TriggerData<Self::TriggerConfigRequest>,
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
        _trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        Err(Error::BadRequest(
            "NATS triggers are not available in open source version".to_string(),
        ))
    }
}
