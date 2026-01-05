-- Add up migration script here
CREATE TYPE TRIGGER_MODE AS ENUM ('enabled', 'disabled', 'suspended');

-- Rename enabled column to mode and change its type for each trigger table
ALTER TABLE gcp_trigger
ADD COLUMN mode TRIGGER_MODE;

UPDATE gcp_trigger SET mode = CASE WHEN enabled = true THEN 'enabled'::TRIGGER_MODE ELSE 'disabled'::TRIGGER_MODE END;

ALTER TABLE gcp_trigger
ALTER COLUMN mode SET NOT NULL,
ALTER COLUMN mode SET DEFAULT 'enabled'::TRIGGER_MODE,
DROP COLUMN enabled;

ALTER TABLE kafka_trigger
ADD COLUMN mode TRIGGER_MODE;

UPDATE kafka_trigger SET mode = CASE WHEN enabled = true THEN 'enabled'::TRIGGER_MODE ELSE 'disabled'::TRIGGER_MODE END;

ALTER TABLE kafka_trigger
ALTER COLUMN mode SET NOT NULL,
ALTER COLUMN mode SET DEFAULT 'enabled'::TRIGGER_MODE,
DROP COLUMN enabled;

ALTER TABLE mqtt_trigger
ADD COLUMN mode TRIGGER_MODE;

UPDATE mqtt_trigger SET mode = CASE WHEN enabled = true THEN 'enabled'::TRIGGER_MODE ELSE 'disabled'::TRIGGER_MODE END;

ALTER TABLE mqtt_trigger
ALTER COLUMN mode SET NOT NULL,
ALTER COLUMN mode SET DEFAULT 'enabled'::TRIGGER_MODE,
DROP COLUMN enabled;

ALTER TABLE nats_trigger
ADD COLUMN mode TRIGGER_MODE;

UPDATE nats_trigger SET mode = CASE WHEN enabled = true THEN 'enabled'::TRIGGER_MODE ELSE 'disabled'::TRIGGER_MODE END;

ALTER TABLE nats_trigger
ALTER COLUMN mode SET NOT NULL,
ALTER COLUMN mode SET DEFAULT 'enabled'::TRIGGER_MODE,
DROP COLUMN enabled;

ALTER TABLE postgres_trigger
ADD COLUMN mode TRIGGER_MODE;

UPDATE postgres_trigger SET mode = CASE WHEN enabled = true THEN 'enabled'::TRIGGER_MODE ELSE 'disabled'::TRIGGER_MODE END;

ALTER TABLE postgres_trigger
ALTER COLUMN mode SET NOT NULL,
ALTER COLUMN mode SET DEFAULT 'enabled'::TRIGGER_MODE,
DROP COLUMN enabled;

ALTER TABLE sqs_trigger
ADD COLUMN mode TRIGGER_MODE;

UPDATE sqs_trigger SET mode = CASE WHEN enabled = true THEN 'enabled'::TRIGGER_MODE ELSE 'disabled'::TRIGGER_MODE END;

ALTER TABLE sqs_trigger
ALTER COLUMN mode SET NOT NULL,
ALTER COLUMN mode SET DEFAULT 'enabled'::TRIGGER_MODE,
DROP COLUMN enabled;

ALTER TABLE websocket_trigger
ADD COLUMN mode TRIGGER_MODE;

UPDATE websocket_trigger SET mode = CASE WHEN enabled = true THEN 'enabled'::TRIGGER_MODE ELSE 'disabled'::TRIGGER_MODE END;

ALTER TABLE websocket_trigger
ALTER COLUMN mode SET NOT NULL,
ALTER COLUMN mode SET DEFAULT 'enabled'::TRIGGER_MODE,
DROP COLUMN enabled;

ALTER TABLE http_trigger
ADD COLUMN mode TRIGGER_MODE;

UPDATE http_trigger SET mode = CASE WHEN enabled = true THEN 'enabled'::TRIGGER_MODE ELSE 'disabled'::TRIGGER_MODE END;

ALTER TABLE http_trigger
ALTER COLUMN mode SET NOT NULL,
ALTER COLUMN mode SET DEFAULT 'enabled'::TRIGGER_MODE,
DROP COLUMN enabled;

ALTER TABLE email_trigger
ADD COLUMN mode TRIGGER_MODE;

UPDATE email_trigger SET mode = CASE WHEN enabled = true THEN 'enabled'::TRIGGER_MODE ELSE 'disabled'::TRIGGER_MODE END;

ALTER TABLE email_trigger
ALTER COLUMN mode SET NOT NULL,
ALTER COLUMN mode SET DEFAULT 'enabled'::TRIGGER_MODE,
DROP COLUMN enabled;

