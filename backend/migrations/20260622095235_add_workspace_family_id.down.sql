DROP INDEX IF EXISTS workspace_family_id_idx;
ALTER TABLE workspace DROP COLUMN IF EXISTS family_id;
