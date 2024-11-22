-- Add up migration script here
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'flowscript';

-- Same as `flow_version` but with a "lite" value (raw scripts replaced by `flow_script`).
CREATE TABLE flow_version_lite (
    id BIGSERIAL PRIMARY KEY,
    value JSONB,
    FOREIGN KEY (id) REFERENCES flow_version (id) ON DELETE CASCADE
);

-- Either a script or a flow.
CREATE TABLE flow_node (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    hash BIGINT NOT NULL,
    path VARCHAR(255) NOT NULL, -- flow path.
    lock TEXT,
    code TEXT,
    flow JSONB,
    FOREIGN KEY (path, workspace_id) REFERENCES flow (path, workspace_id) ON DELETE CASCADE
);

CREATE INDEX flow_node_hash ON flow_node (hash);

-- Load `raw_flow` from `flow` when job is a flow.
DO $$
DECLARE
  t TEXT;
BEGIN
  FOR t IN VALUES ('queue'), ('completed_job') LOOP
    EXECUTE format(
      'CREATE OR REPLACE VIEW '||t||'_view AS
       SELECT %s, job_logs.log_offset, job_logs.log_file_index
       FROM '||t||'
       LEFT JOIN job ON '||t||'.id = job.id AND '||t||'.workspace_id = job.workspace_id
       LEFT JOIN job_logs ON '||t||'.id = job_logs.job_id', (
        SELECT string_agg(
          CASE
            WHEN column_name = 'logs' THEN -- Concatenate logs from base and job_logs.
              'concat(coalesce('||t||'.logs, ''''), coalesce(job_logs.logs, '''')) as logs'
            WHEN column_name IN ('raw_code', 'raw_lock') THEN -- Coalesce column from base and job.
              format('coalesce('||t||'.%s, job.%s) as %s', column_name, column_name, column_name)
            WHEN column_name = 'raw_flow' THEN -- When job_kind is 'flow', get raw_flow from flow table, otherwise get raw_flow from base or job.
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
