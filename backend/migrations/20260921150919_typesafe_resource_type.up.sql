-- The credential an AI agent step with decision output runs TypeSafe's Jev with. Seeded under
-- 'admins' so every workspace sees it, in the shape the hub gives the other AI providers, which
-- the step reads as `api_key` and `base_url`. A hub sync that later publishes the same name
-- updates the schema in place.
INSERT INTO resource_type (workspace_id, name, schema, description, created_by, edited_at)
VALUES (
    'admins',
    'typesafe',
    '{
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "order": ["api_key", "base_url"],
        "properties": {
            "api_key": {
                "type": "string",
                "description": "API key from the TypeSafe dashboard.",
                "password": true,
                "nullable": false,
                "default": ""
            },
            "base_url": {
                "type": "string",
                "description": "Optional. Only needed to overwrite the default base URL.",
                "default": ""
            }
        },
        "required": ["api_key"]
    }',
    'API key for TypeSafe''s Jev decision model, which answers typed questions with calibrated probabilities. Used by AI agent steps with decision output.',
    'system',
    now()
)
ON CONFLICT (workspace_id, name) DO NOTHING;
