-- Add up migration script here
DELETE FROM job
WHERE NOT EXISTS (SELECT 1 FROM completed_job WHERE completed_job.id = job.id)
  AND NOT EXISTS (SELECT 1 FROM queue WHERE queue.id = job.id);
