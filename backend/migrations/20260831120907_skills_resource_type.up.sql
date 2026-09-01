-- AI chat skills move from the `ai_skill` table onto ordinary resources, so they
-- gain folder ACLs, version history, workspace export and git-sync. An `ai_skill`
-- resource holds the SKILL.md body in `value.content`; its description lives in
-- the resource's own `description` column and its name is the path basename.
--
-- Nothing here is destructive. `ai_skill` is left in place, unread, for a later
-- release to drop once operators have confirmed the copy. That is what lets every
-- step below skip on conflict rather than resolve one: a skipped row is still in
-- the table, so it is not lost, and the migration needs no record of what it did
-- in order to be reversible.

-- `format_extension` makes the resource editor render `value.content` as a plain
-- .md file. Seeded under 'admins' so every workspace sees it.
INSERT INTO resource_type (workspace_id, name, schema, description, created_by, format_extension, edited_at)
VALUES (
    'admins',
    'ai_skill',
    '{"type": "object", "properties": {"content": {"type": "string"}}}',
    'A reusable instruction set for the AI chat, in the SKILL.md format. The resource description is what the assistant sees when deciding whether the skill applies; the file body is the instructions it follows.',
    'system',
    'md',
    now()
)
ON CONFLICT (workspace_id, name) DO NOTHING;

-- Shared home matching the admin-only upload these skills had. A workspace that
-- already has a `skills` folder keeps it untouched, ACL and all: adopting one
-- would hand its own grants — possibly write for everyone — over a set of
-- instructions the assistant follows.
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms)
SELECT DISTINCT workspace_id, 'skills', 'Skills', ARRAY[]::TEXT[], '{"g/all": false}'::jsonb
FROM ai_skill
ON CONFLICT (workspace_id, name) DO NOTHING;

-- Copied only where the destination is free and the folder is the restrictive one
-- above. Anything else stays in `ai_skill` for an operator to place deliberately.
INSERT INTO resource (workspace_id, path, value, description, resource_type, created_by, edited_at)
SELECT
    s.workspace_id,
    'f/skills/' || s.name,
    jsonb_build_object('content', s.instructions),
    s.description,
    'ai_skill',
    s.edited_by,
    s.edited_at
FROM ai_skill s
JOIN folder f
    ON f.workspace_id = s.workspace_id
   AND f.name = 'skills'
   AND f.extra_perms = '{"g/all": false}'::jsonb
ON CONFLICT (workspace_id, path) DO NOTHING;
