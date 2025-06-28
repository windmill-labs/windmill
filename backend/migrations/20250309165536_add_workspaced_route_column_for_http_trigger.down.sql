-- Add down migration script here
ALTER TABLE http_trigger
DROP COLUMN workspaced_route;