-- Remove error handler and retry fields from http_trigger
ALTER TABLE http_trigger 
    DROP COLUMN error_handler_path,
    DROP COLUMN error_handler_args,
    DROP COLUMN retry;

-- Remove error handler and retry fields from nats_trigger
ALTER TABLE nats_trigger 
    DROP COLUMN error_handler_path,
    DROP COLUMN error_handler_args,
    DROP COLUMN retry;

-- Remove error handler and retry fields from postgres_trigger
ALTER TABLE postgres_trigger 
    DROP COLUMN error_handler_path,
    DROP COLUMN error_handler_args,
    DROP COLUMN retry;

-- Remove error handler and retry fields from mqtt_trigger
ALTER TABLE mqtt_trigger 
    DROP COLUMN error_handler_path,
    DROP COLUMN error_handler_args,
    DROP COLUMN retry;

-- Remove error handler and retry fields from kafka_trigger
ALTER TABLE kafka_trigger 
    DROP COLUMN error_handler_path,
    DROP COLUMN error_handler_args,
    DROP COLUMN retry;

-- Remove error handler and retry fields from websocket_trigger
ALTER TABLE websocket_trigger 
    DROP COLUMN error_handler_path,
    DROP COLUMN error_handler_args,
    DROP COLUMN retry;

-- Remove error handler and retry fields from gcp_trigger
ALTER TABLE gcp_trigger 
    DROP COLUMN error_handler_path,
    DROP COLUMN error_handler_args,
    DROP COLUMN retry;

-- Remove error handler and retry fields from sqs_trigger
ALTER TABLE sqs_trigger 
    DROP COLUMN error_handler_path,
    DROP COLUMN error_handler_args,
    DROP COLUMN retry;