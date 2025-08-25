-- Add up migration script here
CREATE TABLE job (
  id UUID PRIMARY KEY,
  raw_code TEXT,
  raw_lock TEXT,
  raw_flow jsonb NULL,
  tag VARCHAR(50),
  workspace_id VARCHAR(50)
);

-- Create `queue_view` and `completed_job_view` views.
DO $$
DECLARE
  t TEXT;
BEGIN
  FOR t IN VALUES ('queue'), ('completed_job') LOOP
    EXECUTE format(
      'CREATE OR REPLACE VIEW '||t||'_view AS
       SELECT %s, job_logs.log_offset, job_logs.log_file_index FROM '||t||'
       LEFT JOIN job ON '||t||'.id = job.id AND '||t||'.workspace_id = job.workspace_id
       LEFT JOIN job_logs ON '||t||'.id = job_logs.job_id', (
        SELECT string_agg(
          CASE
            WHEN column_name = 'logs' THEN -- Concatenate logs from base and job_logs.
              'concat(coalesce('||t||'.logs, ''''), coalesce(job_logs.logs, '''')) as logs'
            WHEN column_name IN ('raw_code', 'raw_lock', 'raw_flow') THEN -- Coalesce column from base and job.
              format('coalesce('||t||'.%s, job.%s) as %s', column_name, column_name, column_name)
            ELSE
              format('%s.%s', t, column_name)
          END,
          ', '
        )
        FROM information_schema.columns
        WHERE table_name = t
      )
    );
  END LOOP;
END $$;
