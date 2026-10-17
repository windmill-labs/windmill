DROP TRIGGER IF EXISTS role_overrides_change_trigger ON workspace_settings;
DROP FUNCTION IF EXISTS notify_role_overrides_change();
DROP TABLE IF EXISTS role_seat;
ALTER TABLE workspace_settings DROP COLUMN IF EXISTS role_overrides;
ALTER TABLE workspace_invite DROP COLUMN IF EXISTS workspace_role;
ALTER TABLE usr DROP COLUMN IF EXISTS workspace_role;
