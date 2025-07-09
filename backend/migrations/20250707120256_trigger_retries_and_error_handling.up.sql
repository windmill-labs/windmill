-- Add error handler and retry fields to http_trigger
ALTER TABLE http_trigger 
    ADD COLUMN error_handler_path VARCHAR(255) NULL,
    ADD COLUMN error_handler_args JSONB NULL,
    ADD COLUMN retry JSONB NULL;

-- Add error handler and retry fields to nats_trigger
ALTER TABLE nats_trigger 
    ADD COLUMN error_handler_path VARCHAR(255) NULL,
    ADD COLUMN error_handler_args JSONB NULL,
    ADD COLUMN retry JSONB NULL;

-- Add error handler and retry fields to postgres_trigger
ALTER TABLE postgres_trigger 
    ADD COLUMN error_handler_path VARCHAR(255) NULL,
    ADD COLUMN error_handler_args JSONB NULL,
    ADD COLUMN retry JSONB NULL;

-- Add error handler and retry fields to mqtt_trigger
ALTER TABLE mqtt_trigger 
    ADD COLUMN error_handler_path VARCHAR(255) NULL,
    ADD COLUMN error_handler_args JSONB NULL,
    ADD COLUMN retry JSONB NULL;

-- Add error handler and retry fields to kafka_trigger
ALTER TABLE kafka_trigger 
    ADD COLUMN error_handler_path VARCHAR(255) NULL,
    ADD COLUMN error_handler_args JSONB NULL,
    ADD COLUMN retry JSONB NULL;

-- Add error handler and retry fields to websocket_trigger
ALTER TABLE websocket_trigger 
    ADD COLUMN error_handler_path VARCHAR(255) NULL,
    ADD COLUMN error_handler_args JSONB NULL,
    ADD COLUMN retry JSONB NULL;

-- Add error handler and retry fields to gcp_trigger
ALTER TABLE gcp_trigger 
    ADD COLUMN error_handler_path VARCHAR(255) NULL,
    ADD COLUMN error_handler_args JSONB NULL,
    ADD COLUMN retry JSONB NULL;

-- Add error handler and retry fields to sqs_trigger
ALTER TABLE sqs_trigger 
    ADD COLUMN error_handler_path VARCHAR(255) NULL,
    ADD COLUMN error_handler_args JSONB NULL,
    ADD COLUMN retry JSONB NULL;