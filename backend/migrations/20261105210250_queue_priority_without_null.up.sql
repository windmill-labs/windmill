-- Workers pull `ORDER BY priority DESC NULLS LAST`. A job without priority is pushed as 0 and
-- flow steps and dependency jobs as 1, so that a negative (low) priority sorts after the
-- former. Moves the jobs already queued to those values so they keep their place; a NULL left
-- behind would sort after every low-priority job.
UPDATE v2_job_queue
SET priority = CASE WHEN priority IS NULL THEN 0 ELSE 1 END
WHERE priority IS NULL OR priority = 0;
