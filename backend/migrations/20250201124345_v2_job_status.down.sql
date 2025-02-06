-- Add down migration script here
-- Lock `queue` in access exclusive to prevent deadlocks when dropping the foreign key to `queue`.
LOCK TABLE queue IN ACCESS EXCLUSIVE MODE;
DROP TABLE v2_job_status CASCADE;
