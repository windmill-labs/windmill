-- Add down migration script here
-- Revert mode column back to enabled boolean for each trigger table
ALTER TABLE email_trigger
ADD COLUMN enabled BOOLEAN;

UPDATE email_trigger SET enabled = CASE WHEN mode = 'enabled'::TRIGGER_MODE THEN true ELSE false END;

ALTER TABLE email_trigger
ALTER COLUMN enabled SET NOT NULL,
ALTER COLUMN enabled SET DEFAULT true,
DROP COLUMN mode;

ALTER TABLE http_trigger
ADD COLUMN enabled BOOLEAN;

UPDATE http_trigger SET enabled = CASE WHEN mode = 'enabled'::TRIGGER_MODE THEN true ELSE false END;

ALTER TABLE http_trigger
ALTER COLUMN enabled SET NOT NULL,
ALTER COLUMN enabled SET DEFAULT true,
DROP COLUMN mode;

ALTER TABLE websocket_trigger
ADD COLUMN enabled BOOLEAN;

UPDATE websocket_trigger SET enabled = CASE WHEN mode = 'enabled'::TRIGGER_MODE THEN true ELSE false END;

ALTER TABLE websocket_trigger
ALTER COLUMN enabled SET NOT NULL,
ALTER COLUMN enabled SET DEFAULT true,
DROP COLUMN mode;

ALTER TABLE sqs_trigger
ADD COLUMN enabled BOOLEAN;

UPDATE sqs_trigger SET enabled = CASE WHEN mode = 'enabled'::TRIGGER_MODE THEN true ELSE false END;

ALTER TABLE sqs_trigger
ALTER COLUMN enabled SET NOT NULL,
ALTER COLUMN enabled SET DEFAULT true,
DROP COLUMN mode;

ALTER TABLE postgres_trigger
ADD COLUMN enabled BOOLEAN;

UPDATE postgres_trigger SET enabled = CASE WHEN mode = 'enabled'::TRIGGER_MODE THEN true ELSE false END;

ALTER TABLE postgres_trigger
ALTER COLUMN enabled SET NOT NULL,
ALTER COLUMN enabled SET DEFAULT true,
DROP COLUMN mode;

ALTER TABLE nats_trigger
ADD COLUMN enabled BOOLEAN;

UPDATE nats_trigger SET enabled = CASE WHEN mode = 'enabled'::TRIGGER_MODE THEN true ELSE false END;

ALTER TABLE nats_trigger
ALTER COLUMN enabled SET NOT NULL,
ALTER COLUMN enabled SET DEFAULT true,
DROP COLUMN mode;

ALTER TABLE mqtt_trigger
ADD COLUMN enabled BOOLEAN;

UPDATE mqtt_trigger SET enabled = CASE WHEN mode = 'enabled'::TRIGGER_MODE THEN true ELSE false END;

ALTER TABLE mqtt_trigger
ALTER COLUMN enabled SET NOT NULL,
ALTER COLUMN enabled SET DEFAULT true,
DROP COLUMN mode;

ALTER TABLE kafka_trigger
ADD COLUMN enabled BOOLEAN;

UPDATE kafka_trigger SET enabled = CASE WHEN mode = 'enabled'::TRIGGER_MODE THEN true ELSE false END;

ALTER TABLE kafka_trigger
ALTER COLUMN enabled SET NOT NULL,
ALTER COLUMN enabled SET DEFAULT true,
DROP COLUMN mode;

ALTER TABLE gcp_trigger
ADD COLUMN enabled BOOLEAN;

UPDATE gcp_trigger SET enabled = CASE WHEN mode = 'enabled'::TRIGGER_MODE THEN true ELSE false END;

ALTER TABLE gcp_trigger
ALTER COLUMN enabled SET NOT NULL,
ALTER COLUMN enabled SET DEFAULT true,
DROP COLUMN mode;

DROP TYPE IF EXISTS TRIGGER_MODE;