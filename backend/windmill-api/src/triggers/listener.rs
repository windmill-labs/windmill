use crate::triggers::{crud::TriggerCrud, Trigger};
use async_trait::async_trait;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sql_builder::SqlBuilder;
use sqlx::{FromRow, Row};
use windmill_common::{
    error::{Error, Result},
    jobs::JobTriggerKind,
    triggers::TriggerKind,
    DB,
};

#[async_trait]
pub trait Listener: TriggerCrud {
    type Client;

    const TRIGGER_KIND: TriggerKind;
    const JOB_TRIGGER_KIND: JobTriggerKind;
    const EXTRA_WHERE_CLAUSE: &[&'static str];

    async fn generate_client(&self, workspace_id: &str) -> Result<Self::Client>;
    async fn listen(&self, workspace_id: &str, client: &Self::Client) -> Result<()>;
    async fn fetch_enabled_unlistened_triggers(
        &self,
        db: &DB,
    ) -> Result<Vec<ListeningTrigger<Self::TriggerConfig>>> {
        let mut fields = vec![
            "workspace_id",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "email",
        ];

        fields.extend_from_slice(&["error_handler_path", "error_handler_args", "retry"]);
        fields.extend(self.additional_select_fields());

        let mut sqlb = SqlBuilder::select_from(Self::TABLE_NAME);

        sqlb.fields(&fields).and_where("enabled IS TRUE").and_where(
            "(last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')",
        );

        for where_clause in Self::EXTRA_WHERE_CLAUSE {
            sqlb.and_where(where_clause);
        }

        let sql = sqlb
            .sql()
            .map_err(|e| Error::InternalErr(format!("SQL error: {}", e)))?;

        let triggers: Vec<Trigger<Self::TriggerConfig>> =
            sqlx::query_as(&sql).fetch_all(db).await?;

        let triggers = triggers
            .into_iter()
            .map(|trigger| ListeningTrigger {
                path: trigger.base.path,
                workspace_id: trigger.base.workspace_id,
                is_flow: trigger.base.is_flow,
                username: trigger.base.edited_by,
                email: trigger.base.email,
                script_path: trigger.base.script_path,
                trigger_config: trigger.config,
                capture_mode: false,
            })
            .collect_vec();

        Ok(triggers)
    }

    async fn fetch_unlistened_captures(
        &self,
        db: &DB,
    ) -> Result<Vec<ListeningTrigger<Self::TriggerConfig>>> {
        let fields = vec![
            "path",
            "is_flow",
            "workspace_id",
            "owner AS username",
            "email",
            "trigger_config",
        ];

        let mut sqlb = SqlBuilder::select_from("capture_config");
        sqlb.fields(&fields)
            .and_where("trigger_kind = 'postgres'")
            .and_where("last_client_ping > NOW() - INTERVAL '10 seconds'")
            .and_where("trigger_config IS NOT NULL")
            .and_where(
                "(last_server_ping IS NULL OR last_server_ping < NOW() - INTERVAL '15 seconds')",
            );

        let sql = sqlb.sql().expect("failed to build SQL");

        let captures: Vec<Capture<Self::TriggerConfig>> =
            sqlx::query_as(&sql).fetch_all(db).await?;

        let captures = captures
            .into_iter()
            .map(|capture| ListeningTrigger {
                username: capture.username,
                path: capture.path,
                workspace_id: capture.workspace_id,
                script_path: "".to_string(),
                email: capture.email,
                trigger_config: capture.trigger_config,
                capture_mode: true,
                is_flow: capture.is_flow,
            })
            .collect_vec();

        Ok(captures)
    }
}

async fn listen_to_unlistened_events<T: Listener + 'static>(
    trigger: &T,
    db: DB,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) {
    let unlistend_enabled_triggers = trigger.fetch_enabled_unlistened_triggers(&db).await;
    
    let unlisted_captures = trigger.fetch_unlistened_captures(&db).await;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Capture<T>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow>,
{
    path: String,
    is_flow: bool,
    workspace_id: String,
    username: String,
    email: String,
    trigger_config: T,
}

impl<T> FromRow<'_, sqlx::postgres::PgRow> for Capture<T>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow>,
{
    fn from_row(row: &sqlx::postgres::PgRow) -> std::result::Result<Self, sqlx::Error> {
        Ok(Capture {
            path: row.try_get("path")?,
            is_flow: row.try_get("is_flow")?,
            workspace_id: row.try_get("workspace_id")?,
            username: row.try_get("username")?,
            email: row.try_get("email")?,
            trigger_config: T::from_row(row)?,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListeningTrigger<T> {
    path: String,
    is_flow: bool,
    workspace_id: String,
    username: String,
    email: String,
    trigger_config: T,
    script_path: String,
    capture_mode: bool,
}

async fn listen_to<T: Listener>(
    trigger: T,
    db: &DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let db = db.clone();
    tokio::spawn(async move {
        listen_to_unlistened_events(&trigger, db.clone(), &killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_events(&trigger, db.clone(), &killpill_rx).await
                }
            }
        }
    });
}
