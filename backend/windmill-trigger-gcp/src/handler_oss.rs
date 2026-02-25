#[allow(unused)]
#[cfg(feature = "private")]
pub use super::handler_ee::*;

#[cfg(not(feature = "private"))]
use {
    super::GcpTrigger,
    axum::async_trait,
    sqlx::PgConnection,
    windmill_api_auth::ApiAuthed,
    windmill_common::{
        error::{Error, Result},
        DB,
    },
    windmill_git_sync::DeployedObject,
    windmill_trigger::{TriggerCrud, TriggerData},
};

#[cfg(not(feature = "private"))]
#[async_trait]
impl TriggerCrud for GcpTrigger {
    type Trigger = ();
    type TriggerConfig = ();
    type TriggerConfigRequest = ();
    type TestConnectionConfig = ();

    const TABLE_NAME: &'static str = "";
    const TRIGGER_TYPE: &'static str = "";
    const SUPPORTS_SERVER_STATE: bool = false;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str = "/gcp_triggers";
    const DEPLOYMENT_NAME: &'static str = "";
    const IS_ALLOWED_ON_CLOUD: bool = false;

    fn get_deployed_object(path: String, parent_path: Option<String>) -> DeployedObject {
        DeployedObject::GcpTrigger { path, parent_path }
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
        _trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        Err(Error::BadRequest(
            "GCP triggers are not available in open source version".to_string(),
        ))
    }
}
