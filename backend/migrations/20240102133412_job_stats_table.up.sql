-- Add up migration script here
CREATE TYPE METRIC_KIND AS ENUM ('scalar_int', 'scalar_float', 'timeseries_int', 'timeseries_float');

CREATE TABLE IF NOT EXISTS job_stats (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
	job_id UUID NOT NULL,
	metric_id VARCHAR(50) NOT NULL,
	metric_name VARCHAR(255),
	metric_kind METRIC_KIND NOT NULL,
	scalar_int INTEGER,
	scalar_float REAL,
	timestamps TIMESTAMP WITH TIME ZONE[],
	timeseries_int INTEGER[],
	timeseries_float REAL[],
    PRIMARY KEY (workspace_id, job_id, metric_id)
);
