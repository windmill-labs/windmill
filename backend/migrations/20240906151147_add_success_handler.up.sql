-- Add up migration script here
ALTER TABLE schedule ADD COLUMN on_success VARCHAR(1000), ADD COLUMN on_success_extra_args json;