-- Add down migration script here
DROP INDEX IF EXISTS idx_websocket_trigger_active_mode;
DROP INDEX IF EXISTS idx_sqs_trigger_active_mode;
DROP INDEX IF EXISTS idx_postgres_trigger_active_mode;
DROP INDEX IF EXISTS idx_nats_trigger_active_mode;
DROP INDEX IF EXISTS idx_mqtt_trigger_active_mode;
DROP INDEX IF EXISTS idx_kafka_trigger_active_mode;
DROP INDEX IF EXISTS idx_http_trigger_active_mode;
DROP INDEX IF EXISTS idx_gcp_trigger_active_mode;

ALTER TABLE websocket_trigger 
DROP COLUMN IF EXISTS active_mode;

ALTER TABLE sqs_trigger 
DROP COLUMN IF EXISTS active_mode;

ALTER TABLE postgres_trigger 
DROP COLUMN IF EXISTS active_mode;

ALTER TABLE nats_trigger 
DROP COLUMN IF EXISTS active_mode;

ALTER TABLE mqtt_trigger 
DROP COLUMN IF EXISTS active_mode;

ALTER TABLE kafka_trigger 
DROP COLUMN IF EXISTS active_mode;

ALTER TABLE http_trigger 
DROP COLUMN IF EXISTS active_mode;

ALTER TABLE gcp_trigger 
DROP COLUMN IF EXISTS active_mode;