use std::{collections::HashMap, fmt::Debug, sync::Arc};

use crate::{
    capture::insert_capture_payload,
    handler::TriggerCrud,
    trigger_helpers::{trigger_runnable, TriggerJobArgs},
    types::{Trigger, TriggerErrorHandling, TriggerMode},
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
use windmill_api_auth::ApiAuthed;
use windmill_common::{
    error::{Error, Result},
    jobs::{ChosenBy, JobTriggerKind},
    runnable_ref::RunnableRef,
    triggers::{TriggerKind, TriggerMetadata},
    utils::{report_critical_error, report_recovered_critical_error},
    worker::to_raw_value,
    DB, INSTANCE_NAME,
};

#[allow(unused)]
#[async_trait]
pub trait Listener: TriggerCrud + TriggerJobArgs {
    type Consumer: Send;
    type Extra: Send + Sync;
    type ExtraState: Send + Sync;

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
        let mut fields = vec![
            "workspace_id",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "permissioned_as",
            "edited_at",
            "extra_perms",
            "mode",
            "error_handler_path",
            "error_handler_args",
            "retry",
            "labels",
        ];

        fields.extend_from_slice(Self::ADDITIONAL_SELECT_FIELDS);

        let mut sqlb = SqlBuilder::select_from(Self::TABLE_NAME);

        sqlb.fields(&fields)
            .and_where("(mode = 'enabled'::TRIGGER_MODE OR mode = 'suspended'::TRIGGER_MODE)")
            .and_where(
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
                edited_by: trigger.base.edited_by,
                permissioned_as: trigger.base.permissioned_as,
                script_path: trigger.base.script_path,
                trigger_config: trigger.config,
                error_handling: Some(trigger.error_handling),
                trigger_mode: true,
                suspended_mode: trigger.base.mode == TriggerMode::Suspended,
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
            .map(|capture| {
                let permissioned_as =
                    windmill_common::users::username_to_permissioned_as(&capture.username);
                ListeningTrigger {
                    edited_by: capture.username,
                    path: capture.path,
                    workspace_id: capture.workspace_id,
                    script_path: "".to_string(),
                    permissioned_as,
                    trigger_config: capture.trigger_config,
                    trigger_mode: false,
                    is_flow: capture.is_flow,
                    error_handling: None,
                    suspended_mode: false,
                }
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
        if listening_trigger.trigger_mode {
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
        // SAFETY: Self::TABLE_NAME is a compile-time constant, not user input.
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
                    (mode = 'enabled'::TRIGGER_MODE OR mode = 'suspended'::TRIGGER_MODE)
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
        if listening_trigger.trigger_mode {
            // SAFETY: Self::TABLE_NAME is a compile-time constant.
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
        if listening_trigger.trigger_mode {
            // Contract on `record_in_disable_tx`: one transaction so the row
            // lock spans both writes.
            let mut history_err = None;
            let report_status = async {
                let mut tx = db.begin().await?;
                // SAFETY: Self::TABLE_NAME is a compile-time constant.
                let rows = sqlx::query(&format!(
                    r#"
                    UPDATE
                        {}
                    SET
                        mode = 'disabled'::TRIGGER_MODE,
                        error = $1,
                        server_id = NULL,
                        last_server_ping = NULL
                    WHERE
                        workspace_id = $2 AND
                        path = $3 AND
                        mode <> 'disabled'::TRIGGER_MODE
                "#,
                    Self::TABLE_NAME
                ))
                .bind(&error)
                .bind(&listening_trigger.workspace_id)
                .bind(&listening_trigger.path)
                .execute(&mut *tx)
                .await?
                .rows_affected();

                // Zero rows: deleted, or a user disabled it first — no
                // transition of ours to record.
                if rows > 0 {
                    // `to_key`, not `Display`: it is what lines up with the
                    // `TRIGGER_TYPE` the API records under.
                    let trigger_kind = Self::TRIGGER_KIND.to_key();
                    history_err = windmill_common::trigger_history::record_in_disable_tx(
                        &mut tx,
                        windmill_common::trigger_history::TriggerHistoryEvent::server_disable(
                            &listening_trigger.workspace_id,
                            &trigger_kind,
                            &listening_trigger.path,
                            serde_json::json!({ "mode": { "new": "disabled" } }),
                            &error,
                        ),
                    )
                    .await;
                }
                tx.commit().await?;
                Ok::<(), Error>(())
            }
            .await;

            if let Some(history_err) = history_err {
                // Spawned: the commit above made the cleared `server_id` visible,
                // so the ping branch of the enclosing `select!` is about to
                // finish and drop everything left in this future. Awaiting the
                // alert here would lose the one signal that the row is missing.
                let message = format!(
                    "Disabled {} trigger {} but could not record it in the trigger history: {}",
                    Self::TRIGGER_KIND,
                    listening_trigger.path,
                    history_err
                );
                let (db, workspace_id) = (db.clone(), listening_trigger.workspace_id.clone());
                tokio::spawn(async move {
                    report_critical_error(message, db, Some(&workspace_id), None).await;
                });
            }

            match report_status {
                Ok(()) => {
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
        mut trigger_info: HashMap<String, Box<RawValue>>,
        _extra: Option<Self::Extra>,
    ) -> Result<()> {
        trigger_info.insert(
            "trigger_path".to_string(),
            to_raw_value(&listening_trigger.path),
        );
        let runnable =
            RunnableRef::from_path(&listening_trigger.script_path, listening_trigger.is_flow)?;
        let args = Self::build_job_args(
            &runnable,
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
            ChosenBy::Deployed,
            &listening_trigger.workspace_id,
            &runnable,
            args,
            retry,
            error_handler_path.as_deref(),
            error_handler_args,
            format!("{}_trigger/{}", Self::TRIGGER_KIND, listening_trigger.path),
            None,
            listening_trigger.suspended_mode,
            TriggerMetadata::new(Some(listening_trigger.path.clone()), Self::JOB_TRIGGER_KIND),
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
        if listening_trigger.trigger_mode {
            if let Err(err) = self
                .handle_trigger(db, listening_trigger, payload, trigger_info, extra)
                .await
            {
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

        let mut trigger_info = trigger_info;
        trigger_info.insert(
            "trigger_path".to_string(),
            to_raw_value(&listening_trigger.path),
        );
        let (main_args, preprocessor_args) = Self::build_capture_payloads(&payload, trigger_info);
        if let Err(err) = insert_capture_payload(
            db,
            &listening_trigger.workspace_id,
            &listening_trigger.path,
            listening_trigger.is_flow,
            &Self::TRIGGER_KIND,
            main_args,
            preprocessor_args,
            &listening_trigger.edited_by,
        )
        .await
        {
            tracing::error!("Error inserting capture payload: {:?}", err);
            return Err(err);
        }
        Ok(())
    }
}

/// Unit of [`reconnect_delay`] between a trigger's connection attempts.
const RECONNECT_BASE_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

/// Upper bound on [`reconnect_delay`].
const MAX_RECONNECT_DELAY: std::time::Duration = std::time::Duration::from_secs(300);

/// Exponential backoff for the `attempt`th consecutive failure (1-based): `2^attempt`
/// units of `base`, capped at [`MAX_RECONNECT_DELAY`].
fn reconnect_delay(base: std::time::Duration, attempt: u32) -> std::time::Duration {
    base.saturating_mul(1u32 << attempt.min(20))
        .min(MAX_RECONNECT_DELAY)
}

/// A consumer that ran at least this long before exiting is reconnected at once; a
/// shorter one counts as a failed attempt, so a server that accepts and immediately
/// closes (e.g. at its per-client connection limit) is retried with backoff rather than
/// in a hot loop.
const MIN_STABLE_CONSUMER_RUN: std::time::Duration = std::time::Duration::from_secs(30);

/// A blip, or a server rotating its connections, recovers within the first retries:
/// only a failure streak this long raises a critical alert, repeated every
/// [`ALERT_EVERY_N_FAILURES`] failures after that.
const FIRST_ALERT_AFTER_FAILURES: u32 = 3;
const ALERT_EVERY_N_FAILURES: u32 = 10;

/// Whether a failure to open or keep a connection can succeed on a later attempt, so
/// that a trigger retries it instead of being disabled. An allowlist: config,
/// permission and lookup failures (`BadConfig`, `NotFound`, `PermissionDenied`, ...)
/// fail the same way every time and must disable rather than retry forever. A listener
/// reports a transient failure that has no such kind with [`transient_error`].
pub fn is_transient_error(err: &Error) -> bool {
    match err {
        Error::Generic(status, _) => status.is_server_error(),
        Error::InternalErr(_)
        | Error::InternalErrLoc { .. }
        | Error::InternalErrAt(..)
        | Error::Anyhow { .. }
        | Error::SqlErr { .. }
        | Error::IoErr { .. }
        | Error::ConnectingToDatabase(_)
        | Error::BadGateway(_) => true,
        _ => false,
    }
}

pub fn transient_error(message: impl Into<String>) -> Error {
    Error::Generic(http::StatusCode::SERVICE_UNAVAILABLE, message.into())
}

/// Counts a failed connection or a consumer that stopped too soon, and reports it as a
/// critical alert once the streak is long enough to matter.
async fn record_connection_failure<T: Listener>(
    db: &DB,
    listening_trigger: &ListeningTrigger<T::TriggerConfig>,
    failures: &mut u32,
    delay: std::time::Duration,
    error: &str,
) {
    *failures += 1;
    tracing::warn!(
        "[{}] Trigger {} failed to connect (attempt {}), retrying in {:?}: {}",
        T::TRIGGER_KIND,
        listening_trigger.path,
        failures,
        delay,
        error
    );
    if *failures >= FIRST_ALERT_AFTER_FAILURES
        && (*failures - FIRST_ALERT_AFTER_FAILURES) % ALERT_EVERY_N_FAILURES == 0
    {
        report_critical_error(
            format!(
                "{} trigger {} failed to connect {} times in a row, retrying in {}s. This alert repeats every {} failed attempts. Last error: {}",
                T::TRIGGER_KIND,
                listening_trigger.path,
                failures,
                delay.as_secs(),
                ALERT_EVERY_N_FAILURES,
                error
            ),
            db.clone(),
            Some(&listening_trigger.workspace_id),
            Some(&connection_alert_id::<T>(listening_trigger)),
        )
        .await;
    }
}

fn connection_alert_id<T: Listener>(
    listening_trigger: &ListeningTrigger<T::TriggerConfig>,
) -> String {
    // A recovery acknowledges every alert with this id, whatever its workspace.
    format!(
        "{}_trigger:{}:{}",
        T::TRIGGER_KIND.to_key(),
        listening_trigger.workspace_id,
        listening_trigger.path
    )
}

#[allow(unused)]
pub async fn listening<T: Listener>(
    db: DB,
    listener: T,
    listening_trigger: ListeningTrigger<T::TriggerConfig>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let killpill_rx_consumer = killpill_rx.resubscribe();

    let loop_ping_status = Arc::new(RwLock::new(None));
    let extra_state = listener.get_extra_state().await;
    let path = listening_trigger.path.clone();
    // Capture is an interactive test: a failure surfaces at once rather than being
    // retried, and the next claim poll starts over while the client still pings.
    let reconnect = listening_trigger.trigger_mode;
    tokio::select! {
        biased;
        _ = killpill_rx.recv() => {
            tracing::info!("[{}] Killing pill received, stopping consumer for trigger {}", T::TRIGGER_KIND, path);
        }
        _ = listener.loop_ping(&db, &listening_trigger, loop_ping_status.clone(), Some("Connecting...".to_string())) => {
            tracing::info!("[{}] Loop ping exited, stopping consumer for trigger {}", T::TRIGGER_KIND, path);
        }
        _ = async {
            // Consecutive failed connections and consumers that stopped too soon.
            let mut failures = 0;
            loop {
                tracing::info!("[{}] Getting consumer for trigger {}", T::TRIGGER_KIND, path);
                match listener.get_consumer(&db, &listening_trigger, loop_ping_status.clone(), killpill_rx_consumer.resubscribe()).await {
                    Ok(Some(consumer)) => {
                        listener.update_ping_and_loop_ping_status(&db, &listening_trigger, loop_ping_status.clone(), None).await;
                        tracing::info!("[{}] Starting consumer for trigger {}", T::TRIGGER_KIND, path);
                        let consume = listener.consume(&db, consumer, &listening_trigger, loop_ping_status.clone(), killpill_rx_consumer.resubscribe(), extra_state.as_ref());
                        tokio::pin!(consume);
                        let stable = tokio::select! {
                            _ = &mut consume => false,
                            _ = tokio::time::sleep(MIN_STABLE_CONSUMER_RUN) => true,
                        };
                        if stable {
                            if failures >= FIRST_ALERT_AFTER_FAILURES {
                                report_recovered_critical_error(
                                    format!("{} trigger {} reconnected", T::TRIGGER_KIND, path),
                                    db.clone(),
                                    Some(&listening_trigger.workspace_id),
                                    Some(&connection_alert_id::<T>(&listening_trigger)),
                                )
                                .await;
                            }
                            failures = 0;
                            consume.await;
                        }
                        tracing::info!("[{}] Consumer stopped for trigger {}", T::TRIGGER_KIND, path);
                        if !reconnect {
                            return;
                        }
                        if !stable {
                            let status = loop_ping_status.read().await.clone();
                            // A consumer that disabled its own trigger also stops early: that
                            // is not a failed connection to count or alert on.
                            if listener.update_ping_and_loop_ping_status(&db, &listening_trigger, loop_ping_status.clone(), status.clone()).await.is_none() {
                                return;
                            }
                            let status = status.unwrap_or_else(|| "Consumer stopped".to_string());
                            let delay = reconnect_delay(RECONNECT_BASE_DELAY, failures + 1);
                            record_connection_failure::<T>(&db, &listening_trigger, &mut failures, delay, &status).await;
                            update_rw_lock(loop_ping_status.clone(), Some(format!("{status}. Reconnecting in {}s...", delay.as_secs()))).await;
                            tokio::time::sleep(delay).await;
                        }
                        // `consume` may have stopped because the trigger was disabled or
                        // edited (it can disable it itself): only reconnect while the
                        // lease is still ours.
                        if listener.update_ping_and_loop_ping_status(&db, &listening_trigger, loop_ping_status.clone(), Some("Reconnecting...".to_string())).await.is_none() {
                            return;
                        }
                        tracing::info!("[{}] Reconnecting consumer for trigger {}", T::TRIGGER_KIND, path);
                    }
                    Err(error) if reconnect && is_transient_error(&error) => {
                        let delay = reconnect_delay(RECONNECT_BASE_DELAY, failures + 1);
                        record_connection_failure::<T>(&db, &listening_trigger, &mut failures, delay, &error.to_string()).await;
                        let next_attempt_at = chrono::Utc::now() + chrono::Duration::from_std(delay).unwrap_or_default();
                        update_rw_lock(
                            loop_ping_status.clone(),
                            Some(format!(
                                "Connection attempt {} failed, retrying in {}s (at {}): {}",
                                failures,
                                delay.as_secs(),
                                next_attempt_at.format("%Y-%m-%d %H:%M:%S UTC"),
                                error
                            )),
                        )
                        .await;
                        tokio::time::sleep(delay).await;
                    }
                    Err(error) => {
                        let error = if failures > 0 {
                            format!("{} (connection attempt {})", error, failures + 1)
                        } else {
                            error.to_string()
                        };
                        tracing::error!("[{}] Disabling trigger {} due to consumer error: {}", T::TRIGGER_KIND, path, error);
                        listener.disable_with_error(&db, &listening_trigger, error).await;
                        return;
                    }
                    Ok(None) => {
                        tracing::error!("[{}] Consumer is None for trigger {}", T::TRIGGER_KIND, path);
                        return;
                    }
                }
            }
        } => {}
    }
    let _ = listener
        .cleanup(&db, &listening_trigger, extra_state.as_ref())
        .await;
}

#[allow(unused)]
pub async fn listen_to_unlistened_events<T: Copy + Listener>(
    listener: T,
    db: DB,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) {
    let unlistend_enabled_triggers = listener.fetch_enabled_unlistened_triggers(&db).await;

    match unlistend_enabled_triggers {
        Ok(mut unlistend_enabled_triggers) => {
            unlistend_enabled_triggers.shuffle(&mut rand::rng());
            for trigger in unlistend_enabled_triggers {
                // SAFETY: T::TABLE_NAME is a compile-time constant.
                let has_lock = sqlx::query_scalar(&format!(
                    r#"
                        UPDATE
                            {}
                        SET
                            server_id = $1,
                            last_server_ping = now(),
                            error = 'Connecting...'
                        WHERE
                            (mode = 'enabled'::TRIGGER_MODE OR mode = 'suspended'::TRIGGER_MODE)
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
pub struct Capture<T>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow>,
{
    path: String,
    is_flow: bool,
    workspace_id: String,
    username: String,
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
            trigger_config,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListeningTrigger<T> {
    pub path: String,
    pub is_flow: bool,
    pub workspace_id: String,
    pub edited_by: String,
    pub permissioned_as: String,
    pub trigger_config: T,
    pub script_path: String,
    pub trigger_mode: bool,
    pub error_handling: Option<TriggerErrorHandling>,
    pub suspended_mode: bool,
}

impl<T> ListeningTrigger<T> {
    pub async fn authed(&self, db: &DB, trigger_kind: &str) -> Result<ApiAuthed> {
        let email = windmill_common::users::get_email_from_permissioned_as(
            &self.permissioned_as,
            &self.workspace_id,
            db,
        )
        .await?;
        windmill_api_auth::fetch_api_authed_from_permissioned_as(
            self.permissioned_as.clone(),
            email,
            &self.workspace_id,
            db,
            Some(format!("{}-{}", trigger_kind, self.path)),
        )
        .await
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

// start_all_listeners() stays in windmill-api since it references concrete trigger types
