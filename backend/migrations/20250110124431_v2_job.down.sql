-- Add down migration script here
DROP POLICY see_folder_extra_perms_user ON v2_job;
DROP POLICY see_own_path ON v2_job;
DROP POLICY see_member_path ON v2_job;
DROP POLICY see_own ON v2_job;
DROP POLICY see_member ON v2_job;
ALTER TABLE v2_job
    DROP COLUMN created_at CASCADE,
    DROP COLUMN created_by CASCADE,
    DROP COLUMN permissioned_as CASCADE,
    DROP COLUMN permissioned_as_email CASCADE,
    DROP COLUMN kind CASCADE,
    DROP COLUMN runnable_id CASCADE,
    DROP COLUMN runnable_path CASCADE,
    DROP COLUMN parent_job CASCADE,
    DROP COLUMN script_lang CASCADE,
    DROP COLUMN flow_step CASCADE,
    DROP COLUMN flow_step_id CASCADE,
    DROP COLUMN flow_root_job CASCADE,
    DROP COLUMN schedule_path CASCADE,
    DROP COLUMN same_worker CASCADE,
    DROP COLUMN visible_to_owner CASCADE,
    DROP COLUMN concurrent_limit CASCADE,
    DROP COLUMN concurrency_time_window_s CASCADE,
    DROP COLUMN cache_ttl CASCADE,
    DROP COLUMN timeout CASCADE,
    DROP COLUMN priority CASCADE,
    DROP COLUMN args CASCADE,
    DROP COLUMN pre_run_error CASCADE;
