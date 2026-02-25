ALTER TABLE volume ADD COLUMN IF NOT EXISTS last_read_at TIMESTAMPTZ;
ALTER TABLE volume ADD COLUMN IF NOT EXISTS last_write_at TIMESTAMPTZ;
UPDATE volume SET last_read_at = last_used_at, last_write_at = last_used_at;
ALTER TABLE volume DROP COLUMN IF EXISTS last_used_at;
