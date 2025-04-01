CREATE TYPE AUTHENTICATION_METHOD AS ENUM (
    'none',
    'windmill',
    'api_key',
    'basic_http',
    'custom_script',
    'signature'
);

ALTER TABLE http_trigger
    RENAME COLUMN requires_auth TO authentication_method;

ALTER TABLE http_trigger
    ADD COLUMN authentication_resource_path VARCHAR(255) DEFAULT NULL,
    ALTER COLUMN authentication_method DROP DEFAULT,
    ALTER COLUMN authentication_method TYPE AUTHENTICATION_METHOD
    USING CASE
        WHEN authentication_method = true THEN 'windmill'::AUTHENTICATION_METHOD
        ELSE 'none'::AUTHENTICATION_METHOD
    END,
    ALTER COLUMN authentication_method SET NOT NULL,
    ALTER COLUMN authentication_method SET DEFAULT 'none'::AUTHENTICATION_METHOD;
