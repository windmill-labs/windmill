-- Workspace-scoped AI chat skills (Claude/Codex-style SKILL.md instructions).
-- `name` is the skill folder slug; `description` is advertised in the AI chat
-- system prompt, `instructions` is the SKILL.md body fetched on demand.
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
