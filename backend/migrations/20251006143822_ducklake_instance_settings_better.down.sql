INSERT INTO global_settings (name, value) VALUES (
  'ducklake_user_pg_pwd',
  (SELECT g2.value->'ducklake_user_pg_pwd' FROM global_settings g2 WHERE g2.name = 'ducklake_settings')
);

DELETE FROM global_settings WHERE name = 'ducklake_settings';