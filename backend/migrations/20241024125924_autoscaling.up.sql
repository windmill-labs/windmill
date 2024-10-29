-- Add up migration script here
CREATE TYPE AUTOSCALING_EVENT_TYPE AS ENUM ('full_scaleout', 'scalein', 'scaleout');

CREATE TABLE autoscaling_event (
    id SERIAL PRIMARY KEY,
    worker_group TEXT NOT NULL,
    event_type AUTOSCALING_EVENT_TYPE NOT NULL,
    desired_workers INTEGER NOT NULL,
    applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reason TEXT
);

CREATE INDEX autoscaling_event_worker_group_idx ON autoscaling_event (worker_group, applied_at);