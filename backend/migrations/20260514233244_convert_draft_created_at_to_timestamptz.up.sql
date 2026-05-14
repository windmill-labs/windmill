-- `draft.created_at` was originally created as `TIMESTAMP` (no timezone). The
-- new `*WithDraft` API responses surface it as `chrono::DateTime<Utc>` for the
-- frontend's staleness check, which requires `TIMESTAMPTZ`. Existing values
-- are interpreted as UTC (matching `now()`'s behaviour on a UTC server, the
-- expected deployment for Windmill).
ALTER TABLE draft
    ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'UTC';
