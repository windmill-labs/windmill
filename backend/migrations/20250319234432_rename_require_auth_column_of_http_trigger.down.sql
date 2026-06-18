ALTER TABLE http_trigger
    DROP COLUMN authentication_resource_path,
    ALTER COLUMN authentication_method DROP DEFAULT,
    ALTER COLUMN authentication_method TYPE boolean
    USING CASE
        WHEN authentication_method = 'windmill'::AUTHENTICATION_METHOD THEN true
        ELSE false
    END,
    ALTER COLUMN authentication_method SET NOT NULL,
    ALTER COLUMN authentication_method SET DEFAULT false;

ALTER TABLE http_trigger
    RENAME COLUMN authentication_method TO requires_auth;

DROP TYPE AUTHENTICATION_METHOD;