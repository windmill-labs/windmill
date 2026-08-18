-- workspace_diff / skip_workspace_diff_tally are keyed by workspace id with no FK
-- cascade, so until the matching delete_workspace cleanup landed, deleting a fork
-- left its cached diff rows behind. Workspace ids are reused (recreating a fork
-- under the same name), so those orphaned rows leaked onto the new fork and
-- produced a spurious "changes not visible" warning that hid the deploy button.

-- Drop rows referencing workspaces that no longer exist (leftovers from past deletes).
DELETE FROM workspace_diff
WHERE source_workspace_id NOT IN (SELECT id FROM workspace)
   OR fork_workspace_id NOT IN (SELECT id FROM workspace);

DELETE FROM skip_workspace_diff_tally
WHERE workspace_id NOT IN (SELECT id FROM workspace);

-- Reused-id victims keep a skip-tally row pointing at a LIVE workspace, so the
-- orphan cleanup above can't reach them. compare_workspaces consults
-- skip_workspace_diff_tally first and short-circuits to an empty comparison, so a
-- stale skip row would also defeat the has_changes reset below. A genuinely
-- skipped workspace is excluded from tallying and therefore never has
-- workspace_diff rows — so a skip row that coexists with workspace_diff rows for
-- that id is provably leaked from a previous occupant of a reused id. Drop those.
DELETE FROM skip_workspace_diff_tally s
WHERE EXISTS (
    SELECT 1 FROM workspace_diff d WHERE d.fork_workspace_id = s.workspace_id
);

-- Remaining reused-id victims keep workspace_diff rows pointing at live
-- workspaces, so they can't be told apart from valid cache. Reset the cached
-- verdict to force a recompute on the next compare; compare_workspaces
-- re-evaluates NULL rows and corrects or deletes them.
UPDATE workspace_diff SET has_changes = NULL;
