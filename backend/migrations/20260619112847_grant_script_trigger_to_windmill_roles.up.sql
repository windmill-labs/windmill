-- The script_trigger table (migration 20260423050000_script_trigger) was
-- created relying on ALTER DEFAULT PRIVILEGES to grant access to windmill_user
-- and windmill_admin. Those default privileges only apply to objects created by
-- the role that set them (migration 20250205131523), so deployments whose
-- migration runner is a different role leave script_trigger ungranted. Direct
-- application writes run as the invoking role (clear_script_triggers and
-- insert_script_trigger in windmill-common/src/assets.rs, every script save)
-- and fail with "permission denied for table script_trigger". Grant explicitly
-- to guarantee access regardless of who ran the migrations (same fix as
-- notify_event in 20260619091631).
GRANT ALL ON script_trigger TO windmill_user;
GRANT ALL ON script_trigger TO windmill_admin;
GRANT ALL ON SEQUENCE script_trigger_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE script_trigger_id_seq TO windmill_admin;
