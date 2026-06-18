ALTER TABLE v2_job_queue
DROP COLUMN runnable_settings_handle;

ALTER TABLE script
DROP COLUMN runnable_settings_handle;

DROP TABLE IF EXISTS job_settings;
DROP TABLE IF EXISTS runnable_settings;
DROP TABLE IF EXISTS concurrency_settings;
DROP TABLE IF EXISTS debouncing_settings;

