-- Rename singlescriptflow to singlestepflow in job_kind enum
ALTER TYPE job_kind RENAME VALUE 'singlescriptflow' TO 'singlestepflow';
