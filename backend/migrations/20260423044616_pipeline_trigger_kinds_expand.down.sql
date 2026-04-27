-- Postgres doesn't support removing enum values in-place. The only safe
-- rollback is to recreate the type with the original set and rewrite the
-- column, deleting any rows using values introduced in the up migration.
DELETE FROM script_trigger
 WHERE trigger_kind NOT IN ('asset', 'schedule');

CREATE TYPE SCRIPT_TRIGGER_KIND_OLD AS ENUM ('asset', 'schedule');
ALTER TABLE script_trigger
  ALTER COLUMN trigger_kind TYPE SCRIPT_TRIGGER_KIND_OLD
  USING trigger_kind::text::SCRIPT_TRIGGER_KIND_OLD;
DROP TYPE SCRIPT_TRIGGER_KIND;
ALTER TYPE SCRIPT_TRIGGER_KIND_OLD RENAME TO SCRIPT_TRIGGER_KIND;
