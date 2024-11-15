-- Add up migration script here
DO $$
DECLARE
  t TEXT;
BEGIN
  FOR t IN VALUES ('queue'), ('completed_job') LOOP
    EXECUTE format(
      'CREATE OR REPLACE VIEW '||t||'_view AS
      SELECT %s, job_logs.log_offset FROM '||t||'
      LEFT JOIN job ON '||t||'.id = job.id AND '||t||'.workspace_id = job.workspace_id
      LEFT JOIN job_logs ON '||t||'.id = job_logs.job_id', (
        SELECT string_agg(
          CASE
            WHEN column_name = 'logs' THEN
              -- Concatenate logs from base and job_logs.
              'concat(coalesce('||t||'.logs, ''''), coalesce(job_logs.logs, '''')) as logs'
            WHEN column_name = 'raw_code' OR column_name = 'raw_lock' THEN
              -- Coalesce column from base and job.
              format('coalesce('||t||'.%s, job.%s) as %s', column_name, column_name, column_name)
            WHEN column_name = 'raw_flow' THEN
              -- When job_kind is 'flow', get raw_flow from flow table, otherwise get raw_flow from base or job.
              'CASE WHEN '||t||'.job_kind = ''flow'' THEN (SELECT value FROM flow WHERE flow.path = '||t||'.script_path AND flow.workspace_id = '||t||'.workspace_id) ELSE coalesce('||t||'.raw_flow, job.raw_flow) END as raw_flow'
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
