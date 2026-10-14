ALTER TABLE job_perms DROP COLUMN IF EXISTS job_token_scopes;
ALTER TABLE flow DROP COLUMN IF EXISTS job_token_scopes;
ALTER TABLE script DROP COLUMN IF EXISTS job_token_scopes;
