/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Per-task enrichment context for OTEL log export.
//!
//! This is the MDC-equivalent for windmill: a tokio task-local that carries
//! request/job identifiers from the entry point (axum middleware in
//! windmill-api, job execution wrapper in windmill-worker) to log emission
//! sites. The EE OTEL log bridge reads this at event time and attaches the
//! fields to exported LogRecords, so Sentry / OTLP backends can filter logs
//! by workspace_id, email, script_path, etc., without joining against
//! traces.
//!
//! This deliberately lives alongside (not instead of) tracing spans — the
//! same fields are still recorded on `info_span!("request", …)` and
//! `info_span!("job", …)` for OTEL trace export. LogContext is the log-side
//! parallel that `opentelemetry-appender-tracing` would otherwise be unable
//! to see.

use std::sync::{Arc, Mutex};

/// Fields promoted from request/job context onto exported log records.
#[derive(Clone, Debug, Default)]
pub struct LogContext {
    // HTTP request span (windmill-api/src/tracing_init.rs)
    pub method: Option<String>,
    pub uri: Option<String>,
    pub trace_id: Option<String>,

    // Auth (windmill-api-auth/src/auth.rs)
    pub email: Option<String>,
    pub username: Option<String>,

    // Workspace
    pub workspace_id: Option<String>,

    // Worker / job span (windmill-worker/src/worker.rs)
    pub worker: Option<String>,
    pub hostname: Option<String>,
    pub tag: Option<String>,
    pub job_id: Option<String>,
    pub parent_job: Option<String>,
    pub root_job: Option<String>,
    pub script_path: Option<String>,
    pub script_hash: Option<String>,
    pub job_kind: Option<String>,
    pub language: Option<String>,
    pub flow_step_id: Option<String>,
    pub trigger_kind: Option<String>,
    pub trigger: Option<String>,
    pub created_by: Option<String>,
}

tokio::task_local! {
    pub static LOG_CONTEXT: Arc<Mutex<LogContext>>;
}

/// Run a future inside a freshly-seeded LogContext scope.
///
/// Use at request/job entry points. Downstream code can read the context
/// via [`current_log_context`] and mutate it via [`update_log_context`].
pub async fn with_log_context<F>(ctx: LogContext, fut: F) -> F::Output
where
    F: std::future::Future,
{
    LOG_CONTEXT.scope(Arc::new(Mutex::new(ctx)), fut).await
}

/// Snapshot the current LogContext. Returns `None` outside any scope.
pub fn current_log_context() -> Option<LogContext> {
    LOG_CONTEXT
        .try_with(|c| c.lock().ok().map(|g| g.clone()))
        .ok()
        .flatten()
}

/// Mutate the current LogContext in place. No-op outside a scope.
///
/// Used by auth middleware to push email/username/workspace_id as soon as
/// they're resolved.
pub fn update_log_context<F>(f: F)
where
    F: FnOnce(&mut LogContext),
{
    let _ = LOG_CONTEXT.try_with(|c| {
        if let Ok(mut g) = c.lock() {
            f(&mut g);
        }
    });
}

/// Spawn a future with a snapshot of the current LogContext forwarded to
/// the new task. Use in place of `tokio::spawn` when you want the spawned
/// task's logs to inherit the calling task's context.
///
/// The snapshot is captured at spawn time — mutations in either task after
/// that point are independent. This is usually what you want for
/// fire-and-forget background work.
pub fn spawn_with_log_context<F>(fut: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let snapshot = current_log_context();
    tokio::spawn(async move {
        match snapshot {
            Some(ctx) => with_log_context(ctx, fut).await,
            None => fut.await,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reads_and_mutates_current_context() {
        let ctx = LogContext {
            workspace_id: Some("acme".into()),
            email: Some("alice@acme.co".into()),
            ..Default::default()
        };
        with_log_context(ctx, async {
            let snap = current_log_context().expect("in scope");
            assert_eq!(snap.workspace_id.as_deref(), Some("acme"));
            assert_eq!(snap.email.as_deref(), Some("alice@acme.co"));

            update_log_context(|c| c.script_path = Some("f/ingest/run".into()));

            let snap = current_log_context().expect("in scope");
            assert_eq!(snap.script_path.as_deref(), Some("f/ingest/run"));
        })
        .await;
    }

    #[tokio::test]
    async fn outside_scope_returns_none() {
        assert!(current_log_context().is_none());
        // update_log_context is a no-op outside a scope, must not panic
        update_log_context(|c| c.workspace_id = Some("ignored".into()));
        assert!(current_log_context().is_none());
    }

    #[tokio::test]
    async fn spawn_inherits_snapshot() {
        let ctx = LogContext { workspace_id: Some("acme".into()), ..Default::default() };
        with_log_context(ctx, async {
            let handle = spawn_with_log_context(async {
                let snap = current_log_context().expect("inherited in spawned task");
                snap.workspace_id
            });
            let ws = handle.await.unwrap();
            assert_eq!(ws.as_deref(), Some("acme"));
        })
        .await;
    }

    #[tokio::test]
    async fn bare_spawn_has_no_context() {
        let ctx = LogContext { workspace_id: Some("acme".into()), ..Default::default() };
        with_log_context(ctx, async {
            let handle = tokio::spawn(async { current_log_context() });
            assert!(
                handle.await.unwrap().is_none(),
                "bare tokio::spawn drops context"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn spawn_snapshot_is_independent_of_parent_mutations() {
        let ctx = LogContext { workspace_id: Some("parent".into()), ..Default::default() };
        with_log_context(ctx, async {
            let handle = spawn_with_log_context(async {
                tokio::task::yield_now().await;
                current_log_context().unwrap().workspace_id
            });
            update_log_context(|c| c.workspace_id = Some("mutated".into()));
            let child_ws = handle.await.unwrap();
            assert_eq!(child_ws.as_deref(), Some("parent"));
        })
        .await;
    }
}
