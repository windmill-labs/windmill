use axum::async_trait;
use itertools::Itertools;
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
    MqttClientBuilder, MqttClientVersion, MqttConfig, MqttConfigRequest, MqttResource, MqttTrigger,
    MqttV3Config, MqttV5Config, SubscribeTopic, TestMqttConfig,
};

#[async_trait]
impl TriggerCrud for MqttTrigger {
    type TriggerConfig = MqttConfig;
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfigRequest = MqttConfigRequest;
    type TestConnectionConfig = TestMqttConfig;

    const TABLE_NAME: &'static str = "mqtt_trigger";
    const TRIGGER_TYPE: &'static str = "mqtt";
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = true;
    const ROUTE_PREFIX: &'static str = "/mqtt_triggers";
    const DEPLOYMENT_NAME: &'static str = "MQTT trigger";
    const ADDITIONAL_SELECT_FIELDS: &[&'static str] = &[
        "mqtt_resource_path",
        "subscribe_topics",
        "v3_config",
        "v5_config",
        "client_id",
        "client_version",
    ];
    const IS_ALLOWED_ON_CLOUD: bool = false;

    fn get_deployed_object(path: String, parent_path: Option<String>) -> DeployedObject {
        DeployedObject::MqttTrigger { path, parent_path }
    }

    async fn validate_config(
        &self,
        _db: &DB,
        config: &Self::TriggerConfigRequest,
        _workspace_id: &str,
    ) -> Result<()> {
        if config.mqtt_resource_path.trim().is_empty() {
            return Err(Error::BadRequest(
                "MQTT resource path cannot be empty".to_string(),
            ));
        }

        if config.subscribe_topics.is_empty() {
            return Err(Error::BadRequest(
                "At least one subscribe topic must be specified".to_string(),
            ));
        }

        Ok(())
    }

    async fn create_trigger(
        &self,
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let resolved_edited_by = trigger.base.resolve_edited_by(authed);
        let resolved_email = trigger.base.resolve_email(authed, db, w_id).await?;
        let subscribe_topics = trigger
            .config
            .subscribe_topics
            .into_iter()
            .map(SqlxJson)
            .collect_vec();
        let v3_config = trigger.config.v3_config.map(SqlxJson);
        let v5_config = trigger.config.v5_config.map(SqlxJson);

        sqlx::query!(
            r#"
            INSERT INTO mqtt_trigger (
                mqtt_resource_path,
                subscribe_topics,
                client_version,
                client_id,
                v3_config,
                v5_config,
                workspace_id,
                path,
                script_path,
                is_flow,
                email,
                mode,
                edited_by,
                error_handler_path,
                error_handler_args,
                retry
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16
            )"#,
            trigger.config.mqtt_resource_path,
            subscribe_topics.as_slice() as &[SqlxJson<SubscribeTopic>],
            trigger.config.client_version as Option<MqttClientVersion>,
            trigger.config.client_id,
            v3_config as Option<SqlxJson<MqttV3Config>>,
            v5_config as Option<SqlxJson<MqttV5Config>>,
            w_id,
            trigger.base.path,
            trigger.base.script_path,
            trigger.base.is_flow,
            resolved_email,
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
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        workspace_id: &str,
        path: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let resolved_edited_by = trigger.base.resolve_edited_by(authed);
        let resolved_email = trigger.base.resolve_email(authed, db, workspace_id).await?;
        let subscribe_topics = trigger
            .config
            .subscribe_topics
            .into_iter()
            .map(SqlxJson)
            .collect_vec();
        let v3_config = trigger.config.v3_config.map(SqlxJson);
        let v5_config = trigger.config.v5_config.map(SqlxJson);

        // Important to set server_id to NULL to stop current mqtt listener
        sqlx::query!(
            r#"
            UPDATE
                mqtt_trigger
            SET
                mqtt_resource_path = $1,
                subscribe_topics = $2,
                client_version = $3,
                client_id = $4,
                v3_config = $5,
                v5_config = $6,
                is_flow = $7,
                edited_by = $8,
                email = $9,
                script_path = $10,
                path = $11,
                edited_at = now(),
                error = NULL,
                server_id = NULL,
                error_handler_path = $14,
                error_handler_args = $15,
                retry = $16
            WHERE
                workspace_id = $12 AND
                path = $13
            "#,
            trigger.config.mqtt_resource_path,
            subscribe_topics.as_slice() as &[SqlxJson<SubscribeTopic>],
            trigger.config.client_version as Option<MqttClientVersion>,
            trigger.config.client_id,
            v3_config as Option<SqlxJson<MqttV3Config>>,
            v5_config as Option<SqlxJson<MqttV5Config>>,
            trigger.base.is_flow,
            &resolved_edited_by,
            resolved_email,
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
        let mqtt_resource = try_get_resource_from_db_as::<MqttResource>(
            authed,
            Some(user_db.clone()),
            db,
            &config.mqtt_resource_path,
            workspace_id,
        )
        .await?;

        let connect_f = async {
            let client_builder = MqttClientBuilder::new(
                mqtt_resource,
                Some(""),
                vec![],
                config.v3_config.as_ref(),
                config.v5_config.as_ref(),
                config.client_version.as_ref(),
            );

            client_builder.build_client().await.map_err(|err| {
                Error::BadConfig(format!(
                    "Error connecting to mqtt broker: {}",
                    err.to_string()
                ))
            })
        };

        connect_f.await?;
        Ok(())
    }
}
