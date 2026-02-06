-- v2_job: drop obsolete indexes and create new ones
DROP INDEX IF EXISTS root_job_index;
DROP INDEX IF EXISTS root_job_index_by_path_2;
DROP INDEX IF EXISTS root_job_index_by_path;
DROP INDEX IF EXISTS ix_job_workspace_id_created_at_new_6;
DROP INDEX IF EXISTS ix_job_workspace_id_created_at_new_7;
DROP INDEX IF EXISTS ix_job_created_at;
DROP INDEX IF EXISTS ix_v2_job_root_by_path;

CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_3
    ON v2_job (workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_8
    ON v2_job (workspace_id, created_at DESC)
    WHERE kind IN ('deploymentcallback') AND parent_job IS NULL;

CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_9
    ON v2_job (workspace_id, created_at DESC)
    WHERE kind IN ('dependencies', 'flowdependencies', 'appdependencies') AND parent_job IS NULL;

CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_5
    ON v2_job (workspace_id, created_at DESC)
    WHERE kind IN ('preview', 'flowpreview') AND parent_job IS NULL;

CREATE INDEX IF NOT EXISTS ix_v2_job_labels
    ON v2_job USING GIN (labels)
    WHERE labels IS NOT NULL;

ALTER TABLE v2_job ENABLE ROW LEVEL SECURITY;

CREATE INDEX IF NOT EXISTS ix_v2_job_workspace_id_created_at
    ON v2_job (workspace_id, created_at DESC)
    WHERE kind IN ('script', 'flow', 'singlestepflow') AND parent_job IS NULL;

CREATE INDEX IF NOT EXISTS idx_job_v2_job_root_by_path_2
    ON v2_job (workspace_id, runnable_path)
    WHERE parent_job IS NULL;

CREATE INDEX IF NOT EXISTS ix_job_root_job_index_by_path_2
    ON v2_job (workspace_id, runnable_path, created_at DESC)
    WHERE parent_job IS NULL;
