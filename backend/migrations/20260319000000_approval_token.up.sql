CREATE TABLE IF NOT EXISTS approval_token (
    token TEXT PRIMARY KEY,
    job_id UUID NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspace(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_approval_token_job_id ON approval_token(job_id);
