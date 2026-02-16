CREATE TABLE IF NOT EXISTS jobs_pending_deletion (
    id UUID PRIMARY KEY,
    marked_by VARCHAR(10) NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_jobs_pending_deletion_marked_by
    ON jobs_pending_deletion (marked_by);

-- Aggressive autovacuum: this table churns heavily (bulk insert + delete each cycle).
-- Vacuum after every batch clear to prevent dead tuple bloat.
ALTER TABLE jobs_pending_deletion SET (
    autovacuum_vacuum_scale_factor = 0,
    autovacuum_vacuum_threshold = 50,
    autovacuum_analyze_scale_factor = 0,
    autovacuum_analyze_threshold = 50
);
