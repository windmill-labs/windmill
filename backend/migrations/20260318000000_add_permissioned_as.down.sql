-- Revert: re-add email columns to trigger tables, drop permissioned_as

ALTER TABLE http_trigger ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE http_trigger DROP COLUMN permissioned_as;

ALTER TABLE websocket_trigger ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE websocket_trigger DROP COLUMN permissioned_as;

ALTER TABLE postgres_trigger ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE postgres_trigger DROP COLUMN permissioned_as;

ALTER TABLE mqtt_trigger ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE mqtt_trigger DROP COLUMN permissioned_as;

ALTER TABLE kafka_trigger ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE kafka_trigger DROP COLUMN permissioned_as;

ALTER TABLE nats_trigger ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE nats_trigger DROP COLUMN permissioned_as;

ALTER TABLE sqs_trigger ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE sqs_trigger DROP COLUMN permissioned_as;

ALTER TABLE gcp_trigger ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE gcp_trigger DROP COLUMN permissioned_as;

ALTER TABLE email_trigger ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE email_trigger DROP COLUMN permissioned_as;

ALTER TABLE schedule DROP COLUMN permissioned_as;
