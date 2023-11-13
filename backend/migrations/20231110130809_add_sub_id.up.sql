-- Add up migration script here
alter table workspace_settings add column seats int, add column subscription_id varchar(255);