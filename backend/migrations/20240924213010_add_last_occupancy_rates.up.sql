-- Add up migration script here
ALTER TABLE worker_ping
ADD COLUMN occupancy_rate_15s REAL,
ADD COLUMN occupancy_rate_5m REAL,
ADD COLUMN occupancy_rate_30m REAL;
