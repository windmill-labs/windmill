-- Consolidate live index migrations into a regular SQL migration.
-- On existing installs these are already done (checked via windmill_migrations).
-- On fresh installs this creates all required indexes in one shot.

DO $$
BEGIN
    -- Skip entirely if any of the live migrations already ran (existing install)
    IF EXISTS (SELECT 1 FROM windmill_migrations WHERE name = 'fix_job_completed_index_2') THEN
        RAISE NOTICE 'Live index migrations already applied, skipping';
        RETURN;
    END IF;

    -- === DROP obsolete indexes ===
    -- From fix_job_completed_index_2
    DROP INDEX IF EXISTS ix_completed_job_workspace_id_created_at;
    DROP INDEX IF EXISTS ix_completed_job_workspace_id_created_at_new;

    -- From fix_job_completed_index_3
    DROP INDEX IF EXISTS index_completed_job_on_schedule_path;
    DROP INDEX IF EXISTS concurrency_limit_stats_queue;
    DROP INDEX IF EXISTS root_job_index;
    DROP INDEX IF EXISTS index_completed_on_created;

    -- From fix_job_index_1_II
    DROP INDEX IF EXISTS root_job_index_by_path_2;
    DROP INDEX IF EXISTS ix_completed_job_workspace_id_created_at_new_2;
    DROP INDEX IF EXISTS ix_completed_job_workspace_id_started_at_new;
    DROP INDEX IF EXISTS root_job_index_by_path;

    -- From fix_labeled_jobs_index (recreated below)
    DROP INDEX IF EXISTS labeled_jobs_on_jobs;

    -- From v2_improve_v2_job_indices_ii
    DROP INDEX IF EXISTS ix_job_workspace_id_created_at_new_6;
    DROP INDEX IF EXISTS ix_job_workspace_id_created_at_new_7;

    -- From v2_improve_v2_queued_jobs_indices
    DROP INDEX IF EXISTS queue_sort;
    DROP INDEX IF EXISTS queue_sort_2;

    -- From remove_redundant_log_file_index
    DROP INDEX IF EXISTS log_file_hostname_log_ts_idx;

    -- From v2_job_completed_completed_at_9
    DROP INDEX IF EXISTS ix_completed_job_workspace_id_started_at_new_2;
    DROP INDEX IF EXISTS ix_job_created_at;
    DROP INDEX IF EXISTS ix_v2_job_root_by_path;

    -- === CREATE indexes ===
    -- From fix_job_index_1_II
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

    -- From fix_labeled_jobs_index
    CREATE INDEX IF NOT EXISTS labeled_jobs_on_jobs
        ON v2_job_completed USING GIN ((result -> 'wm_labels'))
        WHERE result ? 'wm_labels';

    -- From v2_labeled_jobs_index
    CREATE INDEX IF NOT EXISTS ix_v2_job_labels
        ON v2_job USING GIN (labels)
        WHERE labels IS NOT NULL;

    -- From v2_jobs_rls
    ALTER TABLE v2_job ENABLE ROW LEVEL SECURITY;

    -- From v2_improve_v2_job_indices_ii
    CREATE INDEX IF NOT EXISTS ix_v2_job_workspace_id_created_at
        ON v2_job (workspace_id, created_at DESC)
        WHERE kind IN ('script', 'flow', 'singlestepflow') AND parent_job IS NULL;

    -- From v2_improve_v2_queued_jobs_indices
    CREATE INDEX IF NOT EXISTS queue_sort_v2
        ON v2_job_queue (priority DESC NULLS LAST, scheduled_for, tag)
        WHERE running = false;

    -- From audit_timestamps
    CREATE INDEX IF NOT EXISTS ix_audit_timestamps
        ON audit (timestamp DESC);

    -- From job_completed_completed_at
    CREATE INDEX IF NOT EXISTS ix_job_completed_completed_at
        ON v2_job_completed (completed_at DESC);

    -- From alerts_by_workspace
    CREATE INDEX IF NOT EXISTS alerts_by_workspace
        ON alerts (workspace_id);

    -- From v2_job_queue_suspend
    CREATE INDEX IF NOT EXISTS v2_job_queue_suspend
        ON v2_job_queue (workspace_id, suspend)
        WHERE suspend > 0;

    -- From audit_recent_login_activities
    CREATE INDEX IF NOT EXISTS idx_audit_recent_login_activities
        ON audit (timestamp, username)
        WHERE operation IN ('users.login', 'oauth.login', 'users.token.refresh');

    -- From v2_script_lock_index
    CREATE INDEX IF NOT EXISTS script_not_archived
        ON script (workspace_id, path, created_at DESC)
        WHERE archived = false;

    -- From v2_job_completed_completed_at_9
    CREATE INDEX IF NOT EXISTS ix_job_workspace_id_completed_at_all
        ON v2_job_completed (workspace_id, completed_at DESC);

    CREATE INDEX IF NOT EXISTS idx_job_v2_job_root_by_path_2
        ON v2_job (workspace_id, runnable_path)
        WHERE parent_job IS NULL;

    CREATE INDEX IF NOT EXISTS ix_job_root_job_index_by_path_2
        ON v2_job (workspace_id, runnable_path, created_at DESC)
        WHERE parent_job IS NULL;

    -- === Mark all live migrations as done ===
    INSERT INTO windmill_migrations (name) VALUES
        ('fix_job_completed_index_2'),
        ('fix_job_completed_index_3'),
        ('fix_job_index_1_II'),
        ('fix_labeled_jobs_index'),
        ('v2_labeled_jobs_index'),
        ('v2_jobs_rls'),
        ('v2_improve_v2_job_indices_ii'),
        ('v2_improve_v2_queued_jobs_indices'),
        ('audit_timestamps'),
        ('job_completed_completed_at'),
        ('alerts_by_workspace'),
        ('remove_redundant_log_file_index'),
        ('v2_job_queue_suspend'),
        ('audit_recent_login_activities'),
        ('v2_script_lock_index'),
        ('v2_job_completed_completed_at_9')
    ON CONFLICT DO NOTHING;
END $$;
