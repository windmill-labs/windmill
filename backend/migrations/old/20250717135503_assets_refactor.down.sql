-- Add 'variable' kind back
ALTER TABLE asset ALTER column kind TYPE VARCHAR;
DROP TYPE asset_kind;
CREATE TYPE ASSET_KIND AS ENUM ('s3object', 'resource', 'variable');
ALTER TABLE asset ALTER column kind TYPE ASSET_KIND using kind::ASSET_KIND;

ALTER TABLE script DROP COLUMN assets;
