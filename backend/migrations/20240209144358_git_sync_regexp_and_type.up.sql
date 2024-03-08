-- Add up migration script here
UPDATE workspace_settings SET git_sync = (
    CASE
        WHEN git_sync is null THEN null
        WHEN git_sync = '[]'::jsonb THEN null
        ELSE jsonb_build_object(
                'repositories', git_sync,
                'include_path', '["f/**"]'::jsonb,
                'include_type', '["script", "flow", "app", "folder"]'::jsonb

             )
    END
);
