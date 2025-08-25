-- Add up migration script here
ALTER TABLE account ADD COLUMN owner VARCHAR(50) NOT NULL;
ALTER TABLE account ADD COLUMN client VARCHAR(50) NOT NULL;
ALTER TABLE resource ADD COLUMN is_oauth BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE variable ADD COLUMN is_oauth BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE resource DROP COLUMN account;

ALTER TABLE account ALTER COLUMN expires_at TYPE TIMESTAMP WITH TIME ZONE;

ALTER TABLE account ALTER COLUMN expires_at SET NOT NULL;
ALTER TABLE account ALTER COLUMN refresh_token SET NOT NULL;

ALTER TABLE account ENABLE ROW LEVEL SECURITY;


CREATE POLICY see_own ON account FOR ALL
USING (SPLIT_PART(account.owner, '/', 1) = 'u' AND SPLIT_PART(account.owner, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member ON account FOR ALL
USING (SPLIT_PART(account.owner, '/', 1) = 'g' AND SPLIT_PART(account.owner, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));
