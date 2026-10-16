-- The conversation of an AI agent step parked on its approval tool, read back when the job is
-- pulled again. No foreign key to v2_job_queue: one would lock that table on every write here.
-- The worker deletes its row when the agent ends; the monitor deletes rows whose job left the queue.
CREATE TABLE IF NOT EXISTS ai_agent_checkpoint (
    job_id UUID PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    checkpoint JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
