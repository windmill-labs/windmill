-- select * from global_settings where name = 'git_app_portal_secret';
-- select * from global_settings where name = 'git_installations';
-- -- delete from global_settings where name = 'git_app_portal_secret';
-- select * from global_settings where name = 'git_app_portal_secret';
SELECT jsonb_array_elements(git_sync->'repositories')
            FROM workspace_settings
            WHERE workspace_id = 'test'

-- select * from resource 