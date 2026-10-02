-- The credential an AI decision step runs Cloudflare's Clef with, in the shape the hub already
-- publishes for `cloudflare`, which the step reads as `token` and `account_id`. Seeded under
-- 'admins' for instances that never synced it from the hub; one that did keeps its own row.
INSERT INTO resource_type (workspace_id, name, schema, description, created_by, edited_at)
VALUES (
    'admins',
    'cloudflare',
    '{
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "order": ["token", "account_id", "email", "key"],
        "properties": {
            "token": {"type": "string", "description": "", "default": ""},
            "account_id": {"type": "string", "description": "", "default": ""},
            "email": {"type": "string", "description": "deprecated", "default": ""},
            "key": {"type": "string", "description": "deprecated", "default": ""}
        },
        "required": ["token"]
    }',
    'Credentials for the Cloudflare API using an API token, with the account ID.',
    'system',
    now()
)
ON CONFLICT (workspace_id, name) DO NOTHING;
