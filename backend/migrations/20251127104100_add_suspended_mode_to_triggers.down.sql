-- Add down migration script here
ALTER TABLE websocket_trigger 
DROP COLUMN IF EXISTS suspended_mode;

ALTER TABLE sqs_trigger 
DROP COLUMN IF EXISTS suspended_mode;

ALTER TABLE postgres_trigger 
DROP COLUMN IF EXISTS suspended_mode;

ALTER TABLE nats_trigger 
DROP COLUMN IF EXISTS suspended_mode;

ALTER TABLE mqtt_trigger 
DROP COLUMN IF EXISTS suspended_mode;

ALTER TABLE kafka_trigger 
DROP COLUMN IF EXISTS suspended_mode;

ALTER TABLE http_trigger 
DROP COLUMN IF EXISTS suspended_mode;

ALTER TABLE gcp_trigger 
DROP COLUMN IF EXISTS suspended_mode;

ALTER TABLE email_trigger 
DROP COLUMN IF EXISTS suspended_mode;