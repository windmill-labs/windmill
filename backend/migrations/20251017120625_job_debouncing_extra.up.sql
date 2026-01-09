-- Job debouncing feature: consolidate multiple job requests within a time window
-- This reduces redundant work when the same script/flow is triggered multiple times rapidly
-- debounce_key: Custom key template for grouping jobs (e.g., "$workspace/$path-$args[id]")
-- debounce_delay_s: Delay in seconds before job execution to allow consolidation window
ALTER TABLE script ADD COLUMN IF NOT EXISTS debounce_key VARCHAR(255);
ALTER TABLE script ADD COLUMN IF NOT EXISTS debounce_delay_s INTEGER;
