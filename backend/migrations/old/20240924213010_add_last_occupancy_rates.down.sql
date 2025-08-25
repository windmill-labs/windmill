-- Add down migration script here
ALTER TABLE worker_ping
DROP COLUMN occupancy_rate_15s,
DROP COLUMN occupancy_rate_5m,
DROP COLUMN occupancy_rate_30m;
