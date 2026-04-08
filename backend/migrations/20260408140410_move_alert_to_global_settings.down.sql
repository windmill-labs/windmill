-- Move alert config back from global_settings to the config table.
INSERT INTO config (name, config)
SELECT 'alert__job_queue_waiting', value FROM global_settings WHERE name = 'alert_job_queue_waiting'
ON CONFLICT (name) DO UPDATE SET config = EXCLUDED.config;

DELETE FROM global_settings WHERE name = 'alert_job_queue_waiting';
