-- V2 Migration Finalization
-- This migration finalizes the v2 job table migration by removing the compatibility layer.
-- All operations are idempotent (safe to run on databases where this was already done via live_migrations).

-- 1. Disable RLS on v2 tables (no-op if already disabled)
ALTER TABLE v2_job_queue DISABLE ROW LEVEL SECURITY;
ALTER TABLE v2_job_completed DISABLE ROW LEVEL SECURITY;

-- 2. Drop sync functions (CASCADE drops associated triggers)
DROP FUNCTION IF EXISTS v2_job_after_update CASCADE;
DROP FUNCTION IF EXISTS v2_job_queue_before_insert CASCADE;
DROP FUNCTION IF EXISTS v2_job_queue_after_insert CASCADE;
DROP FUNCTION IF EXISTS v2_job_queue_before_update CASCADE;
DROP FUNCTION IF EXISTS v2_job_completed_before_insert CASCADE;
DROP FUNCTION IF EXISTS v2_job_completed_before_update CASCADE;
DROP FUNCTION IF EXISTS v2_job_runtime_before_insert CASCADE;
DROP FUNCTION IF EXISTS v2_job_runtime_before_update CASCADE;
DROP FUNCTION IF EXISTS v2_job_status_before_insert CASCADE;
DROP FUNCTION IF EXISTS v2_job_status_before_update CASCADE;

-- 3. Drop compatibility views
DROP VIEW IF EXISTS completed_job CASCADE;
DROP VIEW IF EXISTS completed_job_view CASCADE;
DROP VIEW IF EXISTS job CASCADE;
DROP VIEW IF EXISTS queue CASCADE;
DROP VIEW IF EXISTS queue_view CASCADE;

-- 4. Drop __ columns from v2_job_queue
ALTER TABLE v2_job_queue
    DROP COLUMN IF EXISTS __parent_job CASCADE,
    DROP COLUMN IF EXISTS __created_by CASCADE,
    DROP COLUMN IF EXISTS __script_hash CASCADE,
    DROP COLUMN IF EXISTS __script_path CASCADE,
    DROP COLUMN IF EXISTS __args CASCADE,
    DROP COLUMN IF EXISTS __logs CASCADE,
    DROP COLUMN IF EXISTS __raw_code CASCADE,
    DROP COLUMN IF EXISTS __canceled CASCADE,
    DROP COLUMN IF EXISTS __last_ping CASCADE,
    DROP COLUMN IF EXISTS __job_kind CASCADE,
    DROP COLUMN IF EXISTS __env_id CASCADE,
    DROP COLUMN IF EXISTS __schedule_path CASCADE,
    DROP COLUMN IF EXISTS __permissioned_as CASCADE,
    DROP COLUMN IF EXISTS __flow_status CASCADE,
    DROP COLUMN IF EXISTS __raw_flow CASCADE,
    DROP COLUMN IF EXISTS __is_flow_step CASCADE,
    DROP COLUMN IF EXISTS __language CASCADE,
    DROP COLUMN IF EXISTS __same_worker CASCADE,
    DROP COLUMN IF EXISTS __raw_lock CASCADE,
    DROP COLUMN IF EXISTS __pre_run_error CASCADE,
    DROP COLUMN IF EXISTS __email CASCADE,
    DROP COLUMN IF EXISTS __visible_to_owner CASCADE,
    DROP COLUMN IF EXISTS __mem_peak CASCADE,
    DROP COLUMN IF EXISTS __root_job CASCADE,
    DROP COLUMN IF EXISTS __leaf_jobs CASCADE,
    DROP COLUMN IF EXISTS __concurrent_limit CASCADE,
    DROP COLUMN IF EXISTS __concurrency_time_window_s CASCADE,
    DROP COLUMN IF EXISTS __timeout CASCADE,
    DROP COLUMN IF EXISTS __flow_step_id CASCADE,
    DROP COLUMN IF EXISTS __cache_ttl CASCADE;

-- 5. Drop __ columns from v2_job_completed
ALTER TABLE v2_job_completed
    DROP COLUMN IF EXISTS __parent_job CASCADE,
    DROP COLUMN IF EXISTS __created_by CASCADE,
    DROP COLUMN IF EXISTS __created_at CASCADE,
    DROP COLUMN IF EXISTS __success CASCADE,
    DROP COLUMN IF EXISTS __script_hash CASCADE,
    DROP COLUMN IF EXISTS __script_path CASCADE,
    DROP COLUMN IF EXISTS __args CASCADE,
    DROP COLUMN IF EXISTS __logs CASCADE,
    DROP COLUMN IF EXISTS __raw_code CASCADE,
    DROP COLUMN IF EXISTS __canceled CASCADE,
    DROP COLUMN IF EXISTS __job_kind CASCADE,
    DROP COLUMN IF EXISTS __env_id CASCADE,
    DROP COLUMN IF EXISTS __schedule_path CASCADE,
    DROP COLUMN IF EXISTS __permissioned_as CASCADE,
    DROP COLUMN IF EXISTS __raw_flow CASCADE,
    DROP COLUMN IF EXISTS __is_flow_step CASCADE,
    DROP COLUMN IF EXISTS __language CASCADE,
    DROP COLUMN IF EXISTS __is_skipped CASCADE,
    DROP COLUMN IF EXISTS __raw_lock CASCADE,
    DROP COLUMN IF EXISTS __email CASCADE,
    DROP COLUMN IF EXISTS __visible_to_owner CASCADE,
    DROP COLUMN IF EXISTS __tag CASCADE,
    DROP COLUMN IF EXISTS __priority CASCADE;
