-- Add up migration script here
-- Add error handler email fields to http_trigger
ALTER TABLE http_trigger 
    ADD COLUMN email_recipients TEXT[] DEFAULT NULL;

-- Add error handler email fields to nats_trigger
ALTER TABLE nats_trigger 
    ADD COLUMN email_recipients TEXT[] DEFAULT NULL;

-- Add error handler email fields to postgres_trigger
ALTER TABLE postgres_trigger 
    ADD COLUMN email_recipients TEXT[] DEFAULT NULL;

-- Add error handler email fields to mqtt_trigger
ALTER TABLE mqtt_trigger 
    ADD COLUMN email_recipients TEXT[] DEFAULT NULL;

-- Add error handler email fields to kafka_trigger
ALTER TABLE kafka_trigger 
    ADD COLUMN email_recipients TEXT[] DEFAULT NULL;

-- Add error handler email fields to websocket_trigger
ALTER TABLE websocket_trigger 
    ADD COLUMN email_recipients TEXT[] DEFAULT NULL;

-- Add error handler email fields to gcp_trigger
ALTER TABLE gcp_trigger 
    ADD COLUMN email_recipients TEXT[] DEFAULT NULL;

-- Add error handler email fields to sqs_trigger
ALTER TABLE sqs_trigger 
    ADD COLUMN email_recipients TEXT[] DEFAULT NULL;