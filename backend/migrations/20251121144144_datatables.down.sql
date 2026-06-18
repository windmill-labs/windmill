-- Add down migration script here

ALTER TABLE workspace_settings
DROP COLUMN datatable;

-- Remove 'datatable' kind
DELETE FROM asset WHERE kind = 'datatable';
ALTER TABLE asset ALTER column kind TYPE VARCHAR;
DROP TYPE asset_kind;
CREATE TYPE ASSET_KIND AS ENUM ('s3object', 'resource', 'variable', 'ducklake');
ALTER TABLE asset ALTER column kind TYPE ASSET_KIND using kind::ASSET_KIND;