ALTER TABLE v2_job_queue
DROP COLUMN cache_ignore_s3_path;

ALTER TABLE script
DROP COLUMN cache_ignore_s3_path;
