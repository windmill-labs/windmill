DROP POLICY IF EXISTS insert_any ON audit_partitioned;
CREATE POLICY schedule ON audit_partitioned FOR INSERT TO windmill_user
    WITH CHECK ((username)::text ~~ 'schedule-%'::text);
CREATE POLICY schedule_audit ON audit_partitioned FOR INSERT TO windmill_user
    WITH CHECK ((parameters ->> 'end_user'::text) ~~ 'schedule-%'::text);
CREATE POLICY webhook ON audit_partitioned FOR INSERT TO windmill_user
    WITH CHECK ((username)::text ~~ 'webhook-%'::text);
