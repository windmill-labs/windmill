-- Add down migration script here
ALTER TABLE IF exists config RENAME TO worker_group_config ;
