ALTER TABLE volume ADD COLUMN IF NOT EXISTS last_used_at TIMESTAMPTZ;
UPDATE volume SET last_used_at = GREATEST(last_read_at, last_write_at);
ALTER TABLE volume DROP COLUMN IF EXISTS last_read_at;
ALTER TABLE volume DROP COLUMN IF EXISTS last_write_at;
