-- Add up migration script here
ALTER TABLE schedule ADD COLUMN summary VARCHAR(512);
ALTER TABLE schedule ADD COLUMN no_flow_overlap BOOLEAN NOT NULL DEFAULT FALSE;
