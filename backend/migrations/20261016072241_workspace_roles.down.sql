DROP TABLE IF EXISTS role_seat;
ALTER TABLE workspace_settings DROP COLUMN IF EXISTS role_overrides;
ALTER TABLE workspace_invite DROP COLUMN IF EXISTS workspace_role;
ALTER TABLE usr DROP COLUMN IF EXISTS workspace_role;
