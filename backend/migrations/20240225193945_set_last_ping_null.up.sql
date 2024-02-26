-- Add up migration script here
UPDATE queue SET last_ping = NULL WHERE job_kind = 'flow' OR job_kind = 'flowpreview';