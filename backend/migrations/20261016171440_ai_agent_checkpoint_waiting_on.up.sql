-- The background tool jobs a parked AI agent waits on: the monitor wakes it once one of them has
-- completed. Empty while it waits on an approval, whose answer comes first.
ALTER TABLE ai_agent_checkpoint ADD COLUMN IF NOT EXISTS waiting_on UUID[] NOT NULL DEFAULT '{}';
