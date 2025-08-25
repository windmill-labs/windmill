-- Add up migration script here

DROP POLICY IF EXISTS see_own ON account;
DROP POLICY IF EXISTS see_member ON account;
DROP POLICY IF EXISTS see_folder_extra_perms_user on account; 
ALTER TABLE account DISABLE ROW LEVEL SECURITY;

ALTER TABLE account DROP COLUMN IF EXISTS owner;

GRANT ALL ON account TO windmill_admin;
GRANT ALL ON account TO windmill_user;
