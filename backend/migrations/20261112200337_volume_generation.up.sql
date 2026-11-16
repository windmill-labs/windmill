-- Odd while a writer is changing the volume's objects, even otherwise. Read-only mounts take no
-- lease and use it to detect a write that overlapped their pull.
ALTER TABLE volume ADD COLUMN IF NOT EXISTS generation BIGINT NOT NULL DEFAULT 0;
