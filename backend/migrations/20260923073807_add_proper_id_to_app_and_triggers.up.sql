-- NOT VALID: rows written before this constraint existed are left alone so an
-- instance holding one still migrates; every new or updated row is checked.
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
    EXECUTE FORMAT(
      $$
        ALTER TABLE %1$I DROP CONSTRAINT IF EXISTS proper_id;
        ALTER TABLE %1$I ADD CONSTRAINT proper_id CHECK (path ~ '^[ufg](\/[\w-]+){2,}$') NOT VALID;
      $$,
      i
    );
  END LOOP;
  END
$do$;
