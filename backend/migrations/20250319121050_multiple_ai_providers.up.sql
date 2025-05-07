UPDATE workspace_settings
SET ai_resource = CASE
    WHEN ai_resource IS NULL OR ai_resource->>'path' IS NULL OR ai_resource->>'provider' IS NULL THEN NULL
    ELSE jsonb_build_object(
        'providers', jsonb_build_object(
            ai_resource->>'provider',
            jsonb_build_object(
                'resource_path', ai_resource->>'path',
                'models', to_jsonb(ai_models)
            )
        ),
        'default_model',
            CASE
                WHEN array_length(ai_models, 1) > 0 THEN jsonb_build_object(
                    'model', ai_models[1],
                    'provider', ai_resource->>'provider'
                )
                ELSE NULL
            END,
        'code_completion_model',
            CASE
                WHEN code_completion_model IS NULL THEN NULL
                ELSE jsonb_build_object(
                    'model', code_completion_model,
                    'provider', ai_resource->>'provider'
                )
            END
    )
END;

ALTER TABLE workspace_settings
DROP COLUMN code_completion_model,
DROP COLUMN ai_models;

ALTER TABLE workspace_settings RENAME COLUMN ai_resource TO ai_config;


-- { providers: { [provider]: { resource_path: resource_path, models: ai_models}, default_model: ai_models[0], code_completion_model: code_completion_model}
