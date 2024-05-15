-- Add up migration script here
ALTER TABLE usr DROP CONSTRAINT proper_email;
ALTER TABLE workspace_invite DROP CONSTRAINT proper_email;