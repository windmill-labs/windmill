-- Add down migration script here
DROP TABLE job_result_stream_v2;

ALTER TABLE job_result_stream ADD CONSTRAINT fk_job_result_stream_job_id FOREIGN KEY (job_id) REFERENCES v2_job_queue(id) ON DELETE CASCADE;