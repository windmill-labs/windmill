-- Store timeseries timestamps as a start time + integer centisecond offsets
-- instead of full TIMESTAMPTZ[] arrays. Saves ~4 bytes per data point.
-- i32 centiseconds gives ~248 days of range with 10ms precision.
ALTER TABLE job_stats ADD COLUMN IF NOT EXISTS timeseries_start TIMESTAMPTZ;
ALTER TABLE job_stats ADD COLUMN IF NOT EXISTS offsets_cs INTEGER[];
