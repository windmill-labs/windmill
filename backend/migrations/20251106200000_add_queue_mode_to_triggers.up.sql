-- Add up migration script here
ALTER TABLE gcp_trigger 
ADD COLUMN suspend_number INTEGER NULL;

ALTER TABLE http_trigger 
ADD COLUMN suspend_number INTEGER NULL;

ALTER TABLE kafka_trigger 
ADD COLUMN suspend_number INTEGER NULL;

ALTER TABLE mqtt_trigger 
ADD COLUMN suspend_number INTEGER NULL;

ALTER TABLE nats_trigger 
ADD COLUMN suspend_number INTEGER NULL;

ALTER TABLE postgres_trigger 
ADD COLUMN suspend_number INTEGER NULL;

ALTER TABLE sqs_trigger 
ADD COLUMN suspend_number INTEGER NULL;

ALTER TABLE websocket_trigger 
ADD COLUMN suspend_number INTEGER NULL;

ALTER TABLE email_trigger 
ADD COLUMN suspend_number INTEGER NULL;

CREATE INDEX idx_gcp_trigger_suspend_number ON gcp_trigger(workspace_id,path,suspend_number) WHERE suspend_number IS NOT NULL;
CREATE INDEX idx_http_trigger_suspend_number ON http_trigger(workspace_id,path,suspend_number) WHERE suspend_number IS NOT NULL;
CREATE INDEX idx_kafka_trigger_suspend_number ON kafka_trigger(workspace_id,path,suspend_number) WHERE suspend_number IS NOT NULL;
CREATE INDEX idx_mqtt_trigger_suspend_number ON mqtt_trigger(workspace_id,path,suspend_number) WHERE suspend_number IS NOT NULL;
CREATE INDEX idx_nats_trigger_suspend_number ON nats_trigger(workspace_id,path,suspend_number) WHERE suspend_number IS NOT NULL;
CREATE INDEX idx_postgres_trigger_suspend_number ON postgres_trigger(workspace_id,path,suspend_number) WHERE suspend_number IS NOT NULL;
CREATE INDEX idx_sqs_trigger_suspend_number ON sqs_trigger(workspace_id,path,suspend_number) WHERE suspend_number IS NOT NULL;
CREATE INDEX idx_websocket_trigger_suspend_number ON websocket_trigger(workspace_id,path,suspend_number) WHERE suspend_number IS NOT NULL;
CREATE INDEX idx_email_trigger_suspend_number ON email_trigger(workspace_id,path,suspend_number) WHERE suspend_number IS NOT NULL;