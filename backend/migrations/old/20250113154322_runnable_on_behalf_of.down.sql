-- Add down migration script here
alter table script drop column on_behalf_of_email;
alter table flow drop column on_behalf_of_email;
