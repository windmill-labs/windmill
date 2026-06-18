-- Add up migration script here
-- Create the request_type enum type
CREATE TYPE REQUEST_TYPE AS ENUM ('sync', 'async', 'sync_sse');

-- Add the new request_type column with a default value
ALTER TABLE http_trigger ADD COLUMN request_type REQUEST_TYPE;

-- Migrate existing is_async values to request_type
-- is_async = FALSE -> 'sync'
-- is_async = TRUE -> 'async'
UPDATE http_trigger SET request_type = CASE
    WHEN is_async = TRUE THEN 'async'::REQUEST_TYPE
    ELSE 'sync'::REQUEST_TYPE
END;

-- Make request_type NOT NULL now that all values are populated
ALTER TABLE http_trigger ALTER COLUMN request_type SET NOT NULL;

-- Set default for new rows
ALTER TABLE http_trigger ALTER COLUMN request_type SET DEFAULT 'sync'::REQUEST_TYPE;

-- Drop the old is_async column
ALTER TABLE http_trigger DROP COLUMN is_async;
