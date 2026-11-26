-- Adding nullable columns to the partitioned parent is a catalog change on every partition,
-- with no rewrite.
ALTER TABLE audit_partitioned ADD COLUMN IF NOT EXISTS client_ip TEXT, ADD COLUMN IF NOT EXISTS user_agent TEXT;
