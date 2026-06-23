-- Immutable family identity for a workspace fork family.
--
-- Workspace ids are reusable after a permanent delete (see check_fork_w_id_conflict),
-- so they cannot key client-side per-family session storage without crossover. family_id
-- is a never-reused UUID, minted once on the root workspace and inherited by every fork in
-- the family (copied from the parent at fork creation). It is the stable identity the
-- frontend keys its per-family session IndexedDB by. Mirrors the token_family pattern in
-- mcp_oauth_refresh_token.

ALTER TABLE workspace ADD COLUMN family_id UUID;

-- Backfill roots (and severed forks, whose parent_workspace_id was SET NULL on parent
-- delete — they are now their own family) with a fresh uuid.
UPDATE workspace SET family_id = gen_random_uuid()
WHERE parent_workspace_id IS NULL AND family_id IS NULL;

-- Backfill forks by walking the (still-intact) parent chain to the family root and copying
-- the root's family_id.
WITH RECURSIVE fam AS (
    SELECT id, parent_workspace_id, id AS root_id
    FROM workspace
    WHERE parent_workspace_id IS NULL
    UNION ALL
    SELECT w.id, w.parent_workspace_id, f.root_id
    FROM workspace w
    JOIN fam f ON w.parent_workspace_id = f.id
)
UPDATE workspace w
SET family_id = root.family_id
FROM fam
JOIN workspace root ON root.id = fam.root_id
WHERE w.id = fam.id AND w.family_id IS NULL;

-- Any rows still null (cycles / unreachable) get their own family so the column can be
-- NOT NULL — defensive, should not occur.
UPDATE workspace SET family_id = gen_random_uuid() WHERE family_id IS NULL;

-- New non-fork workspaces get a fresh family automatically; fork creation overrides this
-- with the parent's family_id explicitly.
ALTER TABLE workspace ALTER COLUMN family_id SET DEFAULT gen_random_uuid();
ALTER TABLE workspace ALTER COLUMN family_id SET NOT NULL;
