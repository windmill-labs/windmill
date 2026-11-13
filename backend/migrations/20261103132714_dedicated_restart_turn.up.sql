ALTER TABLE worker_ping ADD COLUMN IF NOT EXISTS dedicated_restart_at TIMESTAMPTZ;
