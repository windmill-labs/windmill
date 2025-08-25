-- Add up migration script here
ALTER TABLE script ADD COLUMN envs VARCHAR(1000)[];
