-- Store timeseries timestamps as a start time + integer millisecond offsets
-- instead of full TIMESTAMPTZ[] arrays. Saves ~4 bytes per data point.
ALTER TABLE job_stats ADD COLUMN IF NOT EXISTS timeseries_start TIMESTAMPTZ;
ALTER TABLE job_stats ADD COLUMN IF NOT EXISTS offsets_ms INTEGER[];
