-- Add non_diffable flag to resource table.
-- When true, the resource is excluded from workspace diff comparisons (e.g. auto-created resources during fork).
ALTER TABLE resource ADD COLUMN IF NOT EXISTS non_diffable BOOLEAN NOT NULL DEFAULT false;
