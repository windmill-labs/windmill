-- Add down migration script here
DROP INDEX IF EXISTS idx_websocket_trigger_suspend_number;
DROP INDEX IF EXISTS idx_sqs_trigger_suspend_number;
DROP INDEX IF EXISTS idx_postgres_trigger_suspend_number;
DROP INDEX IF EXISTS idx_nats_trigger_suspend_number;
DROP INDEX IF EXISTS idx_mqtt_trigger_suspend_number;
DROP INDEX IF EXISTS idx_kafka_trigger_suspend_number;
DROP INDEX IF EXISTS idx_http_trigger_suspend_number;
DROP INDEX IF EXISTS idx_gcp_trigger_suspend_number;

ALTER TABLE websocket_trigger 
DROP COLUMN IF EXISTS suspend_number;

ALTER TABLE sqs_trigger 
DROP COLUMN IF EXISTS suspend_number;

ALTER TABLE postgres_trigger 
DROP COLUMN IF EXISTS suspend_number;

ALTER TABLE nats_trigger 
DROP COLUMN IF EXISTS suspend_number;

ALTER TABLE mqtt_trigger 
DROP COLUMN IF EXISTS suspend_number;

ALTER TABLE kafka_trigger 
DROP COLUMN IF EXISTS suspend_number;

ALTER TABLE http_trigger 
DROP COLUMN IF EXISTS suspend_number;

ALTER TABLE gcp_trigger 
DROP COLUMN IF EXISTS suspend_number;