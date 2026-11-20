-- Worker groups whose autoscaling decisions are being recorded, until `until`.
CREATE TABLE IF NOT EXISTS autoscaling_recording (
    worker_group TEXT PRIMARY KEY,
    until TIMESTAMPTZ NOT NULL,
    started_by TEXT NOT NULL
);

-- One row per autoscaling tick of a recorded worker group: what it saw and what it decided.
CREATE TABLE IF NOT EXISTS autoscaling_decision (
    id BIGSERIAL PRIMARY KEY,
    worker_group TEXT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    worker_count INTEGER NOT NULL,
    occupancy_rate_15s REAL,
    occupancy_rate_5m REAL,
    occupancy_rate_30m REAL,
    jobs_waiting JSONB NOT NULL,
    decision TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS autoscaling_decision_worker_group_idx
    ON autoscaling_decision (worker_group, recorded_at);
