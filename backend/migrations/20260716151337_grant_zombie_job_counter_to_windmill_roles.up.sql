-- The zombie_job_counter table (migration 20250205131522) was created relying on
-- ALTER DEFAULT PRIVILEGES to grant access to windmill_user and windmill_admin.
-- Those default privileges only apply to objects created by the role that set
-- them (migration 20250205131523), so deployments whose migration runner is a
-- different role leave zombie_job_counter ungranted. The table used to be
-- reached only through ON DELETE CASCADE, which bypasses caller permissions,
-- but 20260625092813_drop_v2_job_side_table_cascades replaced that cascade with
-- an explicit DELETE in delete_jobs (windmill-common/src/jobs.rs) which runs as
-- the invoking role and fails with "permission denied for table
-- zombie_job_counter". Grant explicitly to guarantee access regardless of who
-- ran the migrations (same fix as notify_event in 20260619091631,
-- script_trigger in 20260619112847 and dispatch_event in 20260701080313).
GRANT ALL ON zombie_job_counter TO windmill_user;
GRANT ALL ON zombie_job_counter TO windmill_admin;
