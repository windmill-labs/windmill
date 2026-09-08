-- `ahead`/`behind` count writes on a side without recording what kind of write it
-- was or who made it, which leaves one row shape undecidable: an item the parent
-- has and the fork does not can mean the parent added it, the fork deleted it, or
-- a sync reverted it in the fork. Record the last event per side so the merge
-- direction can offer a removal on evidence.
--
-- `kind` is 'write' | 'delete' | 'rename_from' (the path was vacated by a rename
-- to another path); create and update are not split because nothing at the tally
-- point tells them apart for every item kind. `origin` is 'authored' | 'sync'
-- (applied by a git-sync pull or a workspace-to-workspace deploy rather than
-- authored in that workspace) and is the load-bearing half: a sync-origin delete
-- is a revert, not a fork deletion.
--
-- NULL on every row written before this migration, and on a side that has not
-- been written since: such a row has no history and must keep the conservative
-- rule (no removal offered).
ALTER TABLE workspace_diff
    ADD COLUMN fork_last_event_kind VARCHAR(20),
    ADD COLUMN fork_last_event_origin VARCHAR(20),
    ADD COLUMN source_last_event_kind VARCHAR(20),
    ADD COLUMN source_last_event_origin VARCHAR(20);
