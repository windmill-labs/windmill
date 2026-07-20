-- Generic product-telemetry accumulator: day-bucketed counters (entity_id = '')
-- and per-entity accumulators (e.g. messages per AI session). Aggregated into
-- the anonymous usage stats payload and pruned after 60 days.
CREATE TABLE feature_usage (
    feature VARCHAR(50) NOT NULL,
    kind VARCHAR(50) NOT NULL,
    key VARCHAR(100) NOT NULL DEFAULT '',
    entity_id VARCHAR(50) NOT NULL DEFAULT '',
    day DATE NOT NULL DEFAULT CURRENT_DATE,
    value BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (feature, kind, key, entity_id, day)
);
