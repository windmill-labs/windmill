INSERT INTO group_ (workspace_id, name, summary, extra_perms)
SELECT id, 'wm_deployers', 'Members can preserve the original author when deploying to this workspace', '{}'::jsonb
FROM workspace
WHERE NOT deleted
ON CONFLICT (workspace_id, name) DO UPDATE SET summary = EXCLUDED.summary;
