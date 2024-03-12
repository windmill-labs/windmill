INSERT INTO
  global_settings (name, value)
SELECT 'automate_username_creation', 'true'
WHERE NOT EXISTS (SELECT 1 FROM workspace WHERE id != 'admins')
ON CONFLICT DO NOTHING;
