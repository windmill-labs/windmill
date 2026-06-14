-- Restore the `draft_only` column on script/flow/app. Data is not
-- resurrected — stubs deleted in the up migration are gone and the
-- column comes back NULL everywhere.
ALTER TABLE script ADD COLUMN draft_only BOOLEAN;
ALTER TABLE flow ADD COLUMN draft_only BOOLEAN;
ALTER TABLE app ADD COLUMN draft_only BOOLEAN;
