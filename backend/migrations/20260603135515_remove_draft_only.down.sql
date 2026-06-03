-- Re-add the draft_only columns (nullable, as originally created). The stub
-- rows deleted by the up migration are not restored — their content lives on
-- in the `draft` table, which is where draft-only items now belong. This keeps
-- the schema fully reversible; rolling forward again is a no-op for those
-- already-transferred drafts (ON CONFLICT DO NOTHING).
ALTER TABLE script ADD COLUMN draft_only BOOLEAN;
ALTER TABLE flow ADD COLUMN draft_only BOOLEAN;
ALTER TABLE app ADD COLUMN draft_only BOOLEAN;
