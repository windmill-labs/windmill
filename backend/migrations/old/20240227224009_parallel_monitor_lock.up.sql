-- Add up migration script here
CREATE TABLE parallel_monitor_lock (
    parent_flow_id uuid NOT NULL,
    job_id uuid NOT NULL,
    last_ping TIMESTAMP WITH TIME ZONE,
    PRIMARY KEY (parent_flow_id, job_id)
);
