-- Add up migration script here
DROP POLICY see_own ON audit;
DROP POLICY schedule ON audit;

ALTER TABLE audit ALTER COLUMN username TYPE varchar(255);

CREATE POLICY see_own ON audit FOR ALL
USING (audit.username = current_setting('session.user'));
CREATE POLICY schedule ON audit FOR INSERT
WITH CHECK (audit.username LIKE 'schedule-%');

