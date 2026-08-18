-- The notify_event table (migration 20260203172950_polling_based_events) was
-- created relying on ALTER DEFAULT PRIVILEGES to grant access to windmill_user
-- and windmill_admin. Those default privileges only apply to objects created by
-- the role that set them (migration 20250205131523), so deployments whose
-- migration runner is a different role leave notify_event ungranted. Trigger
-- inserts were worked around with SECURITY DEFINER, but direct application
-- inserts (clear_static_asset_usage in assets.rs, restart_worker_group in
-- settings) run as the invoking role and fail with "permission denied for table
-- notify_event". Grant explicitly to guarantee access regardless of who ran the
-- migrations.
GRANT ALL ON notify_event TO windmill_user;
GRANT ALL ON notify_event TO windmill_admin;
GRANT ALL ON SEQUENCE notify_event_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE notify_event_id_seq TO windmill_admin;
