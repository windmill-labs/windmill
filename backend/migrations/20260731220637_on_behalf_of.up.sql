-- Add up migration script here
-- Authorization identity a script/flow runs as. NULL means "not recorded"; the migration that
-- follows this one backfills every row that had an on_behalf_of_email, so by the end of this
-- release a NULL principal means the runnable runs as its caller.
ALTER TABLE script ADD COLUMN IF NOT EXISTS on_behalf_of VARCHAR(255);
ALTER TABLE flow ADD COLUMN IF NOT EXISTS on_behalf_of VARCHAR(255);
