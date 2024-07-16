-- Add up migration script here
alter table schedule add column paused_until timestamptz;