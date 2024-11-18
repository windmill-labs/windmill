-- Add up migration script here
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'inlinescript';

CREATE TABLE inline_script (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    flow VARCHAR(255) NOT NULL,
    lock TEXT,
    path TEXT,
    hash BIGINT NOT NULL,
    content TEXT NOT NULL,
    PRIMARY KEY (flow, hash),
    FOREIGN KEY (flow, workspace_id) REFERENCES flow (path, workspace_id) ON DELETE CASCADE
);
