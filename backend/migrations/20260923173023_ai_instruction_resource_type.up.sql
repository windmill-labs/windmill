-- Folder instructions for the AI chat: a markdown file resource whose
-- `value.content` applies to every item under the directory it lives in.
-- `format_extension` makes the resource editor render it as a plain .md file.
-- Seeded under 'admins' so every workspace sees it before the next hub sync.
INSERT INTO resource_type (workspace_id, name, schema, description, created_by, format_extension, edited_at)
VALUES (
    'admins',
    'ai_instruction',
    '{"type": "object", "properties": {"content": {"type": "string"}}}',
    'Instructions for the AI chat that apply to every item under the folder this resource lives in, like an AGENTS.md: an ai_instruction at f/billing/AGENTS is given to the assistant whenever it works on something under f/billing/. The file body is the instructions it follows.',
    'system',
    'md',
    now()
)
ON CONFLICT (workspace_id, name) DO NOTHING;
