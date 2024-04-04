-- Add up migration script here
CREATE POLICY "webhook" ON audit FOR INSERT
TO windmill_user
WITH CHECK (((username)::text ~~ 'webhook-%'::text))