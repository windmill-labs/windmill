-- Create separate table for storing flow iterator data (itered arrays)
-- This avoids expensive JSONB_SET operations on large itered arrays during parallel loop execution
CREATE TABLE IF NOT EXISTS flow_iterator_data (
    job_id      UUID PRIMARY KEY REFERENCES v2_job_queue (id) ON DELETE CASCADE NOT NULL,
    itered      JSONB NOT NULL,
    created_at  TIMESTAMP WITH TIME ZONE DEFAULT now()
);

-- Index not needed beyond primary key since all lookups are by job_id
