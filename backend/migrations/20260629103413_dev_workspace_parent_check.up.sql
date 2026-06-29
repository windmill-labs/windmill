-- A dev workspace is a fork, so it must have a parent. Enforce the invariant at the schema level so
-- no path (or manual write) can persist a "root dev workspace".
-- Downgrade any pre-existing invalid rows first (e.g. a dev whose parent was hard-deleted before
-- this constraint existed) so the constraint can be validated.
UPDATE workspace SET is_dev_workspace = false WHERE is_dev_workspace AND parent_workspace_id IS NULL;

ALTER TABLE workspace ADD CONSTRAINT workspace_dev_requires_parent
    CHECK (NOT is_dev_workspace OR parent_workspace_id IS NOT NULL);
