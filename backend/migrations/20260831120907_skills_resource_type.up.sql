-- AI chat skills move from the `ai_skill` table onto ordinary resources, so they
-- gain folder ACLs, version history, workspace export and git-sync. An `ai_skill`
-- resource holds the SKILL.md body in `value.content`; its description lives in
-- the resource's own `description` column and its name is the path basename.

-- `format_extension` makes the resource editor render `value.content` as a plain
-- .md file. Seeded under 'admins' so every workspace sees it; a later hub sync
-- carrying the same type keeps the extension rather than replacing the row.
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


-- Skills were writable only by admins, so they must land in a folder this
-- migration creates with that ACL. An existing `skills` folder is left alone and
-- sidestepped: adopting it would silently hand its own grants — possibly write for
-- everyone — over a set of instructions the assistant follows.
--
-- The path within that folder is chosen the same way, walking to the first free
-- one. A single fixed alternative would be silently swallowed by a second
-- collision, and the table is dropped below, so a skipped row would be gone for
-- good: a skill under a surprising name is recoverable, a dropped one is not.
DO $$
DECLARE
    ws RECORD;
    s RECORD;
    folder TEXT;
    candidate TEXT;
    suffix INT;
BEGIN
    FOR ws IN SELECT DISTINCT workspace_id FROM ai_skill LOOP
        folder := 'skills';
        suffix := 0;
        WHILE EXISTS (
            SELECT 1 FROM folder f WHERE f.workspace_id = ws.workspace_id AND f.name = folder
        ) LOOP
            suffix := suffix + 1;
            folder := 'ai_skills' || CASE WHEN suffix = 1 THEN '' ELSE '_' || suffix::TEXT END;
        END LOOP;

        INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms)
        VALUES (ws.workspace_id, folder, 'Skills', ARRAY[]::TEXT[], '{"g/all": false}'::jsonb);

        FOR s IN SELECT * FROM ai_skill WHERE workspace_id = ws.workspace_id LOOP
            candidate := 'f/' || folder || '/' || s.name;
            suffix := 0;
            WHILE EXISTS (
                SELECT 1 FROM resource r
                WHERE r.workspace_id = s.workspace_id AND r.path = candidate
            ) LOOP
                suffix := suffix + 1;
                candidate := 'f/' || folder || '/' || s.name || '_migrated' ||
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
    END LOOP;
END $$;

DROP TABLE ai_skill;
