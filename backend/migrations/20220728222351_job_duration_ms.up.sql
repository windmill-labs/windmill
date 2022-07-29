ALTER TABLE completed_job
     RENAME duration to duration_ms;
UPDATE completed_job
   SET duration_ms = duration_ms * 1000;
