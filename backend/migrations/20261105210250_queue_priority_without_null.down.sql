-- Flow steps and dependency jobs at 1 still sort ahead of NULL, so only the jobs without
-- priority are moved back.
UPDATE v2_job_queue SET priority = NULL WHERE priority = 0;
