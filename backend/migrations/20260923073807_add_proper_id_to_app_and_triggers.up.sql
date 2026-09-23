-- A CHECK constraint is evaluated on every UPDATE of a row, not only on writes
-- to `path`, so adding one over a row that violates it would break listener
-- heartbeats and app redeploys for that row. A table already holding such a
-- row is skipped; create and rename still validate the path in the API.
DO
$do$
  DECLARE
    i text;
    bad boolean;
    arr text[] := array['app', 'http_trigger', 'websocket_trigger', 'kafka_trigger',
      'sqs_trigger', 'gcp_trigger', 'azure_trigger', 'nats_trigger', 'postgres_trigger',
      'mqtt_trigger', 'amqp_trigger', 'email_trigger'];
  BEGIN
  FOREACH i IN ARRAY arr
  LOOP
    EXECUTE FORMAT($$SELECT EXISTS (SELECT 1 FROM %I WHERE path !~ '^[ufg](\/[\w-]+){2,}$')$$, i)
      INTO bad;
    IF bad THEN
      RAISE WARNING 'not adding proper_id to %: it holds paths outside u/<user>/<name>, f/<folder>/<name>, g/<group>/<name>', i;
    ELSE
      EXECUTE FORMAT(
        $$
          ALTER TABLE %1$I DROP CONSTRAINT IF EXISTS proper_id;
          ALTER TABLE %1$I ADD CONSTRAINT proper_id CHECK (path ~ '^[ufg](\/[\w-]+){2,}$');
        $$,
        i
      );
    END IF;
  END LOOP;
  END
$do$;
