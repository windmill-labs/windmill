ALTER TABLE workspace_settings
DROP COLUMN ducklake;

-- Remove 'ducklake' kind
DELETE FROM asset WHERE kind = 'ducklake';
ALTER TABLE asset ALTER column kind TYPE VARCHAR;
DROP TYPE asset_kind;
CREATE TYPE ASSET_KIND AS ENUM ('s3object', 'resource', 'variable');
ALTER TABLE asset ALTER column kind TYPE ASSET_KIND using kind::ASSET_KIND;