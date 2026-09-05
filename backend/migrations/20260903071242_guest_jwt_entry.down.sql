ALTER TABLE guest_activity DROP COLUMN jwt_entry;
ALTER TABLE workspace_settings
    DROP CONSTRAINT workspace_settings_guest_jwt_one_key,
    DROP COLUMN guest_jwt_public_key,
    DROP COLUMN guest_jwt_jwks_url;
