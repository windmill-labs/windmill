-- Add up migration script here
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms) SELECT id, 'app_groups', 'App Groups', ARRAY[]::TEXT[], '{"g/all": false}' FROM workspace ON CONFLICT DO NOTHING;
