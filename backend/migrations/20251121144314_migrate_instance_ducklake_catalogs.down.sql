-- Add up migration script here
UPDATE global_settings
SET name = 'ducklake_settings',
    value = jsonb_build_object(
        'ducklake_user_pg_pwd', value->'user_pwd',
        'instance_catalog_db_status', value->'status'
    )
WHERE name = 'custom_instance_pg_databases';

ALTER ROLE custom_instance_user RENAME TO ducklake_user;