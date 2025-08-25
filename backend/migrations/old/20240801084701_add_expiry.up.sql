-- Add up migration script here
ALTER TABLE variable ADD COLUMN IF NOT EXISTS expires_at TIMESTAMP WITH TIME ZONE;
UPDATE variable SET expires_at = (now() + '7 days'::interval)  WHERE path LIKE  'u/%/secret_arg/%'
