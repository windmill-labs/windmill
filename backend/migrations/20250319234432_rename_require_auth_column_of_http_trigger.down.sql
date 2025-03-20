-- Add down migration script here
ALTER TABLE http_trigger
  RENAME COLUMN windmill_auth TO requires_auth;