-- Add permissioned_as column to all trigger tables and schedule
-- permissioned_as stores 'u/{username}', 'g/{group}', or raw email

-- Trigger tables: add permissioned_as, drop email
ALTER TABLE http_trigger ADD COLUMN permissioned_as VARCHAR(255) NOT NULL DEFAULT '';
UPDATE http_trigger SET permissioned_as = 'u/' || edited_by;
ALTER TABLE http_trigger DROP COLUMN email;

ALTER TABLE websocket_trigger ADD COLUMN permissioned_as VARCHAR(255) NOT NULL DEFAULT '';
UPDATE websocket_trigger SET permissioned_as = 'u/' || edited_by;
ALTER TABLE websocket_trigger DROP COLUMN email;

ALTER TABLE postgres_trigger ADD COLUMN permissioned_as VARCHAR(255) NOT NULL DEFAULT '';
UPDATE postgres_trigger SET permissioned_as = 'u/' || edited_by;
ALTER TABLE postgres_trigger DROP COLUMN email;

ALTER TABLE mqtt_trigger ADD COLUMN permissioned_as VARCHAR(255) NOT NULL DEFAULT '';
UPDATE mqtt_trigger SET permissioned_as = 'u/' || edited_by;
ALTER TABLE mqtt_trigger DROP COLUMN email;

ALTER TABLE kafka_trigger ADD COLUMN permissioned_as VARCHAR(255) NOT NULL DEFAULT '';
UPDATE kafka_trigger SET permissioned_as = 'u/' || edited_by;
ALTER TABLE kafka_trigger DROP COLUMN email;

ALTER TABLE nats_trigger ADD COLUMN permissioned_as VARCHAR(255) NOT NULL DEFAULT '';
UPDATE nats_trigger SET permissioned_as = 'u/' || edited_by;
ALTER TABLE nats_trigger DROP COLUMN email;

ALTER TABLE sqs_trigger ADD COLUMN permissioned_as VARCHAR(255) NOT NULL DEFAULT '';
UPDATE sqs_trigger SET permissioned_as = 'u/' || edited_by;
ALTER TABLE sqs_trigger DROP COLUMN email;

ALTER TABLE gcp_trigger ADD COLUMN permissioned_as VARCHAR(255) NOT NULL DEFAULT '';
UPDATE gcp_trigger SET permissioned_as = 'u/' || edited_by;
ALTER TABLE gcp_trigger DROP COLUMN email;

ALTER TABLE email_trigger ADD COLUMN permissioned_as VARCHAR(255) NOT NULL DEFAULT '';
UPDATE email_trigger SET permissioned_as = 'u/' || edited_by;
ALTER TABLE email_trigger DROP COLUMN email;

-- Schedule table: add permissioned_as, keep email for backwards compat with old workers
ALTER TABLE schedule ADD COLUMN permissioned_as VARCHAR(255) NOT NULL DEFAULT '';
UPDATE schedule SET permissioned_as = 'u/' || edited_by;
