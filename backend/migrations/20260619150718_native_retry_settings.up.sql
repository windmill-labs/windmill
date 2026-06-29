CREATE TABLE IF NOT EXISTS retry_settings(
    hash                        BIGINT PRIMARY KEY,
    constant_attempts           INTEGER,
    constant_seconds            INTEGER,
    exponential_attempts        INTEGER,
    exponential_multiplier      INTEGER,
    exponential_seconds         INTEGER,
    exponential_random_factor   INTEGER,
    retry_if_expr               TEXT
);

ALTER TABLE runnable_settings
ADD COLUMN IF NOT EXISTS retry_settings BIGINT DEFAULT NULL;

GRANT ALL ON retry_settings TO windmill_admin;
GRANT ALL ON retry_settings TO windmill_user;
