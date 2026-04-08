-- Move alert config from the generic config table to global_settings
-- where it belongs alongside other instance-level settings.
INSERT INTO global_settings (name, value)
SELECT 'alert_job_queue_waiting', config FROM config WHERE name = 'alert__job_queue_waiting'
ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value;

DELETE FROM config WHERE name = 'alert__job_queue_waiting';
