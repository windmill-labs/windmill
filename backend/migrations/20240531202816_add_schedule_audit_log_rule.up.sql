-- Add up migration script here
CREATE POLICY "schedule_audit" on audit FOR INSERT
      TO windmill_user
      WITH CHECK (((parameters->>'end_user')::text ~~ 'schedule-%'::text));