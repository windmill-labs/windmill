-- Sparse marker: one row per native retry attempt (a re-pushed Script job that
-- gained a retry). Presence = "this job is a native retry attempt"; `attempt` is
-- the chain position (1-based). Lets consumers — asset-cascade dispatch, the
-- per-occurrence schedule-handler counting, and the run-page chain — identify
-- retries explicitly instead of inferring from incidental fields (parent_job +
-- runnable + flow_innermost), which collides with schedule handlers and WAC
-- inline children. Sparse: only failed-and-retried jobs under a retry policy
-- produce rows. Lifecycle: removed with their job in the retention sweep (no FK,
-- to keep the bulk job delete cheap).
CREATE TABLE IF NOT EXISTS native_retry_attempt (
    job_id UUID PRIMARY KEY,
    -- `integer` matches the retry policy's i32 attempt count; avoids any narrowing
    -- on the maybe_enqueue read/write path.
    attempt INTEGER NOT NULL
);

GRANT ALL ON native_retry_attempt TO windmill_admin;
GRANT ALL ON native_retry_attempt TO windmill_user;
