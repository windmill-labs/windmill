-- Tracks the live state of long-running background tasks coordinated across
-- multiple server replicas (log cleanup, object-storage usage scan, staggered
-- graceful restart). Previously these were stored in-memory per-process or in
-- `global_settings` (which is conceptually for user-configurable instance
-- settings and is exposed via the settings UI / export).
--
-- `value` holds task-specific JSON. `running`/`owner`/`updated_at` implement
-- a lease: a task is "free" if running=false OR updated_at is older than the
-- heartbeat-stale threshold (handled in application code, see
-- windmill-common/src/background_task.rs).
CREATE TABLE background_task_state (
    name TEXT PRIMARY KEY,
    value JSONB NOT NULL,
    running BOOLEAN NOT NULL DEFAULT FALSE,
    owner TEXT,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
