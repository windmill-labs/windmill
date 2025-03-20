-- Add up migration script here
ALTER TABLE http_trigger
  RENAME COLUMN requires_auth TO windmill_auth;