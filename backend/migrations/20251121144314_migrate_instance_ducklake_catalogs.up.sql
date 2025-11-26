-- Superadmins have the ability to create databases in the Windmill Postgres instance
-- for use as Ducklake catalogs or Data tables. These databases can be accessed by
-- the 'custom_instance_user'. The setting below stores the password and logs
-- about the creation status of these databases.

ALTER ROLE ducklake_user RENAME TO custom_instance_user;

UPDATE global_settings
SET name = 'custom_instance_pg_databases',
    value = jsonb_build_object(
        'user_pwd', value->'ducklake_user_pg_pwd',
        'status', value->'instance_catalog_db_status'
    )
WHERE name = 'ducklake_settings';

UPDATE global_settings
SET value = jsonb_build_object(
    'databases', (
        SELECT jsonb_object_agg(
            key,
            value || jsonb_build_object('tag', 'ducklake')
        )
        FROM jsonb_each(value->'status')
    ),
    'user_pwd', value->'user_pwd'
)
WHERE name = 'custom_instance_pg_databases';
