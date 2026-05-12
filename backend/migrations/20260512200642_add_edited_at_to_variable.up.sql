-- Add `edited_at` and `edited_by` so the UI can detect when a variable has
-- been modified remotely while a local autosave was in flight (see the
-- UserDraft staleness check). Mirrors what `resource` already has.

ALTER TABLE variable
    ADD COLUMN edited_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    ADD COLUMN edited_by VARCHAR(50);
