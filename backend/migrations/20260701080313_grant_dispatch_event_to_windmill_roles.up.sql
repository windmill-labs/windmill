-- The dispatch_event table (migration 20260523055641_dispatch_event) was
-- created relying on ALTER DEFAULT PRIVILEGES to grant access to windmill_user
-- and windmill_admin. Those default privileges only apply to objects created by
-- the role that set them (migration 20250205131523), so deployments whose
-- migration runner is a different role leave dispatch_event ungranted. Direct
-- writes run as the invoking role -- the dispatcher insert (asset_dispatch.rs)
-- and the DELETE in delete_jobs (windmill-common/src/jobs.rs), reached whenever
-- a job's side rows are reaped, e.g. on schedule disable -- and fail with
-- "permission denied for table dispatch_event". Grant explicitly to guarantee
-- access regardless of who ran the migrations (same fix as notify_event in
-- 20260619091631 and script_trigger in 20260619112847).
GRANT ALL ON dispatch_event TO windmill_user;
GRANT ALL ON dispatch_event TO windmill_admin;
GRANT ALL ON SEQUENCE dispatch_event_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE dispatch_event_id_seq TO windmill_admin;
