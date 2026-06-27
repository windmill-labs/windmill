-- Built-in `ai_agent` resource type backing reusable AI agent steps.
-- A resource of this type stores an agent's brain (provider/model/system prompt/etc.),
-- its tool set, and its eval suite. Flow steps link to it via FlowModuleValue::AIAgent.agent.
--
-- Seeded into the `admins` workspace: list_resource_types unions `workspace_id = 'admins'`,
-- so this single row is visible from every workspace (existing and future), mirroring how
-- hub-synced built-in types (e.g. s3object) are made globally available.
INSERT INTO resource_type (workspace_id, name, schema, description, edited_at) VALUES
    ('admins', 'ai_agent', '{
        "type": "object",
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "required": ["provider"],
        "properties": {
            "provider": {
                "type": "object",
                "format": "ai-provider",
                "description": "AI provider + model + credentials resource for the agent."
            },
            "system_prompt": { "type": "string", "description": "System prompt for the agent." },
            "temperature": { "type": "number", "description": "Sampling temperature (0.0-2.0)." },
            "max_completion_tokens": { "type": "number", "description": "Maximum output tokens." },
            "max_iterations": { "type": "number", "description": "Max reasoning/tool-use loops." },
            "output_type": { "type": "string", "enum": ["text", "image"], "default": "text" },
            "output_schema": { "type": "object", "format": "json-schema", "description": "Structured-output JSON schema." },
            "streaming": { "type": "boolean" },
            "memory": { "type": "object", "description": "Conversation memory config (off/auto/manual)." },
            "tools": { "type": "array", "description": "Reusable tool definitions available to the agent." },
            "evals": {
                "type": "object",
                "description": "Eval suite: cases graded by deterministic assertions and/or an LLM judge.",
                "properties": {
                    "cases": { "type": "array" },
                    "judge": { "type": "object" }
                }
            }
        }
    }'::jsonb,
    'A reusable AI agent: provider/model, system prompt, tools and an eval suite. Referenced by AI agent flow steps.',
    now())
ON CONFLICT (workspace_id, name) DO UPDATE
    SET schema = EXCLUDED.schema, description = EXCLUDED.description, edited_at = now();
