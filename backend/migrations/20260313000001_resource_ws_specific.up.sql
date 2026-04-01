-- Add ws_specific flag to resource table.
-- When true, the resource is excluded from workspace diff comparisons (e.g. auto-created resources during fork).
ALTER TABLE resource ADD COLUMN IF NOT EXISTS ws_specific BOOLEAN NOT NULL DEFAULT false;
