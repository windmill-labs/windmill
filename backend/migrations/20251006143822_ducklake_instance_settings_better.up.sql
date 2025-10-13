INSERT INTO global_settings (name, value) VALUES (
  'ducklake_settings',
  (SELECT json_build_object(
    'ducklake_user_pg_pwd', g2.value,
    'instance_catalog_db_status', '{}'::json
  ) FROM global_settings g2 WHERE g2.name = 'ducklake_user_pg_pwd')
);

DELETE FROM global_settings WHERE name = 'ducklake_user_pg_pwd';