ALTER TABLE asset
  DROP COLUMN IF EXISTS created_at;

DELETE FROM asset WHERE usage_kind = 'job';