-- Add up migration script here
create table pending_user (
  email varchar(255) not null primary key,
	created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
  username varchar(50) not null
);