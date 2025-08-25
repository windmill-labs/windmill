-- Add up migration script here
CREATE TYPE SCRIPT_LANG AS ENUM ('python3', 'deno');

ALTER TABLE script
ADD COLUMN language SCRIPT_LANG NOT NULL DEFAULT 'python3';

ALTER TABLE queue
ADD COLUMN language SCRIPT_LANG NOT NULL DEFAULT 'python3';

ALTER TABLE completed_job
ADD COLUMN language SCRIPT_LANG NOT NULL DEFAULT 'python3';
