use serde::{de::DeserializeOwned, Serialize};
use sqlx::{FromRow, PgExecutor};

use crate::{
    error::{Error, Result},
    jobs::JobTriggerKind,
    triggers::{base::TriggerErrorHandling, query::StandardTriggerQuery},
};

#[async_trait::async_trait]
pub trait TriggerCrud: Send + Sync + 'static {
    type Trigger: Serialize
        + DeserializeOwned
        + for<'r> FromRow<'r, sqlx::postgres::PgRow>
        + Clone
        + Send
        + Sync
        + Unpin;

    type NewTrigger: DeserializeOwned + Send + Sync;

    type EditTrigger: DeserializeOwned + Send + Sync;

    const TABLE_NAME: &'static str;

    const TRIGGER_TYPE: &'static str;

    const TRIGGER_KIND: JobTriggerKind;

    const SUPPORTS_ENABLED: bool = true;

    const SUPPORTS_SERVER_STATE: bool = true;

    fn extract_error_handling(&self, trigger: &Self::Trigger) -> Result<TriggerErrorHandling> {
        Ok(TriggerErrorHandling::default())
    }

    async fn validate_new(
        &self,
        _workspace_id: &str,
        _new_trigger: &Self::NewTrigger,
    ) -> Result<()> {
        Ok(())
    }

    async fn validate_edit(
        &self,
        _workspace_id: &str,
        _path: &str,
        _edit: &Self::EditTrigger,
    ) -> Result<()> {
        Ok(())
    }

    fn additional_select_fields(&self) -> Vec<&'static str> {
        vec![]
    }

    async fn test_connection(
        &self,
        _workspace_id: &str,
        _config: serde_json::Value,
    ) -> Result<serde_json::Value> {
        Err(
            anyhow::anyhow!("Test connection not supported for this trigger type".to_string(),)
                .into(),
        )
    }

    fn get_scope(&self, operation: &str, path: &str) -> String {
        format!("triggers.{}.{}:{}", operation, Self::TRIGGER_TYPE, path)
    }

    async fn insert_trigger<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        trigger: &Self::Trigger,
    ) -> Result<()>;

    async fn update_trigger<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        workspace_id: &str,
        path: &str,
        trigger: &Self::Trigger,
    ) -> Result<()>;

    async fn get_trigger_by_path<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        workspace_id: &str,
        path: &str,
    ) -> Result<Self::Trigger> {
        let mut fields = vec![
            "workspace_id",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "email",
            "edited_at",
            "extra_perms",
        ];

        if Self::SUPPORTS_SERVER_STATE {
            fields.extend_from_slice(&["enabled", "server_id", "last_server_ping", "error"]);
        }

        fields.extend_from_slice(&["error_handler_path", "error_handler_args", "retry"]);
        fields.extend(self.additional_select_fields());

        let sql = format!(
            "SELECT {} FROM {} WHERE workspace_id = $1 AND path = $2",
            fields.join(", "),
            Self::TABLE_NAME
        );

        sqlx::query_as(&sql)
            .bind(workspace_id)
            .bind(path)
            .fetch_optional(executor)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Trigger not found at path: {}", path)))
    }

    fn validate_trigger(trigger: &Self::Trigger) -> Result<()> {
        Ok(())
    }

    async fn exists<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        workspace_id: &str,
        path: &str,
    ) -> Result<bool> {
        let exists = sqlx::query_scalar(&format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE workspace_id = $1 AND path = $2)",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(path)
        .fetch_one(executor)
        .await?;

        Ok(exists)
    }

    async fn delete_by_path<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        workspace_id: &str,
        path: &str,
    ) -> Result<bool> {
        let deleted = sqlx::query(&format!(
            "DELETE FROM {} WHERE workspace_id = $1 AND path = $2",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(path)
        .execute(executor)
        .await?
        .rows_affected();

        Ok(deleted > 0)
    }

    async fn set_enabled<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        workspace_id: &str,
        path: &str,
        enabled: bool,
    ) -> Result<bool> {
        if !Self::SUPPORTS_SERVER_STATE {
            return Err(anyhow::anyhow!(
                "Enable/disable not supported for this trigger type".to_string(),
            )
            .into());
        }

        let updated = sqlx::query(&format!(
            "UPDATE {} SET enabled = $3 WHERE workspace_id = $1 AND path = $2",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(path)
        .bind(enabled)
        .execute(executor)
        .await?
        .rows_affected();

        Ok(updated > 0)
    }

    async fn list_triggers<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        workspace_id: &str,
        query: &StandardTriggerQuery,
    ) -> Result<Vec<Self::Trigger>> {
        use sql_builder::{bind::Bind, SqlBuilder};

        let mut fields = vec![
            "workspace_id",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "email",
            "edited_at",
            "extra_perms",
        ];

        if Self::SUPPORTS_SERVER_STATE {
            fields.extend_from_slice(&["enabled", "server_id", "last_server_ping", "error"]);
        }

        fields.extend_from_slice(&["error_handler_path", "error_handler_args", "retry"]);
        fields.extend(self.additional_select_fields());

        let mut sqlb = SqlBuilder::select_from(Self::TABLE_NAME);

        sqlb.fields(&fields)
            .order_by(query.order_field(), query.order_desc.unwrap_or(true))
            .and_where("workspace_id = ?".bind(&workspace_id))
            .offset(query.offset())
            .limit(query.limit());

        if let Some(path) = &query.path {
            sqlb.and_where_eq("script_path", "?".bind(path));
        }

        if let Some(is_flow) = query.is_flow {
            sqlb.and_where_eq("is_flow", "?".bind(&is_flow));
        }

        if let Some(path_start) = &query.path_start {
            sqlb.and_where_like_left("path", path_start);
        }

        if let Some(enabled) = query.enabled {
            if Self::SUPPORTS_SERVER_STATE {
                sqlb.and_where_eq("enabled", "?".bind(&enabled));
            }
        }

        let sql = sqlb
            .sql()
            .map_err(|e| Error::InternalErr(format!("SQL error: {}", e)))?;

        let triggers = sqlx::query_as(&sql).fetch_all(executor).await?;

        Ok(triggers)
    }
}

#[macro_export]
macro_rules! trigger_sql {
    (select_fields, $trigger_type:expr, $supports_server_state:expr) => {{
        let mut fields = vec![
            "workspace_id",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "email",
            "edited_at",
            "extra_perms",
        ];

        if $supports_server_state {
            fields.extend_from_slice(&["enabled", "server_id", "last_server_ping", "error"]);
        }

        fields.extend_from_slice(&["error_handler_path", "error_handler_args", "retry"]);
        fields
    }};
}
