-- Add down migration script here
ALTER TABLE draft DROP COLUMN id;

-- Recreate the enum. Drop any rows whose typ falls outside the original
-- enum values; without this the type cast below would fail.
DELETE FROM draft WHERE typ NOT IN ('script', 'flow', 'app');
CREATE TYPE DRAFT_TYPE AS ENUM ('script', 'flow', 'app');
ALTER TABLE draft ALTER COLUMN typ TYPE DRAFT_TYPE USING typ::DRAFT_TYPE;
