-- Add down migration script here
-- Add error handler email fields to http_trigger
ALTER TABLE http_trigger 
    DROP COLUMN IF EXISTS email_recipients;

-- Add error handler email fields to nats_trigger
ALTER TABLE nats_trigger 
    DROP COLUMN IF EXISTS email_recipients;

-- Add error handler email fields to postgres_trigger
ALTER TABLE postgres_trigger 
    DROP COLUMN IF EXISTS email_recipients;

-- Add error handler email fields to mqtt_trigger
ALTER TABLE mqtt_trigger 
    DROP COLUMN IF EXISTS email_recipients;

-- Add error handler email fields to kafka_trigger
ALTER TABLE kafka_trigger 
    DROP COLUMN IF EXISTS email_recipients;

-- Add error handler email fields to websocket_trigger
ALTER TABLE websocket_trigger 
    DROP COLUMN IF EXISTS email_recipients;

-- Add error handler email fields to gcp_trigger
ALTER TABLE gcp_trigger 
    DROP COLUMN IF EXISTS email_recipients;

-- Add error handler email fields to sqs_trigger
ALTER TABLE sqs_trigger 
    DROP COLUMN IF EXISTS email_recipients;