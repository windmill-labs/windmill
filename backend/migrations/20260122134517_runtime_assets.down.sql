ALTER TABLE asset
  DROP COLUMN IF EXISTS created_at,
  DROP COLUMN IF EXISTS id;

DELETE FROM asset WHERE usage_kind = 'job';