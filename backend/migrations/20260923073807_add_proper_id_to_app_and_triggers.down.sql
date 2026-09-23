DO
$do$
  DECLARE
    i text;
    arr text[] := array['app', 'http_trigger', 'websocket_trigger', 'kafka_trigger',
      'sqs_trigger', 'gcp_trigger', 'azure_trigger', 'nats_trigger', 'postgres_trigger',
      'mqtt_trigger', 'amqp_trigger', 'email_trigger'];
  BEGIN
  FOREACH i IN ARRAY arr
  LOOP
    EXECUTE FORMAT('ALTER TABLE %I DROP CONSTRAINT IF EXISTS proper_id', i);
  END LOOP;
  END
$do$;
