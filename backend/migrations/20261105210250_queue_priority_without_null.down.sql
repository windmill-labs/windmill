UPDATE v2_job_queue
SET priority = CASE WHEN priority = 0 THEN NULL ELSE 0 END
WHERE priority = 0 OR priority = 1;
