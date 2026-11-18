-- A trigger's target is compared with a renamed runnable's old path and with the path a runnable
-- lists its triggers by, neither of which starts with `/`. A target with more than one leading `/`
-- names nothing and is left as is: firing the trigger refuses it.
UPDATE http_trigger SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
UPDATE email_trigger SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
UPDATE websocket_trigger SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
UPDATE kafka_trigger SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
UPDATE postgres_trigger SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
UPDATE mqtt_trigger SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
UPDATE nats_trigger SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
UPDATE sqs_trigger SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
UPDATE amqp_trigger SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
UPDATE gcp_trigger SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
UPDATE azure_trigger SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
-- A restore puts the trashed row back as saved, so the copies awaiting one get the same fix.
UPDATE trashbin
SET item_data = jsonb_set(item_data, '{row,script_path}',
    to_jsonb(substr(item_data #>> '{row,script_path}', 2)))
WHERE item_data ->> 'table_name' IN ('http_trigger', 'email_trigger', 'websocket_trigger',
        'kafka_trigger', 'postgres_trigger', 'mqtt_trigger', 'nats_trigger', 'sqs_trigger',
        'amqp_trigger', 'gcp_trigger', 'azure_trigger')
    AND item_data #>> '{row,script_path}' LIKE '/%'
    AND item_data #>> '{row,script_path}' NOT LIKE '//%';
