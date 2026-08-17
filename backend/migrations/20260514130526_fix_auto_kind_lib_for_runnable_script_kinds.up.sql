-- Failure, Trigger, and Approval scripts are runnable entrypoints by
-- definition. A prior parser regression occasionally classified them as
-- `auto_kind = 'lib'`, which hid them from the flow error-handler /
-- trigger / approval pickers. Clear those stray values so existing
-- affected scripts re-appear without requiring a redeploy.
UPDATE script
SET auto_kind = NULL
WHERE auto_kind = 'lib'
  AND kind IN ('failure', 'trigger', 'approval');
