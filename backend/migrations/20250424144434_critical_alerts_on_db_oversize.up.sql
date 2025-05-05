INSERT INTO global_settings (name, value) 
VALUES ('critical_alerts_on_db_oversize', '{}') 
ON CONFLICT (name) DO NOTHING;
