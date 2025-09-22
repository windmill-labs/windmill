-- Add up migration script here

CREATE TYPE IF NOT EXISTS job_kind AS ENUM (
    'script', 'preview', 'flow', 'dependencies', 'flowpreview', 'script_hub', 
    'identity', 'flowdependencies', 'http', 'graphql', 'postgresql', 'noop', 
    'appdependencies', 'deploymentcallback', 'singlescriptflow', 'flowscript', 
    'flownode', 'appscript'
);


CREATE TYPE IF NOT EXISTS job_status AS ENUM ('success', 'failure', 'canceled', 'skipped');



CREATE TYPE IF NOT EXISTS job_trigger_kind AS ENUM (
    'webhook', 'http', 'websocket', 'kafka', 'email', 'nats', 'schedule', 
    'app', 'ui', 'postgres', 'sqs', 'gcp', 'mqtt'
);

CREATE TYPE IF NOT EXISTS script_lang AS ENUM (
    'python3', 'deno', 'go', 'bash', 'postgresql', 'nativets', 'bun', 'mysql', 
    'bigquery', 'snowflake', 'graphql', 'powershell', 'mssql', 'php', 'bunnative', 
    'rust', 'ansible', 'csharp', 'oracledb', 'nu', 'java', 'duckdb'
);


CREATE TABLE IF NOT EXISTS v2_job (
    id UUID PRIMARY KEY,
    raw_code TEXT,
    raw_lock TEXT,
    raw_flow JSONB,
    tag VARCHAR(255),
    workspace_id VARCHAR(100) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_by VARCHAR(100) NOT NULL,
    permissioned_as VARCHAR(100) NOT NULL,
    permissioned_as_email VARCHAR(500),
    kind job_kind NOT NULL,
    runnable_id BIGINT,
    runnable_path VARCHAR(500),
    parent_job UUID,
    root_job UUID,
    script_lang script_lang,
    script_entrypoint_override VARCHAR(500),
    flow_step INTEGER,
    flow_step_id VARCHAR(100),
    flow_innermost_root_job UUID,
    trigger VARCHAR(500),
    trigger_kind job_trigger_kind,
    same_worker BOOLEAN NOT NULL DEFAULT FALSE,
    visible_to_owner BOOLEAN NOT NULL DEFAULT TRUE,
    concurrent_limit INTEGER,
    concurrency_time_window_s INTEGER,
    cache_ttl INTEGER,
    timeout INTEGER,
    priority SMALLINT,
    preprocessed BOOLEAN NOT NULL DEFAULT FALSE,
    args JSONB,
    labels TEXT[],
    pre_run_error TEXT
);

-- Create indices for v2_job
CREATE INDEX IF NOT EXISTS ix_job_created_at ON v2_job (created_at DESC);
CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_3 ON v2_job (workspace_id, created_at DESC);
CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_5 ON v2_job (workspace_id, created_at DESC) 
    WHERE ((kind = ANY (ARRAY['preview'::job_kind, 'flowpreview'::job_kind])) 
           AND (parent_job IS NULL));
CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_8 ON v2_job (workspace_id, created_at DESC) 
    WHERE ((kind = 'deploymentcallback'::job_kind) AND (parent_job IS NULL));
CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_9 ON v2_job (workspace_id, created_at DESC) 
    WHERE ((kind = ANY (ARRAY['dependencies'::job_kind, 'flowdependencies'::job_kind, 'appdependencies'::job_kind])) 
           AND (parent_job IS NULL));
CREATE INDEX IF NOT EXISTS ix_v2_job_labels ON v2_job USING gin (labels) WHERE (labels IS NOT NULL);
CREATE INDEX IF NOT EXISTS ix_v2_job_workspace_id_created_at ON v2_job (workspace_id, created_at DESC) 
    WHERE ((kind = ANY (ARRAY['script'::job_kind, 'flow'::job_kind, 'singlescriptflow'::job_kind])) 
           AND (parent_job IS NULL));
CREATE INDEX IF NOT EXISTS ix_job_root_job_index_by_path_2 ON v2_job (workspace_id, runnable_path, created_at DESC) 
    WHERE (parent_job IS NULL);