-- Add up migration script here
-- Authorization identity a script/flow runs as when on_behalf_of_email is set.
-- NULL means "not recorded": run time falls back to deriving it from created_by /
-- edited_by, which is what every row written before this column existed relies on.
ALTER TABLE script ADD COLUMN IF NOT EXISTS on_behalf_of VARCHAR(255);
ALTER TABLE flow ADD COLUMN IF NOT EXISTS on_behalf_of VARCHAR(255);
