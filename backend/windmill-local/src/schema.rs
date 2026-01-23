//! SQLite schema for local mode
//!
//! This is a minimal schema supporting preview job execution.
//! Key differences from PostgreSQL:
//! - ENUMs are TEXT with CHECK constraints
//! - JSONB is JSON (stored as TEXT in SQLite)
//! - Arrays are JSON arrays
//! - No FOR UPDATE SKIP LOCKED (single worker, in-process coordination)

/// SQL to create the minimal schema for local mode preview execution
pub const SCHEMA: &str = r#"
-- Job kinds (equivalent to PostgreSQL ENUM)
-- Values: script, preview, flow, flowpreview, dependencies, flowdependencies,
--         script_hub, identity, http, graphql, postgresql, noop, appdependencies,
--         deploymentcallback, singlescriptflow, flowscript, flownode, appscript

-- Job status (equivalent to PostgreSQL ENUM)
-- Values: success, failure, canceled, skipped

-- Script languages (equivalent to PostgreSQL ENUM)
-- Values: python3, deno, go, bash, postgresql, nativets, bun, mysql, bigquery,
--         snowflake, graphql, powershell, mssql, php, bunnative, rust, ansible,
--         csharp, oracledb, nu, java, duckdb

-- Main job table (minimal for preview)
CREATE TABLE IF NOT EXISTS v2_job (
    id TEXT PRIMARY KEY,  -- UUID as TEXT
    workspace_id TEXT NOT NULL DEFAULT 'local',

    -- Raw code for preview jobs
    raw_code TEXT,
    raw_lock TEXT,
    raw_flow TEXT,  -- JSON for flow definitions

    -- Job metadata
    tag TEXT DEFAULT 'deno',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    created_by TEXT NOT NULL DEFAULT 'local_user',

    -- Permission context (simplified for local mode)
    permissioned_as TEXT NOT NULL DEFAULT 'u/local_user',
    permissioned_as_email TEXT DEFAULT 'local@windmill.local',

    -- Job type info
    kind TEXT NOT NULL DEFAULT 'preview' CHECK (kind IN (
        'script', 'preview', 'flow', 'flowpreview', 'dependencies',
        'flowdependencies', 'script_hub', 'identity', 'http', 'graphql',
        'postgresql', 'noop', 'appdependencies', 'deploymentcallback',
        'singlescriptflow', 'flowscript', 'flownode', 'appscript'
    )),

    -- Script execution details
    script_lang TEXT CHECK (script_lang IN (
        'python3', 'deno', 'go', 'bash', 'postgresql', 'nativets', 'bun',
        'mysql', 'bigquery', 'snowflake', 'graphql', 'powershell', 'mssql',
        'php', 'bunnative', 'rust', 'ansible', 'csharp', 'oracledb', 'nu',
        'java', 'duckdb'
    )),

    -- Flow execution details
    parent_job TEXT,  -- UUID reference
    root_job TEXT,    -- UUID reference
    flow_step INTEGER,
    flow_step_id TEXT,
    flow_innermost_root_job TEXT,

    -- Execution settings
    timeout INTEGER,
    priority INTEGER DEFAULT 0,
    same_worker INTEGER DEFAULT 0,  -- BOOLEAN as INTEGER
    visible_to_owner INTEGER DEFAULT 1,

    -- Arguments (JSON)
    args TEXT,  -- JSON object

    -- Pre-run error if validation failed
    pre_run_error TEXT
);

-- Job queue table
CREATE TABLE IF NOT EXISTS v2_job_queue (
    id TEXT PRIMARY KEY,  -- UUID, references v2_job.id
    workspace_id TEXT NOT NULL DEFAULT 'local',

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    started_at TEXT,
    scheduled_for TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    -- Queue state
    running INTEGER NOT NULL DEFAULT 0,  -- BOOLEAN
    canceled_by TEXT,
    canceled_reason TEXT,

    -- Suspend state (for approval flows)
    suspend INTEGER DEFAULT 0,
    suspend_until TEXT,

    -- Execution settings
    tag TEXT DEFAULT 'deno',
    priority INTEGER DEFAULT 0,
    worker TEXT,

    FOREIGN KEY (id) REFERENCES v2_job(id)
);

-- Index for queue ordering (simulates queue_sort_v2)
CREATE INDEX IF NOT EXISTS idx_queue_sort ON v2_job_queue (
    priority DESC, scheduled_for ASC, tag
) WHERE running = 0;

-- Job runtime tracking (heartbeat/ping)
CREATE TABLE IF NOT EXISTS v2_job_runtime (
    id TEXT PRIMARY KEY,  -- UUID, references v2_job.id
    ping TEXT,  -- Timestamp
    memory_peak INTEGER,

    FOREIGN KEY (id) REFERENCES v2_job(id)
);

-- Completed jobs with results
CREATE TABLE IF NOT EXISTS v2_job_completed (
    id TEXT PRIMARY KEY,  -- UUID, references v2_job.id
    workspace_id TEXT NOT NULL DEFAULT 'local',

    -- Timing
    started_at TEXT,
    completed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    duration_ms INTEGER,

    -- Result
    result TEXT,  -- JSON
    result_columns TEXT,  -- JSON array of column names

    -- Status
    status TEXT NOT NULL DEFAULT 'success' CHECK (status IN (
        'success', 'failure', 'canceled', 'skipped'
    )),

    -- Cancellation details
    canceled_by TEXT,
    canceled_reason TEXT,

    -- Flow status (for flow jobs)
    flow_status TEXT,  -- JSON

    -- Execution details
    memory_peak INTEGER,
    worker TEXT,
    deleted INTEGER DEFAULT 0,  -- BOOLEAN

    FOREIGN KEY (id) REFERENCES v2_job(id)
);

-- Index for completed job lookup by workspace and time
CREATE INDEX IF NOT EXISTS idx_completed_workspace_time ON v2_job_completed (
    workspace_id, completed_at DESC
);

-- Flow status tracking (separate from completed to allow updates during execution)
CREATE TABLE IF NOT EXISTS v2_job_status (
    id TEXT PRIMARY KEY,  -- UUID, references v2_job.id
    flow_status TEXT,  -- JSON object tracking flow module execution
    flow_leaf_jobs TEXT,  -- JSON object
    workflow_as_code_status TEXT,  -- JSON object

    FOREIGN KEY (id) REFERENCES v2_job(id)
);

-- Simplified job permissions (for local mode, mostly unused)
CREATE TABLE IF NOT EXISTS job_perms (
    job_id TEXT PRIMARY KEY,
    email TEXT DEFAULT 'local@windmill.local',
    username TEXT DEFAULT 'local_user',
    is_admin INTEGER DEFAULT 1,  -- BOOLEAN
    is_operator INTEGER DEFAULT 0,  -- BOOLEAN
    workspace_id TEXT DEFAULT 'local',
    groups TEXT,  -- JSON array
    folders TEXT,  -- JSON array of objects

    FOREIGN KEY (job_id) REFERENCES v2_job(id)
);

-- Simple audit log (optional for local mode)
CREATE TABLE IF NOT EXISTS audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT DEFAULT 'local',
    timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    username TEXT DEFAULT 'local_user',
    operation TEXT NOT NULL,
    action_kind TEXT CHECK (action_kind IN ('create', 'update', 'delete', 'execute')),
    resource TEXT,
    parameters TEXT  -- JSON
);

-- Job logs storage
CREATE TABLE IF NOT EXISTS job_logs (
    job_id TEXT PRIMARY KEY,
    workspace_id TEXT DEFAULT 'local',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    logs TEXT,
    log_offset INTEGER DEFAULT 0,

    FOREIGN KEY (job_id) REFERENCES v2_job(id)
);
"#;

/// SQL to drop all tables (for testing/reset)
pub const DROP_SCHEMA: &str = r#"
DROP TABLE IF EXISTS job_logs;
DROP TABLE IF EXISTS audit;
DROP TABLE IF EXISTS job_perms;
DROP TABLE IF EXISTS v2_job_status;
DROP TABLE IF EXISTS v2_job_completed;
DROP TABLE IF EXISTS v2_job_runtime;
DROP TABLE IF EXISTS v2_job_queue;
DROP TABLE IF EXISTS v2_job;
"#;
