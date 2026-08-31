-- AI chat skills move from the `ai_skill` table onto ordinary resources, so they
-- gain folder ACLs, version history, workspace export and git-sync. An `ai_skill`
-- resource holds the SKILL.md body in `value.content`; its description lives in
-- the resource's own `description` column and its name is the path basename.

-- `format_extension` makes the resource editor render `value.content` as a plain
-- .md file. Seeded under 'admins' so every workspace sees it. The hub sync only
-- overwrites `schema` and `description` on conflict, so it cannot strip this.
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

-- Shared home for the skills that were workspace-wide before the move: readable
-- by everyone, writable by admins, matching the admin-only upload they had.
INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms)
SELECT DISTINCT workspace_id, 'skills', 'Skills', ARRAY[]::TEXT[], '{"g/all": false}'::jsonb
FROM ai_skill
ON CONFLICT DO NOTHING;

-- `f/skills/<name>` may already be taken by an unrelated resource, and the table
-- is dropped below, so a skipped row would be gone for good. Each skill is placed
-- on the first free path instead of a single fixed alternative, which a second
-- collision would have silently swallowed: a skill under a surprising name is
-- recoverable, a dropped one is not.
DO $$
DECLARE
    s RECORD;
    candidate TEXT;
    suffix INT;
BEGIN
    FOR s IN SELECT * FROM ai_skill LOOP
        candidate := 'f/skills/' || s.name;
        suffix := 0;
        WHILE EXISTS (
            SELECT 1 FROM resource r WHERE r.workspace_id = s.workspace_id AND r.path = candidate
        ) LOOP
            suffix := suffix + 1;
            candidate := 'f/skills/' || s.name || '_migrated' ||
                CASE WHEN suffix = 1 THEN '' ELSE '_' || suffix::TEXT END;
        END LOOP;
        INSERT INTO resource (workspace_id, path, value, description, resource_type, created_by, edited_at)
        VALUES (
            s.workspace_id,
            candidate,
            jsonb_build_object('content', s.instructions),
            s.description,
            'ai_skill',
            s.edited_by,
            s.edited_at
        );
    END LOOP;
END $$;

DROP TABLE ai_skill;
