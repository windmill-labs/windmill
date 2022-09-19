DROP TABLE resume_job;

ALTER TABLE queue
DROP COLUMN suspend,
DROP COLUMN suspend_until;
