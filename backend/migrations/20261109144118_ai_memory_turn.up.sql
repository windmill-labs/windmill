-- The run that holds each memory id: a run started with a memory id is refused while the one
-- holding it is still queued. A row whose job left the queue is free to take over, and the
-- monitor deletes it. No foreign key to v2_job_queue: one would lock that table on every push.
CREATE TABLE IF NOT EXISTS ai_memory_turn (
    workspace_id VARCHAR(50) NOT NULL,
    memory_id UUID NOT NULL,
    job_id UUID NOT NULL,
    -- Background jobs of a parked turn this run took over, by flow step id: that step's agent
    -- adopts them when it starts.
    handed_jobs JSONB,
    PRIMARY KEY (workspace_id, memory_id)
);
