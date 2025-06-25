CREATE TYPE ASSET_USAGE_KIND AS ENUM ('script', 'flow', 'flow_step');
CREATE TYPE ASSET_KIND AS ENUM ('s3_object', 'resource');

CREATE TABLE assets (
  path VARCHAR(255) NOT NULL,
  kind ASSET_KIND NOT NULL,
  usage_path VARCHAR(255) NOT NULL,
  usage_kind ASSET_USAGE_KIND NOT NULL
);