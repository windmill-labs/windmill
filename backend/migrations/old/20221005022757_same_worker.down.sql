-- Add down migration script here
ALTER TABLE queue DROP COLUMN same_worker;
