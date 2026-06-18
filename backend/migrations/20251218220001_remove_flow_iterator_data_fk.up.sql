-- Remove foreign key constraint - we'll handle cleanup manually in monitor.rs
ALTER TABLE flow_iterator_data
DROP CONSTRAINT IF EXISTS flow_iterator_data_job_id_fkey;

-- Remove created_at column as it's not needed
ALTER TABLE flow_iterator_data
DROP COLUMN IF EXISTS created_at;
