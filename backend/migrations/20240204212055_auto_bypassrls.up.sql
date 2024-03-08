-- Add up migration script here
create table windmill_migrations (name text primary key, created_at timestamp default current_timestamp);