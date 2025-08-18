use std::{collections::HashMap, sync::Arc};

use crate::{
    capture::insert_capture_payload,
    db::ApiAuthed,
    trigger_helpers::{trigger_runnable, TriggerJobArgs},
    triggers::{handler::TriggerCrud, Trigger, TriggerErrorHandling},
    users::fetch_api_authed,
};
use async_trait::async_trait;
use itertools::Itertools;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use sql_builder::SqlBuilder;
use sqlx::{FromRow, Row};
use tokio::sync::RwLock;
use windmill_common::{
    error::{Error, Result},
    jobs::JobTriggerKind,
    triggers::TriggerKind,
    utils::report_critical_error,
    DB, INSTANCE_NAME,
};

#[async_trait]
pub trait Listener: TriggerCrud + TriggerJobArgs {
    type Consumer: Send;

    const JOB_TRIGGER_KIND: JobTriggerKind;
    const EXTRA_TRIGGER_WHERE_CLAUSE: &[&'static str] = &[];
    const EXTRA_CAPTURE_WHERE_CLAUSE: &[&'static str] = &[];

    async fn get_consumer(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
    ) -> Result<Self::Consumer>;
    async fn consume(
        &self,
        db: &DB,
        consumer: Self::Consumer,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        err_message: Arc<RwLock<Option<String>>>,
    );
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
            "edited_at",
            "extra_perms",
        ];

        if Self::SUPPORTS_SERVER_STATE {
            fields.extend_from_slice(&["enabled", "server_id", "last_server_ping", "error"]);
        }
        fields.extend_from_slice(&["error_handler_path", "error_handler_args", "retry"]);
        fields.extend(self.additional_select_fields());

        let mut sqlb = SqlBuilder::select_from(Self::TABLE_NAME);

        sqlb.fields(&fields).and_where("enabled IS TRUE").and_where(
            "(last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')",
        );

        for where_clause in Self::EXTRA_TRIGGER_WHERE_CLAUSE {
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
                error_handling: Some(trigger.error_handling),
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
            .and_where(format!("trigger_kind = '{}'", Self::TRIGGER_KIND.to_key()))
            .and_where("last_client_ping > NOW() - INTERVAL '10 seconds'")
            .and_where("trigger_config IS NOT NULL")
            .and_where(
                "(last_server_ping IS NULL OR last_server_ping < NOW() - INTERVAL '15 seconds')",
            );

        for where_clause in Self::EXTRA_CAPTURE_WHERE_CLAUSE {
            sqlb.and_where(where_clause);
        }

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
                error_handling: None,
            })
            .collect_vec();

        Ok(captures)
    }

    async fn cleanup(
        &self,
        _db: &DB,
        _listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
    ) -> Result<()> {
        Ok(())
    }

    async fn loop_ping(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        message: Arc<RwLock<Option<String>>>,
    ) {
        loop {
            if let None = self
                .update_ping(db, listening_trigger, message.read().await.as_deref())
                .await
            {
                return;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    async fn update_ping(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        error: Option<&str>,
    ) -> Option<()> {
        if listening_trigger.capture_mode {
            self.update_capture_ping(db, listening_trigger, error).await
        } else {
            self.update_trigger_ping(db, listening_trigger, error).await
        }
    }

    async fn update_ping_and_loop_ping_status(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        loop_ping_status: Arc<RwLock<Option<String>>>,
        error: Option<String>,
    ) -> Option<()> {
        // update immediately the ping status and update the loop ping status so that the next loop pings will display the new status
        update_rw_lock(loop_ping_status.clone(), error.clone()).await;
        if let None = self
            .update_ping(db, listening_trigger, error.as_deref())
            .await
        {
            return None;
        }
        Some(())
    }

    async fn update_trigger_ping(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        error: Option<&str>,
    ) -> Option<()> {
        let updated = sqlx::query_scalar::<_, i32>(&format!(
            r#"
            UPDATE 
                {}
            SET 
                last_server_ping = now(), error = $1
            WHERE 
                workspace_id = $2 AND 
                path = $3 AND 
                server_id = $4 AND 
                enabled IS TRUE
            RETURNING 1
            "#,
            Self::TABLE_NAME
        ))
        .bind(error)
        .bind(&listening_trigger.workspace_id)
        .bind(&listening_trigger.path)
        .bind(&*INSTANCE_NAME)
        .fetch_optional(db)
        .await;

        self.handle_ping_result(updated, db, listening_trigger, "trigger")
            .await
    }

    async fn update_capture_ping(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        error: Option<&str>,
    ) -> Option<()> {
        let updated = sqlx::query_scalar!(
            r#"
            UPDATE 
                capture_config
            SET 
                last_server_ping = now(), error = $1
            WHERE 
                workspace_id = $2 AND 
                path = $3 AND 
                is_flow = $4 AND 
                trigger_kind = $5 AND 
                server_id = $6 AND 
                last_client_ping > NOW() - INTERVAL '10 seconds'
            RETURNING 1
            "#,
            error,
            &listening_trigger.workspace_id,
            &listening_trigger.path,
            &listening_trigger.is_flow,
            Self::TRIGGER_KIND as TriggerKind,
            &*INSTANCE_NAME
        )
        .fetch_optional(db)
        .await
        .map(|result| result.flatten());

        self.handle_ping_result(updated, db, listening_trigger, "capture")
            .await
    }

    async fn handle_ping_result(
        &self,
        result: sqlx::Result<Option<i32>>,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        entity_type: &str,
    ) -> Option<()> {
        match result {
            Ok(updated) => {
                if updated.is_none() {
                    self.reset_ping_for_restart(db, listening_trigger).await;
                    tracing::info!(
                        "{} {} {} changed, disabled, or deleted, stopping...",
                        Self::TRIGGER_KIND,
                        entity_type,
                        listening_trigger.path
                    );
                    return None;
                }
            }
            Err(error) => {
                tracing::warn!(
                    "Error updating ping of {} {} {}: {:?}",
                    Self::TRIGGER_KIND,
                    entity_type,
                    &listening_trigger.path,
                    error
                );
            }
        }

        Some(())
    }

    async fn reset_ping_for_restart(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
    ) {
        if listening_trigger.capture_mode {
            let _ = sqlx::query!(
                r#"
                UPDATE 
                    capture_config
                SET 
                    last_server_ping = NULL
                WHERE 
                    workspace_id = $1 AND 
                    path = $2 AND 
                    is_flow = $3 AND 
                    trigger_kind = $4 AND 
                    server_id IS NULL
                "#,
                &listening_trigger.workspace_id,
                &listening_trigger.path,
                &listening_trigger.is_flow,
                Self::TRIGGER_KIND as TriggerKind
            )
            .execute(db)
            .await;
        } else {
            let _ = sqlx::query(&format!(
                r#"
                UPDATE 
                    {}
                SET 
                    last_server_ping = NULL
                WHERE 
                    workspace_id = $1 AND 
                    path = $2 AND 
                    server_id IS NULL
                "#,
                Self::TABLE_NAME
            ))
            .bind(&listening_trigger.workspace_id)
            .bind(&listening_trigger.path)
            .execute(db)
            .await;
        }
    }

    async fn disable_with_error(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        error: String,
    ) {
        if !listening_trigger.capture_mode {
            let report_status = sqlx::query(&format!(
                r#"
                UPDATE 
                    {} 
                SET 
                    enabled = FALSE, 
                    error = $1, 
                    server_id = NULL, 
                    last_server_ping = NULL 
                WHERE 
                    workspace_id = $2 AND 
                    path = $3
            "#,
                Self::TABLE_NAME
            ))
            .bind(&error)
            .bind(&listening_trigger.workspace_id)
            .bind(&listening_trigger.path)
            .execute(db)
            .await;

            match report_status {
                Ok(_) => {
                    report_critical_error(
                        format!(
                            "Disabling {} trigger {} because of error: {}",
                            Self::TRIGGER_KIND,
                            listening_trigger.path,
                            error
                        ),
                        db.clone(),
                        Some(&listening_trigger.workspace_id),
                        None,
                    )
                    .await;
                }
                Err(disable_err) => {
                    report_critical_error(
                    format!("Could not disable {} trigger {} with err {}, disabling because of error {}", Self::TRIGGER_KIND, listening_trigger.path, disable_err, error), 
                    db.clone(),
                    Some(&listening_trigger.workspace_id),
                    None,
                ).await;
                }
            }
            return;
        }

        let report_status = sqlx::query!(
            r#"
            UPDATE 
                capture_config 
            SET 
                error = $1, 
                server_id = NULL, 
                last_server_ping = NULL 
            WHERE 
                workspace_id = $2 AND 
                path = $3 AND 
                is_flow = $4 AND 
                trigger_kind = $5
            "#,
            error,
            listening_trigger.workspace_id,
            listening_trigger.path,
            listening_trigger.is_flow,
            Self::TRIGGER_KIND as TriggerKind
        )
        .execute(db)
        .await;

        if let Err(disable_err) = report_status {
            tracing::error!(
                "Could not disable {} capture {} ({}) with err {}, disabling because of error {}",
                Self::TRIGGER_KIND,
                listening_trigger.path,
                listening_trigger.workspace_id,
                disable_err,
                error
            )
        }
    }

    async fn handle_trigger(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        payload: &Self::Payload,
    ) -> Result<()> {
        let args = Self::build_job_args(
            &listening_trigger.script_path,
            listening_trigger.is_flow,
            &listening_trigger.workspace_id,
            db,
            payload,
            HashMap::new(),
        )
        .await?;

        let authed = fetch_api_authed(
            listening_trigger.username.clone(),
            listening_trigger.email.clone(),
            &listening_trigger.workspace_id,
            db,
            Some(format!("{}-{}", Self::TRIGGER_KIND, listening_trigger.path)),
        )
        .await?;

        let (retry, error_handler_path, error_handler_args) =
            match listening_trigger.error_handling.as_ref() {
                Some(error_handling) => (
                    error_handling.retry.as_ref(),
                    error_handling.error_handler_path.as_deref(),
                    error_handling.error_handler_args.as_ref(),
                ),
                None => (None, None, None),
            };

        trigger_runnable(
            db,
            None,
            authed,
            &listening_trigger.workspace_id,
            &listening_trigger.script_path,
            listening_trigger.is_flow,
            args,
            retry,
            error_handler_path.as_deref(),
            error_handler_args,
            format!("{}_trigger/{}", Self::TRIGGER_KIND, listening_trigger.path),
        )
        .await?;

        Ok(())
    }

    async fn handle_event(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        payload: Self::Payload,
    ) {
        if listening_trigger.capture_mode {
            let main_args = Self::build_job_args_v2(false, &payload, HashMap::new());
            let preprocessor_args = Self::build_job_args_v2(true, &payload, HashMap::new());
            if let Err(err) = insert_capture_payload(
                db,
                &listening_trigger.workspace_id,
                &listening_trigger.path,
                listening_trigger.is_flow,
                &TriggerKind::Postgres,
                main_args,
                preprocessor_args,
                &listening_trigger.username,
            )
            .await
            {
                tracing::error!("Error inserting capture payload: {:?}", err);
            }
            return;
        }

        if let Err(err) = self.handle_trigger(db, listening_trigger, &payload).await {
            report_critical_error(
                format!(
                    "Failed to trigger job from {} event {}: {:?}",
                    Self::TRIGGER_KIND,
                    listening_trigger.path,
                    err
                ),
                db.clone(),
                Some(&listening_trigger.workspace_id),
                None,
            )
            .await;
        };
    }
}

async fn listening<T: Listener>(
    db: DB,
    listener: T,
    listening_trigger: ListeningTrigger<T::TriggerConfig>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let loop_ping_status = Arc::new(RwLock::new(Some("Connecting...".to_string())));

    tokio::select! {
        biased;
        _ = killpill_rx.recv() => {
            let _ = listener.cleanup(&db, &listening_trigger).await;
        }
        _ = listener.loop_ping(&db, &listening_trigger, loop_ping_status.clone()) => {
            let _ = listener.cleanup(&db, &listening_trigger).await;
        }
        consumer = listener.get_consumer(&db, &listening_trigger) => {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    let _ = listener.cleanup(&db, &listening_trigger).await;
                    return;
                }
                _ = listener.loop_ping(&db, &listening_trigger, loop_ping_status.clone()) => {
                    let _ = listener.cleanup(&db, &listening_trigger).await;
                    return;
                }
                _ = async {
                    match consumer {
                        Ok(consumer) => {
                            listener.update_ping_and_loop_ping_status(&db, &listening_trigger, loop_ping_status.clone(), None).await;
                            let _ = listener.consume(&db, consumer, &listening_trigger, loop_ping_status.clone()).await;
                        }
                        Err(error) => {
                            listener.disable_with_error(&db, &listening_trigger, error.to_string()).await;
                        }
                    }
                } => {
                    let _ = listener.cleanup(&db, &listening_trigger).await;
                    return;
                }
            }
        }
    }
}
async fn listen_to_unlistened_events<T: Copy + Listener>(
    listener: T,
    db: DB,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) {
    let unlistend_enabled_triggers = listener.fetch_enabled_unlistened_triggers(&db).await;

    match unlistend_enabled_triggers {
        Ok(mut unlistend_enabled_triggers) => {
            unlistend_enabled_triggers.shuffle(&mut rand::rng());
            for trigger in unlistend_enabled_triggers {
                let has_lock = sqlx::query_scalar(&format!(
                    r#"
                    UPDATE 
                        {} 
                    SET 
                        server_id = $1, 
                        last_server_ping = now(),
                        error = 'Connecting...'
                    WHERE 
                        enabled IS TRUE 
                        AND workspace_id = $2 
                        AND path = $3 
                        AND (last_server_ping IS NULL 
                            OR last_server_ping < now() - INTERVAL '15 seconds'
                        ) 
                    RETURNING true
                "#,
                    T::TABLE_NAME,
                ))
                .bind(&*INSTANCE_NAME)
                .bind(&trigger.workspace_id)
                .bind(&trigger.path)
                .fetch_optional(&db)
                .await;

                match has_lock {
                    Ok(has_lock) => {
                        if has_lock.flatten().unwrap_or(false) {
                            tracing::info!(
                                "Spawning new task to listen for {} event",
                                T::TABLE_NAME
                            );
                            tokio::spawn({
                                let db = db.clone();
                                let killpill_rx = killpill_rx.resubscribe();
                                async move { listening(db, listener, trigger, killpill_rx).await }
                            });
                        } else {
                            tracing::info!(
                                "{} trigger {} already being listened to",
                                T::TRIGGER_KIND,
                                trigger.path
                            );
                        }
                    }
                    Err(err) => {
                        tracing::error!(
                            "Error acquiring lock for {} trigger {}: {:?}",
                            T::TRIGGER_KIND,
                            trigger.path,
                            err
                        );
                    }
                };
            }
        }
        Err(err) => {
            tracing::error!("Error fetching {} triggers: {:?}", T::TRIGGER_KIND, err,);
        }
    }

    let unlisted_captures = listener.fetch_unlistened_captures(&db).await;

    match unlisted_captures {
        Ok(unlistened_captures) => {
            for capture in unlistened_captures {
                let has_lock = sqlx::query_scalar!(
                    r#"
                        UPDATE 
                            capture_config 
                        SET 
                            server_id = $1,
                            last_server_ping = now(), 
                            error = 'Connecting...' 
                        WHERE 
                            last_client_ping > NOW() - INTERVAL '10 seconds' AND 
                            workspace_id = $2 AND 
                            path = $3 AND 
                            is_flow = $4 AND 
                            trigger_kind = $5 AND 
                            (last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds') 
                        RETURNING true
                    "#,
                    *INSTANCE_NAME,
                    &capture.workspace_id,
                    &capture.path,
                    &capture.is_flow,
                    T::TRIGGER_KIND as TriggerKind
                )
                .fetch_optional(&db)
                .await;
                match has_lock {
                    Ok(has_lock) => {
                        if has_lock.flatten().unwrap_or(false) {
                            tokio::spawn({
                                let db = db.clone();
                                let killpill_rx = killpill_rx.resubscribe();
                                async move { listening(db, listener, capture, killpill_rx) }
                            });
                        } else {
                            tracing::info!(
                                "{} capture {} already being listened to",
                                T::TRIGGER_KIND.to_string(),
                                capture.path
                            );
                        }
                    }
                    Err(err) => {
                        tracing::error!(
                            "Error acquiring lock for capture {} {}: {:?}",
                            T::TRIGGER_KIND,
                            capture.path,
                            err
                        );
                    }
                };
            }
        }
        Err(err) => {
            tracing::error!(
                "Error fetching captures {} triggers: {:?}",
                T::TRIGGER_KIND,
                err
            );
        }
    }
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
    #[serde(flatten)]
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
    pub path: String,
    pub is_flow: bool,
    pub workspace_id: String,
    pub username: String,
    pub email: String,
    pub trigger_config: T,
    pub script_path: String,
    pub capture_mode: bool,
    pub error_handling: Option<TriggerErrorHandling>,
}

impl<T> ListeningTrigger<T> {
    pub async fn authed(&self, db: &DB, username: &str) -> Result<ApiAuthed> {
        fetch_api_authed(
            self.username.clone(),
            self.email.clone(),
            &self.workspace_id,
            db,
            Some(format!("{}-{}", username, self.path)),
        )
        .await
    }
}

pub async fn update_rw_lock<T>(lock: std::sync::Arc<tokio::sync::RwLock<T>>, value: T) -> () {
    let mut w = lock.write().await;
    *w = value;
}

fn listen_to<T: Copy + Listener>(
    trigger: T,
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        listen_to_unlistened_events(trigger, db.clone(), &killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_events(trigger, db.clone(), &killpill_rx).await
                }
            }
        }
    });
}

pub fn start_all_listeners(db: DB, killpill_rx: &tokio::sync::broadcast::Receiver<()>) {
    tracing::info!("Starting trigger listeners based on available features...");

    #[cfg(feature = "postgres_trigger")]
    {
        let postgres_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::postgres::PostgresTriggerHandler;

        listen_to(PostgresTriggerHandler, db.clone(), postgres_killpill_rx)
    }

    tracing::info!("All available trigger listeners have been started");
}
