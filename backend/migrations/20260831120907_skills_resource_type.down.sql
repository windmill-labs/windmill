CREATE TABLE ai_skill (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    instructions TEXT NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    edited_by VARCHAR(255) NOT NULL DEFAULT '',
    PRIMARY KEY (workspace_id, name)
);

GRANT ALL ON ai_skill TO windmill_user;
GRANT ALL ON ai_skill TO windmill_admin;

-- Only the folder the up migration created comes back, and only those resources
-- are removed: a skill authored anywhere else has no name the old flat table can
-- hold, so it is left in place (orphaned by the type removal below) rather than
-- destroyed. The folder name is matched rather than fixed because up sidesteps a
-- pre-existing `skills` folder rather than adopting its ACL.
INSERT INTO ai_skill (workspace_id, name, description, instructions, edited_at, edited_by)
SELECT
    workspace_id,
    substring(path FROM '^f/[^/]+/(.*)$'),
    coalesce(description, ''),
    coalesce(value->>'content', ''),
    coalesce(edited_at, now()),
    coalesce(created_by, '')
FROM resource
WHERE resource_type = 'ai_skill' AND path ~ '^f/(skills|ai_skills(_[0-9]+)?)/'
ON CONFLICT DO NOTHING;

DELETE FROM resource WHERE resource_type = 'ai_skill' AND path ~ '^f/(skills|ai_skills(_[0-9]+)?)/';
DELETE FROM resource_type WHERE workspace_id = 'admins' AND name = 'ai_skill';
