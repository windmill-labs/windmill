-- Add down migration script here
-- Add back the is_async column
ALTER TABLE http_trigger ADD COLUMN is_async BOOLEAN;

-- Migrate request_type values back to is_async
-- 'async' -> TRUE
-- 'sync' or 'sync_sse' -> FALSE
UPDATE http_trigger SET is_async = CASE
    WHEN request_type = 'async'::REQUEST_TYPE THEN TRUE
    ELSE FALSE
END;

-- Make is_async NOT NULL with default
ALTER TABLE http_trigger ALTER COLUMN is_async SET NOT NULL;
ALTER TABLE http_trigger ALTER COLUMN is_async SET DEFAULT FALSE;

-- Drop the request_type column and type
ALTER TABLE http_trigger DROP COLUMN request_type;
DROP TYPE REQUEST_TYPE;
