-- New top-level jobs of a workspace are held until `paused_until`. Runtime state, kept out of
-- workspace_settings so a settings push cannot clear it mid-deploy and a fork never starts paused.
-- A row past its deadline reads as not paused; the monitor deletes it and releases its jobs.
CREATE TABLE IF NOT EXISTS workspace_job_pause (
    workspace_id VARCHAR(50) PRIMARY KEY REFERENCES workspace(id) ON DELETE CASCADE,
    paused_until TIMESTAMPTZ NOT NULL,
    paused_by VARCHAR(255) NOT NULL,
    -- When this pause started; extending it keeps it. Jobs pushed well after it were already
    -- judged by the push itself, so the queue sweep leaves them alone.
    paused_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reason TEXT,
    -- Renewed by every pause request, so whoever paused can resume only if no one paused since.
    pause_id UUID NOT NULL DEFAULT gen_random_uuid()
);

-- The jobs a workspace pause holds, as opposed to those a suspended trigger holds: both are queued
-- as `unassigned_*`. No foreign key to the queue, so completing or cancelling a job pays nothing
-- for it; a row whose job is gone is dropped when the jobs are released.
CREATE TABLE IF NOT EXISTS workspace_held_job (
    job_id UUID PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    -- When the job was due before it was held: a schedule tick queued ahead of time keeps its time.
    scheduled_for TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS workspace_held_job_workspace_id_idx ON workspace_held_job (workspace_id);

GRANT ALL ON workspace_job_pause TO windmill_user;
GRANT ALL ON workspace_job_pause TO windmill_admin;
GRANT ALL ON workspace_held_job TO windmill_user;
GRANT ALL ON workspace_held_job TO windmill_admin;

CREATE OR REPLACE FUNCTION notify_workspace_job_pause_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload)
    VALUES ('notify_workspace_job_pause_change', COALESCE(NEW.workspace_id, OLD.workspace_id));
    RETURN NULL;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

DROP TRIGGER IF EXISTS workspace_job_pause_change_trigger ON workspace_job_pause;
CREATE TRIGGER workspace_job_pause_change_trigger
AFTER INSERT OR UPDATE OR DELETE ON workspace_job_pause
FOR EACH ROW
EXECUTE FUNCTION notify_workspace_job_pause_change();
