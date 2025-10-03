-- Revert singlestepflow back to singlescriptflow in job_kind enum
ALTER TYPE job_kind RENAME VALUE 'singlestepflow' TO 'singlescriptflow';
