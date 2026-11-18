-- Same grant gap fixed for notify_event (20260619091631), script_trigger
-- (20260619112847), and dispatch_event: tables created after the one-time
-- GRANT ALL in 20250205131523 rely on ALTER DEFAULT PRIVILEGES, which only
-- applies to objects created by the role that set them. On deployments whose
-- migration runner is a different role, these tables end up ungranted, and
-- writes that run under the RLS role (a transaction opened via
-- user_db.begin(&authed) -> SET LOCAL ROLE windmill_user/windmill_admin) fail
-- with "permission denied for table <name>".
--
-- Each table below has a confirmed write on a user_db transaction:
--   * workspace_diff        -- UPDATE in set_ws_specific (workspaces.rs)
--   * materialized_partition -- INSERT via record_materialization (assets API)
--   * debounce_stale_data   -- DELETE in resume_suspended_trigger_jobs
--                              (global_handler.rs), the same tx that reaps a
--                              job's side rows
-- None has a sequence, so only table grants are needed.
GRANT ALL ON workspace_diff TO windmill_user;
GRANT ALL ON workspace_diff TO windmill_admin;
GRANT ALL ON materialized_partition TO windmill_user;
GRANT ALL ON materialized_partition TO windmill_admin;
GRANT ALL ON debounce_stale_data TO windmill_user;
GRANT ALL ON debounce_stale_data TO windmill_admin;
