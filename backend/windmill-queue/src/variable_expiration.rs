/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Dispatches the workspace variable expiration handler for variables coming due.

use std::collections::{HashMap, HashSet};

use serde_json::value::RawValue;
use sqlx::types::Json;
use windmill_common::jobs::get_payload_tag_from_prefixed_path;
use windmill_common::utils::{report_critical_error, report_recovered_critical_error};
use windmill_common::DB;

use crate::jobs::{push_variable_expiration_handler, ExpiringVariable};

/// Lead time when the handler setting has no `notify_before_secs`.
const VARIABLE_EXPIRATION_DEFAULT_NOTIFY_BEFORE_SECS: i64 = 24 * 3600;

/// Subtracted from `remind_every_secs` so a reminder whose interval is a multiple of the
/// sweep's period is not skipped by tick jitter.
const VARIABLE_EXPIRATION_REMINDER_SLACK_SECS: i64 = 30;

/// Advisory lock id ensuring only one server replica sweeps at a time. Must not reuse one of
/// the other monitor pass locks (737_483_920..=737_483_925): both sides try-lock and skip.
const VARIABLE_EXPIRATION_LOCK_ID: i64 = 737_483_926;

/// System caller only: dispatches across every workspace. A handler job that fails is not
/// re-dispatched before its next reminder, as re-running it would repeat its side effects
/// against the third party it rotates; retrying is the handler's job.
pub async fn dispatch_expiring_variables(db: &DB) {
    // Transaction-scoped, not session-scoped: a session lock would go back to the pool still
    // held if this future were dropped mid-pass, wedging the sweep on every replica.
    let mut lock_tx = match db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("variable expiration: failed to begin lock tx: {e:#}");
            return;
        }
    };
    let locked: bool = match sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(VARIABLE_EXPIRATION_LOCK_ID)
        .fetch_one(&mut *lock_tx)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("variable expiration: advisory lock failed: {e:#}");
            return;
        }
    };
    if !locked {
        return;
    }

    dispatch_expiring_variables_inner(db).await;

    if let Err(e) = lock_tx.rollback().await {
        tracing::error!("variable expiration: releasing lock failed: {e:#}");
    }
}

async fn dispatch_expiring_variables_inner(db: &DB) {
    resolve_removed_handlers(db).await;

    // No lower bound on `value_valid_until`, so a downed instance catches up. No cap either:
    // everything due is tried every pass, so no workspace or kind of notice can be crowded out.
    let due = sqlx::query!(
        r#"SELECT v.workspace_id, v.path, v.description, v.value_valid_until AS "value_valid_until!",
                  v.is_secret, v.expiration_dispatched_for, v.expiration_notified_at,
                  ws.variable_expiration_handler->>'path' AS "handler_path!",
                  (ws.variable_expiration_handler->'extra_args')::text::json
                      AS "extra_args: Json<Box<RawValue>>"
           FROM variable v
           JOIN workspace_settings ws ON ws.workspace_id = v.workspace_id
           JOIN workspace w ON w.id = v.workspace_id
           WHERE NOT w.deleted
             AND v.value_valid_until IS NOT NULL
             AND (v.value_valid_until IS DISTINCT FROM v.expiration_dispatched_for
                  OR (v.value_valid_until > now()
                      AND v.expiration_notified_at <= now() - make_interval(secs =>
                          NULLIF((ws.variable_expiration_handler->>'remind_every_secs')::bigint, 0) - $2)))
             AND v.value_valid_until <= now() + make_interval(secs =>
                 COALESCE((ws.variable_expiration_handler->>'notify_before_secs')::bigint, $1))
             AND ws.variable_expiration_handler->>'path' IS NOT NULL
             AND (NOT COALESCE((ws.variable_expiration_handler->>'muted_on_user_path')::boolean, false)
                  OR v.path NOT LIKE 'u/%')"#,
        VARIABLE_EXPIRATION_DEFAULT_NOTIFY_BEFORE_SECS,
        VARIABLE_EXPIRATION_REMINDER_SLACK_SECS,
    )
    .fetch_all(db)
    .await;

    let due = match due {
        Ok(due) => due,
        Err(e) => {
            tracing::error!("Error listing expiring variables: {e:#}");
            return;
        }
    };

    let mut dispatched = 0usize;
    // A handler fails alike for every variable of its workspace: try it once per pass.
    let mut failed_this_pass: HashSet<String> = HashSet::new();
    // Hub handlers are shared by every workspace using them, so a hub outage is fetched once
    // per pass rather than once per workspace.
    let mut failed_hub_handlers: HashMap<String, String> = HashMap::new();

    for row in due {
        if failed_this_pass.contains(&row.workspace_id) {
            continue;
        }
        // Resolved before the claim's transaction opens, so a pass never holds one connection
        // while waiting on another.
        let handler = match failed_hub_handlers.get(&row.handler_path) {
            Some(error) => Err(error.clone()),
            None => get_payload_tag_from_prefixed_path(&row.handler_path, db, &row.workspace_id)
                .await
                .map_err(|e| {
                    let error = format!("{e:#}");
                    if row.handler_path.starts_with("script/hub/")
                        || row.handler_path.starts_with("flow/hub/")
                    {
                        failed_hub_handlers.insert(row.handler_path.clone(), error.clone());
                    }
                    error
                }),
        };
        let handler = match handler {
            Ok(handler) => handler,
            Err(error) => {
                failed_this_pass.insert(row.workspace_id.clone());
                record_failure(db, &row.workspace_id, &row.handler_path, &error).await;
                continue;
            }
        };

        // Claim and push in one transaction. The claim matches the row as it was read, so it
        // loses both to a concurrent re-arm and to whichever replica got there first.
        let mut tx = match db.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!("Error opening tx for expiring variable {}: {e:#}", row.path);
                continue;
            }
        };

        let claimed = sqlx::query_scalar!(
            "UPDATE variable SET expiration_dispatched_for = $3, expiration_notified_at = now()
             WHERE workspace_id = $1 AND path = $2 AND value_valid_until = $3
               AND expiration_dispatched_for IS NOT DISTINCT FROM $4
               AND expiration_notified_at IS NOT DISTINCT FROM $5
             RETURNING 1",
            row.workspace_id,
            row.path,
            row.value_valid_until,
            row.expiration_dispatched_for,
            row.expiration_notified_at,
        )
        .fetch_optional(&mut *tx)
        .await;

        match claimed {
            Ok(Some(_)) => {}
            Ok(None) => continue,
            Err(e) => {
                tracing::error!("Error claiming expiring variable {}: {e:#}", row.path);
                continue;
            }
        }

        let expiring = ExpiringVariable {
            workspace_id: row.workspace_id.clone(),
            variable_path: row.path.clone(),
            description: row.description,
            value_valid_until: row.value_valid_until,
            is_secret: row.is_secret,
        };

        let pushed = push_variable_expiration_handler(
            db,
            tx,
            &row.workspace_id,
            handler,
            &expiring,
            row.extra_args,
        )
        .await;

        let tx = match pushed {
            Ok((_job_id, tx)) => tx,
            Err(e) => {
                // Dropping the tx un-claims the variable, so the next pass retries it.
                failed_this_pass.insert(row.workspace_id.clone());
                record_failure(db, &row.workspace_id, &row.handler_path, &format!("{e:#}")).await;
                continue;
            }
        };

        if let Err(e) = tx.commit().await {
            tracing::error!("Error committing dispatch for {}: {e:#}", row.path);
            continue;
        }
        windmill_common::feature_usage::log_feature_usage(
            "variable_expiration",
            "dispatched",
            if row.is_secret { "secret" } else { "plain" },
        );
        dispatched += 1;
        recover(db, &row.workspace_id, &row.handler_path).await;
    }

    if dispatched > 0 {
        tracing::info!("Dispatched {dispatched} variable expiration handler job(s)");
    }
}

/// Alerts once per failure, not per pass: the alert is resolved when the handler next starts.
async fn record_failure(db: &DB, w_id: &str, handler_path: &str, error: &str) {
    let newly_failing = sqlx::query_scalar!(
        "UPDATE workspace_settings SET variable_expiration_failed_at = now()
         WHERE workspace_id = $1 AND variable_expiration_failed_at IS NULL
         RETURNING 1",
        w_id,
    )
    .fetch_optional(db)
    .await;
    match newly_failing {
        Ok(Some(_)) => {
            report_critical_error(
                format!(
                    "The variable expiration handler {handler_path} of workspace {w_id} failed to \
                     start, so variables in this workspace are not being notified. It is retried \
                     on every check, and this alert is resolved once it starts: {error}"
                ),
                db.clone(),
                Some(w_id),
                Some(&alert_resource(w_id)),
            )
            .await
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("Error recording variable expiration handler failure of {w_id}: {e:#}")
        }
    }
}

/// Tags a failing handler's critical alerts, so they can be resolved together.
fn alert_resource(w_id: &str) -> String {
    format!("variable_expiration_handler:{w_id}")
}

/// A removed handler has nothing left to retry, so its failure would never be resolved.
async fn resolve_removed_handlers(db: &DB) {
    let removed = sqlx::query_scalar!(
        "UPDATE workspace_settings SET variable_expiration_failed_at = NULL
         WHERE variable_expiration_failed_at IS NOT NULL AND variable_expiration_handler IS NULL
         RETURNING workspace_id",
    )
    .fetch_all(db)
    .await;
    match removed {
        Ok(removed) => {
            for w_id in removed {
                report_recovered_critical_error(
                    format!("The variable expiration handler of workspace {w_id} was removed"),
                    db.clone(),
                    Some(&w_id),
                    Some(&alert_resource(&w_id)),
                )
                .await;
            }
        }
        Err(e) => tracing::error!("Error resolving removed variable expiration handlers: {e:#}"),
    }
}

async fn recover(db: &DB, w_id: &str, handler_path: &str) {
    let cleared = sqlx::query_scalar!(
        "UPDATE workspace_settings SET variable_expiration_failed_at = NULL
         WHERE workspace_id = $1 AND variable_expiration_failed_at IS NOT NULL
         RETURNING 1",
        w_id,
    )
    .fetch_optional(db)
    .await;
    match cleared {
        Ok(Some(_)) => {
            report_recovered_critical_error(
                format!(
                "The variable expiration handler {handler_path} of workspace {w_id} started again"
            ),
                db.clone(),
                Some(w_id),
                Some(&alert_resource(w_id)),
            )
            .await
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("Error clearing variable expiration handler failure of {w_id}: {e:#}")
        }
    }
}
