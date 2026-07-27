-- Sparse annotation: one row per failed job whose failure has been handled, either
-- because a later attempt succeeded or because someone explained it. Resolution is
-- orthogonal to `v2_job_completed.status`, which stays 'failure' so failure rates,
-- error handlers and critical alerts keep counting the true failure; only the human
-- triage surfaces (runs list, run detail) render a resolved failure differently.
-- Sparse: only failures that someone or something resolved produce rows.
-- Lifecycle: removed with its job by delete_jobs (no FK, to keep the bulk job delete
-- cheap), with an orphan sweep in the monitor as a backstop.
CREATE TABLE IF NOT EXISTS job_resolution (
    job_id UUID PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    resolved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Who and why are enterprise-only, so both are NULL on a manual resolution in CE.
    -- That is why "resolved automatically" is `automatic`, not `resolved_by IS NULL`:
    -- overloading attribution would make every CE resolution look automatic.
    resolved_by VARCHAR(255),
    note TEXT,
    automatic BOOLEAN NOT NULL DEFAULT false
);

ALTER TABLE job_resolution ADD COLUMN IF NOT EXISTS automatic BOOLEAN NOT NULL DEFAULT false;

-- windmill_user needs write access: the resolve endpoint runs under user_db so that
-- v2_job's row-level security decides which runs the caller may annotate.
GRANT ALL ON job_resolution TO windmill_admin;
GRANT ALL ON job_resolution TO windmill_user;
