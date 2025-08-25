ALTER TABLE workspace_settings RENAME COLUMN ai_config TO ai_resource;

ALTER TABLE workspace_settings
ADD COLUMN ai_models VARCHAR(255)[] NOT NULL DEFAULT '{}',
ADD COLUMN code_completion_model VARCHAR(255);

UPDATE workspace_settings
SET ai_resource = CASE
    WHEN ai_resource IS NULL THEN NULL
    ELSE jsonb_build_object(
        'provider',
            COALESCE((SELECT jsonb_object_keys(COALESCE(ai_resource->>'providers', '{}')::jsonb) LIMIT 1), 'openai'), -- Get the first provider key
        'path',
            ai_resource->'providers'->(SELECT jsonb_object_keys(COALESCE(ai_resource->>'providers', '{}')::jsonb) LIMIT 1)->>'resource_path'
    )
END,
ai_models = COALESCE((
    SELECT array_agg(model)
    FROM jsonb_array_elements_text(
        COALESCE(ai_resource->'providers'->(SELECT jsonb_object_keys(COALESCE(ai_resource->>'providers', '{}')::jsonb) LIMIT 1)->>'models', '[]')::jsonb
    ) model
    WHERE model IS NOT NULL
), '{}'),
code_completion_model = ai_resource->'code_completion_model'->>'model';
