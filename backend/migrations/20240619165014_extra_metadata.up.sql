-- Add up migration script here
alter table resource add column edited_at timestamptz, add column created_by varchar(50);
alter table resource_type add column edited_at timestamptz, add column created_by varchar(50);
alter table folder add column summary text, add column edited_at timestamptz, add column created_by varchar(50);