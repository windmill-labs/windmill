ALTER TABLE token ADD COLUMN IF NOT EXISTS allowed_ips cidr[];
