-- Add `edited_at` and `edited_by` so the UI can detect when a variable has
-- been modified remotely while a local autosave was in flight (see the
-- UserDraft staleness check). Mirrors what `resource` already has.
--
-- Backfill: existing rows get `edited_at = now()` via the column's DEFAULT.
-- All pre-migration variables therefore appear to share a single edit
-- timestamp (the migration time). The staleness check only consumes
-- `edited_at` as an opaque rev string — it doesn't display or sort on it —
-- and only after the user edits a variable forward at least once. So the
-- collision is harmless: no UI flow looks at the pre-migration timestamp
-- before it gets overwritten by a real edit.

ALTER TABLE variable
    ADD COLUMN edited_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    ADD COLUMN edited_by VARCHAR(50);
