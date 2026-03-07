ALTER TABLE worker_ping ADD COLUMN IF NOT EXISTS uses_batch_http_pull BOOLEAN NOT NULL DEFAULT false;
