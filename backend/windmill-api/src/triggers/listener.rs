use std::{collections::HashMap, fmt::Debug, sync::Arc};

use crate::{
    capture::insert_capture_payload,
    db::ApiAuthed,
    triggers::{
        handler::TriggerCrud,
        trigger_helpers::{trigger_runnable, ActionToTake, TriggerJobArgs},
        Trigger, TriggerErrorHandling, COMMON_TRIGGER_FIELDS,
    },
    users::fetch_api_authed,
};
use async_trait::async_trait;
use itertools::Itertools;
use rand::seq::SliceRandom;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sql_builder::SqlBuilder;
use sqlx::{FromRow, Row};
use tokio::sync::RwLock;
use windmill_common::{
    error::{to_anyhow, Error, Result},
    jobs::JobTriggerKind,
    mailbox::{Mailbox, MailboxType},
    triggers::TriggerKind,
    utils::report_critical_error,
    DB, INSTANCE_NAME,
};

#[allow(unused)]
#[async_trait]
pub trait Listener: TriggerCrud + TriggerJobArgs {
    type Consumer: Send;
    type Extra: Send + Sync;
    type ExtraState: Send + Sync;

    //to use in next PR to add job trigger kind to eow
    #[allow(unused)]
    const JOB_TRIGGER_KIND: JobTriggerKind;
    const EXTRA_TRIGGER_AND_WHERE_CLAUSE: &[&'static str] = &[];
    const EXTRA_CAPTURE_AND_WHERE_CLAUSE: &[&'static str] = &[];

    async fn get_consumer(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        err_message: Arc<RwLock<Option<String>>>,
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<Option<Self::Consumer>>;
    async fn consume(
        &self,
        db: &DB,
        consumer: Self::Consumer,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        err_message: Arc<RwLock<Option<String>>>,
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
        extra: Option<&Self::ExtraState>,
    );
    async fn fetch_enabled_unlistened_triggers(
        &self,
        db: &DB,
    ) -> Result<Vec<ListeningTrigger<Self::TriggerConfig>>> {
        let mut fields = Vec::from(COMMON_TRIGGER_FIELDS);

        fields.extend_from_slice(&["error_handler_path", "error_handler_args", "retry"]);
        fields.extend_from_slice(Self::ADDITIONAL_SELECT_FIELDS);

        let mut sqlb = SqlBuilder::select_from(Self::TABLE_NAME);

        sqlb.fields(&fields).and_where("enabled IS TRUE").and_where(
            "(last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')",
        );

        for where_clause in Self::EXTRA_TRIGGER_AND_WHERE_CLAUSE {
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
                mode: Mode::Trigger(trigger.base.action_to_take),
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

        for where_clause in Self::EXTRA_CAPTURE_AND_WHERE_CLAUSE {
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
                mode: Mode::Capture,
                is_flow: capture.is_flow,
                error_handling: None,
            })
            .collect_vec();

        Ok(captures)
    }

    async fn get_extra_state(&self) -> Option<Self::ExtraState> {
        None
    }

    async fn cleanup(
        &self,
        _db: &DB,
        _listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        _extra: Option<&Self::ExtraState>,
    ) -> Result<()> {
        Ok(())
    }

    async fn loop_ping(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        status: Arc<RwLock<Option<String>>>,
        error_message: Option<String>,
    ) {
        update_rw_lock(status.clone(), error_message).await;
        loop {
            if let None = self
                .update_ping(db, listening_trigger, status.read().await.as_deref())
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
        if listening_trigger.is_trigger() {
            self.update_trigger_ping(db, listening_trigger, error).await
        } else {
            self.update_capture_ping(db, listening_trigger, error).await
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
        if listening_trigger.is_trigger() {
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
        } else {
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
        }
    }

    async fn disable_with_error(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        error: String,
    ) {
        if listening_trigger.is_trigger() {
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
        payload: Self::Payload,
        trigger_info: HashMap<String, Box<RawValue>>,
        _extra: Option<Self::Extra>,
    ) -> Result<()> {
        let args = Self::build_job_args(
            &listening_trigger.script_path,
            listening_trigger.is_flow,
            &listening_trigger.workspace_id,
            db,
            payload,
            trigger_info,
        )
        .await?;

        let authed = listening_trigger
            .authed(db, &Self::TRIGGER_KIND.to_string())
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

        tracing::debug!(
            "Triggering job from {} event {} with args {:?}",
            Self::TRIGGER_KIND,
            listening_trigger.path,
            args
        );

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
            None,
        )
        .await?;

        Ok(())
    }

    async fn handle_event(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        payload: Self::Payload,
        trigger_info: HashMap<String, Box<RawValue>>,
        extra: Option<Self::Extra>,
    ) -> Result<()> {
        if let Mode::Trigger(action_to_take) = listening_trigger.mode {
            let result = match action_to_take {
                ActionToTake::RunJob => {
                    self.handle_trigger(db, listening_trigger, payload, trigger_info, extra)
                        .await
                }
                ActionToTake::SendToMailbox => {
                    let mailbox = Mailbox::open(
                        Some(&self.generate_mailbox_id(&listening_trigger.path)),
                        MailboxType::Trigger,
                        &listening_trigger.workspace_id,
                    );
                    let payload = Self::build_job_args(
                        &listening_trigger.script_path,
                        listening_trigger.is_flow,
                        &listening_trigger.workspace_id,
                        db,
                        payload,
                        trigger_info,
                    )
                    .await?;
                    let payload =
                        serde_json::to_value(payload).map_err(|e| Error::from(to_anyhow(e)));
                    let result = match payload {
                        Ok(payload) => mailbox.push(payload, db).await,
                        Err(err) => Err::<(), _>(err),
                    };
                    result
                }
            };

            if let Err(err) = result {
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
                return Err(err);
            };
            return Ok(());
        }

        let (main_args, preprocessor_args) = Self::build_capture_payloads(&payload, trigger_info);
        if let Err(err) = insert_capture_payload(
            db,
            &listening_trigger.workspace_id,
            &listening_trigger.path,
            listening_trigger.is_flow,
            &Self::TRIGGER_KIND,
            main_args,
            preprocessor_args,
            &listening_trigger.username,
        )
        .await
        {
            tracing::error!("Error inserting capture payload: {:?}", err);
            return Err(err);
        }
        Ok(())
    }
}

#[allow(unused)]
async fn listening<T: Listener>(
    db: DB,
    listener: T,
    listening_trigger: ListeningTrigger<T::TriggerConfig>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let killpill_rx_consumer = killpill_rx.resubscribe();
    let killpill_rx_get_consumer = killpill_rx.resubscribe();

    let loop_ping_status = Arc::new(RwLock::new(None));
    let extra_state = listener.get_extra_state().await;
    tokio::select! {
        biased;
        _ = killpill_rx.recv() => {
            let _ = listener.cleanup(&db, &listening_trigger, extra_state.as_ref()).await;
        }
        _ = listener.loop_ping(&db, &listening_trigger, loop_ping_status.clone(), Some("Connecting...".to_string())) => {
            let _ = listener.cleanup(&db, &listening_trigger, extra_state.as_ref()).await;
        }
        consumer = {
            listener.get_consumer(&db, &listening_trigger, loop_ping_status.clone(), killpill_rx_get_consumer)
        } => {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    let _ = listener.cleanup(&db, &listening_trigger, extra_state.as_ref()).await;
                    return;
                }
                _ = listener.loop_ping(&db, &listening_trigger, loop_ping_status.clone(), None) => {
                    let _ = listener.cleanup(&db, &listening_trigger, extra_state.as_ref()).await;
                    return;
                }
                _ = async {
                    match consumer {
                        Ok(Some(consumer)) => {
                            listener.update_ping_and_loop_ping_status(&db, &listening_trigger, loop_ping_status.clone(), None).await;
                            let _ = listener.consume(&db, consumer, &listening_trigger, loop_ping_status.clone(), killpill_rx_consumer, extra_state.as_ref()).await;
                            tracing::debug!("Stopping consumer for trigger");
                        }
                        Err(error) => {
                            tracing::warn!("Disabling trigger due to consumer error: {}", error);
                            listener.disable_with_error(&db, &listening_trigger, error.to_string()).await;
                        }
                        _ => {}
                    }
                } => {
                    let _ = listener.cleanup(&db, &listening_trigger, extra_state.as_ref()).await;
                    return;
                }
            }
        }
    }
}

#[allow(unused)]
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
                                async move { listening(db, listener, capture, killpill_rx).await }
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
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + DeserializeOwned,
{
    fn from_row(row: &sqlx::postgres::PgRow) -> std::result::Result<Self, sqlx::Error> {
        let trigger_config_value = row.try_get("trigger_config")?;
        let trigger_config: T = serde_json::from_value(trigger_config_value)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        Ok(Capture {
            path: row.try_get("path")?,
            is_flow: row.try_get("is_flow")?,
            workspace_id: row.try_get("workspace_id")?,
            username: row.try_get("username")?,
            email: row.try_get("email")?,
            trigger_config,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Mode {
    Trigger(ActionToTake),
    Capture,
}

#[derive(Debug, Clone)]
pub struct ListeningTrigger<T> {
    pub path: String,
    pub is_flow: bool,
    pub workspace_id: String,
    pub username: String,
    pub email: String,
    pub trigger_config: T,
    pub script_path: String,
    pub mode: Mode,
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

    pub fn is_trigger(&self) -> bool {
        matches!(&self.mode, &Mode::Trigger(_))
    }
}

#[allow(unused)]
pub async fn update_rw_lock<T>(lock: std::sync::Arc<tokio::sync::RwLock<T>>, value: T) -> () {
    let mut w = lock.write().await;
    *w = value;
}

#[allow(unused)]
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

#[allow(unused)]
pub fn start_all_listeners(db: DB, killpill_rx: &tokio::sync::broadcast::Receiver<()>) {
    tracing::info!("Starting trigger listeners based on available features...");

    #[cfg(feature = "postgres_trigger")]
    {
        let postgres_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::postgres::PostgresTrigger;

        listen_to(PostgresTrigger, db.clone(), postgres_killpill_rx)
    }

    #[cfg(feature = "mqtt_trigger")]
    {
        let mqtt_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::mqtt::MqttTrigger;

        listen_to(MqttTrigger, db.clone(), mqtt_killpill_rx)
    }

    #[cfg(feature = "websocket")]
    {
        let mqtt_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::websocket::WebsocketTrigger;

        listen_to(WebsocketTrigger, db.clone(), mqtt_killpill_rx)
    }

    #[cfg(all(feature = "gcp_trigger", feature = "enterprise", feature = "private"))]
    {
        let gcp_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::gcp::GcpTrigger;

        listen_to(GcpTrigger, db.clone(), gcp_killpill_rx);
    }

    #[cfg(all(feature = "sqs_trigger", feature = "enterprise", feature = "private"))]
    {
        let gcp_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::sqs::SqsTrigger;

        listen_to(SqsTrigger, db.clone(), gcp_killpill_rx);
    }

    #[cfg(all(feature = "nats", feature = "enterprise", feature = "private"))]
    {
        let gcp_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::nats::NatsTrigger;

        listen_to(NatsTrigger, db.clone(), gcp_killpill_rx);
    }

    #[cfg(all(feature = "kafka", feature = "enterprise", feature = "private"))]
    {
        let gcp_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::kafka::KafkaTrigger;

        listen_to(KafkaTrigger, db.clone(), gcp_killpill_rx);
    }

    tracing::info!("All available trigger listeners have been started");
}
