DROP TRIGGER IF EXISTS workspace_job_pause_change_trigger ON workspace_job_pause;
DROP FUNCTION IF EXISTS notify_workspace_job_pause_change();
DROP TABLE IF EXISTS workspace_held_job;
DROP TABLE IF EXISTS workspace_job_pause;
