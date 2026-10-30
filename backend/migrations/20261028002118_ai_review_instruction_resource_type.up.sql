-- Review instructions for the AI review of fork deployment requests: a markdown
-- file resource whose `value.content` applies to every changed item under the
-- directory it lives in. Read from the parent workspace, the one the changes merge into.
-- Seeded under 'admins' so every workspace sees it before the next hub sync.
INSERT INTO resource_type (workspace_id, name, schema, description, created_by, format_extension, edited_at)
VALUES (
    'admins',
    'ai_review_instruction',
    '{"type": "object", "properties": {"content": {"type": "string"}}}',
    'Instructions for the AI review of deployment requests that apply to every changed item under the folder this resource lives in: an ai_review_instruction at f/billing/REVIEW is given to the reviewer whenever a fork''s changes touch something under f/billing/. It is read from the workspace the changes are merged into. The file body is the instructions the review follows.',
    'system',
    'md',
    now()
)
ON CONFLICT (workspace_id, name) DO NOTHING;
