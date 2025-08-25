-- Add up migration script here
CREATE TABLE workspace_env (
  workspace_id varchar(50) not null,
  name varchar(255) not null,
  value varchar(1000) not null,
  primary key (workspace_id, name)
)