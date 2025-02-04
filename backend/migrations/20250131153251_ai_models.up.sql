ALTER TABLE workspace_settings 
    ADD COLUMN ai_models varchar(255)[] DEFAULT '{}' NOT NULL,
    ADD COLUMN code_completion_model varchar(255);

UPDATE workspace_settings
SET ai_models = CASE
    WHEN ai_resource->>'provider' = 'openai' THEN ARRAY['gpt-4o']
    WHEN ai_resource->>'provider' = 'anthropic' THEN ARRAY['claude-3-5-sonnet-latest']
    WHEN ai_resource->>'provider' = 'mistral' THEN ARRAY['codestral-latest']
    ELSE ai_models
END
WHERE ai_resource->>'path' IS NOT NULL;

UPDATE workspace_settings
SET code_completion_model = CASE
    WHEN ai_resource->>'provider' = 'openai' THEN 'gpt-4o'
    WHEN ai_resource->>'provider' = 'anthropic' THEN 'claude-3-5-sonnet-latest'
    WHEN ai_resource->>'provider' = 'mistral' THEN 'codestral-latest'
    ELSE code_completion_model
END
WHERE code_completion_enabled IS TRUE;


ALTER TABLE workspace_settings DROP COLUMN code_completion_enabled;