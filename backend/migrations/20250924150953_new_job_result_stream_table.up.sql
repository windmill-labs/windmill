-- Add up migration script here
CREATE TABLE job_result_stream_v2 (
    job_id UUID NOT NULL,
    workspace_id TEXT NOT NULL,
    stream TEXT NOT NULL,
    idx INT NOT NULL,
    PRIMARY KEY (job_id, idx),
    FOREIGN KEY (job_id) REFERENCES v2_job_queue(id) ON DELETE CASCADE
);

GRANT ALL ON TABLE job_result_stream_v2 TO windmill_admin;
GRANT ALL ON TABLE job_result_stream_v2 TO windmill_user;

