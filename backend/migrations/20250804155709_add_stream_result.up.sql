-- Add up migration script here
CREATE TABLE job_result_stream (
    workspace_id TEXT NOT NULL,
    job_id UUID NOT NULL,
    stream TEXT NOT NULL,
    PRIMARY KEY (workspace_id, job_id)
);

ALTER TABLE job_result_stream ADD CONSTRAINT fk_job_result_stream_job_id FOREIGN KEY (job_id) REFERENCES v2_job_queue(id) ON DELETE CASCADE;

GRANT ALL ON TABLE job_result_stream TO windmill_admin;
GRANT ALL ON TABLE job_result_stream TO windmill_user;