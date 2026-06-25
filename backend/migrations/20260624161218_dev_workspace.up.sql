-- A dev workspace is a fork (parent_workspace_id set) that is the standing editable
-- environment paired with its parent ("prod"), as opposed to a throwaway fork.
ALTER TABLE workspace ADD COLUMN is_dev_workspace BOOLEAN NOT NULL DEFAULT false;

-- At most one active canonical dev workspace per parent (one editable source per prod).
-- Excludes soft-deleted (archived) workspaces so a new dev can replace an archived one.
CREATE UNIQUE INDEX workspace_canonical_dev_idx ON workspace (parent_workspace_id)
    WHERE is_dev_workspace AND deleted = false;
