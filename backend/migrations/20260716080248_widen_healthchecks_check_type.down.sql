-- Rows written since the up migration may exceed 50 chars; truncate so the cast succeeds.
ALTER TABLE healthchecks ALTER COLUMN check_type TYPE varchar(50) USING left(check_type, 50);
