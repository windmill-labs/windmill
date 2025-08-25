-- Add up migration script here
DROP POLICY see_own ON audit;
CREATE POLICY see_own ON audit FOR all 
      USING (((username)::text = current_setting('session.user'::text, true)));