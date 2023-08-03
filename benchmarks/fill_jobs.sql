INSERT INTO workspace(id, name, owner) VALUES
	('benchmark', 'benchmark', 'admin@windmill.dev') ON CONFLICT DO NOTHING;

INSERT INTO queue (id, workspace_id, scheduled_for, created_by, suspend, suspend_until)
SELECT gen_random_uuid(),  'benchmark', now(), 'benchmark', 2, now()
FROM (SELECT generate_series(1,20) AS i) as t;


DO
$do$
BEGIN
   FOR i IN 1..100 LOOP
        INSERT INTO completed_job (id, workspace_id, started_at, created_at, duration_ms, success, created_by)
        SELECT gen_random_uuid(),  'benchmark', now(), now(), 100, true, 'benchmark'
        FROM (SELECT generate_series(1,20000) AS i) as t;
        raise notice 'i: %', i;
   END LOOP;
END
$do$;

DO
$do$
BEGIN
   FOR i IN 1..1 LOOP
        INSERT INTO completed_job (id, workspace_id, started_at, created_at, duration_ms, success, created_by, schedule_path)
        SELECT gen_random_uuid(),  'benchmark', now(), now(), 100, true, 'benchmark', 'f/benchmark/benchmark'
        FROM (SELECT generate_series(1,20000) AS i) as t;
        raise notice 'i: %', i;
   END LOOP;
END
$do$;

INSERT INTO schedule (workspace_id, path, schedule, timezone, edited_by, script_path,
is_flow, args, enabled, email, on_failure) VALUES ('benchmark', 'f/benchmark/benchmark', '*/60 0 0 0 0', 'Europe/Paris', 'benchmark', 'f/benchmark/benchmark', false, '{}', true, 'admin@windmill.dev', null)
ON CONFLICT DO NOTHING;