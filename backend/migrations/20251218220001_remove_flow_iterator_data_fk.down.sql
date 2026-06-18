-- Re-add created_at column
ALTER TABLE flow_iterator_data
ADD COLUMN created_at TIMESTAMP WITH TIME ZONE DEFAULT now();

-- Re-add foreign key constraint
ALTER TABLE flow_iterator_data
ADD CONSTRAINT flow_iterator_data_job_id_fkey
FOREIGN KEY (job_id) REFERENCES v2_job_queue (id) ON DELETE CASCADE;
