-- Add up migration script here
alter table workspace add column is_overquota boolean not null default false;