ALTER TABLE script ADD COLUMN IF NOT EXISTS job_token_scopes text[];
ALTER TABLE flow ADD COLUMN IF NOT EXISTS job_token_scopes text[];
ALTER TABLE job_perms ADD COLUMN IF NOT EXISTS job_token_scopes text[];
