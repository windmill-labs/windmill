-- Symmetric to the up migration: Postgres's default `TIMESTAMPTZ -> TIMESTAMP`
-- cast strips the timezone by representing the instant in the session's
-- current timezone, mirroring how the original `now()` values were
-- truncated on insert.
ALTER TABLE draft ALTER COLUMN created_at TYPE TIMESTAMP;
