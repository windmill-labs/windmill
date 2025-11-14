-- Add up migration script here
ALTER TABLE gcp_trigger 
ADD COLUMN active_mode BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE http_trigger 
ADD COLUMN active_mode BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE kafka_trigger 
ADD COLUMN active_mode BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE mqtt_trigger 
ADD COLUMN active_mode BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE nats_trigger 
ADD COLUMN active_mode BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE postgres_trigger 
ADD COLUMN active_mode BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE sqs_trigger 
ADD COLUMN active_mode BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE websocket_trigger 
ADD COLUMN active_mode BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE email_trigger 
ADD COLUMN active_mode BOOLEAN NOT NULL DEFAULT TRUE;

CREATE INDEX idx_gcp_trigger_active_mode ON gcp_trigger(workspace_id,path,active_mode);
CREATE INDEX idx_http_trigger_active_mode ON http_trigger(workspace_id,path,active_mode);
CREATE INDEX idx_kafka_trigger_active_mode ON kafka_trigger(workspace_id,path,active_mode);
CREATE INDEX idx_mqtt_trigger_active_mode ON mqtt_trigger(workspace_id,path,active_mode);
CREATE INDEX idx_nats_trigger_active_mode ON nats_trigger(workspace_id,path,active_mode);
CREATE INDEX idx_postgres_trigger_active_mode ON postgres_trigger(workspace_id,path,active_mode);
CREATE INDEX idx_sqs_trigger_active_mode ON sqs_trigger(workspace_id,path,active_mode);
CREATE INDEX idx_websocket_trigger_active_mode ON websocket_trigger(workspace_id,path,active_mode);
CREATE INDEX idx_email_trigger_active_mode ON email_trigger(workspace_id,path,active_mode);