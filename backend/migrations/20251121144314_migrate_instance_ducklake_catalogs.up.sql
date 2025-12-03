-- Superadmins have the ability to create databases in the Windmill Postgres instance
-- for use as Ducklake catalogs or Data tables. These databases can be accessed by
-- the 'custom_instance_user'. The setting below stores the password and logs
-- about the creation status of these databases.

DO $$
DECLARE
    new_settings_value text;
    old_setting_value text;
BEGIN
    ALTER ROLE ducklake_user RENAME TO custom_instance_user;
EXCEPTION
    WHEN others THEN
    RAISE NOTICE 'ducklake_user migration error, skipping.';
END
$$;

-- Rename to more generic names
UPDATE global_settings
SET name = 'custom_instance_pg_databases',
    value = jsonb_build_object(
        'user_pwd', value->'ducklake_user_pg_pwd',
        'databases', value->'instance_catalog_db_status'
    )
WHERE name = 'ducklake_settings';

-- Add ducklake tag to existing databases
UPDATE global_settings
SET value = jsonb_build_object(
    'databases', (
      SELECT COALESCE(
        jsonb_object_agg(key, value || jsonb_build_object('tag', 'ducklake')),
        '{}'::jsonb
      )
      FROM jsonb_each(value->'databases')
    ),
    'user_pwd', value->'user_pwd'
)
WHERE name = 'custom_instance_pg_databases';
