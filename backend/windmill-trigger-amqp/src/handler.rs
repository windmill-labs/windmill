use async_trait::async_trait;
use sqlx::{types::Json as SqlxJson, PgConnection};
use windmill_api_auth::ApiAuthed;
use windmill_common::DB;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
};
use windmill_git_sync::DeployedObject;
use windmill_store::resources::try_get_resource_from_db_as;
use windmill_trigger::{Trigger, TriggerCrud, TriggerData};

use super::{
    AmqpClientBuilder, AmqpConfig, AmqpConfigRequest, AmqpOptions, AmqpResource, AmqpTrigger,
    ExchangeConfig, TestAmqpConfig,
};

#[async_trait]
impl TriggerCrud for AmqpTrigger {
    type TriggerConfig = AmqpConfig;
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfigRequest = AmqpConfigRequest;
    type TestConnectionConfig = TestAmqpConfig;

    const TABLE_NAME: &'static str = "amqp_trigger";
    const TRIGGER_TYPE: &'static str = "amqp";
    const DRAFT_KIND: windmill_common::user_drafts::UserDraftItemKind =
        windmill_common::user_drafts::UserDraftItemKind::TriggerAmqp;
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = true;
    const ROUTE_PREFIX: &'static str = "/amqp_triggers";
    const DEPLOYMENT_NAME: &'static str = "AMQP trigger";
    const ADDITIONAL_SELECT_FIELDS: &[&'static str] =
        &["amqp_resource_path", "queue_name", "exchange", "options"];
    const IS_ALLOWED_ON_CLOUD: bool = false;

    fn get_deployed_object(path: String, parent_path: Option<String>) -> DeployedObject {
        DeployedObject::AmqpTrigger { path, parent_path }
    }

    async fn validate_config(
        &self,
        _db: &DB,
        config: &Self::TriggerConfigRequest,
        _workspace_id: &str,
    ) -> Result<()> {
        if config.amqp_resource_path.trim().is_empty() {
            return Err(Error::BadRequest(
                "AMQP resource path cannot be empty".to_string(),
            ));
        }

        if config.queue_name.trim().is_empty() {
            return Err(Error::BadRequest("Queue name cannot be empty".to_string()));
        }

        super::validate_amqp_options(config.options.as_ref()).map_err(Error::BadRequest)?;

        Ok(())
    }

    async fn create_trigger(
        &self,
        _db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let resolved_edited_by = trigger.base.resolve_edited_by(authed);
        let resolved_permissioned_as = trigger.base.resolve_permissioned_as(authed);
        let exchange = trigger.config.exchange.map(SqlxJson);
        let options = trigger.config.options.map(SqlxJson);

        sqlx::query!(
            r#"
            INSERT INTO amqp_trigger (
                amqp_resource_path,
                queue_name,
                exchange,
                options,
                workspace_id,
                path,
                script_path,
                is_flow,
                permissioned_as,
                mode,
                edited_by,
                error_handler_path,
                error_handler_args,
                retry
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            )"#,
            trigger.config.amqp_resource_path,
            trigger.config.queue_name,
            exchange as Option<SqlxJson<ExchangeConfig>>,
            options as Option<SqlxJson<AmqpOptions>>,
            w_id,
            trigger.base.path,
            trigger.base.script_path,
            trigger.base.is_flow,
            resolved_permissioned_as,
            trigger.base.mode() as _,
            &resolved_edited_by,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _
        )
        .execute(tx)
        .await?;

        Ok(())
    }

    async fn update_trigger(
        &self,
        _db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        workspace_id: &str,
        path: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let resolved_edited_by = trigger.base.resolve_edited_by(authed);
        let resolved_permissioned_as = trigger.base.resolve_permissioned_as(authed);
        let exchange = trigger.config.exchange.map(SqlxJson);
        let options = trigger.config.options.map(SqlxJson);

        // Important to set server_id to NULL to stop the current amqp listener
        sqlx::query!(
            r#"
            UPDATE
                amqp_trigger
            SET
                amqp_resource_path = $1,
                queue_name = $2,
                exchange = $3,
                options = $4,
                is_flow = $5,
                edited_by = $6,
                permissioned_as = $7,
                script_path = $8,
                path = $9,
                edited_at = now(),
                error = NULL,
                server_id = NULL,
                error_handler_path = $12,
                error_handler_args = $13,
                retry = $14
            WHERE
                workspace_id = $10 AND
                path = $11
            "#,
            trigger.config.amqp_resource_path,
            trigger.config.queue_name,
            exchange as Option<SqlxJson<ExchangeConfig>>,
            options as Option<SqlxJson<AmqpOptions>>,
            trigger.base.is_flow,
            &resolved_edited_by,
            resolved_permissioned_as,
            trigger.base.script_path,
            trigger.base.path,
            workspace_id,
            path,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _
        )
        .execute(tx)
        .await?;

        Ok(())
    }

    async fn test_connection(
        &self,
        db: &DB,
        authed: &ApiAuthed,
        user_db: &UserDB,
        workspace_id: &str,
        config: Self::TestConnectionConfig,
    ) -> Result<()> {
        let amqp_resource = try_get_resource_from_db_as::<AmqpResource>(
            authed,
            Some(user_db.clone()),
            db,
            &config.amqp_resource_path,
            workspace_id,
        )
        .await?;

        let client_builder = AmqpClientBuilder::new(amqp_resource, "", None, None);

        client_builder
            .test_connection()
            .await
            .map_err(|err| Error::BadConfig(format!("Error connecting to AMQP broker: {}", err)))?;

        Ok(())
    }
}
